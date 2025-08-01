use crate::tests::helpers::{THROTTLE_RETRY_DELAY, login_with_retries};

use super::{Error, TestContext, config::TEST_CONFIG, helpers, ui};
use anyhow::Context;
use mullvad_api::DevicesProxy;
use mullvad_management_interface::{MullvadProxyClient, client::DaemonEvent};
use mullvad_types::{
    device::{Device, DeviceState},
    states::TunnelState,
};
use std::time::Duration;
use talpid_types::net::{wireguard, wireguard::PublicKey};
use test_macro::test_function;
use test_rpc::ServiceClient;

/// Log out and remove the current device
/// from the account.
#[test_function(priority = 100)]
pub async fn test_logout(
    _: TestContext,
    _rpc: ServiceClient,
    mut mullvad_client: MullvadProxyClient,
) -> Result<(), Error> {
    log::info!("Removing device");

    mullvad_client
        .logout_account()
        .await
        .expect("logout failed");

    Ok(())
}

/// Try to log in when there are too many devices. Make sure it fails as expected.
#[test_function(priority = -151)]
pub async fn test_too_many_devices(
    _: TestContext,
    rpc: ServiceClient,
    mut mullvad_client: MullvadProxyClient,
) -> anyhow::Result<()> {
    log::info!("Using up all devices");

    let device_client = new_device_client().await.context("Create device client")?;

    const MAX_ATTEMPTS: usize = 15;

    let fill_devices = || async {
        for _ in 0..MAX_ATTEMPTS {
            let pubkey = wireguard::PrivateKey::new_from_random().public_key();

            match device_client
                .create(TEST_CONFIG.account_number.clone(), pubkey)
                .await
            {
                Ok(_) => (),
                Err(mullvad_api::rest::Error::ApiError(_status, ref code))
                    if code == mullvad_api::MAX_DEVICES_REACHED =>
                {
                    break;
                }
                Err(error) => {
                    log::error!(
                        "Failed to generate device: {error:?}. Retrying after {} seconds",
                        THROTTLE_RETRY_DELAY.as_secs()
                    );
                    // Sleep for an overly long time.
                    // TODO: Only sleep for this long if the error is caused by throttling.
                    tokio::time::sleep(THROTTLE_RETRY_DELAY).await;
                }
            }
        }
    };
    fill_devices().await;

    log::info!("Log in with too many devices");
    let login_result = login_with_retries(&mut mullvad_client).await;

    assert!(
        matches!(
            login_result,
            Err(mullvad_management_interface::Error::TooManyDevices)
        ),
        "Expected too many devices error, got {login_result:?}"
    );

    mullvad_client.logout_account().await?;
    fill_devices().await;

    // Run UI test
    let ui_result = ui::run_test_env(
        &rpc,
        &["too-many-devices.spec"],
        [("ACCOUNT_NUMBER", &*TEST_CONFIG.account_number)],
    )
    .await
    .unwrap();

    if let Err(error) = clear_devices(&device_client).await {
        log::error!("Failed to clear devices: {error}");
    }

    assert!(ui_result.success());

    Ok(())
}

/// Test whether the daemon can detect that the current device has been revoked, and enters the
/// error state in that case.
///
/// # Limitations
///
/// Currently, this test does not check whether the daemon automatically detects that the device has
/// been revoked while reconnecting.
#[test_function(priority = -150)]
pub async fn test_revoked_device(
    _: TestContext,
    rpc: ServiceClient,
    mut mullvad_client: MullvadProxyClient,
) -> anyhow::Result<()> {
    mullvad_client.connect_tunnel().await?;

    let device_id = mullvad_client
        .get_device()
        .await
        .context("failed to get device data")?
        .logged_in()
        .context("Client is not logged in to a valid account")?
        .device
        .id;

    log::debug!("Removing current device");

    let device_client = new_device_client()
        .await
        .context("Failed to create device client")?;
    retry_if_throttled(|| {
        device_client.remove(TEST_CONFIG.account_number.clone(), device_id.clone())
    })
    .await
    .expect("failed to revoke device");

    // Sleep for a while: the device state is only updated if sufficiently old,
    // so `update_device` might be a no-op if called too often.
    const PRE_UPDATE_SLEEP: Duration = Duration::from_secs(12);
    tokio::time::sleep(PRE_UPDATE_SLEEP).await;

    // Begin listening to tunnel state changes first, so that we catch changes due to
    // `update_device`.
    let events = mullvad_client
        .events_listen()
        .await
        .context("Failed to begin listening for state changes")?;
    let next_state =
        helpers::find_next_tunnel_state(events, |state| matches!(state, TunnelState::Error(..),));

    log::debug!("Update device state");

    // Update the device status, which performs a device validation.
    let _ = mullvad_client.update_device().await;

    // Ensure that the tunnel state transitions to "error". Fail if it transitions to some other
    // state.
    let new_state = next_state.await?;
    assert!(
        matches!(&new_state, TunnelState::Error(error_state) if error_state.is_blocking()),
        "expected blocking error state, got {new_state:?}"
    );

    // Verify that the device state is `Revoked`.
    let device_state = mullvad_client
        .get_device()
        .await
        .context("Failed to get device data")?;
    assert!(
        matches!(device_state, DeviceState::Revoked),
        "expected device to be revoked"
    );

    // Run UI test
    let ui_result = ui::run_test(&rpc, &["device-revoked.spec"]).await.unwrap();
    assert!(ui_result.success());

    Ok(())
}

/// Assert that an old Wireguard key is automatically rotated by the daemon.
#[test_function]
pub async fn test_automatic_wireguard_rotation(
    ctx: TestContext,
    rpc: ServiceClient,
    mut mullvad_client: MullvadProxyClient,
) -> anyhow::Result<()> {
    const ROTATION_TIMEOUT: Duration = Duration::from_secs(120);

    // Make note of current WG key
    let old_key = get_current_wireguard_key(&mut mullvad_client).await?;

    log::info!("Old wireguard key: {old_key}");

    rpc.stop_mullvad_daemon().await?;

    log::info!("Changing created field of `device.json` to more than 7 days ago");
    rpc.make_device_json_old()
        .await
        .context("Could not change device.json to have an old created timestamp")?;

    rpc.start_mullvad_daemon().await?;

    // NOTE: Need to create a new `mullvad_client` here after the restart otherwise we can't
    // communicate with the daemon
    log::info!("Reconnecting to daemon");
    drop(mullvad_client);
    let mut mullvad_client = ctx.rpc_provider.new_client().await;

    // Check if the key rotation has already occurred when connected to the daemon, otherwise
    // listen for device daemon events until we observe the change. We have to register the event
    // listener before polling the current key to be sure we don't miss the change.
    log::info!("Verifying that wireguard key has changed");
    let event_listener = mullvad_client
        .events_listen()
        .await
        .context("Failed to begin listening for state changes")?;
    let new_key = get_current_wireguard_key(&mut mullvad_client).await?;

    // If key has not yet been updated, listen for changes to it
    if new_key == old_key {
        // Verify rotation has happened within `ROTATION_TIMEOUT` - if the key hasn't been rotated
        // after that, the rotation probably won't happen anytime soon.
        log::info!("Listening for device daemon event");
        let device_event = |daemon_event| match daemon_event {
            DaemonEvent::Device(device_event) => Some(device_event),
            _ => None,
        };
        let device_event_listener = tokio::time::timeout(
            ROTATION_TIMEOUT,
            helpers::find_daemon_event(event_listener, device_event),
        );
        let _ = device_event_listener.await;

        // Note: The key rotation could possible have happened without us noticing due to
        // some raceiness in the timeframe between starting the daemon and us starting to
        // listen for new daemon events. Thus, it is probably a good idea to check manually if the
        // device key was rotated.
        let new_key = get_current_wireguard_key(&mut mullvad_client).await?;

        assert_ne!(old_key, new_key);
    }

    Ok(())
}

async fn get_current_wireguard_key(
    mullvad_client: &mut MullvadProxyClient,
) -> anyhow::Result<PublicKey> {
    let pubkey = mullvad_client
        .get_device()
        .await?
        .logged_in()
        .context("Client is not logged in to a valid account")?
        .device
        .pubkey;
    Ok(pubkey)
}

/// Remove all devices on the current account
pub async fn clear_devices(device_client: &DevicesProxy) -> anyhow::Result<()> {
    log::debug!("Removing all devices for account");

    for dev in list_devices_with_retries(device_client).await?.into_iter() {
        if let Err(error) = device_client
            .remove(TEST_CONFIG.account_number.clone(), dev.id)
            .await
        {
            log::warn!("Failed to remove device: {error}");
        }
    }
    Ok(())
}

pub async fn new_device_client() -> anyhow::Result<DevicesProxy> {
    use mullvad_api::{ApiEndpoint, proxy::ApiConnectionMode};

    let api_host = format!("api.{}", TEST_CONFIG.mullvad_host);

    let api_host_with_port = format!("{api_host}:443");
    let api_address = helpers::resolve_hostname_with_retries(api_host_with_port)
        .await
        .context("failed to resolve API host")?;

    // Override the API endpoint to use the one specified in the test config
    let endpoint = ApiEndpoint::new(api_host, api_address, false);

    let api = mullvad_api::Runtime::new(tokio::runtime::Handle::current(), &endpoint);

    let rest_handle = api.mullvad_rest_handle(ApiConnectionMode::Direct.into_provider());
    Ok(DevicesProxy::new(rest_handle))
}

pub async fn list_devices_with_retries(
    device_client: &DevicesProxy,
) -> Result<Vec<Device>, mullvad_api::rest::Error> {
    retry_if_throttled(|| device_client.list(TEST_CONFIG.account_number.clone())).await
}

pub async fn retry_if_throttled<
    F: std::future::Future<Output = Result<T, mullvad_api::rest::Error>>,
    T,
>(
    new_attempt: impl Fn() -> F,
) -> Result<T, mullvad_api::rest::Error> {
    loop {
        match new_attempt().await {
            Ok(val) => break Ok(val),
            // Work around throttling errors by sleeping
            Err(mullvad_api::rest::Error::ApiError(
                mullvad_api::rest::StatusCode::TOO_MANY_REQUESTS,
                _,
            )) => {
                log::debug!(
                    "Device list fetch failed due to throttling. Sleeping for {} seconds",
                    THROTTLE_RETRY_DELAY.as_secs()
                );

                tokio::time::sleep(THROTTLE_RETRY_DELAY).await;
            }
            Err(error) => break Err(error),
        }
    }
}

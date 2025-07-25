use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use futures::channel::{mpsc, oneshot};
use futures::future::Fuse;
use futures::{FutureExt, StreamExt};
use talpid_routing::RouteManagerHandle;
use talpid_tunnel::tun_provider::TunProvider;
use talpid_tunnel::{EventHook, TunnelArgs, TunnelEvent, TunnelMetadata};
use talpid_types::ErrorExt;
use talpid_types::net::{AllowedClients, AllowedEndpoint, AllowedTunnelTraffic, TunnelParameters};
use talpid_types::tunnel::{ErrorStateCause, FirewallPolicyError};

use super::connected_state::TunnelEventsReceiver;
use super::{
    AfterDisconnect, ConnectedState, DisconnectingState, ErrorState, EventConsequence, EventResult,
    SharedTunnelStateValues, TunnelCommand, TunnelCommandReceiver, TunnelState,
    TunnelStateTransition,
};

#[cfg(target_os = "macos")]
use crate::dns::DnsConfig;
use crate::firewall::FirewallPolicy;
#[cfg(target_os = "macos")]
use crate::resolver::LOCAL_DNS_RESOLVER;
use crate::tunnel::{self, TunnelMonitor};

pub(crate) type TunnelCloseEvent = Fuse<oneshot::Receiver<Option<ErrorStateCause>>>;

#[cfg(target_os = "android")]
const MAX_ATTEMPTS_WITH_SAME_TUN: u32 = 5;
const MIN_TUNNEL_ALIVE_TIME: Duration = Duration::from_millis(1000);
#[cfg(target_os = "windows")]
const MAX_ATTEMPT_CREATE_TUN: u32 = 4;

const INITIAL_ALLOWED_TUNNEL_TRAFFIC: AllowedTunnelTraffic = AllowedTunnelTraffic::None;

/// The tunnel has been started, but it is not established/functional.
pub struct ConnectingState {
    tunnel_events: TunnelEventsReceiver,
    tunnel_parameters: TunnelParameters,
    tunnel_metadata: Option<TunnelMetadata>,
    allowed_tunnel_traffic: AllowedTunnelTraffic,
    tunnel_close_event: TunnelCloseEvent,
    tunnel_close_tx: oneshot::Sender<()>,
    retry_attempt: u32,
}

impl ConnectingState {
    pub(super) fn enter(
        shared_values: &mut SharedTunnelStateValues,
        retry_attempt: u32,
    ) -> (Box<dyn TunnelState>, TunnelStateTransition) {
        #[cfg(target_os = "macos")]
        if *LOCAL_DNS_RESOLVER {
            // Set system DNS to our local DNS resolver
            let system_dns = DnsConfig::default().resolve(
                &[shared_values.filtering_resolver.listening_addr().ip()],
                shared_values.filtering_resolver.listening_addr().port(),
            );
            let _ = shared_values
                .dns_monitor
                .set("lo", system_dns)
                .inspect_err(|err| {
                    log::error!(
                        "{}",
                        err.display_chain_with_msg(
                            "Failed to configure system to use filtering resolver"
                        )
                    );
                });
        }

        let ip_availability = match shared_values.connectivity.availability() {
            Some(ip_availability) => ip_availability,
            // If we're offline, enter the offline state
            None => {
                // FIXME: Temporary: Nudge route manager to update the default interface
                #[cfg(target_os = "macos")]
                {
                    log::debug!("Poking route manager to update default routes");
                    let _ = shared_values.route_manager.refresh_routes();
                }
                return ErrorState::enter(shared_values, ErrorStateCause::IsOffline);
            }
        };

        match shared_values.runtime.block_on(
            shared_values
                .tunnel_parameters_generator
                .generate(retry_attempt, ip_availability),
        ) {
            Err(err) => {
                ErrorState::enter(shared_values, ErrorStateCause::TunnelParameterError(err))
            }
            Ok(tunnel_parameters) => {
                #[cfg(windows)]
                if let Err(error) = shared_values.split_tunnel.set_tunnel_addresses(None) {
                    log::error!(
                        "{}",
                        error.display_chain_with_msg(
                            "Failed to reset addresses in split tunnel driver"
                        )
                    );

                    return ErrorState::enter(shared_values, ErrorStateCause::SplitTunnelError);
                }

                if let Err(error) = Self::set_firewall_policy(
                    shared_values,
                    &tunnel_parameters,
                    &None,
                    AllowedTunnelTraffic::None,
                ) {
                    ErrorState::enter(
                        shared_values,
                        ErrorStateCause::SetFirewallPolicyError(error),
                    )
                } else {
                    // HACK: On Android, DNS is part of creating the VPN interface, this call
                    // ensures that the vpn_config is prepared with correct DNS servers in case they
                    // previously set to something else, e.g. in the case of blocking. This call
                    // should probably be part of start_tunnel call.
                    #[cfg(target_os = "android")]
                    {
                        shared_values.prepare_tun_config(false);
                        if retry_attempt > 0 && retry_attempt % MAX_ATTEMPTS_WITH_SAME_TUN == 0 {
                            if let Err(error) =
                                { shared_values.tun_provider.lock().unwrap().open_tun_forced() }
                            {
                                log::error!(
                                    "{}",
                                    error.display_chain_with_msg("Failed to recreate tun device")
                                );
                            }
                        }
                    }

                    let connecting_state = Self::start_tunnel(
                        shared_values.runtime.clone(),
                        tunnel_parameters,
                        &shared_values.log_dir,
                        &shared_values.resource_dir,
                        shared_values.tun_provider.clone(),
                        &shared_values.route_manager,
                        retry_attempt,
                    );

                    let params = connecting_state.tunnel_parameters.clone();
                    (
                        Box::new(connecting_state),
                        TunnelStateTransition::Connecting(params.get_tunnel_endpoint()),
                    )
                }
            }
        }
    }

    fn set_firewall_policy(
        shared_values: &mut SharedTunnelStateValues,
        params: &TunnelParameters,
        tunnel_metadata: &Option<TunnelMetadata>,
        allowed_tunnel_traffic: AllowedTunnelTraffic,
    ) -> Result<(), FirewallPolicyError> {
        #[cfg(target_os = "linux")]
        shared_values.disable_connectivity_check();

        let endpoint = params.get_next_hop_endpoint();

        #[cfg(target_os = "windows")]
        let clients = AllowedClients::from(
            TunnelMonitor::get_relay_client(&shared_values.resource_dir, params)
                .into_iter()
                .collect::<Vec<_>>(),
        );

        #[cfg(not(target_os = "windows"))]
        let clients = if params.get_openvpn_local_proxy_settings().is_some() {
            AllowedClients::All
        } else {
            AllowedClients::Root
        };

        let peer_endpoint = AllowedEndpoint { endpoint, clients };

        #[cfg(target_os = "macos")]
        let redirect_interface = shared_values
            .runtime
            .block_on(shared_values.split_tunnel.interface());

        let policy = FirewallPolicy::Connecting {
            peer_endpoint,
            tunnel: tunnel_metadata.clone(),
            allow_lan: shared_values.allow_lan,
            allowed_endpoint: shared_values.allowed_endpoint.clone(),
            allowed_tunnel_traffic,
            #[cfg(target_os = "macos")]
            redirect_interface,
        };
        shared_values
            .firewall
            .apply_policy(policy)
            .map_err(|error| {
                log::error!(
                    "{}",
                    error.display_chain_with_msg(
                        "Failed to apply firewall policy for connecting state"
                    )
                );
                match error {
                    #[cfg(windows)]
                    crate::firewall::Error::ApplyingConnectingPolicy(policy_error) => policy_error,
                    _ => FirewallPolicyError::Generic,
                }
            })
    }

    fn start_tunnel(
        runtime: tokio::runtime::Handle,
        parameters: TunnelParameters,
        log_dir: &Option<PathBuf>,
        resource_dir: &Path,
        tun_provider: Arc<Mutex<TunProvider>>,
        route_manager: &RouteManagerHandle,
        retry_attempt: u32,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded();
        let event_hook = EventHook::new(event_tx);

        let route_manager = route_manager.clone();
        let log_dir = log_dir.clone();
        let resource_dir = resource_dir.to_path_buf();

        let (tunnel_close_tx, tunnel_close_rx) = oneshot::channel();
        let (tunnel_close_event_tx, tunnel_close_event_rx) = oneshot::channel();

        let tunnel_parameters = parameters.clone();

        tokio::task::spawn_blocking(move || {
            let start = Instant::now();

            let args = TunnelArgs {
                runtime,
                resource_dir: &resource_dir,
                event_hook,
                tunnel_close_rx,
                tun_provider,
                retry_attempt,
                route_manager,
            };

            let block_reason = match TunnelMonitor::start(&tunnel_parameters, &log_dir, args) {
                Ok(monitor) => {
                    let reason = Self::wait_for_tunnel_monitor(monitor, retry_attempt);
                    log::debug!("Tunnel monitor exited with block reason: {:?}", reason);
                    reason
                }
                Err(error) if should_retry(&error, retry_attempt) => {
                    log::warn!(
                        "{}",
                        error.display_chain_with_msg(
                            "Retrying to connect after failing to start tunnel"
                        )
                    );
                    None
                }
                Err(error) => {
                    log::error!("{}", error.display_chain_with_msg("Failed to start tunnel"));
                    Some(error.into())
                }
            };

            if block_reason.is_none()
                && let Some(remaining_time) = MIN_TUNNEL_ALIVE_TIME.checked_sub(start.elapsed())
            {
                thread::sleep(remaining_time);
            }

            if tunnel_close_event_tx.send(block_reason).is_err() {
                log::warn!("Tunnel state machine stopped before receiving tunnel closed event");
            }

            log::trace!("Tunnel monitor thread exit");
        });

        ConnectingState {
            tunnel_events: event_rx.fuse(),
            tunnel_parameters: parameters,
            tunnel_metadata: None,
            allowed_tunnel_traffic: INITIAL_ALLOWED_TUNNEL_TRAFFIC,
            tunnel_close_event: tunnel_close_event_rx.fuse(),
            tunnel_close_tx,
            retry_attempt,
        }
    }

    fn wait_for_tunnel_monitor(
        tunnel_monitor: TunnelMonitor,
        retry_attempt: u32,
    ) -> Option<ErrorStateCause> {
        match tunnel_monitor.wait() {
            Ok(_) => None,
            Err(error) => match error {
                tunnel::Error::WireguardTunnelMonitoringError(
                    talpid_wireguard::Error::TimeoutError,
                ) => {
                    log::debug!("WireGuard tunnel timed out");
                    None
                }
                error @ tunnel::Error::WireguardTunnelMonitoringError(..)
                    if !should_retry(&error, retry_attempt) =>
                {
                    log::error!(
                        "{}",
                        error.display_chain_with_msg("Tunnel has stopped unexpectedly")
                    );
                    Some(ErrorStateCause::StartTunnelError)
                }
                error => {
                    log::warn!(
                        "{}",
                        error.display_chain_with_msg("Tunnel has stopped unexpectedly")
                    );
                    None
                }
            },
        }
    }

    fn reset_routes(
        #[cfg(target_os = "windows")] shared_values: &SharedTunnelStateValues,
        #[cfg(not(target_os = "windows"))] shared_values: &mut SharedTunnelStateValues,
    ) {
        if let Err(error) = shared_values.route_manager.clear_routes() {
            log::error!("{}", error.display_chain_with_msg("Failed to clear routes"));
        }
        #[cfg(target_os = "linux")]
        if let Err(error) = shared_values
            .runtime
            .block_on(shared_values.route_manager.clear_routing_rules())
        {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to clear routing rules")
            );
        }
    }

    fn disconnect(
        self,
        shared_values: &mut SharedTunnelStateValues,
        after_disconnect: AfterDisconnect,
    ) -> EventConsequence {
        Self::reset_routes(shared_values);

        EventConsequence::NewState(DisconnectingState::enter(
            self.tunnel_close_tx,
            self.tunnel_close_event,
            after_disconnect,
        ))
    }

    #[cfg(not(target_os = "android"))]
    fn reset_firewall(
        self: Box<Self>,
        shared_values: &mut SharedTunnelStateValues,
    ) -> EventConsequence {
        match Self::set_firewall_policy(
            shared_values,
            &self.tunnel_parameters,
            &self.tunnel_metadata,
            self.allowed_tunnel_traffic.clone(),
        ) {
            Ok(()) => EventConsequence::SameState(self),
            Err(error) => self.disconnect(
                shared_values,
                AfterDisconnect::Block(ErrorStateCause::SetFirewallPolicyError(error)),
            ),
        }
    }

    fn handle_commands(
        self: Box<Self>,
        command: Option<TunnelCommand>,
        shared_values: &mut SharedTunnelStateValues,
    ) -> EventConsequence {
        use self::EventConsequence::*;

        match command {
            Some(TunnelCommand::AllowLan(allow_lan, complete_tx)) => {
                let consequence = if shared_values.set_allow_lan(allow_lan) {
                    #[cfg(target_os = "android")]
                    {
                        self.disconnect(shared_values, AfterDisconnect::Reconnect(0))
                    }
                    #[cfg(not(target_os = "android"))]
                    self.reset_firewall(shared_values)
                } else {
                    SameState(self)
                };
                let _ = complete_tx.send(());
                consequence
            }
            #[cfg(not(target_os = "android"))]
            Some(TunnelCommand::AllowEndpoint(endpoint, tx)) => {
                if shared_values.allowed_endpoint != endpoint {
                    shared_values.allowed_endpoint = endpoint;
                    if let Err(error) = Self::set_firewall_policy(
                        shared_values,
                        &self.tunnel_parameters,
                        &self.tunnel_metadata,
                        self.allowed_tunnel_traffic.clone(),
                    ) {
                        let _ = tx.send(());
                        return self.disconnect(
                            shared_values,
                            AfterDisconnect::Block(ErrorStateCause::SetFirewallPolicyError(error)),
                        );
                    }
                }
                let _ = tx.send(());
                SameState(self)
            }
            Some(TunnelCommand::Dns(servers, complete_tx)) => {
                let consequence = if shared_values.set_dns_config(servers) {
                    #[cfg(target_os = "android")]
                    {
                        self.disconnect(shared_values, AfterDisconnect::Reconnect(0))
                    }
                    #[cfg(not(target_os = "android"))]
                    SameState(self)
                } else {
                    SameState(self)
                };

                let _ = complete_tx.send(());
                consequence
            }
            #[cfg(not(target_os = "android"))]
            Some(TunnelCommand::BlockWhenDisconnected(block_when_disconnected, complete_tx)) => {
                shared_values.block_when_disconnected = block_when_disconnected;
                let _ = complete_tx.send(());
                SameState(self)
            }
            Some(TunnelCommand::Connectivity(connectivity)) => {
                shared_values.connectivity = connectivity;
                if connectivity.is_offline() {
                    self.disconnect(
                        shared_values,
                        AfterDisconnect::Block(ErrorStateCause::IsOffline),
                    )
                } else {
                    SameState(self)
                }
            }
            Some(TunnelCommand::Connect) => {
                self.disconnect(shared_values, AfterDisconnect::Reconnect(0))
            }
            Some(TunnelCommand::Disconnect) | None => {
                self.disconnect(shared_values, AfterDisconnect::Nothing)
            }
            Some(TunnelCommand::Block(reason)) => {
                self.disconnect(shared_values, AfterDisconnect::Block(reason))
            }
            #[cfg(target_os = "android")]
            Some(TunnelCommand::BypassSocket(fd, done_tx)) => {
                shared_values.bypass_socket(fd, done_tx);
                SameState(self)
            }
            #[cfg(windows)]
            Some(TunnelCommand::SetExcludedApps(result_tx, paths)) => {
                shared_values.exclude_paths(paths, result_tx);
                SameState(self)
            }
            #[cfg(target_os = "android")]
            Some(TunnelCommand::SetExcludedApps(result_tx, paths)) => {
                if shared_values.set_excluded_paths(paths) {
                    let _ = result_tx.send(Ok(()));
                    self.disconnect(shared_values, AfterDisconnect::Reconnect(0))
                } else {
                    let _ = result_tx.send(Ok(()));
                    SameState(self)
                }
            }
            #[cfg(target_os = "macos")]
            Some(TunnelCommand::SetExcludedApps(result_tx, paths)) => {
                match shared_values.set_exclude_paths(paths) {
                    Ok(added_device) => {
                        let _ = result_tx.send(Ok(()));

                        if added_device {
                            if let Err(error) = Self::set_firewall_policy(
                                shared_values,
                                &self.tunnel_parameters,
                                &self.tunnel_metadata,
                                self.allowed_tunnel_traffic.clone(),
                            ) {
                                return self.disconnect(
                                    shared_values,
                                    AfterDisconnect::Block(
                                        ErrorStateCause::SetFirewallPolicyError(error),
                                    ),
                                );
                            }
                        }
                    }
                    Err(error) => {
                        let cause = ErrorStateCause::from(&error);
                        let _ = result_tx.send(Err(error));
                        return self.disconnect(shared_values, AfterDisconnect::Block(cause));
                    }
                }
                SameState(self)
            }
        }
    }

    fn handle_tunnel_events(
        mut self: Box<Self>,
        event: Option<(tunnel::TunnelEvent, oneshot::Sender<()>)>,
        shared_values: &mut SharedTunnelStateValues,
    ) -> EventConsequence {
        use self::EventConsequence::*;

        match event {
            Some((TunnelEvent::AuthFailed(reason), _)) => self.disconnect(
                shared_values,
                AfterDisconnect::Block(ErrorStateCause::AuthFailed(reason)),
            ),
            Some((TunnelEvent::InterfaceUp(metadata, allowed_tunnel_traffic), _done_tx)) => {
                #[cfg(windows)]
                if let Err(error) = shared_values
                    .split_tunnel
                    .set_tunnel_addresses(Some(&metadata))
                {
                    log::error!(
                        "{}",
                        error.display_chain_with_msg(
                            "Failed to register addresses with split tunnel driver"
                        )
                    );
                    return self.disconnect(
                        shared_values,
                        AfterDisconnect::Block(ErrorStateCause::SplitTunnelError),
                    );
                }

                #[cfg(target_os = "macos")]
                if let Err(error) = shared_values.enable_split_tunnel(&metadata) {
                    return self.disconnect(shared_values, AfterDisconnect::Block(error));
                }

                self.allowed_tunnel_traffic = allowed_tunnel_traffic;
                self.tunnel_metadata = Some(metadata);

                match Self::set_firewall_policy(
                    shared_values,
                    &self.tunnel_parameters,
                    &self.tunnel_metadata,
                    self.allowed_tunnel_traffic.clone(),
                ) {
                    Ok(()) => SameState(self),
                    Err(error) => self.disconnect(
                        shared_values,
                        AfterDisconnect::Block(ErrorStateCause::SetFirewallPolicyError(error)),
                    ),
                }
            }
            Some((TunnelEvent::Up(metadata), _)) => NewState(ConnectedState::enter(
                shared_values,
                metadata,
                self.tunnel_events,
                self.tunnel_parameters,
                self.tunnel_close_event,
                self.tunnel_close_tx,
            )),
            Some((TunnelEvent::Down, _)) => {
                // It is important to reset this before the tunnel device is down,
                // or else commands that reapply the firewall rules will fail since
                // they refer to a non-existent device.
                self.allowed_tunnel_traffic = INITIAL_ALLOWED_TUNNEL_TRAFFIC;
                self.tunnel_metadata = None;

                SameState(self)
            }
            None => {
                // The channel was closed
                log::debug!("The tunnel disconnected unexpectedly");
                let retry_attempt = self.retry_attempt + 1;
                self.disconnect(shared_values, AfterDisconnect::Reconnect(retry_attempt))
            }
        }
    }

    fn handle_tunnel_close_event(
        self,
        block_reason: Option<ErrorStateCause>,
        shared_values: &mut SharedTunnelStateValues,
    ) -> EventConsequence {
        use self::EventConsequence::*;

        if let Some(block_reason) = block_reason {
            Self::reset_routes(shared_values);
            return NewState(ErrorState::enter(shared_values, block_reason));
        }

        log::info!(
            "Tunnel closed. Reconnecting, attempt {}.",
            self.retry_attempt + 1
        );
        Self::reset_routes(shared_values);
        EventConsequence::NewState(ConnectingState::enter(
            shared_values,
            self.retry_attempt + 1,
        ))
    }
}

#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
fn should_retry(error: &tunnel::Error, retry_attempt: u32) -> bool {
    #[cfg(target_os = "windows")]
    if error.get_tunnel_device_error().is_some() {
        return retry_attempt < MAX_ATTEMPT_CREATE_TUN;
    }
    error.is_recoverable()
}

impl TunnelState for ConnectingState {
    fn handle_event(
        mut self: Box<Self>,
        runtime: &tokio::runtime::Handle,
        commands: &mut TunnelCommandReceiver,
        shared_values: &mut SharedTunnelStateValues,
    ) -> EventConsequence {
        let result = runtime.block_on(async {
            futures::select! {
                command = commands.next() => EventResult::Command(command),
                event = self.tunnel_events.next() => EventResult::Event(event),
                result = &mut self.tunnel_close_event => EventResult::Close(result),
            }
        });

        match result {
            EventResult::Command(command) => self.handle_commands(command, shared_values),
            EventResult::Event(event) => self.handle_tunnel_events(event, shared_values),
            EventResult::Close(result) => {
                if result.is_err() {
                    log::warn!("Tunnel monitor thread has stopped unexpectedly");
                }
                let block_reason = result.unwrap_or(None);
                self.handle_tunnel_close_event(block_reason, shared_values)
            }
        }
    }
}

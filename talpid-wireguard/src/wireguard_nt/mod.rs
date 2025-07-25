#![allow(clippy::undocumented_unsafe_blocks)] // Remove me if you dare.

use super::{
    Tunnel,
    config::Config,
    logging,
    stats::{Stats, StatsMap},
};
use bitflags::bitflags;
use futures::SinkExt;
use ipnetwork::IpNetwork;
use once_cell::sync::OnceCell;
use std::{
    ffi::CStr,
    fmt,
    future::Future,
    io,
    mem::{self, MaybeUninit},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    os::windows::io::RawHandle,
    path::Path,
    pin::Pin,
    ptr,
    sync::{Arc, LazyLock, Mutex},
};
#[cfg(daita)]
use std::{ffi::c_uchar, path::PathBuf};
use talpid_types::{BoxedError, ErrorExt};
use talpid_windows::net;
use widestring::{U16CStr, U16CString};
use windows_sys::{
    Win32::{
        Foundation::{BOOL, ERROR_MORE_DATA, FreeLibrary, HMODULE},
        NetworkManagement::Ndis::NET_LUID_LH,
        Networking::WinSock::{
            ADDRESS_FAMILY, AF_INET, AF_INET6, IN_ADDR, IN6_ADDR, SOCKADDR_INET,
        },
        System::LibraryLoader::{GetProcAddress, LOAD_WITH_ALTERED_SEARCH_PATH, LoadLibraryExW},
    },
    core::GUID,
};

#[cfg(daita)]
mod daita;

static WG_NT_DLL: OnceCell<WgNtDll> = OnceCell::new();
static ADAPTER_TYPE: LazyLock<U16CString> =
    LazyLock::new(|| U16CString::from_str("Mullvad").unwrap());
static ADAPTER_ALIAS: LazyLock<U16CString> =
    LazyLock::new(|| U16CString::from_str("Mullvad").unwrap());

const ADAPTER_GUID: GUID = GUID {
    data1: 0x514a3988,
    data2: 0x9716,
    data3: 0x43d5,
    data4: [0x8b, 0x05, 0x31, 0xda, 0x25, 0xa0, 0x44, 0xa9],
};

type WireGuardCreateAdapterFn = unsafe extern "stdcall" fn(
    name: *const u16,
    tunnel_type: *const u16,
    requested_guid: *const GUID,
) -> RawHandle;
type WireGuardCloseAdapterFn = unsafe extern "stdcall" fn(adapter: RawHandle);
type WireGuardGetAdapterLuidFn =
    unsafe extern "stdcall" fn(adapter: RawHandle, luid: *mut NET_LUID_LH);
type WireGuardSetConfigurationFn = unsafe extern "stdcall" fn(
    adapter: RawHandle,
    config: *const MaybeUninit<u8>,
    bytes: u32,
) -> BOOL;
type WireGuardGetConfigurationFn = unsafe extern "stdcall" fn(
    adapter: RawHandle,
    config: *const MaybeUninit<u8>,
    bytes: *mut u32,
) -> BOOL;
type WireGuardSetStateFn =
    unsafe extern "stdcall" fn(adapter: RawHandle, state: WgAdapterState) -> BOOL;

#[repr(C)]
#[allow(dead_code)]
enum LogLevel {
    Info = 0,
    Warn = 1,
    Err = 2,
}

impl From<LogLevel> for logging::LogLevel {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Info => Self::Info,
            LogLevel::Warn => Self::Warning,
            LogLevel::Err => Self::Error,
        }
    }
}

type WireGuardLoggerCb = extern "stdcall" fn(LogLevel, timestamp: u64, *const u16);
type WireGuardSetLoggerFn = extern "stdcall" fn(Option<WireGuardLoggerCb>);

#[repr(C)]
#[allow(dead_code)]
enum WireGuardAdapterLogState {
    Off = 0,
    On = 1,
    OnWithPrefix = 2,
}

type WireGuardSetAdapterLoggingFn =
    unsafe extern "stdcall" fn(adapter: RawHandle, state: WireGuardAdapterLogState) -> BOOL;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Failed to load WireGuardNT
    #[error("Failed to load mullvad-wireguard.dll")]
    LoadDll(#[source] io::Error),

    /// Failed to create tunnel interface
    #[error("Failed to create WireGuard device")]
    CreateTunnelDevice(#[source] io::Error),

    /// Failed to obtain tunnel interface alias
    #[error("Failed to obtain interface name")]
    ObtainAlias(#[source] io::Error),

    /// Failed to get WireGuard tunnel config for device
    #[error("Failed to get tunnel WireGuard config")]
    GetWireGuardConfig(#[source] io::Error),

    /// Failed to set WireGuard tunnel config on device
    #[error("Failed to set tunnel WireGuard config")]
    SetWireGuardConfig(#[source] io::Error),

    /// Error listening to tunnel IP interfaces
    #[error("Failed to wait on tunnel IP interfaces")]
    IpInterfaces(#[source] io::Error),

    /// Failed to set MTU and metric on tunnel device
    #[error("Failed to set tunnel interface MTU")]
    SetTunnelMtu(#[source] io::Error),

    /// Failed to set the tunnel state to up
    #[error("Failed to enable the tunnel adapter")]
    EnableTunnel(#[source] io::Error),

    /// Unknown address family
    #[error("Unknown address family: {0}")]
    UnknownAddressFamily(u16),

    /// Failure to set up logging
    #[error("Failed to set up logging")]
    InitLogging(#[source] logging::Error),

    /// Invalid allowed IP
    #[error("Invalid CIDR prefix")]
    InvalidAllowedIpCidr,

    /// Allowed IP contains non-zero host bits
    #[error("Allowed IP contains non-zero host bits")]
    InvalidAllowedIpBits,

    /// Failed to parse data returned by the driver
    #[error("Failed to parse data returned by wireguard-nt")]
    InvalidConfigData,

    /// DAITA machinist failed
    #[cfg(daita)]
    #[error("Failed to enable DAITA on tunnel device")]
    EnableTunnelDaita(#[source] io::Error),

    /// DAITA machinist failed
    #[cfg(daita)]
    #[error("Failed to initialize DAITA machinist")]
    InitializeMachinist(#[source] daita::Error),
}

pub struct WgNtTunnel {
    #[cfg(daita)]
    resource_dir: PathBuf,
    config: Arc<Mutex<Config>>,
    device: Option<Arc<WgNtAdapter>>,
    interface_name: String,
    setup_handle: tokio::task::JoinHandle<()>,
    #[cfg(daita)]
    daita_handle: Option<daita::MachinistHandle>,
    _logger_handle: LoggerHandle,
}

const WIREGUARD_KEY_LENGTH: usize = 32;

/// See `WIREGUARD_ALLOWED_IP` at <https://git.zx2c4.com/wireguard-nt/tree/api/wireguard.h>.
#[derive(Clone, Copy)]
#[repr(C, align(8))]
union WgIpAddr {
    v4: IN_ADDR,
    v6: IN6_ADDR,
}

impl From<IpAddr> for WgIpAddr {
    fn from(address: IpAddr) -> Self {
        match address {
            IpAddr::V4(addr) => WgIpAddr::from(addr),
            IpAddr::V6(addr) => WgIpAddr::from(addr),
        }
    }
}

impl From<Ipv6Addr> for WgIpAddr {
    fn from(address: Ipv6Addr) -> Self {
        Self {
            v6: net::in6addr_from_ipaddr(address),
        }
    }
}

impl From<Ipv4Addr> for WgIpAddr {
    fn from(address: Ipv4Addr) -> Self {
        Self {
            v4: net::inaddr_from_ipaddr(address),
        }
    }
}

/// See `WIREGUARD_ALLOWED_IP` at <https://git.zx2c4.com/wireguard-nt/tree/api/wireguard.h>.
#[derive(Clone, Copy)]
#[repr(C, align(8))]
struct WgAllowedIp {
    address: WgIpAddr,
    address_family: u16,
    cidr: u8,
}

impl WgAllowedIp {
    fn new(address: WgIpAddr, address_family: ADDRESS_FAMILY, cidr: u8) -> Result<Self> {
        Self::validate(&address, address_family, cidr)?;
        Ok(Self {
            address,
            address_family,
            cidr,
        })
    }

    fn validate(address: &WgIpAddr, address_family: ADDRESS_FAMILY, cidr: u8) -> Result<()> {
        match address_family {
            AF_INET => {
                if cidr > 32 {
                    return Err(Error::InvalidAllowedIpCidr);
                }
                let host_mask = u32::MAX.checked_shr(u32::from(cidr)).unwrap_or(0);
                if host_mask & unsafe { address.v4.S_un.S_addr }.to_be() != 0 {
                    return Err(Error::InvalidAllowedIpBits);
                }
            }
            AF_INET6 => {
                if cidr > 128 {
                    return Err(Error::InvalidAllowedIpCidr);
                }
                let mut host_mask = u128::MAX.checked_shr(u32::from(cidr)).unwrap_or(0);
                let bytes = unsafe { address.v6.u.Byte };
                for byte in bytes.iter().rev() {
                    if byte & ((host_mask & 0xff) as u8) != 0 {
                        return Err(Error::InvalidAllowedIpBits);
                    }
                    host_mask >>= 8;
                }
            }
            family => return Err(Error::UnknownAddressFamily(family)),
        }
        Ok(())
    }
}

impl PartialEq for WgAllowedIp {
    fn eq(&self, other: &Self) -> bool {
        if self.cidr != other.cidr {
            return false;
        }
        match self.address_family {
            AF_INET => {
                net::ipaddr_from_inaddr(unsafe { self.address.v4 })
                    == net::ipaddr_from_inaddr(unsafe { other.address.v4 })
            }
            AF_INET6 => {
                net::ipaddr_from_in6addr(unsafe { self.address.v6 })
                    == net::ipaddr_from_in6addr(unsafe { other.address.v6 })
            }
            _ => {
                log::error!("Allowed IP uses unknown address family");
                true
            }
        }
    }
}
impl Eq for WgAllowedIp {}

impl fmt::Debug for WgAllowedIp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("WgAllowedIp");
        match self.address_family {
            AF_INET => s.field(
                "address",
                &net::ipaddr_from_inaddr(unsafe { self.address.v4 }),
            ),
            AF_INET6 => s.field(
                "address",
                &net::ipaddr_from_in6addr(unsafe { self.address.v6 }),
            ),
            _ => s.field("address", &"<unknown>"),
        };
        s.field("address_family", &self.address_family)
            .field("cidr", &self.cidr)
            .finish()
    }
}

bitflags! {
    /// See `WIREGUARD_PEER_FLAG` at <https://git.zx2c4.com/wireguard-nt/tree/api/wireguard.h>.
    struct WgPeerFlag: u32 {
        const HAS_PUBLIC_KEY = 0b00000001;
        const HAS_PRESHARED_KEY = 0b00000010;
        const HAS_PERSISTENT_KEEPALIVE = 0b00000100;
        const HAS_ENDPOINT = 0b00001000;
        const REPLACE_ALLOWED_IPS = 0b00100000;
        const REMOVE = 0b01000000;
        const UPDATE = 0b10000000;
        #[cfg(daita)]
        const HAS_CONSTANT_PACKET_SIZE = 0b100000000;
    }
}

/// See `WIREGUARD_PEER` at <https://git.zx2c4.com/wireguard-nt/tree/api/wireguard.h>.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[repr(C, align(8))]
struct WgPeer {
    flags: WgPeerFlag,
    reserved: u32,
    public_key: [u8; WIREGUARD_KEY_LENGTH],
    preshared_key: [u8; WIREGUARD_KEY_LENGTH],
    persistent_keepalive: u16,
    endpoint: SockAddrInet,
    tx_bytes: u64,
    rx_bytes: u64,
    last_handshake: u64,
    allowed_ips_count: u32,
    #[cfg(daita)]
    constant_packet_size: c_uchar,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct SockAddrInet {
    addr: SOCKADDR_INET,
}

impl From<SOCKADDR_INET> for SockAddrInet {
    fn from(addr: SOCKADDR_INET) -> Self {
        Self { addr }
    }
}
impl PartialEq for SockAddrInet {
    fn eq(&self, other: &Self) -> bool {
        let self_addr = match net::try_socketaddr_from_inet_sockaddr(self.addr) {
            Ok(addr) => addr,
            Err(error) => {
                log::error!(
                    "{}",
                    error.display_chain_with_msg("Failed to convert socket address")
                );
                return true;
            }
        };
        let other_addr = match net::try_socketaddr_from_inet_sockaddr(other.addr) {
            Ok(addr) => addr,
            Err(error) => {
                log::error!(
                    "{}",
                    error.display_chain_with_msg("Failed to convert socket address")
                );
                return true;
            }
        };
        self_addr == other_addr
    }
}
impl Eq for SockAddrInet {}

impl fmt::Debug for SockAddrInet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("SockAddrInet");
        let self_addr = net::try_socketaddr_from_inet_sockaddr(self.addr)
            .map(|addr| addr.to_string())
            .unwrap_or("<unknown>".to_string());
        s.field("addr", &self_addr).finish()
    }
}

bitflags! {
    /// See `WIREGUARD_INTERFACE_FLAG` at <https://git.zx2c4.com/wireguard-nt/tree/api/wireguard.h>.
    struct WgInterfaceFlag: u32 {
        const HAS_PUBLIC_KEY = 0b00000001;
        const HAS_PRIVATE_KEY = 0b00000010;
        const HAS_LISTEN_PORT = 0b00000100;
        const REPLACE_PEERS = 0b00001000;
    }
}

/// See `WIREGUARD_INTERFACE` at <https://git.zx2c4.com/wireguard-nt/tree/api/wireguard.h>.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[repr(C, align(8))]
struct WgInterface {
    flags: WgInterfaceFlag,
    listen_port: u16,
    private_key: [u8; WIREGUARD_KEY_LENGTH],
    public_key: [u8; WIREGUARD_KEY_LENGTH],
    peers_count: u32,
}

/// See `WIREGUARD_ADAPTER_LOG_STATE` at <https://git.zx2c4.com/wireguard-nt/tree/api/wireguard.h>.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[repr(C)]
#[allow(dead_code)]
enum WgAdapterState {
    Down = 0,
    Up = 1,
}

impl WgNtTunnel {
    pub fn start_tunnel(
        config: &Config,
        log_path: Option<&Path>,
        resource_dir: &Path,
        done_tx: futures::channel::mpsc::Sender<std::result::Result<(), BoxedError>>,
    ) -> std::result::Result<Self, super::TunnelError> {
        Self::start_tunnel_inner(config, log_path, resource_dir, done_tx).map_err(|error| {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to setup WireGuardNT tunnel")
            );

            match error {
                Error::CreateTunnelDevice(error) => super::TunnelError::SetupTunnelDevice(
                    talpid_tunnel::tun_provider::Error::Io(error),
                ),
                _ => super::TunnelError::FatalStartWireguardError(Box::new(error)),
            }
        })
    }

    fn start_tunnel_inner(
        config: &Config,
        log_path: Option<&Path>,
        resource_dir: &Path,
        mut done_tx: futures::channel::mpsc::Sender<std::result::Result<(), BoxedError>>,
    ) -> Result<Self> {
        let dll = load_wg_nt_dll(resource_dir)?;
        let logger_handle = LoggerHandle::new(dll, log_path)?;
        let device = WgNtAdapter::create(dll, &ADAPTER_ALIAS, &ADAPTER_TYPE, Some(ADAPTER_GUID))
            .map_err(Error::CreateTunnelDevice)?;

        let interface_name = device.name().map_err(Error::ObtainAlias)?.to_string_lossy();

        if let Err(error) = device.set_logging(WireGuardAdapterLogState::On) {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to set log state on WireGuard interface")
            );
        }
        device.set_config(config)?;
        let device2 = Arc::new(device);
        let device = Some(device2.clone());

        let setup_future = setup_ip_listener(
            device2.clone(),
            u32::from(config.mtu),
            config.tunnel.addresses.iter().any(|addr| addr.is_ipv6()),
        );
        let setup_handle = tokio::spawn(async move {
            let _ = done_tx
                .send(setup_future.await.map_err(BoxedError::new))
                .await;
        });

        Ok(WgNtTunnel {
            #[cfg(daita)]
            resource_dir: resource_dir.to_owned(),
            config: Arc::new(Mutex::new(config.clone())),
            device,
            interface_name,
            setup_handle,
            #[cfg(daita)]
            daita_handle: None,
            _logger_handle: logger_handle,
        })
    }

    fn stop_tunnel(&mut self) {
        self.setup_handle.abort();
        #[cfg(daita)]
        if let Some(daita_handle) = self.daita_handle.take() {
            let _ = daita_handle.close();
        }
        let _ = self.device.take();
    }

    #[cfg(daita)]
    fn spawn_machinist(&mut self) -> Result<()> {
        if let Some(handle) = self.daita_handle.take() {
            log::info!("Stopping previous DAITA machines");
            let _ = handle.close();
        }

        let Some(device) = self.device.clone() else {
            log::debug!("Tunnel is stopped; not starting machines");
            return Ok(());
        };

        let config = self.config.lock().unwrap();

        log::info!("Initializing DAITA for wireguard device");
        let session = daita::Session::from_adapter(device).map_err(Error::EnableTunnelDaita)?;
        self.daita_handle = Some(
            daita::Machinist::spawn(
                &self.resource_dir,
                session,
                config.entry_peer.public_key.clone(),
                config.mtu,
            )
            .map_err(Error::InitializeMachinist)?,
        );
        Ok(())
    }
}

async fn setup_ip_listener(device: Arc<WgNtAdapter>, mtu: u32, has_ipv6: bool) -> Result<()> {
    let luid = device.luid();
    let luid = NET_LUID_LH {
        Value: unsafe { luid.Value },
    };

    log::debug!("Waiting for tunnel IP interfaces to arrive");
    net::wait_for_interfaces(luid, true, has_ipv6)
        .await
        .map_err(Error::IpInterfaces)?;
    log::debug!("Waiting for tunnel IP interfaces: Done");

    talpid_tunnel::network_interface::initialize_interfaces(
        luid,
        Some(mtu),
        has_ipv6.then_some(mtu),
    )
    .map_err(Error::SetTunnelMtu)?;

    device
        .set_state(WgAdapterState::Up)
        .map_err(Error::EnableTunnel)
}

impl Drop for WgNtTunnel {
    fn drop(&mut self) {
        self.stop_tunnel();
    }
}

static LOG_CONTEXT: LazyLock<Mutex<Option<u64>>> = LazyLock::new(|| Mutex::new(None));

struct LoggerHandle {
    dll: &'static WgNtDll,
    context: u64,
}

impl LoggerHandle {
    fn new(dll: &'static WgNtDll, log_path: Option<&Path>) -> Result<Self> {
        let context = logging::initialize_logging(log_path).map_err(Error::InitLogging)?;
        {
            *(LOG_CONTEXT.lock().unwrap()) = Some(context);
        }
        dll.set_logger(Some(Self::logging_callback));
        Ok(Self { dll, context })
    }

    extern "stdcall" fn logging_callback(level: LogLevel, _timestamp: u64, message: *const u16) {
        if message.is_null() {
            return;
        }
        let mut message = unsafe { U16CStr::from_ptr_str(message) }.to_string_lossy();
        message.push_str("\r\n");

        if let Some(context) = &*LOG_CONTEXT.lock().unwrap() {
            // Horribly broken, because callback does not provide a context
            logging::log(*context, level.into(), "wireguard-nt", &message);
        }
    }
}

impl Drop for LoggerHandle {
    fn drop(&mut self) {
        let mut ctx = LOG_CONTEXT.lock().unwrap();
        if *ctx == Some(self.context) {
            *ctx = None;
            self.dll.set_logger(None);
        }
        logging::clean_up_logging(self.context);
    }
}

struct WgNtAdapter {
    dll_handle: &'static WgNtDll,
    handle: RawHandle,
}

impl fmt::Debug for WgNtAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WgNtAdapter")
            .field("handle", &self.handle)
            .finish()
    }
}

unsafe impl Send for WgNtAdapter {}
unsafe impl Sync for WgNtAdapter {}

impl WgNtAdapter {
    fn create(
        dll_handle: &'static WgNtDll,
        name: &U16CStr,
        tunnel_type: &U16CStr,
        requested_guid: Option<GUID>,
    ) -> io::Result<Self> {
        let handle = dll_handle.create_adapter(name, tunnel_type, requested_guid)?;
        Ok(Self { dll_handle, handle })
    }

    fn name(&self) -> io::Result<U16CString> {
        net::alias_from_luid(&self.luid()).and_then(|alias| {
            U16CString::from_os_str(alias).map_err(|_| io::Error::other("unexpected null char"))
        })
    }

    fn luid(&self) -> NET_LUID_LH {
        unsafe { self.dll_handle.get_adapter_luid(self.handle) }
    }

    fn set_config(&self, config: &Config) -> Result<()> {
        let config_buffer = serialize_config(config)?;
        unsafe {
            self.dll_handle
                .set_config(self.handle, config_buffer.as_ptr(), config_buffer.len())
                .map_err(Error::SetWireGuardConfig)
        }
    }

    #[allow(clippy::type_complexity)]
    fn get_config(&self) -> Result<(WgInterface, Vec<(WgPeer, Vec<WgAllowedIp>)>)> {
        unsafe {
            deserialize_config(
                &self
                    .dll_handle
                    .get_config(self.handle)
                    .map_err(Error::GetWireGuardConfig)?,
            )
        }
    }

    fn set_state(&self, state: WgAdapterState) -> io::Result<()> {
        unsafe { self.dll_handle.set_adapter_state(self.handle, state) }
    }

    fn set_logging(&self, state: WireGuardAdapterLogState) -> io::Result<()> {
        unsafe { self.dll_handle.set_adapter_logging(self.handle, state) }
    }
}

impl Drop for WgNtAdapter {
    fn drop(&mut self) {
        unsafe { self.dll_handle.close_adapter(self.handle) };
    }
}

struct WgNtDll {
    handle: HMODULE,
    func_create: WireGuardCreateAdapterFn,
    func_close: WireGuardCloseAdapterFn,
    func_get_adapter_luid: WireGuardGetAdapterLuidFn,
    func_set_configuration: WireGuardSetConfigurationFn,
    func_get_configuration: WireGuardGetConfigurationFn,
    func_set_adapter_state: WireGuardSetStateFn,
    func_set_logger: WireGuardSetLoggerFn,
    func_set_adapter_logging: WireGuardSetAdapterLoggingFn,
    #[cfg(daita)]
    func_daita_activate: daita::bindings::WireGuardDaitaActivateFn,
    #[cfg(daita)]
    func_daita_event_data_available_event: daita::bindings::WireGuardDaitaEventDataAvailableEventFn,
    #[cfg(daita)]
    func_daita_receive_events: daita::bindings::WireGuardDaitaReceiveEventsFn,
    #[cfg(daita)]
    func_daita_send_action: daita::bindings::WireGuardDaitaSendActionFn,
}

unsafe impl Send for WgNtDll {}
unsafe impl Sync for WgNtDll {}

impl WgNtDll {
    pub fn new(resource_dir: &Path) -> io::Result<Self> {
        let wg_nt_dll =
            U16CString::from_os_str_truncate(resource_dir.join("mullvad-wireguard.dll"));

        let handle =
            unsafe { LoadLibraryExW(wg_nt_dll.as_ptr(), 0, LOAD_WITH_ALTERED_SEARCH_PATH) };
        if handle == 0 {
            return Err(io::Error::last_os_error());
        }
        Self::new_inner(handle, Self::get_proc_address)
    }

    fn new_inner(
        handle: HMODULE,
        get_proc_fn: unsafe fn(HMODULE, &CStr) -> io::Result<unsafe extern "system" fn() -> isize>,
    ) -> io::Result<Self> {
        Ok(WgNtDll {
            handle,
            func_create: unsafe {
                *((&get_proc_fn(handle, c"WireGuardCreateAdapter")?) as *const _ as *const _)
            },
            func_close: unsafe {
                *((&get_proc_fn(handle, c"WireGuardCloseAdapter")?) as *const _ as *const _)
            },
            func_get_adapter_luid: unsafe {
                *((&get_proc_fn(handle, c"WireGuardGetAdapterLUID")?) as *const _ as *const _)
            },
            func_set_configuration: unsafe {
                *((&get_proc_fn(handle, c"WireGuardSetConfiguration")?) as *const _ as *const _)
            },
            func_get_configuration: unsafe {
                *((&get_proc_fn(handle, c"WireGuardGetConfiguration")?) as *const _ as *const _)
            },
            func_set_adapter_state: unsafe {
                *((&get_proc_fn(handle, c"WireGuardSetAdapterState")?) as *const _ as *const _)
            },
            func_set_logger: unsafe {
                *((&get_proc_fn(handle, c"WireGuardSetLogger")?) as *const _ as *const _)
            },
            func_set_adapter_logging: unsafe {
                *((&get_proc_fn(handle, c"WireGuardSetAdapterLogging")?) as *const _ as *const _)
            },
            #[cfg(daita)]
            func_daita_activate: unsafe {
                *((&get_proc_fn(handle, c"WireGuardDaitaActivate")?) as *const _ as *const _)
            },
            #[cfg(daita)]
            func_daita_event_data_available_event: unsafe {
                *((&get_proc_fn(handle, c"WireGuardDaitaEventDataAvailableEvent")?) as *const _
                    as *const _)
            },
            #[cfg(daita)]
            func_daita_receive_events: unsafe {
                *((&get_proc_fn(handle, c"WireGuardDaitaReceiveEvents")?) as *const _ as *const _)
            },
            #[cfg(daita)]
            func_daita_send_action: unsafe {
                *((&get_proc_fn(handle, c"WireGuardDaitaSendAction")?) as *const _ as *const _)
            },
        })
    }

    unsafe fn get_proc_address(
        handle: HMODULE,
        name: &CStr,
    ) -> io::Result<unsafe extern "system" fn() -> isize> {
        let handle = unsafe { GetProcAddress(handle, name.as_ptr() as *const u8) };
        handle.ok_or(io::Error::last_os_error())
    }

    pub fn create_adapter(
        &self,
        name: &U16CStr,
        tunnel_type: &U16CStr,
        requested_guid: Option<GUID>,
    ) -> io::Result<RawHandle> {
        let guid_ptr = match requested_guid.as_ref() {
            Some(guid) => guid as *const _,
            None => ptr::null_mut(),
        };
        let handle = unsafe { (self.func_create)(name.as_ptr(), tunnel_type.as_ptr(), guid_ptr) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(handle)
    }

    pub unsafe fn close_adapter(&self, adapter: RawHandle) {
        unsafe { (self.func_close)(adapter) };
    }

    pub unsafe fn get_adapter_luid(&self, adapter: RawHandle) -> NET_LUID_LH {
        let mut luid = mem::MaybeUninit::<NET_LUID_LH>::zeroed();
        unsafe {
            (self.func_get_adapter_luid)(adapter, luid.as_mut_ptr());
            luid.assume_init()
        }
    }

    pub unsafe fn set_config(
        &self,
        adapter: RawHandle,
        config: *const MaybeUninit<u8>,
        config_size: usize,
    ) -> io::Result<()> {
        let result = unsafe {
            (self.func_set_configuration)(adapter, config, u32::try_from(config_size).unwrap())
        };
        if result == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub unsafe fn get_config(&self, adapter: RawHandle) -> io::Result<Vec<MaybeUninit<u8>>> {
        let mut config_size = 0;
        let mut config = vec![];
        loop {
            let result = unsafe {
                (self.func_get_configuration)(adapter, config.as_mut_ptr(), &mut config_size)
            };
            if result == 0 {
                let last_error = io::Error::last_os_error();
                if last_error.raw_os_error() != Some(ERROR_MORE_DATA as i32) {
                    break Err(last_error);
                }
                config.resize(config_size as usize, MaybeUninit::new(0u8));
            } else {
                break Ok(config);
            }
        }
    }

    pub unsafe fn set_adapter_state(
        &self,
        adapter: RawHandle,
        state: WgAdapterState,
    ) -> io::Result<()> {
        let result = unsafe { (self.func_set_adapter_state)(adapter, state) };
        if result == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn set_logger(&self, cb: Option<WireGuardLoggerCb>) {
        (self.func_set_logger)(cb);
    }

    pub unsafe fn set_adapter_logging(
        &self,
        adapter: RawHandle,
        state: WireGuardAdapterLogState,
    ) -> io::Result<()> {
        if unsafe { (self.func_set_adapter_logging)(adapter, state) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(daita)]
    pub unsafe fn daita_activate(
        &self,
        adapter: RawHandle,
        events_capacity: usize,
        actions_capacity: usize,
    ) -> io::Result<()> {
        if unsafe { (self.func_daita_activate)(adapter, events_capacity, actions_capacity) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(daita)]
    pub unsafe fn daita_event_data_available_event(
        &self,
        adapter: RawHandle,
    ) -> io::Result<RawHandle> {
        let ready_event = unsafe { (self.func_daita_event_data_available_event)(adapter) };
        if ready_event.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(ready_event)
    }

    #[cfg(daita)]
    pub unsafe fn daita_receive_events(
        &self,
        adapter: RawHandle,
        events: *mut daita::Event,
    ) -> io::Result<usize> {
        let num_events = unsafe { (self.func_daita_receive_events)(adapter, events) };
        if num_events == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(num_events)
    }

    #[cfg(daita)]
    pub unsafe fn daita_send_action(
        &self,
        adapter: RawHandle,
        action: *const daita::Action,
    ) -> io::Result<()> {
        if unsafe { (self.func_daita_send_action)(adapter, action) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for WgNtDll {
    fn drop(&mut self) {
        unsafe { FreeLibrary(self.handle) };
    }
}

fn load_wg_nt_dll(resource_dir: &Path) -> Result<&'static WgNtDll> {
    WG_NT_DLL.get_or_try_init(|| WgNtDll::new(resource_dir).map_err(Error::LoadDll))
}

fn serialize_config(config: &Config) -> Result<Vec<MaybeUninit<u8>>> {
    let mut buffer = vec![];

    let header = WgInterface {
        flags: WgInterfaceFlag::HAS_PRIVATE_KEY | WgInterfaceFlag::REPLACE_PEERS,
        listen_port: 0,
        private_key: config.tunnel.private_key.to_bytes(),
        public_key: [0u8; WIREGUARD_KEY_LENGTH],
        peers_count: u32::try_from(config.peers().count()).unwrap(),
    };

    buffer.extend(as_uninit_byte_slice(&header));

    for peer in config.peers() {
        #[cfg(not(daita))]
        let mut flags = WgPeerFlag::HAS_PUBLIC_KEY | WgPeerFlag::HAS_ENDPOINT;
        #[cfg(daita)]
        let mut flags = WgPeerFlag::HAS_PUBLIC_KEY
            | WgPeerFlag::HAS_ENDPOINT
            | WgPeerFlag::HAS_CONSTANT_PACKET_SIZE;
        if peer.psk.is_some() {
            flags |= WgPeerFlag::HAS_PRESHARED_KEY;
        }
        #[cfg(daita)]
        let constant_packet_size = if peer.constant_packet_size { 1 } else { 0 };
        let wg_peer = WgPeer {
            flags,
            reserved: 0,
            public_key: *peer.public_key.as_bytes(),
            preshared_key: peer
                .psk
                .as_ref()
                .map(|psk| *psk.as_bytes())
                .unwrap_or([0u8; WIREGUARD_KEY_LENGTH]),
            persistent_keepalive: 0,
            endpoint: net::inet_sockaddr_from_socketaddr(peer.endpoint).into(),
            tx_bytes: 0,
            rx_bytes: 0,
            last_handshake: 0,
            allowed_ips_count: u32::try_from(peer.allowed_ips.len()).unwrap(),
            #[cfg(daita)]
            constant_packet_size,
        };

        buffer.extend(as_uninit_byte_slice(&wg_peer));

        for allowed_ip in &peer.allowed_ips {
            let address_family = match allowed_ip {
                IpNetwork::V4(_) => AF_INET,
                IpNetwork::V6(_) => AF_INET6,
            };
            let address = match allowed_ip {
                IpNetwork::V4(v4_network) => WgIpAddr::from(v4_network.ip()),
                IpNetwork::V6(v6_network) => WgIpAddr::from(v6_network.ip()),
            };

            let wg_allowed_ip = WgAllowedIp::new(address, address_family, allowed_ip.prefix())?;

            buffer.extend(as_uninit_byte_slice(&wg_allowed_ip));
        }
    }

    Ok(buffer)
}

#[allow(clippy::type_complexity)]
unsafe fn deserialize_config(
    config: &[MaybeUninit<u8>],
) -> Result<(WgInterface, Vec<(WgPeer, Vec<WgAllowedIp>)>)> {
    if config.len() < mem::size_of::<WgInterface>() {
        return Err(Error::InvalidConfigData);
    }
    let (head, mut tail) = config.split_at(mem::size_of::<WgInterface>());
    let interface: WgInterface = unsafe { *(head.as_ptr() as *const WgInterface) };

    let mut peers = vec![];
    for _ in 0..interface.peers_count {
        if tail.len() < mem::size_of::<WgPeer>() {
            return Err(Error::InvalidConfigData);
        }
        let (peer_data, new_tail) = tail.split_at(mem::size_of::<WgPeer>());
        let peer: WgPeer = unsafe { *(peer_data.as_ptr() as *const WgPeer) };
        tail = new_tail;

        if let Err(error) = net::try_socketaddr_from_inet_sockaddr(peer.endpoint.addr) {
            log::error!(
                "{}",
                error.display_chain_with_msg("Received invalid endpoint address")
            );
            return Err(Error::InvalidConfigData);
        }

        let mut allowed_ips = vec![];

        for _ in 0..peer.allowed_ips_count {
            if tail.len() < mem::size_of::<WgAllowedIp>() {
                return Err(Error::InvalidConfigData);
            }
            let (allowed_ip_data, new_tail) = tail.split_at(mem::size_of::<WgAllowedIp>());
            let allowed_ip: WgAllowedIp =
                unsafe { *(allowed_ip_data.as_ptr() as *const WgAllowedIp) };
            if let Err(error) = WgAllowedIp::validate(
                &allowed_ip.address,
                allowed_ip.address_family,
                allowed_ip.cidr,
            ) {
                log::error!(
                    "{}",
                    error.display_chain_with_msg("Received invalid allowed IP")
                );
                return Err(Error::InvalidConfigData);
            }
            tail = new_tail;
            allowed_ips.push(allowed_ip);
        }

        peers.push((peer, allowed_ips));
    }

    if !tail.is_empty() {
        return Err(Error::InvalidConfigData);
    }

    Ok((interface, peers))
}

#[async_trait::async_trait]
impl Tunnel for WgNtTunnel {
    fn get_interface_name(&self) -> String {
        self.interface_name.clone()
    }

    async fn get_tunnel_stats(&self) -> std::result::Result<StatsMap, super::TunnelError> {
        let Some(ref device) = self.device else {
            log::error!("Failed to obtain tunnel stats as device no longer exists");
            return Err(super::TunnelError::GetConfigError);
        };

        let device = device.clone();
        tokio::task::spawn_blocking(move || {
            let mut map = StatsMap::new();
            let (_interface, peers) = device.get_config().map_err(|error| {
                log::error!(
                    "{}",
                    error.display_chain_with_msg("Failed to obtain tunnel config")
                );
                super::TunnelError::GetConfigError
            })?;
            for (peer, _allowed_ips) in &peers {
                map.insert(
                    peer.public_key,
                    Stats {
                        tx_bytes: peer.tx_bytes,
                        rx_bytes: peer.rx_bytes,
                    },
                );
            }
            Ok(map)
        })
        .await
        .unwrap()
    }

    fn stop(mut self: Box<Self>) -> std::result::Result<(), super::TunnelError> {
        self.stop_tunnel();
        Ok(())
    }

    fn set_config(
        &mut self,
        config: Config,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<(), super::TunnelError>> + Send>> {
        let device = self.device.clone();
        let current_config = self.config.clone();

        Box::pin(async move {
            let Some(device) = device else {
                log::error!("Failed to set config: No tunnel device");
                return Err(super::TunnelError::SetConfigError);
            };
            let mut current_config = current_config.lock().unwrap();
            *current_config = config;
            device.set_config(&current_config).map_err(|error| {
                log::error!(
                    "{}",
                    error.display_chain_with_msg("Failed to set wg-nt tunnel config")
                );
                super::TunnelError::SetConfigError
            })
        })
    }

    #[cfg(daita)]
    fn start_daita(
        &mut self,
        _: talpid_tunnel_config_client::DaitaSettings,
    ) -> std::result::Result<(), crate::TunnelError> {
        self.spawn_machinist().map_err(|error| {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to start DAITA for wg-nt tunnel")
            );
            super::TunnelError::SetConfigError
        })
    }
}

pub fn as_uninit_byte_slice<T: Copy + Sized>(value: &T) -> &[mem::MaybeUninit<u8>] {
    unsafe { std::slice::from_raw_parts(value as *const _ as *const _, mem::size_of::<T>()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use talpid_types::net::wireguard;

    #[derive(Debug, Eq, PartialEq, Clone, Copy)]
    #[repr(C)]
    struct Interface {
        interface: WgInterface,
        p0: WgPeer,
        p0_allowed_ip_0: WgAllowedIp,
    }

    static WG_PRIVATE_KEY: LazyLock<wireguard::PrivateKey> =
        LazyLock::new(wireguard::PrivateKey::new_from_random);
    static WG_PUBLIC_KEY: LazyLock<wireguard::PublicKey> =
        LazyLock::new(|| wireguard::PrivateKey::new_from_random().public_key());
    static WG_CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
        tunnel: wireguard::TunnelConfig {
            private_key: WG_PRIVATE_KEY.clone(),
            addresses: vec![],
        },
        entry_peer: wireguard::PeerConfig {
            public_key: WG_PUBLIC_KEY.clone(),
            allowed_ips: vec!["1.3.3.0/24".parse().unwrap()],
            endpoint: "1.2.3.4:1234".parse().unwrap(),
            psk: None,
            constant_packet_size: false,
        },
        exit_peer: None,
        ipv4_gateway: "0.0.0.0".parse().unwrap(),
        ipv6_gateway: None,
        mtu: 0,
        obfuscator_config: None,
        #[cfg(daita)]
        daita: false,
        quantum_resistant: false,
    });

    static WG_STRUCT_CONFIG: LazyLock<Interface> = LazyLock::new(|| Interface {
        interface: WgInterface {
            flags: WgInterfaceFlag::HAS_PRIVATE_KEY | WgInterfaceFlag::REPLACE_PEERS,
            listen_port: 0,
            private_key: WG_PRIVATE_KEY.to_bytes(),
            public_key: [0; WIREGUARD_KEY_LENGTH],
            peers_count: 1,
        },
        p0: WgPeer {
            flags: WgPeerFlag::HAS_PUBLIC_KEY
                | WgPeerFlag::HAS_ENDPOINT
                | WgPeerFlag::HAS_CONSTANT_PACKET_SIZE,
            reserved: 0,
            public_key: *WG_PUBLIC_KEY.as_bytes(),
            preshared_key: [0; WIREGUARD_KEY_LENGTH],
            persistent_keepalive: 0,
            endpoint: talpid_windows::net::inet_sockaddr_from_socketaddr(
                "1.2.3.4:1234".parse().unwrap(),
            )
            .into(),
            tx_bytes: 0,
            rx_bytes: 0,
            last_handshake: 0,
            allowed_ips_count: 1,
            constant_packet_size: 0,
        },
        p0_allowed_ip_0: WgAllowedIp {
            address: WgIpAddr::from("1.3.3.0".parse::<Ipv4Addr>().unwrap()),
            address_family: AF_INET,
            cidr: 24,
        },
    });

    fn get_proc_fn(
        _handle: HMODULE,
        _symbol: &CStr,
    ) -> io::Result<unsafe extern "system" fn() -> isize> {
        Ok(null_fn)
    }

    #[test]
    fn test_dll_imports() {
        WgNtDll::new_inner(0, get_proc_fn).unwrap();
    }

    #[test]
    fn test_config_serialization() {
        let serialized_data = serialize_config(&WG_CONFIG).unwrap();
        assert_eq!(mem::size_of::<Interface>(), serialized_data.len());
        let serialized_iface = &unsafe { *(serialized_data.as_ptr() as *const Interface) };
        assert_eq!(&*WG_STRUCT_CONFIG, serialized_iface);
    }

    #[test]
    fn test_config_deserialization() {
        let config_buffer = as_uninit_byte_slice(&*WG_STRUCT_CONFIG);
        let (iface, peers) = unsafe { deserialize_config(config_buffer) }.unwrap();
        assert_eq!(iface, WG_STRUCT_CONFIG.interface);
        assert_eq!(peers.len(), 1);
        let (peer, allowed_ips) = &peers[0];
        assert_eq!(peer, &WG_STRUCT_CONFIG.p0);
        assert_eq!(allowed_ips.len(), 1);
        assert_eq!(allowed_ips[0], WG_STRUCT_CONFIG.p0_allowed_ip_0);
    }

    #[test]
    fn test_wg_allowed_ip_v4() {
        // Valid: /32 prefix
        let address_family = AF_INET;
        let address = WgIpAddr::from("127.0.0.1".parse::<Ipv4Addr>().unwrap());
        let cidr = 32;
        WgAllowedIp::new(address, address_family, cidr).unwrap();

        // Invalid host bits
        let cidr = 24;
        let address = WgIpAddr::from("0.0.0.1".parse::<Ipv4Addr>().unwrap());
        assert!(WgAllowedIp::new(address, address_family, cidr).is_err());

        // Valid host bits
        let cidr = 24;
        let address = WgIpAddr::from("255.255.255.0".parse::<Ipv4Addr>().unwrap());
        WgAllowedIp::new(address, address_family, cidr).unwrap();

        // 0.0.0.0/0
        let cidr = 0;
        let address = WgIpAddr::from("0.0.0.0".parse::<Ipv4Addr>().unwrap());
        WgAllowedIp::new(address, address_family, cidr).unwrap();

        // Invalid CIDR
        let cidr = 33;
        assert!(WgAllowedIp::new(address, address_family, cidr).is_err());
    }

    #[test]
    fn test_wg_allowed_ip_v6() {
        // Valid: /128 prefix
        let address_family = AF_INET6;
        let address = WgIpAddr::from("::1".parse::<Ipv6Addr>().unwrap());
        let cidr = 128;
        WgAllowedIp::new(address, address_family, cidr).unwrap();

        // Invalid host bits
        let cidr = 127;
        assert!(WgAllowedIp::new(address, address_family, cidr).is_err());

        // Valid host bits
        let address = WgIpAddr::from(
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:fffe"
                .parse::<Ipv6Addr>()
                .unwrap(),
        );
        WgAllowedIp::new(address, address_family, cidr).unwrap();

        // ::/0
        let cidr = 0;
        let address = WgIpAddr::from("::".parse::<Ipv6Addr>().unwrap());
        WgAllowedIp::new(address, address_family, cidr).unwrap();

        // Invalid CIDR
        let cidr = 129;
        assert!(WgAllowedIp::new(address, address_family, cidr).is_err());
    }

    unsafe extern "system" fn null_fn() -> isize {
        unreachable!("unexpected call of function")
    }
}

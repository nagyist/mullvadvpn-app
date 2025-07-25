#[cfg(not(target_os = "android"))]
use crate::dns::ResolvedDnsConfig;
use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::LazyLock,
};
use talpid_types::net::{ALLOWED_LAN_NETS, AllowedEndpoint, AllowedTunnelTraffic};

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod imp;

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod imp;

pub use self::imp::Error;

#[cfg(any(target_os = "linux", target_os = "macos"))]
static IPV6_LINK_LOCAL: LazyLock<Ipv6Network> =
    LazyLock::new(|| Ipv6Network::new(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0), 10).unwrap());
/// The allowed target addresses of outbound DHCPv6 requests
#[cfg(any(target_os = "linux", target_os = "macos"))]
static DHCPV6_SERVER_ADDRS: LazyLock<[Ipv6Addr; 2]> = LazyLock::new(|| {
    [
        Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 1, 2),
        Ipv6Addr::new(0xff05, 0, 0, 0, 0, 0, 1, 3),
    ]
});
#[cfg(any(target_os = "linux", target_os = "macos"))]
static ROUTER_SOLICITATION_OUT_DST_ADDR: LazyLock<Ipv6Addr> =
    LazyLock::new(|| Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 2));
#[cfg(any(target_os = "linux", target_os = "macos"))]
static SOLICITED_NODE_MULTICAST: LazyLock<Ipv6Network> = LazyLock::new(|| {
    Ipv6Network::new(Ipv6Addr::new(0xff02, 0, 0, 0, 0, 1, 0xFF00, 0), 104).unwrap()
});
static LOOPBACK_NETS: LazyLock<[IpNetwork; 2]> = LazyLock::new(|| {
    [
        IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(127, 0, 0, 0), 8).unwrap()),
        IpNetwork::V6(Ipv6Network::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 128).unwrap()),
    ]
});

#[cfg(all(unix, not(target_os = "android")))]
const DHCPV4_SERVER_PORT: u16 = 67;
#[cfg(all(unix, not(target_os = "android")))]
const DHCPV4_CLIENT_PORT: u16 = 68;
#[cfg(all(unix, not(target_os = "android")))]
const DHCPV6_SERVER_PORT: u16 = 547;
#[cfg(all(unix, not(target_os = "android")))]
const DHCPV6_CLIENT_PORT: u16 = 546;
#[cfg(all(unix, not(target_os = "android")))]
const ROOT_UID: u32 = 0;

/// Returns whether an address belongs to a private subnet.
pub fn is_local_address(address: &IpAddr) -> bool {
    let address = *address;
    (*ALLOWED_LAN_NETS)
        .iter()
        .chain(&*LOOPBACK_NETS)
        .any(|net| net.contains(address))
}

/// A enum that describes network security strategy
///
/// # Firewall block/allow specification.
///
/// See the [security](../../../docs/security.md) document for the specification on how to
/// implement these policies and what should and should not be allowed to flow.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FirewallPolicy {
    /// Allow traffic only to server
    Connecting {
        /// The peer endpoint that should be allowed.
        peer_endpoint: AllowedEndpoint,
        /// Metadata about the tunnel and tunnel interface.
        tunnel: Option<crate::tunnel::TunnelMetadata>,
        /// Flag setting if communication with LAN networks should be possible.
        allow_lan: bool,
        /// Host that should be reachable while connecting.
        allowed_endpoint: AllowedEndpoint,
        /// Networks for which to permit in-tunnel traffic.
        allowed_tunnel_traffic: AllowedTunnelTraffic,
        /// Interface to redirect (VPN tunnel) traffic to
        #[cfg(target_os = "macos")]
        redirect_interface: Option<String>,
    },

    /// Allow traffic only to server and over tunnel interface
    Connected {
        /// The peer endpoint that should be allowed.
        peer_endpoint: AllowedEndpoint,
        /// Metadata about the tunnel and tunnel interface.
        tunnel: crate::tunnel::TunnelMetadata,
        /// Flag setting if communication with LAN networks should be possible.
        allow_lan: bool,
        /// Servers that are allowed to respond to DNS requests.
        #[cfg(not(target_os = "android"))]
        dns_config: ResolvedDnsConfig,
        /// Interface to redirect (VPN tunnel) traffic to
        #[cfg(target_os = "macos")]
        redirect_interface: Option<String>,
    },

    /// Block all network traffic in and out from the computer.
    Blocked {
        /// Flag setting if communication with LAN networks should be possible.
        allow_lan: bool,
        /// Host that should be reachable while in the blocked state.
        allowed_endpoint: Option<AllowedEndpoint>,
    },
}

impl FirewallPolicy {
    /// Return the tunnel peer endpoint, if available
    pub fn peer_endpoint(&self) -> Option<&AllowedEndpoint> {
        match self {
            FirewallPolicy::Connecting { peer_endpoint, .. }
            | FirewallPolicy::Connected { peer_endpoint, .. } => Some(peer_endpoint),
            _ => None,
        }
    }

    /// Return the allowed endpoint, if available
    pub fn allowed_endpoint(&self) -> Option<&AllowedEndpoint> {
        match self {
            FirewallPolicy::Connecting {
                allowed_endpoint, ..
            }
            | FirewallPolicy::Blocked {
                allowed_endpoint: Some(allowed_endpoint),
                ..
            } => Some(allowed_endpoint),
            _ => None,
        }
    }

    /// Return tunnel metadata, if available
    pub fn tunnel(&self) -> Option<&crate::tunnel::TunnelMetadata> {
        match self {
            FirewallPolicy::Connecting {
                tunnel: Some(tunnel),
                ..
            }
            | FirewallPolicy::Connected { tunnel, .. } => Some(tunnel),
            _ => None,
        }
    }

    /// Return allowed in-tunnel traffic
    pub fn allowed_tunnel_traffic(&self) -> &AllowedTunnelTraffic {
        match self {
            FirewallPolicy::Connecting {
                allowed_tunnel_traffic,
                ..
            } => allowed_tunnel_traffic,
            FirewallPolicy::Connected { .. } => &AllowedTunnelTraffic::All,
            _ => &AllowedTunnelTraffic::None,
        }
    }

    /// Return whether LAN traffic is allowed
    pub fn allow_lan(&self) -> bool {
        match self {
            FirewallPolicy::Connecting { allow_lan, .. }
            | FirewallPolicy::Connected { allow_lan, .. }
            | FirewallPolicy::Blocked { allow_lan, .. } => *allow_lan,
        }
    }

    /// Return the interface to redirect (VPN tunnel) traffic to, if any.
    #[cfg(target_os = "macos")]
    pub fn redirect_interface(&self) -> Option<&str> {
        match self {
            FirewallPolicy::Connecting {
                redirect_interface, ..
            } => redirect_interface.as_deref(),
            FirewallPolicy::Connected {
                redirect_interface, ..
            } => redirect_interface.as_deref(),
            FirewallPolicy::Blocked { .. } => None,
        }
    }
}

impl fmt::Display for FirewallPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FirewallPolicy::Connecting {
                peer_endpoint,
                tunnel,
                allow_lan,
                allowed_endpoint,
                allowed_tunnel_traffic,
                ..
            } => {
                if let Some(tunnel) = tunnel {
                    write!(
                        f,
                        "Connecting to {} over \"{}\" (ip: {}, v4 gw: {}, v6 gw: {:?}, allowed in-tunnel traffic: {}), {} LAN. Allowing endpoint {}",
                        peer_endpoint,
                        tunnel.interface,
                        tunnel
                            .ips
                            .iter()
                            .map(|ip| ip.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                        tunnel.ipv4_gateway,
                        tunnel.ipv6_gateway,
                        allowed_tunnel_traffic,
                        if *allow_lan { "Allowing" } else { "Blocking" },
                        allowed_endpoint,
                    )
                } else {
                    write!(
                        f,
                        "Connecting to {}, {} LAN, interface: none. Allowing endpoint {}",
                        peer_endpoint,
                        if *allow_lan { "Allowing" } else { "Blocking" },
                        allowed_endpoint,
                    )
                }
            }
            FirewallPolicy::Connected {
                peer_endpoint,
                tunnel,
                allow_lan,
                ..
            } => write!(
                f,
                "Connected to {} over \"{}\" (ip: {}, v4 gw: {}, v6 gw: {:?}), {} LAN",
                peer_endpoint,
                tunnel.interface,
                tunnel
                    .ips
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                tunnel.ipv4_gateway,
                tunnel.ipv6_gateway,
                if *allow_lan { "Allowing" } else { "Blocking" }
            ),
            FirewallPolicy::Blocked {
                allow_lan,
                allowed_endpoint,
                ..
            } => write!(
                f,
                "Blocked. {} LAN. Allowing endpoint: {}",
                if *allow_lan { "Allowing" } else { "Blocking" },
                allowed_endpoint
                    .as_ref()
                    .map(|endpoint| -> &dyn std::fmt::Display { endpoint })
                    .unwrap_or(&"none"),
            ),
        }
    }
}

/// Manages network security of the computer/device. Can apply and enforce firewall policies
/// by manipulating the OS firewall and DNS settings.
pub struct Firewall {
    inner: imp::Firewall,
}

/// Arguments required when first initializing the firewall.
pub struct FirewallArguments {
    /// Initial firewall state to enter during init.
    pub initial_state: InitialFirewallState,
    /// This argument is required for the blocked state to configure the firewall correctly.
    pub allow_lan: bool,
    /// Specifies the firewall mark used to identify traffic that is allowed to be excluded from
    /// the tunnel and _leaked_ during blocked states.
    #[cfg(target_os = "linux")]
    pub fwmark: u32,
}

/// State to enter during firewall init.
pub enum InitialFirewallState {
    /// Do not set any policy.
    None,
    /// Atomically enter the blocked state.
    Blocked(AllowedEndpoint),
}

impl Firewall {
    /// Creates a firewall instance with the given arguments.
    pub fn from_args(args: FirewallArguments) -> Result<Self, Error> {
        Ok(Firewall {
            inner: imp::Firewall::from_args(args)?,
        })
    }

    /// Createsa new firewall instance.
    pub fn new(#[cfg(target_os = "linux")] fwmark: u32) -> Result<Self, Error> {
        Ok(Firewall {
            inner: imp::Firewall::new(
                #[cfg(target_os = "linux")]
                fwmark,
            )?,
        })
    }

    /// Applies and starts enforcing the given `FirewallPolicy` Makes sure it is being kept in place
    /// until this method is called again with another policy, or until `reset_policy` is called.
    pub fn apply_policy(&mut self, policy: FirewallPolicy) -> Result<(), Error> {
        log::info!("Applying firewall policy: {}", policy);
        self.inner.apply_policy(policy)
    }

    /// Resets/removes any currently enforced `FirewallPolicy`. Returns the system to the same state
    /// it had before any policy was applied through this `Firewall` instance.
    pub fn reset_policy(&mut self) -> Result<(), Error> {
        log::info!("Resetting firewall policy");
        self.inner.reset_policy()
    }

    /// Sets whether the firewall should persist the blocking rules across a reboot.
    #[cfg(target_os = "windows")]
    pub fn persist(&mut self, persist: bool) {
        self.inner.persist(persist);
    }
}

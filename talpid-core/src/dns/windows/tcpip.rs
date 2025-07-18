use crate::dns::{DnsMonitorT, ResolvedDnsConfig};
use std::{io, net::IpAddr};
use talpid_types::ErrorExt;
use talpid_windows::net::{guid_from_luid, luid_from_alias};
use windows_sys::{Win32::System::Com::StringFromGUID2, core::GUID};
use winreg::{
    RegKey,
    enums::{HKEY_LOCAL_MACHINE, KEY_SET_VALUE},
    transaction::Transaction,
};

/// Errors that can happen when configuring DNS on Windows.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Failure to obtain an interface LUID given an alias.
    #[error("Failed to obtain LUID for the interface alias")]
    ObtainInterfaceLuid(#[source] io::Error),

    /// Failure to obtain an interface GUID.
    #[error("Failed to obtain GUID for the interface")]
    ObtainInterfaceGuid(#[source] io::Error),

    /// Failure to flush DNS cache.
    #[error("Failed to flush DNS resolver cache")]
    FlushResolverCache(#[source] super::dnsapi::Error),

    /// Failed to update DNS servers for interface.
    #[error("Failed to update interface DNS servers")]
    SetResolvers(#[source] io::Error),
}

pub struct DnsMonitor {
    current_guid: Option<GUID>,
    should_flush: bool,
}

impl DnsMonitorT for DnsMonitor {
    type Error = Error;

    fn new() -> Result<Self, Error> {
        Ok(DnsMonitor {
            current_guid: None,
            should_flush: true,
        })
    }

    fn set(&mut self, interface: &str, config: ResolvedDnsConfig) -> Result<(), Error> {
        let servers = config.tunnel_config();

        let guid = guid_from_luid(&luid_from_alias(interface).map_err(Error::ObtainInterfaceLuid)?)
            .map_err(Error::ObtainInterfaceGuid)?;
        set_dns(&guid, servers)?;
        self.current_guid = Some(guid);
        if self.should_flush {
            flush_dns_cache()?;
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<(), Error> {
        if let Some(guid) = self.current_guid.take() {
            let mut result = set_dns(&guid, &[]);
            if self.should_flush {
                result = result.and(flush_dns_cache());
            }
            return result;
        }
        Ok(())
    }
}

impl DnsMonitor {
    pub fn disable_flushing(&mut self) {
        self.should_flush = false;
    }
}

fn set_dns(interface: &GUID, servers: &[IpAddr]) -> Result<(), Error> {
    let transaction = Transaction::new().map_err(Error::SetResolvers)?;
    let result = match set_dns_inner(&transaction, interface, servers) {
        Ok(()) => transaction.commit(),
        Err(error) => transaction.rollback().and(Err(error)),
    };
    result.map_err(Error::SetResolvers)
}

fn set_dns_inner(
    transaction: &Transaction,
    interface: &GUID,
    servers: &[IpAddr],
) -> io::Result<()> {
    let guid_str = string_from_guid(interface);

    config_interface(
        transaction,
        &guid_str,
        "Tcpip",
        servers.iter().filter(|addr| addr.is_ipv4()).copied(),
    )?;

    config_interface(
        transaction,
        &guid_str,
        "Tcpip6",
        servers.iter().filter(|addr| addr.is_ipv6()).copied(),
    )?;

    Ok(())
}

fn config_interface(
    transaction: &Transaction,
    guid: &str,
    service: &str,
    nameservers: impl Iterator<Item = IpAddr>,
) -> io::Result<()> {
    let nameservers = nameservers
        .map(|addr| addr.to_string())
        .collect::<Vec<String>>();

    let reg_path =
        format!(r#"SYSTEM\CurrentControlSet\Services\{service}\Parameters\Interfaces\{guid}"#,);
    let adapter_key = match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_transacted_with_flags(
        reg_path,
        transaction,
        KEY_SET_VALUE,
    ) {
        Ok(adapter_key) => Ok(adapter_key),
        Err(error) => {
            if nameservers.is_empty() && error.kind() == io::ErrorKind::NotFound {
                return Ok(());
            }
            Err(error)
        }
    }?;

    if !nameservers.is_empty() {
        adapter_key.set_value("NameServer", &nameservers.join(","))?;
    } else {
        adapter_key.delete_value("NameServer").or_else(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(error)
            }
        })?;
    }

    // Try to disable LLMNR on the interface
    if let Err(error) = adapter_key.set_value("EnableMulticast", &0u32) {
        log::error!(
            "{}\nService: {service}",
            error.display_chain_with_msg("Failed to disable LLMNR on the tunnel interface")
        );
    }

    Ok(())
}

fn flush_dns_cache() -> Result<(), Error> {
    super::dnsapi::flush_resolver_cache().map_err(Error::FlushResolverCache)
}

/// Obtain a string representation for a GUID object.
fn string_from_guid(guid: &GUID) -> String {
    let mut buffer = [0u16; 40];

    let length =
        // SAFETY: `guid` and `buffer` are valid references.
        // StringFromGUID2 won't write past the end of the provided length.
        unsafe { StringFromGUID2(guid, buffer.as_mut_ptr(), buffer.len() as i32 - 1) } as usize;

    // cannot fail because `buffer` is large enough
    assert!(length > 0);
    let length = length - 1;
    String::from_utf16(&buffer[0..length]).unwrap()
}

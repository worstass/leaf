use std::io;
use std::net::{IpAddr, SocketAddr};
use smoltcp::wire::IpAddress;


pub fn broken_pipe() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe")
}

#[derive(Debug, Clone)]
pub(crate) enum HostName {
    DomainName(String),
    Ip(IpAddr),
}

impl ToString for HostName {
    fn to_string(&self) -> String {
        match self {
            HostName::DomainName(s) => s.clone(),
            HostName::Ip(ip) => ip.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DestinationAddr {
    pub host: HostName,
    pub port: u16,
}

impl HostName {
    pub fn set_domain_name(&mut self, mut domain_name: String) -> Result<(), String> {
        // use trust_dns_resolver::Name;
        domain_name.make_ascii_lowercase();
        *self = HostName::DomainName(
            domain_name,
            // Name::from_utf8(&domain_name)
            //     .map_err(|_| domain_name)?
            //     .to_ascii(),
        );
        Ok(())
    }
    pub fn from_domain_name(domain_name: String) -> Result<Self, String> {
        let mut res = HostName::DomainName(String::new());
        res.set_domain_name(domain_name)?;
        Ok(res)
    }
}

impl From<SocketAddr> for DestinationAddr {
    fn from(socket: SocketAddr) -> Self {
        Self {
            host: HostName::Ip(socket.ip()),
            port: socket.port(),
        }
    }
}

pub fn smoltcp_addr_to_std(addr: IpAddress) -> IpAddr {
    match addr {
        IpAddress::Ipv4(ip) => IpAddr::V4(ip.into()),
        IpAddress::Ipv6(ip) => IpAddr::V6(ip.into()),
        _ => panic!("Cannot convert unknown smoltcp address to std address"),
    }
}

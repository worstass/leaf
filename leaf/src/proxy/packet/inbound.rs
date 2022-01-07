use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use log::*;
use protobuf::Message;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as TokioMutex;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::codec::Framed;

use crate::{
    app::dispatcher::Dispatcher,
    app::fake_dns::{FakeDns, FakeDnsMode},
    app::nat_manager::NatManager,
    config::{Inbound, PacketInboundSettings},
    option, Runner,
};

use crate::proxy::tun::netstack::NetStack;

use bytes::{BufMut, Bytes, BytesMut};
use std::net::SocketAddr;
#[cfg(unix)]
use std::os::unix::io::FromRawFd;
use std::pin::Pin;
use tokio::fs::File;

use crate::config::PacketInboundSettings_Sink;
use crate::proxy::packet::UdpSink;
use crate::proxy::packet::Sink;

#[cfg(unix)]
use crate::proxy::packet::FdSink;

#[cfg(unix)]
fn sink_from_fd(fd: i32) -> Result<Pin<Box<dyn Sink>>> {
    Ok(Box::pin(FdSink::new(fd)))
}

fn sink_from_udp(local_port: u32, remote_port: u32) -> Result<Pin<Box<dyn Sink>>>
{
    let local_addr: SocketAddr = format!("127.0.0.1:{}", local_port).parse()?;
    let remote_addr: SocketAddr = format!("127.0.0.1:{}", remote_port).parse()?;
    let sock = std::net::UdpSocket::bind(&local_addr)?;
    sock.connect(remote_addr)?;
    Ok(Box::pin(UdpSink::new(sock)))
}

impl Sink for File {}

#[cfg(unix)]
fn sink_from_pipe(pipe: &str) -> Result<Pin<Box<dyn Sink>>> {
    Ok(Box::pin(unsafe { File::from_raw_fd(21) }))
}

pub fn new(
    inbound: Inbound,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
) -> Result<Runner> {
    let settings = PacketInboundSettings::parse_from_bytes(&inbound.settings)?;
    Ok(Box::pin(async move {
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(FakeDnsMode::Include)));
        let mut stack = NetStack::new(inbound.tag.clone(), dispatcher, nat_manager, fakedns);
        let mut sink = match settings.sink {
            #[cfg(unix)]
            PacketInboundSettings_Sink::FD => Some(sink_from_fd(settings.fd).unwrap()),
            #[cfg(unix)]
            PacketInboundSettings_Sink::PIPE => Some(sink_from_pipe(settings.pipe.as_str()).unwrap()),
            PacketInboundSettings_Sink::UDP => Some(sink_from_udp(settings.local_port, settings.remote_port).unwrap()),
            #[cfg(not(unix))] _ => None,
        }.expect("Packet sink creation failed");
        info!("packet inbound started, using sink {:?}", settings.sink);
        match tokio::io::copy_bidirectional(&mut sink, &mut stack).await {
            Ok((u, d)) => info!("packet inbound done - up: {}, down: {}", u, d),
            Err(e) => debug!("packet inbound error: {:?}", e)
        }
    }))
}

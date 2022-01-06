use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use log::*;
use protobuf::Message;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as TokioMutex;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::codec::Framed;
// use tokio::net::UdpSocket;

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
use tun::platform::posix::Fd;
use crate::config::PacketInboundSettings_Sink;
use crate::proxy::packet::UdpSink;
use crate::proxy::packet::Sink;
use crate::proxy::packet::FdSink;

#[cfg(unix)]
fn sink_from_fd(fd: i32) -> Result<Pin<Box<dyn Sink>>> {
    Ok(Box::pin(FdSink::new(fd)))
}

fn sink_from_udp(local_port: u32, remote_port: u32) -> Result<Pin<Box<dyn Sink>>>
{
    let local_addr: SocketAddr = format!("127.0.0.1:{}", local_port).parse()?;
    let remote_addr: SocketAddr = format!("127.0.0.1:{}", local_addr).parse()?;
    let sock = std::net::UdpSocket::bind(&local_addr)?;
    sock.connect(remote_addr)?;
    Ok(Box::pin(UdpSink::new(sock)))
}

impl Sink for File {}

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
            PacketInboundSettings_Sink::FD => sink_from_fd(settings.fd).unwrap(),
            PacketInboundSettings_Sink::UDP => sink_from_udp(settings.local_port, settings.remote_port).unwrap(),
            PacketInboundSettings_Sink::PIPE => sink_from_pipe(settings.pipe.as_str()).unwrap()
        };
        info!("packet inbound started");
        tokio::io::copy_bidirectional(&mut sink, &mut stack);
        info!("packet inbound exited");
    }))
}

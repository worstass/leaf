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
use std::net::{SocketAddr, TcpStream};
#[cfg(unix)]
use std::os::unix::io::FromRawFd;
use std::pin::Pin;
use tokio::fs::File;

use crate::config::PacketInboundSettings_Sink;
use crate::proxy::packet::{TcpSink, UdpSink};
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
    debug!("udp sink listen on {}", sock.local_addr()?);
    // sock.connect(remote_addr)?;
    Ok(Box::pin(UdpSink::new(sock)))
}

fn sink_from_tcp(port: u32) -> Result<Pin<Box<dyn Sink>>>
{
    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", port))?;
    let (stream, _) = listener.accept()?;
    debug!("tcp sink listen on {}", port);
    Ok(Box::pin(TcpSink::new(stream)))
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
        info!("packet inbound started");
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(FakeDnsMode::Include)));
        let mut stack = NetStack::new(inbound.tag.clone(), dispatcher, nat_manager, fakedns);
        let mut sink = match settings.sink {
            #[cfg(unix)]
            PacketInboundSettings_Sink::FD =>{
                debug!("using packet fd sink with fd={}", settings.fd);
                Some(sink_from_fd(settings.fd).unwrap())
            } ,
            #[cfg(unix)]
            PacketInboundSettings_Sink::PIPE =>{
                debug!("using packet pipe sink with pipe={}", settings.pipe);
                Some(sink_from_pipe(settings.pipe.as_str()).unwrap())
            },
            PacketInboundSettings_Sink::UDP => {
                debug!("using packet udp sink with ports: local={}, remote={}", settings.local_port, settings.remote_port);
                // Some(sink_from_udp(settings.local_port, settings.remote_port).unwrap())
                Some(sink_from_tcp(settings.local_port).unwrap())
            },
            #[cfg(not(unix))] _ => None,
        }.expect("Packet sink creation failed");
        match tokio::io::copy_bidirectional(&mut sink, &mut stack).await {
            Ok((u, d)) => info!("packet inbound done - up: {}, down: {}", u, d),
            Err(e) => debug!("packet inbound exited with error: {:?}", e)
        }
    }))
}

use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use log::*;
use protobuf::Message;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as TokioMutex;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::codec::Framed;
use tokio::net::UdpSocket;

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
use tokio::fs::File;
use tun::platform::posix::Fd;
use crate::config::PacketInboundSettings_Sink;

#[cfg(unix)]
fn sink_from_fd(fd: i32) -> Result<impl AsyncRead + AsyncWrite + Unpin>
{
    let mut fd_sink = unsafe { File::from_raw_fd(fd) };
    Ok(fd_sink)
}

fn sink_from_udp(local_port: u32, remote_port: u32) -> Result<impl AsyncRead + AsyncWrite + Unpin>
{
    let local_addr: SocketAddr = format!("127.0.0.1:{}", local_port).parse()?;
    let remote_addr: SocketAddr = format!("127.0.0.1:{}", local_addr).parse()?;
    let sock = tokio::net::UdpSocket::bind(&local_addr).await?;
    sock.connect(remote_addr).await?;
    Ok(sock)
}

fn sink_from_pipe(pipe: &str) -> Result<impl AsyncRead + AsyncWrite + Unpin>
{

    // todo!()

    Ok(unsafe { File::from_raw_fd(21) })
}

pub fn new(
    inbound: Inbound,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
) -> Result<Runner> {
    let settings = PacketInboundSettings::parse_from_bytes(&inbound.settings)?;
    Ok(Box::pin(async move {
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(FakeDnsMode::Include)));
        let mut sink = match settings.sink {
            #[cfg(unix)]
            PacketInboundSettings_Sink::FD => sink_from_fd(settings.fd),
            PacketInboundSettings_Sink::UDP => sink_from_udp(settings.local_port, settings.remote_port),
            PacketInboundSettings_Sink::PIPE => sink_from_pipe(settings.pipe.as_str())
        };
        let mut stack = NetStack::new(inbound.tag.clone(), dispatcher, nat_manager, fakedns);
        // let (mut stack_reader, mut stack_writer) = io::split(stack);
        // let packet_sink = sock.clone();
        // let mtu = 1500;
        // // let s2t = Box::pin(async move {
        // //     let mut buf = vec![0; mtu as usize];
        // //     loop {
        // //         match stack_reader.read(&mut buf).await {
        // //             Ok(0) => {
        // //                 debug!("read stack eof");
        // //                 return;
        // //             }
        // //             Ok(n) => match packet_sink.send(&buf[..n]).await {
        // //                 Ok(_) => (),
        // //                 Err(e) => {
        // //                     warn!("send pkt to tun failed: {}", e);
        // //                     return;
        // //                 }
        // //             },
        // //             Err(err) => {
        // //                 warn!("read stack failed {:?}", err);
        // //                 return;
        // //             }
        // //         }
        // //     }
        // // });
        // let packet_stream = sock.clone();
        // // let t2s = Box::pin(async move {
        // //     let mut buf = vec![0; mtu as usize];
        // //     loop {
        // //         match packet_stream.recv(&mut buf).await {
        // //             Ok(0) => {
        // //                 debug!("read stack eof");
        // //                 return;
        // //             }
        // //             Err(err) => {
        // //                 warn!("read stack failed {:?}", err);
        // //                 return;
        // //             }
        // //             Ok(n) => match stack_writer.write(&buf[..n]).await {
        // //                 Ok(_) => (),
        // //                 Err(e) => {
        // //                     warn!("send pkt to tun failed: {}", e);
        // //                     return;
        // //                 }
        // //             },
        // //         }
        // //     }
        // // });
        info!("packet inbound started");
        tokio::io::copy_bidirectional(&mut sink, &mut stack);
        info!("packet inbound exited");
    }))
}
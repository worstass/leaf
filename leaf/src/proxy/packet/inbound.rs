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
    proxy::tun::netstack::NetStack,
};

use bytes::{BufMut, Bytes, BytesMut};
use std::net::SocketAddr;

pub fn new(
    inbound: Inbound,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
) -> Result<Runner> {
    let settings = PacketInboundSettings::parse_from_bytes(&inbound.settings)?;

    Ok(Box::pin(async move {
        // let sock = UdpSocket::bind("0.0.0.0:8081").await?;
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(FakeDnsMode::Include)));
        let remote_port = settings.port;
        let local_port = settings.port;
        let remote_addr: SocketAddr = format!("127.0.0.1:{}", remote_port).parse().unwrap();
        let sock = Arc::new(tokio::net::UdpSocket::bind(&remote_addr).await.unwrap());
        let local_addr: SocketAddr = format!("127.0.0.1:{}", local_port).parse().unwrap();
        sock.connect(local_addr).await.unwrap();
        let stack = NetStack::new(inbound.tag.clone(), dispatcher, nat_manager, fakedns);
        let (mut stack_reader, mut stack_writer) = io::split(stack);

        let packet_sink = sock.clone();

        let mtu = 1500;
        let s2t = Box::pin(async move {
            let mut buf = vec![0; mtu as usize];
            loop {
                match stack_reader.read(&mut buf).await {
                    Ok(0) => {
                        debug!("read stack eof");
                        return;
                    }
                    Ok(n) => match packet_sink.send(&buf[..n]).await {
                        Ok(_) => (),
                        Err(e) => {
                            warn!("send pkt to tun failed: {}", e);
                            return;
                        }
                    },
                    Err(err) => {
                        warn!("read stack failed {:?}", err);
                        return;
                    }
                }
            }
        });
        let packet_stream = sock.clone();
        let t2s = Box::pin(async move {
            let mut buf = vec![0; mtu as usize];
            loop {
                match packet_stream.recv(&mut buf).await {
                    Ok(0) => {
                        debug!("read stack eof");
                        return;
                    }
                    Err(err) => {
                        warn!("read stack failed {:?}", err);
                        return;
                    }
                    Ok(n) => match stack_writer.write(&buf[..n]).await {
                        Ok(_) => (),
                        Err(e) => {
                            warn!("send pkt to tun failed: {}", e);
                            return;
                        }
                    },
                }
            }
        });

        info!("packet inbound started");
        futures::future::select(t2s, s2t).await;
        info!("packet inbound exited");
    }))
}
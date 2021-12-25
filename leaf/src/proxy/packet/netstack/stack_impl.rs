use std::{
    io,
    net::SocketAddr,
    os::raw,
    pin::Pin,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Once,
    },
    time,
};
use std::collections::BTreeMap;
use log::*;
use futures::task::{Context, Poll};
use smoltcp::iface::{InterfaceBuilder, NeighborCache};
use smoltcp::phy::{Medium, RawSocket};
use tokio::{
    self,
    io::{AsyncRead, AsyncWrite, ReadBuf},
};
use tokio::sync::Mutex as TokioMutex;

use crate::{Dispatcher, NatManager};
use crate::app::fake_dns::FakeDns;

use smoltcp::Result;
use smoltcp::socket::{TcpSocket, TcpSocketBuffer, UdpPacketMetadata, UdpSocket, UdpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{IpAddress, IpCidr};
use crate::proxy::packet::netstack::endpoint::Endpoint;

pub struct NetStackImpl {

}

impl NetStackImpl {
    pub fn new(
        inbound_tag: String,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
        fakedns: Arc<TokioMutex<FakeDns>>,
    ) -> Box<Self> {
        let device = Endpoint::new();

        let ip_addrs = [
            IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24),
        ];
        let mut builder = InterfaceBuilder::new(device, vec![])
            .any_ip(true)
            .ip_addrs(ip_addrs);
        let mut iface = builder.finalize();
        let udp_rx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 64]);
        let udp_tx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 128]);
        let udp_socket = UdpSocket::new(udp_rx_buffer, udp_tx_buffer);
        let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 64]);
        let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 128]);
        let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

        let udp_handle = iface.add_socket(udp_socket);
        let tcp_handle = iface.add_socket(tcp_socket);

        tokio::spawn(async move {
            loop {
                let timestamp = Instant::now();
                match iface.poll(timestamp) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("poll error: {}", e);
                    }
                }

                let socket = iface.get_socket::<UdpSocket>(udp_handle);
                if !socket.is_open() {
                    socket.bind(6969).unwrap()
                }

                let socket = iface.get_socket::<TcpSocket>(tcp_handle);
                if !socket.is_open() {
                    socket.listen(6969).unwrap();
                }

            }
        });
        // let socket = iface.get_socket::<TcpSocket>(tcp_handle);
        // if socket.is_active() && !tcp_active {
        //     debug!("connected");
        // } else if !socket.is_active() && tcp_active {
        //     debug!("disconnected");
        // }
        // // socket.listen()
        // let socket1 = iface.get_socket::<UdpSocket>(tcp_handle);

        let stack = Box::new(NetStackImpl {
            // lwip_lock: Arc::new(AtomicMutex::new()),
            // waker: None,
            // tx,
            // rx,
            // dispatcher,
            // nat_manager,
            // fakedns,
        });
        stack
    }
}

impl Drop for NetStackImpl {
    fn drop(&mut self) {
        log::trace!("drop netstack");
        // unsafe {
        //     let _g = self.lwip_lock.lock();
        //     OUTPUT_CB_PTR = 0x0;
        // };
    }
}

impl AsyncRead for NetStackImpl {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for NetStackImpl {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(0))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
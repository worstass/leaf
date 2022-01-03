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
use std::task::Waker;
use log::*;
use futures::task::{Context, Poll};
use smoltcp::iface::{InterfaceBuilder, NeighborCache};
use smoltcp::phy::{Medium};
use tokio::{
    self,
    io::{AsyncRead, AsyncWrite, ReadBuf},
};
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::mpsc::channel as tokio_channel;

use crate::{Dispatcher, NatManager};
use crate::app::fake_dns::FakeDns;

use smoltcp::Result;
use smoltcp::socket::{TcpSocket, TcpSocketBuffer, UdpPacketMetadata, UdpSocket, UdpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{IpAddress, IpCidr};
use crate::app::nat_manager::UdpPacket;
use crate::common::mutex::AtomicMutex;
use crate::proxy::packet::netstack::endpoint::Endpoint;
use crate::proxy::packet::netstack::tcp_stream::TcpStream;
use crate::session::{Network, Session, SocksAddr};

pub struct NetStackImpl {
    waker: Option<Waker>,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
    fakedns: Arc<TokioMutex<FakeDns>>,
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

        let dispatcher = dispatcher.clone();

        tokio::spawn(async move {
            loop {
                let timestamp = Instant::now();
                match iface.poll(timestamp) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("poll error: {}", e);
                    }
                }

                let tcp_socket = iface.get_socket::<TcpSocket>(tcp_handle);
                if !tcp_socket.is_open() {
                    tcp_socket.listen(6969).unwrap();
                }
                let dispatcher = dispatcher.clone();
                tokio::spawn(async move {
                    let mut sess = Session {
                        network: Network::Tcp,
                        source: tcp_socket.remote_endpoint().addr.into(),
                        local_addr: tcp_socket.local_endpoint().addr.into(),
                        destination: SocksAddr::Ip(*tcp_socket.remote_endpoint().addr.into()),
                        inbound_tag: inbound_tag.clone(),
                        ..Default::default()
                    };
                    if fakedns.lock().await.is_fake_ip(&tcp_socket.remote_endpoint().addr.into()) {
                        if let Some(domain) = fakedns
                            .lock()
                            .await
                            .query_domain(&tcp_socket.remote_endpoint().addr.into())
                        {
                            sess.destination =
                                SocksAddr::Domain(domain, tcp_socket.remote_endpoint().port);
                        } else {
                            // Although requests targeting fake IPs are assumed
                            // never happen in real network traffic, which are
                            // likely caused by poisoned DNS cache records, we
                            // still have a chance to sniff the request domain
                            // for TLS traffic in dispatcher.
                            if tcp_socket.remote_endpoint().port != 443 {
                                return;
                            }
                        }
                    }

                    dispatcher
                        .dispatch_tcp(&mut sess, tcp_socket)
                        .await;
                });
                let udp_socket = iface.get_socket::<UdpSocket>(udp_handle);
                if !udp_socket.is_open() {
                    udp_socket.bind(6969).unwrap()
                }
                let (client_ch_tx, mut client_ch_rx): (
                    TokioSender<UdpPacket>,
                    TokioReceiver<UdpPacket>,
                ) = tokio_channel(32);
                tokio::spawn(async move {
                    while let Some(pkt) = client_ch_rx.recv().await {

                    }
                });
            }
        });

        let stack = Box::new(NetStackImpl {
            waker: None,
            tx,
            rx,
            dispatcher,
            nat_manager,
            fakedns,
        });

        stack
    }
}

pub fn send_udp(
    lwip_lock: Arc<AtomicMutex>,
    src_addr: &SocketAddr,
    dst_addr: &SocketAddr,
    pcb: usize,
    data: &[u8],
) {
    unsafe {
        let _g = lwip_lock.lock();
        let pbuf = pbuf_alloc_reference(
            data as *const [u8] as *mut [u8] as *mut raw::c_void,
            data.len() as u16_t,
            pbuf_type_PBUF_ROM,
        );
        let src_ip = match util::to_ip_addr_t(&src_addr.ip()) {
            Ok(v) => v,
            Err(e) => {
                warn!("convert ip failed: {}", e);
                return;
            }
        };
        let dst_ip = match util::to_ip_addr_t(&dst_addr.ip()) {
            Ok(v) => v,
            Err(e) => {
                warn!("convert ip failed: {}", e);
                return;
            }
        };
        let err = udp_sendto(
            pcb as *mut udp_pcb,
            pbuf,
            &dst_ip as *const ip_addr_t,
            dst_addr.port() as u16_t,
            &src_ip as *const ip_addr_t,
            src_addr.port() as u16_t,
        );
        if err != err_enum_t_ERR_OK as err_t {
            warn!("udp_sendto err {}", err);
        }
        // pbuf_free(pbuf);
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
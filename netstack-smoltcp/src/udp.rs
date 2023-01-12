use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use futures::{Stream, StreamExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};


type UdpPkt = (Vec<u8>, SocketAddr, SocketAddr);

pub struct UdpSocket {
    // pcb: usize,
    waker: Option<Waker>,
    tx: Sender<UdpPkt>,
    rx: Receiver<UdpPkt>,
}


impl UdpSocket {
    pub(crate) fn new(buffer_size: usize) -> Box<Self> {
        let (tx, rx): (Sender<UdpPkt>, Receiver<UdpPkt>) = channel(buffer_size);
        let socket = Box::new(Self {
            waker: None,
            tx,
            rx,
        });
        socket
    }

    pub fn split(self: Box<Self>) -> (SendHalf, RecvHalf) {
        (SendHalf {  }, RecvHalf { socket: self })
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
    }
}

impl Stream for UdpSocket {
    type Item = UdpPkt;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(pkt)) => Poll::Ready(Some(pkt)),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub struct SendHalf {
    // pub(crate) pcb: usize,
}

impl SendHalf {
    pub fn send_to(
        &self,
        data: &[u8],
        src_addr: &SocketAddr,
        dst_addr: &SocketAddr,
    ) -> io::Result<()> {
        send_udp(src_addr, dst_addr, data)
    }
}

pub struct RecvHalf {
    pub(crate) socket: Box<UdpSocket>,
}

impl RecvHalf {
    pub async fn recv_from(&mut self) -> io::Result<UdpPkt> {
        match self.socket.next().await {
            Some(pkt) => Ok(pkt),
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("recv_from udp socket faied: tx closed"),
            )),
        }
    }
}

impl Stream for RecvHalf {
    type Item = UdpPkt;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.socket).poll_next(cx)
    }
}

fn send_udp(
    src_addr: &SocketAddr,
    dst_addr: &SocketAddr,
    data: &[u8],
) -> io::Result<()> {
    Ok(())
}
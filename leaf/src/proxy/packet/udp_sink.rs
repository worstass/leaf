use std::io;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::ready;
use libc::SYS_epoll_wait;
use tokio::io::{AsyncRead, AsyncWrite, Interest, ReadBuf};
use std::net::UdpSocket;
use futures_util::future::ok;
use crate::proxy::packet::Sink;

pub struct UdpSink {
    inner: Box<tokio::net::UdpSocket>,
}

impl UdpSink {
    pub fn new(udp: std::net::UdpSocket) -> Self {
        let u = tokio::net::UdpSocket::from_std(udp).unwrap();
        Self {
            inner: Box::new(u),
        }
    }
}

impl Sink for UdpSink {
}

impl AsyncRead for UdpSink {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.inner.poll_recv_ready(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(_) => {
                match self.inner.poll_recv(cx, buf) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(res) => Poll::Ready(res)
                }
            }
        }
    }
}

impl AsyncWrite for UdpSink {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        // todo!()
        match self.inner.poll_send_ready(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(_) => {
                match self.inner.poll_send(cx, buf) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(res) => Poll::Ready(res)
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

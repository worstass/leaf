use std::io;
use std::io::Error;
use std::net::UdpSocket;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::ready;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::io::unix::AsyncFd;

pub struct UdpSink {
    inner: AsyncFd<UdpSocket>
}

impl UdpSink {
    pub fn new(udp: UdpSocket)-> io::Result<Self>  {
        udp.set_nonblocking(true)?;
        Ok(Self {
            inner: AsyncFd::new(udp)?,
        })
    }
}

impl AsyncRead for UdpSink {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        todo!()
    }
}

impl AsyncWrite for UdpSink {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        loop {
            let mut guard = ready!(self.inner.poll_write_ready(cx))?;
            match guard.try_io(|inner| inner.get_ref().write(buf)) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
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
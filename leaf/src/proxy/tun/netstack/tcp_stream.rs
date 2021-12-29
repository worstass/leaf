use std::{io, pin::Pin};

use futures::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::tcp_stream_impl::TcpStreamImpl;

pub struct TcpStream {
    inner: Box<TcpStreamImpl>,
}

impl TcpStream {
    pub fn new(stream: Box<TcpStreamImpl>) -> Self {
        TcpStream { inner: stream }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        AsyncWrite::poll_flush(Pin::new(&mut self.inner), cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        AsyncWrite::poll_shutdown(Pin::new(&mut self.inner), cx)
    }
}

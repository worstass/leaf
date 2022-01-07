use std::io;
use std::io::{Error, Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::ready;
use tokio::io::{AsyncRead, AsyncWrite, Interest, ReadBuf};
use futures_util::future::ok;
use crate::proxy::packet::Sink;

pub struct TcpSink {
    inner: Box<tokio::net::TcpStream>,
}

impl TcpSink {
    pub fn new(tcp: std::net::TcpStream) -> Self {
        let u = tokio::net::TcpStream::from_std(tcp).unwrap();
        Self {
            inner: Box::new(u),
        }
    }
}

impl Sink for TcpSink {}

impl AsyncRead for TcpSink {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for TcpSink {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

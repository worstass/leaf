use std::io;
use std::io::Error;

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::io::unix::AsyncFd;
use tokio::fs::File;
#[cfg(unix)]
use std::os::unix::io::{FromRawFd, RawFd};
use crate::proxy::packet::Sink;

pub struct FdSink {
    inner: File,
}

impl FdSink {
    pub fn new(fd: RawFd) -> Self {
        Self {
            inner: unsafe { File::from_raw_fd(fd) },
        }
    }
}

impl Sink for FdSink {}

impl AsyncRead for FdSink {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for FdSink {
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

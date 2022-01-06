use std::io;
use std::io::Error;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::io::unix::AsyncFd;
use tokio::io::File;

pub struct FdSink {
    inner: File,
}

impl FdSink {
    pub fn new(fd: RawFd) -> io::Result<Self> {
        Ok(Self {
            inner: unsafe { File::from_raw_fd(fd)? },
        })
    }
}

impl AsyncRead for FdSink {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.inner.poll_read(cx, buf)
    }
}

impl AsyncWrite for FdSink {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        self.inner.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.inner.poll_flush(cx, buf)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.inner.poll_shutdown(cx, buf)
    }
}
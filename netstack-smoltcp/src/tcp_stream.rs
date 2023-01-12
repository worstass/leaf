use std::cmp::min;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::raw;
use std::pin::Pin;
use std::task::{Context, Poll};
use log::trace;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::io::{AsyncBufReadExt};
use bytes::BytesMut;

fn broken_pipe() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe")
}

pub struct TcpStream {
    src_addr: SocketAddr,
    dest_addr: SocketAddr,
    // inner: Box<TcpStreamImpl>,
    write_buf: BytesMut,
}

impl TcpStream {
    pub(crate) fn new() -> Self {
        let src_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1111);
        let dest_addr =    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 2222);

        let stream = TcpStream {
            src_addr,
            dest_addr,
            // pcb: pcb as usize,
            write_buf: BytesMut::new(),
            // callback_ctx: TcpStreamContext::new(src_addr, dest_addr, read_tx, read_rx),
        };
        stream
    }

    pub fn local_addr(&self) -> &SocketAddr {
        &self.src_addr
    }

    pub fn remote_addr(&self) -> &SocketAddr {
        &self.dest_addr
    }

}

impl AsyncRead for TcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        todo!()
        // let me = &mut *self;
        //
        // // handle any previously unsent data
        // if !me.write_buf.is_empty() {
        //     let to_read = min(buf.remaining(), me.write_buf.len());
        //     let piece = me.write_buf.split_to(to_read);
        //     buf.put_slice(&piece[..to_read]);
        //     return Poll::Ready(Ok(()));
        // }
        // match Pin::new(&mut ctx.read_rx).poll_recv(cx) {
        //     Poll::Ready(Some(data)) => {
        //         let to_read = min(buf.remaining(), data.len());
        //         buf.put_slice(&data[..to_read]);
        //         if to_read < data.len() {
        //             me.write_buf.extend_from_slice(&data[to_read..]);
        //         }
        //         // unsafe { tcp_recved(me.pcb as *mut tcp_pcb, data.len() as u16_t) };
        //         Poll::Ready(Ok(()))
        //     }
        //     Poll::Ready(None) => Poll::Ready(Err(broken_pipe())),
        //     Poll::Pending => {
        //         // no more buffered data
        //         if ctx.eof {
        //             Poll::Ready(Ok(())) // eof
        //         } else {
        //             Poll::Pending
        //         }
        //     }
        // }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        todo!()
        // AsyncWrite::poll_write(Pin::new(&mut self.inner), cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        todo!()
        // AsyncWrite::poll_flush(Pin::new(&mut self.inner), cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        todo!()
        // AsyncWrite::poll_shutdown(Pin::new(&mut self.inner), cx)
    }
}


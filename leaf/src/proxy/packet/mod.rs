use tokio::io::{AsyncRead, AsyncWrite};

pub mod inbound;

pub trait Sink: AsyncRead + AsyncWrite + Send {}

#[cfg(unix)]
mod fd_sink;
#[cfg(unix)]
pub use fd_sink::*;

mod tcp_sink;
pub use tcp_sink::*;

mod udp_sink;
pub use udp_sink::*;

// mod stack;
mod endpoint;



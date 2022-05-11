use std::convert::TryFrom;
use std::io;

use async_trait::async_trait;

use crate::{
    proxy::*,
    session::{Session, SocksAddr},
};

pub struct Handler {
    pub actors: Vec<AnyOutboundHandler>,
}

impl Handler {
    fn next_connect_addr(&self, start: usize) -> OutboundConnect {
        for a in self.actors[start..].iter() {
            match a.tcp() {
                Ok(h) => {
                    let oc = h.connect_addr();
                    if let OutboundConnect::Next = oc {
                        continue;
                    }
                    return oc;
                }
                _ => match a.udp() {
                    Ok(h) => {
                        let oc = h.connect_addr();
                        if let OutboundConnect::Next = oc {
                            continue;
                        }
                        return oc;
                    }
                    _ => (),
                },
            }
        }
        OutboundConnect::Unknown
    }

    fn next_session(&self, mut sess: Session, start: usize) -> Session {
        if let OutboundConnect::Proxy(_, address, port) = self.next_connect_addr(start) {
            if let Ok(addr) = SocksAddr::try_from((address, port)) {
                sess.destination = addr;
            }
        }
        sess
    }
}

#[async_trait]
impl TcpOutboundHandler for Handler {
    fn connect_addr(&self) -> OutboundConnect {
        self.next_connect_addr(0)
    }

    async fn handle<'a>(
        &'a self,
        sess: &'a Session,
        mut stream: Option<AnyStream>,
    ) -> io::Result<AnyStream> {
        for (i, a) in self.actors.iter().enumerate() {
            let new_sess = self.next_session(sess.clone(), i + 1);
            let s = stream.take();
            stream.replace(a.tcp()?.handle(&new_sess, s).await?);
        }
        Ok(stream
            .map(|x| Box::new(x))
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "chain tcp invalid input"))?)
    }
}

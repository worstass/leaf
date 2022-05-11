use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use protobuf::Message;

use crate::app::dispatcher::Dispatcher;
use crate::app::nat_manager::NatManager;
use crate::config;
use crate::proxy;
use crate::proxy::AnyInboundHandler;
use crate::Runner;

#[cfg(feature = "inbound-amux")]
use crate::proxy::amux;
#[cfg(feature = "inbound-http")]
use crate::proxy::http;
#[cfg(feature = "inbound-quic")]
use crate::proxy::quic;
#[cfg(feature = "inbound-shadowsocks")]
use crate::proxy::shadowsocks;
#[cfg(feature = "inbound-socks")]
use crate::proxy::socks;
#[cfg(feature = "inbound-tls")]
use crate::proxy::tls;
#[cfg(feature = "inbound-trojan")]
use crate::proxy::trojan;
#[cfg(feature = "inbound-ws")]
use crate::proxy::ws;

#[cfg(feature = "inbound-chain")]
use crate::proxy::chain;

use super::network_listener::NetworkInboundListener;

#[cfg(all(
    feature = "inbound-tun",
    any(
        target_os = "ios",
        target_os = "android",
        target_os = "macos",
        target_os = "linux",
        target_os = "windows", // MARKER BEGIN - END
    )
))]
use super::tun_listener::TunInboundListener;

// MARKER BEGIN
#[cfg(feature = "inbound-packet")]
use super::packet_listener::PacketInboundListener;
// MARKER END

pub struct InboundManager {
    network_listeners: HashMap<String, NetworkInboundListener>,
    #[cfg(all(
        feature = "inbound-tun",
        any(
            target_os = "ios",
            target_os = "android",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows", // MARKER BEGIN - END
        )
    ))]
    tun_listener: Option<TunInboundListener>,
    // MARKER BEGIN
    #[cfg(feature = "inbound-packet")]
    packet_listener: Option<PacketInboundListener>,
    // MARKER END
    tun_auto: bool,
}

impl InboundManager {
    pub fn new(
        inbounds: &protobuf::RepeatedField<config::Inbound>,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
    ) -> Result<Self> {
        let mut handlers: HashMap<String, AnyInboundHandler> = HashMap::new();

        for inbound in inbounds.iter() {
            let tag = String::from(&inbound.tag);
            match inbound.protocol.as_str() {
                #[cfg(feature = "inbound-socks")]
                "socks" => {
                    let tcp = Arc::new(socks::inbound::TcpHandler);
                    let udp = Arc::new(socks::inbound::UdpHandler);
                    let handler = Arc::new(proxy::inbound::Handler::new(
                        tag.clone(),
                        Some(tcp),
                        Some(udp),
                    ));
                    handlers.insert(tag.clone(), handler);
                }
                #[cfg(feature = "inbound-http")]
                "http" => {
                    let tcp = Arc::new(http::inbound::TcpHandler);
                    let handler =
                        Arc::new(proxy::inbound::Handler::new(tag.clone(), Some(tcp), None));
                    handlers.insert(tag.clone(), handler);
                }
                #[cfg(feature = "inbound-shadowsocks")]
                "shadowsocks" => {
                    let settings =
                        config::ShadowsocksInboundSettings::parse_from_bytes(&inbound.settings)
                            .map_err(|e| anyhow!("invalid [{}] inbound settings: {}", &tag, e))?;
                    let tcp = Arc::new(shadowsocks::inbound::TcpHandler {
                        cipher: settings.method.clone(),
                        password: settings.password.clone(),
                    });
                    let udp = Arc::new(shadowsocks::inbound::UdpHandler {
                        cipher: settings.method.clone(),
                        password: settings.password.clone(),
                    });
                    let handler = Arc::new(proxy::inbound::Handler::new(
                        tag.clone(),
                        Some(tcp),
                        Some(udp),
                    ));
                    handlers.insert(tag.clone(), handler);
                }
                #[cfg(feature = "inbound-trojan")]
                "trojan" => {
                    let settings =
                        config::TrojanInboundSettings::parse_from_bytes(&inbound.settings).unwrap();
                    let tcp = Arc::new(trojan::inbound::TcpHandler::new(
                        settings.passwords.to_vec(),
                    ));
                    let handler =
                        Arc::new(proxy::inbound::Handler::new(tag.clone(), Some(tcp), None));
                    handlers.insert(tag.clone(), handler);
                }
                #[cfg(feature = "inbound-ws")]
                "ws" => {
                    let settings =
                        config::WebSocketInboundSettings::parse_from_bytes(&inbound.settings)
                            .unwrap();
                    let tcp = Arc::new(ws::inbound::TcpHandler::new(settings.path.clone()));
                    let handler =
                        Arc::new(proxy::inbound::Handler::new(tag.clone(), Some(tcp), None));
                    handlers.insert(tag.clone(), handler);
                }
                #[cfg(feature = "inbound-quic")]
                "quic" => {
                    let settings =
                        config::QuicInboundSettings::parse_from_bytes(&inbound.settings).unwrap();
                    let udp = Arc::new(quic::inbound::UdpHandler::new(
                        settings.certificate.clone(),
                        settings.certificate_key.clone(),
                    ));
                    let handler =
                        Arc::new(proxy::inbound::Handler::new(tag.clone(), None, Some(udp)));
                    handlers.insert(tag.clone(), handler);
                }
                #[cfg(feature = "inbound-tls")]
                "tls" => {
                    let settings =
                        config::TlsInboundSettings::parse_from_bytes(&inbound.settings).unwrap();
                    let tcp = Arc::new(tls::inbound::TcpHandler::new(
                        settings.certificate.clone(),
                        settings.certificate_key.clone(),
                    )?);
                    let handler =
                        Arc::new(proxy::inbound::Handler::new(tag.clone(), Some(tcp), None));
                    handlers.insert(tag.clone(), handler);
                }
                _ => (),
            }
        }

        for _i in 0..4 {
            for inbound in inbounds.iter() {
                let tag = String::from(&inbound.tag);
                #[allow(clippy::single_match)]
                match inbound.protocol.as_str() {
                    #[cfg(feature = "inbound-amux")]
                    "amux" => {
                        let mut actors = Vec::new();
                        let settings =
                            config::AMuxInboundSettings::parse_from_bytes(&inbound.settings)
                                .map_err(|e| {
                                    anyhow!("invalid [{}] inbound settings: {}", &tag, e)
                                })?;
                        for actor in settings.actors.iter() {
                            if let Some(a) = handlers.get(actor) {
                                actors.push(a.clone());
                            }
                        }
                        let tcp = Arc::new(amux::inbound::TcpHandler {
                            actors: actors.clone(),
                        });
                        let handler =
                            Arc::new(proxy::inbound::Handler::new(tag.clone(), Some(tcp), None));
                        handlers.insert(tag.clone(), handler);
                    }
                    #[cfg(feature = "inbound-chain")]
                    "chain" => {
                        let settings =
                            config::ChainInboundSettings::parse_from_bytes(&inbound.settings)
                                .map_err(|e| {
                                    anyhow!("invalid [{}] inbound settings: {}", &tag, e)
                                })?;
                        let mut actors = Vec::new();
                        for actor in settings.actors.iter() {
                            if let Some(a) = handlers.get(actor) {
                                actors.push(a.clone());
                            }
                        }
                        if actors.is_empty() {
                            continue;
                        }
                        let tcp = if actors[0].has_tcp() {
                            let h = Arc::new(chain::inbound::TcpHandler {
                                actors: actors.clone(),
                            });
                            Some(h as crate::proxy::AnyTcpInboundHandler)
                        } else {
                            None
                        };
                        let udp = if actors[0].has_udp() {
                            let h = Arc::new(chain::inbound::UdpHandler { actors });
                            Some(h as crate::proxy::AnyUdpInboundHandler)
                        } else {
                            None
                        };
                        let handler = Arc::new(proxy::inbound::Handler::new(tag.clone(), tcp, udp));
                        handlers.insert(tag.clone(), handler);
                    }
                    _ => (),
                }
            }
        }

        let mut network_listeners: HashMap<String, NetworkInboundListener> = HashMap::new();

        #[cfg(all(
            feature = "inbound-tun",
            any(
                target_os = "ios",
                target_os = "android",
                target_os = "macos",
                target_os = "linux",
                target_os = "windows", // MARKER BEGIN - END
            )
        ))]
        let mut tun_listener: Option<TunInboundListener> = None;

        // MARKER BEGIN
        #[cfg(feature = "inbound-packet")]
        let mut packet_listener: Option<PacketInboundListener> = None;
        // MARKER END

        let mut tun_auto = false;

        for inbound in inbounds.iter() {
            let tag = String::from(&inbound.tag);
            match inbound.protocol.as_str() {
                #[cfg(all(
                    feature = "inbound-tun",
                    any(
                        target_os = "ios",
                        target_os = "android",
                        target_os = "macos",
                        target_os = "linux",
                        target_os = "windows", // MARKER BEGIN - END
                    )
                ))]
                "tun" => {
                    let listener = TunInboundListener {
                        inbound: inbound.clone(),
                        dispatcher: dispatcher.clone(),
                        nat_manager: nat_manager.clone(),
                    };
                    tun_listener.replace(listener);
                    let settings =
                        crate::config::TunInboundSettings::parse_from_bytes(&inbound.settings)?;
                    tun_auto = settings.auto;
                }
                // MARKER BEGIN
                #[cfg(feature = "inbound-packet")]
                "packet" => {
                    let listener = PacketInboundListener {
                        inbound: inbound.clone(),
                        dispatcher: dispatcher.clone(),
                        nat_manager: nat_manager.clone(),
                    };
                    packet_listener.replace(listener);
                }
                // MARKER END
                _ => {
                    if inbound.port != 0 {
                        if let Some(h) = handlers.get(&tag) {
                            let listener = NetworkInboundListener {
                                address: inbound.address.clone(),
                                port: inbound.port as u16,
                                handler: h.clone(),
                                dispatcher: dispatcher.clone(),
                                nat_manager: nat_manager.clone(),
                            };
                            network_listeners.insert(tag.clone(), listener);
                        }
                    }
                }
            }
        }

        Ok(InboundManager {
            network_listeners,
            #[cfg(all(
                feature = "inbound-tun",
                any(
                    target_os = "ios",
                    target_os = "android",
                    target_os = "macos",
                    target_os = "linux",
                    target_os = "windows", // MARKER BEGIN - END
                )
            ))]
            tun_listener,
            // MARKER BEGIN
            #[cfg(feature = "inbound-packet")]
            packet_listener,
            // MARKER END
            tun_auto,
        })
    }

    pub fn get_network_runners(&self) -> Result<Vec<Runner>> {
        let mut runners: Vec<Runner> = Vec::new();
        for (_, listener) in self.network_listeners.iter() {
            runners.append(&mut listener.listen()?);
        }
        Ok(runners)
    }

    #[cfg(all(
        feature = "inbound-tun",
        any(
            target_os = "ios",
            target_os = "android",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows", // MARKER BEGIN - END
        )
    ))]
    pub fn get_tun_runner(&self) -> Result<Runner> {
        if let Some(listener) = &self.tun_listener {
            return listener.listen();
        }
        Err(anyhow!("no tun inbound"))
    }

    #[cfg(all(
        feature = "inbound-tun",
        any(
            target_os = "ios",
            target_os = "android",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows", // MARKER BEGIN - END
        )
    ))]
    pub fn has_tun_listener(&self) -> bool {
        self.tun_listener.is_some()
    }

    pub fn tun_auto(&self) -> bool {
        self.tun_auto
    }

    // MARKER BEGIN
    #[cfg(feature = "inbound-packet")]
    pub fn get_packet_runner(&self) -> Result<Runner> {
        if let Some(listener) = &self.packet_listener {
            return listener.listen();
        }
        Err(anyhow!("no tun inbound"))
    }

    #[cfg(feature = "inbound-packet")]
    pub fn has_packet_listener(&self) -> bool {
        self.packet_listener.is_some()
    }
    // MARKER END
}

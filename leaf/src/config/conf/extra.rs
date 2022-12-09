use std::collections::HashMap;
use std::path::Path;
use protobuf::Message;
use crate::config::conf::{extra, Proxy};
use crate::config::Outbound;
use crate::config::internal;

pub(crate) fn ss_reconfigure(ext_proxy: &Proxy, ext_protocol:&str, outbounds: &mut Vec<Outbound>) {
    let mut outbound = internal::Outbound::new();
    outbound.protocol = ext_protocol.to_string();
    outbound.tag = ext_proxy.tag.clone();
    // tls
    let mut tls_outbound = internal::Outbound::new();
    tls_outbound.protocol = "tls".to_string();
    let mut tls_settings = internal::TlsOutboundSettings::new();
    if let Some(ext_sni) = &ext_proxy.sni {
        tls_settings.server_name = ext_sni.clone();
    }
    if let Some(ext_tls_cert) = &ext_proxy.tls_cert {
        let cert = Path::new(ext_tls_cert);
        if cert.is_absolute() {
            tls_settings.certificate = cert.to_string_lossy().to_string();
        } else {
            let asset_loc = Path::new(&*crate::option::ASSET_LOCATION);
            let path = asset_loc.join(cert).to_string_lossy().to_string();
            tls_settings.certificate = path;
        }
    }
    let tls_settings = tls_settings.write_to_bytes().unwrap();
    tls_outbound.settings = tls_settings;
    tls_outbound.tag = format!("{}_tls_xxx", ext_proxy.tag.clone());

    // ws
    let mut ws_outbound = internal::Outbound::new();
    ws_outbound.protocol = "ws".to_string();
    let mut ws_settings = internal::WebSocketOutboundSettings::new();
    if let Some(ext_ws_path) = &ext_proxy.ws_path {
        ws_settings.path = ext_ws_path.clone();
    } else {
        ws_settings.path = "/".to_string();
    }
    if let Some(ext_ws_host) = &ext_proxy.ws_host {
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), ext_ws_host.clone());
        ws_settings.headers = headers;
    }
    let ws_settings = ws_settings.write_to_bytes().unwrap();
    ws_outbound.settings = ws_settings;
    ws_outbound.tag = format!("{}_ws_xxx", ext_proxy.tag.clone());

    // plain shadowsocks
    // let mut settings = internal::VMessOutboundSettings::new();
    // if !ext_proxy.amux.unwrap() {
    //     if let Some(ext_address) = &ext_proxy.address {
    //         settings.address = ext_address.clone();
    //     }
    //     if let Some(ext_port) = &ext_proxy.port {
    //         settings.port = *ext_port as u32;
    //     }
    // }
    // if let Some(ext_username) = &ext_proxy.username {
    //     settings.uuid = ext_username.clone();
    // }
    // if let Some(ext_encrypt_method) = &ext_proxy.encrypt_method {
    //     settings.security = ext_encrypt_method.clone();
    // } else {
    //     settings.security = "chacha20-ietf-poly1305".to_string();
    // }
    // let settings = settings.write_to_bytes().unwrap();
    // outbound.settings = settings;
    // outbound.tag = format!("{}_vmess_xxx", ext_proxy.tag.clone());

    let mut settings = internal::ShadowsocksOutboundSettings::new();
    if let Some(ext_address) = &ext_proxy.address {
        settings.address = ext_address.clone();
    }
    if let Some(ext_port) = &ext_proxy.port {
        settings.port = *ext_port as u32;
    }
    if let Some(ext_encrypt_method) = &ext_proxy.encrypt_method {
        settings.method = ext_encrypt_method.clone();
    } else {
        settings.method = "chacha20-ietf-poly1305".to_string();
    }
    if let Some(ext_password) = &ext_proxy.password {
        settings.password = ext_password.clone();
    }
    let settings = settings.write_to_bytes().unwrap();
    outbound.settings = settings;
    outbound.tag = format!("{}_shadowsocks_xxx", ext_proxy.tag.clone());

    // chain
    let mut chain_outbound = internal::Outbound::new();
    chain_outbound.tag = ext_proxy.tag.clone();
    let mut chain_settings = internal::ChainOutboundSettings::new();
    if ext_proxy.tls.unwrap() {
        chain_settings.actors.push(tls_outbound.tag.clone());
    }
    if ext_proxy.ws.unwrap() {
        chain_settings.actors.push(ws_outbound.tag.clone());
    }
    if !chain_settings.actors.is_empty() {
        chain_settings.actors.push(outbound.tag.clone());
        let chain_settings = chain_settings.write_to_bytes().unwrap();
        chain_outbound.settings = chain_settings;
        chain_outbound.protocol = "chain".to_string();

        // always push chain first, in case there isn't final rule,
        // the chain outbound will be the default one to use
        outbounds.push(chain_outbound);
        if ext_proxy.tls.unwrap() {
            outbounds.push(tls_outbound);
        }
        if ext_proxy.ws.unwrap() {
            outbounds.push(ws_outbound);
        }
    } else {
        outbound.tag = ext_proxy.tag.clone();
    }
    outbounds.push(outbound);
}

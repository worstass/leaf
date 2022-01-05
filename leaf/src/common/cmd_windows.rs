use std::net::{Ipv4Addr, Ipv6Addr};
use std::process::Command;

use anyhow::Result;

pub fn get_default_ipv4_gateway() -> Result<String> {
    todo!()
}

pub fn get_default_ipv6_gateway() -> Result<String> {
    todo!()
}

pub fn get_default_ipv4_address() -> Result<String> {
    todo!()
}

pub fn get_default_ipv6_address() -> Result<String> {
    todo!()
}

pub fn get_default_interface() -> Result<String> {
    todo!()
}

pub fn add_interface_ipv4_address(
    name: &str,
    addr: Ipv4Addr,
    gw: Ipv4Addr,
    mask: Ipv4Addr,
) -> Result<()> {
    todo!()
}

pub fn add_interface_ipv6_address(name: &str, addr: Ipv6Addr, prefixlen: i32) -> Result<()> {
    todo!()
}

pub fn add_default_ipv4_route(gateway: Ipv4Addr, interface: String, primary: bool) -> Result<()> {
    todo!()
}

pub fn add_default_ipv6_route(gateway: Ipv6Addr, interface: String, primary: bool) -> Result<()> {
    todo!()
}

pub fn delete_default_ipv4_route(ifscope: Option<String>) -> Result<()> {
    todo!()
}

pub fn delete_default_ipv6_route(ifscope: Option<String>) -> Result<()> {
    todo!()
}

pub fn add_default_ipv4_rule(addr: Ipv4Addr) -> Result<()> {
    todo!()
}

pub fn add_default_ipv6_rule(addr: Ipv6Addr) -> Result<()> {
    todo!()
}

pub fn delete_default_ipv4_rule(addr: Ipv4Addr) -> Result<()> {
    todo!()
}

pub fn delete_default_ipv6_rule(addr: Ipv6Addr) -> Result<()> {
    todo!()
}

pub fn get_ipv4_forwarding() -> Result<bool> {
    todo!()
}

pub fn get_ipv6_forwarding() -> Result<bool> {
    todo!()
}

pub fn set_ipv4_forwarding(val: bool) -> Result<()> {
    todo!()
}

pub fn set_ipv6_forwarding(val: bool) -> Result<()> {
    todo!()
}

pub fn add_iptable_forward(interface: &str) -> Result<()> {
    todo!()
}

pub fn delete_iptable_forward(interface: &str) -> Result<()> {
    todo!()
}

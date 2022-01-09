use std::net::{Ipv4Addr, Ipv6Addr};
use std::process::Command;

use anyhow::Result;

pub fn get_default_ipv4_gateway() -> Result<String> {

    Ok("".to_string())
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
    let out = Command::new("ip")
        .arg("route")
        .arg("get")
        .arg("1")
        .output()
        .expect("failed to execute command");
    assert!(out.status.success());
    let out = String::from_utf8_lossy(&out.stdout).to_string();
    let cols: Vec<&str> = out
        .lines()
        .filter(|l| l.contains("via"))
        .next()
        .unwrap()
        .split_whitespace()
        .map(str::trim)
        .collect();
    assert!(cols.len() >= 5);
    let res = cols[4].to_string();
    Ok(res)}

pub fn add_interface_ipv4_address(
    name: &str,
    addr: Ipv4Addr,
    gw: Ipv4Addr,
    mask: Ipv4Addr,
) -> Result<()> {
    Ok(())
}

pub fn add_interface_ipv6_address(name: &str, addr: Ipv6Addr, prefixlen: i32) -> Result<()> {
    Ok(())
}

pub fn add_default_ipv4_route(gateway: Ipv4Addr, interface: String, primary: bool) -> Result<()> {
    Ok(())
}

pub fn add_default_ipv6_route(gateway: Ipv6Addr, interface: String, primary: bool) -> Result<()> {
    Ok(())
}

pub fn delete_default_ipv4_route(ifscope: Option<String>) -> Result<()> {
    Ok(())
}

pub fn delete_default_ipv6_route(ifscope: Option<String>) -> Result<()> {
    Ok(())
}

pub fn add_default_ipv4_rule(addr: Ipv4Addr) -> Result<()> {
    Ok(())
}

pub fn add_default_ipv6_rule(addr: Ipv6Addr) -> Result<()> {
    Ok(())
}

pub fn delete_default_ipv4_rule(addr: Ipv4Addr) -> Result<()> {
    Ok(())
}

pub fn delete_default_ipv6_rule(addr: Ipv6Addr) -> Result<()> {
    Ok(())
}

pub fn get_ipv4_forwarding() -> Result<bool> {

    // "netsh interface ipv4 show interface "
    Ok(false)
}

pub fn get_ipv6_forwarding() -> Result<bool> {
    // "netsh interface ipv6 show interface "

    Ok(false)
}

pub fn set_ipv4_forwarding(val: bool) -> Result<()> {
    //  "HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters"

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

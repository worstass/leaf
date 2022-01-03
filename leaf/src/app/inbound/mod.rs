mod network_listener;

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
mod tun_listener;

#[cfg(feature = "inbound-packet")]
mod packet_listener;

pub mod manager;

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

#[cfg(feature = "inbound-packet")] mod packet_listener; // MARKER BEGIN - END
#[cfg(feature = "inbound-cat")]
mod cat_listener;

pub mod manager;

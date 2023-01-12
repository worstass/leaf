pub mod inbound;

#[cfg(not(feature = "netstack-smoltcp"))]
use netstack_lwip as netstack;
#[cfg(feature = "netstack-smoltcp")]
use netstack_smoltcp as netstack;

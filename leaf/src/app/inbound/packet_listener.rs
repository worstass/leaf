use std::sync::Arc;

use anyhow::Result;

use crate::app::dispatcher::Dispatcher;
use crate::app::nat_manager::NatManager;
use crate::config::Inbound;
use crate::proxy::packet;
use crate::Runner;

pub struct PacketInboundListener {
    pub inbound: Inbound,
    pub dispatcher: Arc<Dispatcher>,
    pub nat_manager: Arc<NatManager>,
}

impl PacketInboundListener {
    pub fn listen(&self) -> Result<Runner> {
        packet::inbound::new(
            self.inbound.clone(),
            self.dispatcher.clone(),
            self.nat_manager.clone(),
        )
    }
}
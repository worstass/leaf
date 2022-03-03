use std::{
    fmt::Debug,
    collections::HashMap,
    sync::Arc,
    sync::Mutex
};

use lazy_static::lazy_static;

use crate::RuntimeId;

lazy_static! {
    pub static ref CALLBACK: Mutex<Arc<HashMap<RuntimeId, Box<dyn Callback>>>> =
        Mutex::new(Arc::new(HashMap::new()));
}

pub trait Callback : Debug + Send + Sync {
   fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, rx_total: u64, tx_total: u64);
}


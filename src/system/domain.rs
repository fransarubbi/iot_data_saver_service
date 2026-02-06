use tracing_subscriber::{fmt, EnvFilter};
use crate::grpc::DataSaverDownload;

#[derive(Debug)]
pub struct System {
    pub host_server: String,
}

impl System {
    pub fn new(host_server: String) -> Self {
        Self { host_server }
    }
}

pub enum InternalEvent {
    IncomingMessage(DataSaverDownload)
}

#[derive(Debug)]
pub enum ErrorType {
    Endpoint,
}

pub fn init_tracing() {
    let filter = EnvFilter::from_default_env()
        .add_directive("info".parse().unwrap());

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .init();
}
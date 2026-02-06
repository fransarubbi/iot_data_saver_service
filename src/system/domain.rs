use std::env;
use tracing_subscriber::{EnvFilter};
use crate::grpc::DataSaverDownload;


#[derive(Debug)]
pub struct System {
    pub database_url: String,
    pub db_pool_size: u32,
    pub grpc_host: String,
    pub grpc_port: u16,
    pub heartbeat_interval_secs: u64,
    pub environment: String,
}


impl System {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Cargar .env
        dotenv::dotenv().ok();

        Ok(System {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL no está configurada"),

            db_pool_size: env::var("DB_POOL_SIZE")
                .unwrap_or("10".to_string())
                .parse()
                .expect("DB_POOL_SIZE debe ser un número"),

            grpc_host: env::var("GRPC_HOST")
                .unwrap_or("localhost".to_string()),

            grpc_port: env::var("GRPC_PORT")
                .unwrap_or("50052".to_string())
                .parse()
                .expect("GRPC_PORT debe ser un número"),

            heartbeat_interval_secs: env::var("HEARTBEAT_INTERVAL_SECS")
                .unwrap_or("30".to_string())
                .parse()
                .expect("HEARTBEAT_INTERVAL_SECS debe ser un número"),

            environment: env::var("ENVIRONMENT")
                .unwrap_or("development".to_string()),
        })
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
    EnvFilter::from_default_env();
}


pub mod postgres {
    use tokio::time::{Duration};
    pub const WAIT_FOR: Duration = Duration::from_secs(5);
    pub const BATCH_SIZE: usize = 100;
}


pub mod grpc_service_const {
    pub const TIMEOUT_SECS: u64 = 10;
    pub const KEEP_ALIVE_TIMEOUT_SECS: u64 = 30;
    pub const KEEP_ALIVE_INTERVAL_SECS: u64 = 15;
}
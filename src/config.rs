pub mod postgres {
    use tokio::time::{Duration};
    
    pub const WAIT_FOR: Duration = Duration::from_secs(5);
    pub const BATCH_SIZE: usize = 100;
    pub const MAX_CONNECTIONS: u32 = 20;
}


pub mod grpc_service {
    pub const TIMEOUT_SECS: u64 = 10;
    pub const KEEP_ALIVE_TIMEOUT_SECS: u64 = 30;
    pub const KEEP_ALIVE_INTERVAL_SECS: u64 = 15;
}
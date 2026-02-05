mod database;
mod heartbeat;
mod message;
mod system;
mod grpc_service;
mod config;

pub mod grpc {
    tonic::include_proto!("grpc");
}

#[tokio::main]
async fn main() {

}

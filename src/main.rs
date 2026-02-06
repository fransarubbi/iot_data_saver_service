use crate::channels::domain::Channels;
use crate::context::domain::AppContext;
use crate::database::logic::{start_dba};
use crate::grpc_service::logic::{start_grpc};
use crate::heartbeat::domain::{start_watchdog};
use crate::heartbeat::logic::{start_heartbeat};
use crate::message::logic::{start_message_download, start_message_upload};
use crate::system::domain::{init_tracing};

mod database;
mod heartbeat;
mod message;
mod system;
mod grpc_service;
mod context;
mod channels;

pub mod grpc {
    tonic::include_proto!("grpc");
}


#[tokio::main]
async fn main() {

    init_tracing();

    let channels = Channels::new();
    let app_context = AppContext::new().await;

    start_heartbeat(channels.heartbeat_to_watchdog,
                    channels.heartbeat_to_upload_message,
                    channels.heartbeat_from_watchdog,
                    app_context.clone());

    start_watchdog(channels.watchdog_to_heartbeat,
                   channels.watchdog_from_heartbeat);

    start_message_upload(channels.upload_message_to_grpc,
                         channels.upload_message_from_heartbeat);

    start_message_download(channels.download_message_to_dba,
                           channels.download_message_from_grpc);

    start_dba(channels.dba_from_download_message,
              app_context.clone());

    start_grpc(channels.grpc_to_download_message,
               channels.grpc_from_upload_message,
               app_context.clone())

}

use tokio::sync::mpsc;
use crate::context::domain::AppContext;
use crate::database::logic::dba_task;
use crate::grpc::DataSaverUpload;
use crate::grpc_service::logic::grpc_task;
use crate::heartbeat::domain::{watchdog_timer_for_heartbeat, Event};
use crate::heartbeat::logic::run_heartbeat;
use crate::message::domain::Message;
use crate::message::logic::{message_from_edge, message_to_edge};
use crate::system::domain::{init_tracing, InternalEvent};

mod database;
mod heartbeat;
mod message;
mod system;
mod grpc_service;
mod config;
mod context;


pub mod grpc {
    tonic::include_proto!("grpc");
}


#[tokio::main]
async fn main() {

    init_tracing();

    let (heartbeat_tx_watchdog, watchdog_rx) = mpsc::channel::<Event>(10);
    let (heartbeat_tx_msg, msg_rx) = mpsc::channel::<Message>(10);
    let (watchdog_tx_heartbeat, heartbeat_rx) = mpsc::channel::<Event>(10);
    let (to_edge_tx_grpc, grpc_rx) = mpsc::channel::<DataSaverUpload>(10);
    let (from_edge_tx_dba, dba_rx) = mpsc::channel::<Message>(10);
    let (grpc_tx_msg, msg_rx_grpc) = mpsc::channel::<InternalEvent>(10);


    let app_context = AppContext::new().await;

    tokio::spawn(run_heartbeat(heartbeat_tx_watchdog,
                               heartbeat_tx_msg,
                               heartbeat_rx
    ));

    tokio::spawn(watchdog_timer_for_heartbeat(watchdog_tx_heartbeat,
                                              watchdog_rx
    ));

    tokio::spawn(message_to_edge(to_edge_tx_grpc,
                                 msg_rx
    ));

    tokio::spawn(message_from_edge(from_edge_tx_dba,
                                   msg_rx_grpc
    ));

    tokio::spawn(dba_task(dba_rx,
                          app_context.clone()
    ));

    tokio::spawn(grpc_task(grpc_tx_msg,
                           grpc_rx,
                           app_context.clone()
    ));

}

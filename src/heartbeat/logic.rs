use std::time::Duration;
use tokio::sync::{mpsc};
use tracing::{error};
use chrono::Utc;
use crate::context::domain::AppContext;
use crate::message::domain::{Heartbeat, Message, Metadata};
use super::domain::Event;


pub async fn run_heartbeat(tx_event: mpsc::Sender<Event>,
                           tx_msg: mpsc::Sender<Message>,
                           mut rx_from_watchdog: mpsc::Receiver<Event>,
                           app_context: AppContext) {

    if tx_event.send(Event::InitTimer(Duration::from_secs(app_context.system.heartbeat_interval_secs))).await.is_err() {
        error!("Error: No se pudo enviar el evento a heartbeat");
    }

    while let Some(event) = rx_from_watchdog.recv().await {
        match event {
            Event::Timeout => {
                let timestamp = Utc::now().timestamp();
                let metadata = Metadata {
                    sender_user_id: "DataSaver".to_string(),
                    destination_id: "all".to_string(),
                    timestamp
                };
                let heartbeat = Heartbeat {
                    metadata,
                    beat: true
                };
                if tx_msg.send(Message::Heartbeat(heartbeat)).await.is_err() {
                    error!("Error: No se pudo enviar el mensaje de heartbeat");
                }
                if tx_event.send(Event::InitTimer(Duration::from_secs(app_context.system.heartbeat_interval_secs))).await.is_err() {
                    error!("Error: No se pudo enviar el evento a heartbeat");
                }
            }
            _ => {}
        }
    }
}


pub fn start_heartbeat(to_watchdog: mpsc::Sender<Event>,
                       to_upload_message: mpsc::Sender<Message>,
                       from_watchdog: mpsc::Receiver<Event>,
                       ctx: AppContext) {

    tokio::spawn(async move {
        run_heartbeat(
            to_watchdog,
            to_upload_message,
            from_watchdog,
            ctx,
        ).await;
    });
}
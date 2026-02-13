//! Lógica de generación de latidos (Heartbeats).
//!
//! Este módulo implementa un **Generador de Heartbeats Reactivo**. Su función es informar
//! periódicamente al Edge que este servicio está operativo.
//!
//! # Arquitectura de Actores
//! Funciona en coordinación con una tarea de temporización (Watchdog/Timer):
//! 1. Esta tarea solicita un temporizador (`Event::InitTimer`).
//! 2. La tarea de temporización espera y responde con `Event::Timeout`.
//! 3. Esta tarea reacciona al timeout enviando el mensaje `Heartbeat` y reiniciando el ciclo.


use std::time::Duration;
use tokio::sync::{mpsc};
use tracing::{debug, error, info, instrument};
use chrono::Utc;
use crate::context::domain::AppContext;
use crate::message::domain::{Heartbeat, Message, Metadata};
use super::domain::Event;


/// Ejecuta el bucle principal de generación de heartbeats.
///
/// Mantiene vivo el ciclo de retroalimentación con el Watchdog y despacha los mensajes
/// de latido hacia la cola de salida.
///
/// # Flujo de Trabajo
/// 1. Envía `Event::InitTimer` al Watchdog para iniciar la cuenta regresiva.
/// 2. Entra en espera asíncrona de mensajes.
/// 3. Al recibir `Event::Timeout`:
///    - Genera un `Message::Heartbeat` con timestamp UTC actual.
///    - Lo envía al canal `tx_msg` (hacia gRPC).
///    - Solicita un nuevo timer al Watchdog para el siguiente ciclo.
///
/// # Argumentos
/// * `tx_event`: Canal para enviar comandos al Watchdog (iniciar timers).
/// * `tx_msg`: Canal para enviar el mensaje de heartbeat generado a la tarea `message_upload`.
/// * `rx_from_watchdog`: Canal para recibir notificaciones de tiempo cumplido (`Timeout`).
/// * `app_context`: Configuración global (para leer `heartbeat_interval_secs`).
#[instrument(
    name = "run_heartbeat_task",
    skip(tx_event, tx_msg, rx_from_watchdog, app_context)
)]
pub async fn run_heartbeat(tx_event: mpsc::Sender<Event>,
                           tx_msg: mpsc::Sender<Message>,
                           mut rx_from_watchdog: mpsc::Receiver<Event>,
                           app_context: AppContext) {

    info!("Info: heartbeat task creada");

    if tx_event.send(Event::InitTimer(Duration::from_secs(app_context.system.heartbeat_interval_secs))).await.is_err() {
        error!("Error: no se pudo enviar el evento a heartbeat");
    }

    while let Some(event) = rx_from_watchdog.recv().await {
        debug!("Debug: evento entrante del watchdog");
        match event {
            Event::Timeout => {
                let timestamp = Utc::now().timestamp();
                let metadata = Metadata {
                    sender_user_id: "data_saver".to_string(),
                    destination_id: "all".to_string(),
                    timestamp
                };
                let heartbeat = Heartbeat {
                    metadata,
                    beat: true
                };
                if tx_msg.send(Message::Heartbeat(heartbeat)).await.is_err() {
                    error!("Error: no se pudo enviar el mensaje de heartbeat");
                }
                if tx_event.send(Event::InitTimer(Duration::from_secs(app_context.system.heartbeat_interval_secs))).await.is_err() {
                    error!("Error: no se pudo enviar el evento a heartbeat");
                }
            }
            _ => {}
        }
    }
    info!("Info: heartbeat task finalizada");
}


/// Inicializa y ejecuta la tarea de heartbeat en segundo plano (tokio task).
///
/// Esta función actúa como el punto de entrada (entrypoint) para el subsistema de heartbeat.
///
/// # Argumentos
/// * `to_watchdog`: Canal hacia el temporizador.
/// * `to_upload_message`: Canal hacia el adaptador de mensajes gRPC.
/// * `from_watchdog`: Canal de entrada desde el temporizador.
/// * `ctx`: Contexto de la aplicación.
pub fn start_heartbeat(to_watchdog: mpsc::Sender<Event>,
                       to_upload_message: mpsc::Sender<Message>,
                       from_watchdog: mpsc::Receiver<Event>,
                       ctx: AppContext) {

    info!("Info: iniciando tarea heartbeat");
    tokio::spawn(async move {
        run_heartbeat(
            to_watchdog,
            to_upload_message,
            from_watchdog,
            ctx,
        ).await;
    });
}
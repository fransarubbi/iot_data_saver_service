//! Tarea administradora de base de datos (DBA).
//!
//! Este módulo implementa un patrón de **Buffering** o **Batching**.
//! En lugar de realizar una transacción SQL por cada mensaje recibido (lo cual sería lento e ineficiente),
//! esta tarea acumula los mensajes en memoria y los inserta en lotes (chunks) cuando alcanzan
//! cierto tamaño.


use tokio::sync::mpsc;
use tokio::time::{Duration};
use tracing::{error, info, instrument};
use crate::bucket::logic::{ProcessedTelemetry};
use crate::context::domain::AppContext;
use crate::message::domain::Message;
use crate::weather::domain::Weather;


enum DbOperation {
    Msg(Message),
    Telemetry(ProcessedTelemetry),
    Weather(Weather),
}



#[instrument(
    name = "dba_task",
    skip(rx, app_context)
)]
pub async fn dba_task(mut rx: mpsc::Receiver<Message>,
                      mut rx_from_sweeper: mpsc::Receiver<ProcessedTelemetry>,
                      mut rx_from_weather: mpsc::Receiver<Weather>,
                      app_context: AppContext) {

    info!("Info: dba task creada");

    loop {
        let operation = tokio::select! {
            Some(msg) = rx.recv() => DbOperation::Msg(msg),
            Some(telemetry) = rx_from_sweeper.recv() => DbOperation::Telemetry(telemetry),
            Some(weather) = rx_from_weather.recv() => DbOperation::Weather(weather),
            else => {
                info!("Info: canales cerrados, finalizando dba_task");
                break;
            }
        };

        execute_with_retry(&app_context, operation).await;
    }
}


/// Helper para intentar la inserción hasta 5 veces antes de descartar el dato
async fn execute_with_retry(app_context: &AppContext, op: DbOperation) {
    let mut counter: u8 = 1;

    loop {
        let result = match &op {
            DbOperation::Msg(msg) => app_context.repo.insert_message(msg.clone()).await,
            DbOperation::Telemetry(telemetry) => app_context.repo.insert_telemetry(telemetry.clone()).await,
            DbOperation::Weather(weather) => app_context.repo.insert_weather_data(weather.clone()).await,
        };

        match result {
            Ok(_) => break, // Éxito, salimos del reintento
            Err(e) => {
                if counter == 5 {
                    error!("Error: se acabaron los 5 intentos para insertar en DB. Dato descartado.");
                    break;
                }
                error!("Error al insertar en DB. Intento {counter}. Reintentando en 5s. Detalle: {e}");
                counter += 1;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}


/// Inicializa y lanza la tarea DBA en segundo plano.
///
/// # Argumentos
/// * `rx_from_msg`: Canal de entrada con los mensajes ya decodificados.
/// * `app_context`: Dependencias globales del sistema.
pub fn start_dba(rx_from_msg: mpsc::Receiver<Message>,
                 rx_from_sweeper: mpsc::Receiver<ProcessedTelemetry>,
                 rx_from_weather: mpsc::Receiver<Weather>,
                 app_context: AppContext) {

    info!("Info: iniciando tarea dba");
    tokio::spawn(async move {
        dba_task(rx_from_msg,
                 rx_from_sweeper,
                 rx_from_weather,
                 app_context
        ).await;
    });
}
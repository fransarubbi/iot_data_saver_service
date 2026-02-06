//! Tarea administradora de base de datos (DBA).
//!
//! Este módulo implementa un patrón de **Buffering** o **Batching**.
//! En lugar de realizar una transacción SQL por cada mensaje recibido (lo cual sería lento e ineficiente),
//! esta tarea acumula los mensajes en memoria y los inserta en lotes (chunks) cuando alcanzan
//! cierto tamaño.


use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};
use crate::context::domain::AppContext;
use crate::database::domain::TableDataVector;
use crate::message::domain::Message;


/// Ejecuta la lógica de acumulación y persistencia de datos.
///
/// Actúa como un sumidero (sink) que recibe mensajes de dominio, los clasifica
/// en vectores específicos (`Measurement`, `Monitor`, etc.) y delega la persistencia
/// al repositorio cuando los buffers se llenan.
///
/// # Lógica de Batching
/// 1. Recibe un mensaje.
/// 2. Lo almacena en el vector correspondiente en memoria.
/// 3. Verifica si algún vector ha alcanzado su capacidad máxima (`BATCH_SIZE`).
/// 4. Si está lleno, clona el lote completo, lo envía al repositorio para inserción asíncrona
///    y **limpia** los buffers locales.
///
/// # Argumentos
/// * `rx`: Canal de recepción de mensajes desde la capa de lógica/traducción.
/// * `app_context`: Contexto global que contiene el Repositorio de base de datos.
#[instrument(
    name = "dba_task",
    skip(rx, app_context)
)]
pub async fn dba_task(mut rx: mpsc::Receiver<Message>,
                      app_context: AppContext) {

    info!("Info: dba task creada");
    let mut vector = TableDataVector::new();

    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Report(report) => {
                debug!("Debug: mensaje entrante Measurement a dba task");
                vector.measurement.push(report);
            }
            Message::Monitor(monitor) => {
                debug!("Debug: mensaje entrante Monitor a dba task");
                vector.monitor.push(monitor);
            }
            Message::Metrics(metrics) => {
                debug!("Debug: mensaje entrante SystemMetrics a dba task");
                vector.system_metrics.push(metrics);
            }
            Message::AlertAir(alert) => {
                debug!("Debug: mensaje entrante AlertAir a dba task");
                vector.alert_air.push(alert);
            }
            Message::AlertTem(alert) => {
                debug!("Debug: mensaje entrante AlertTh a dba task");
                vector.alert_th.push(alert);
            }
            _ => {}
        }

        if vector.is_some_vector_full() {
            debug!("Debug: se ha llenado uno de los vectores");
            match app_context.repo.insert(vector.clone()).await {
                Ok(_) => {}
                Err(e) => error!("Error: no se pudo insertar batch. {e}")
            }
            vector.clear();
        }
    }
}


/// Inicializa y lanza la tarea DBA en segundo plano.
///
/// # Argumentos
/// * `rx_from_msg`: Canal de entrada con los mensajes ya decodificados.
/// * `app_context`: Dependencias globales del sistema.
pub fn start_dba(rx_from_msg: mpsc::Receiver<Message>,
                 app_context: AppContext) {

    info!("Info: iniciando tarea dba");
    tokio::spawn(async move {
        dba_task(rx_from_msg,
                 app_context
        ).await;
    });
}
//! Tarea administradora de base de datos (DBA).
//!
//! Este módulo implementa un patrón de **Buffering** o **Batching**.
//! En lugar de realizar una transacción SQL por cada mensaje recibido (lo cual sería lento e ineficiente),
//! esta tarea acumula los mensajes en memoria y los inserta en lotes (chunks) cuando alcanzan
//! cierto tamaño.


use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
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
    let mut has_uncommitted_data = false;
    let mut flush_interval = interval(Duration::from_secs(60));

    // El primer tick de un interval se dispara inmediatamente, lo consumimos para que el proximo tarde 60s
    flush_interval.tick().await;

    loop {
        tokio::select! {
            // Evento 1: Llega un nuevo mensaje desde el canal
            msg_opt = rx.recv() => {
                match msg_opt {
                    Some(msg) => {
                        has_uncommitted_data = true;

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

                        // Condicion A: Se alcanzo la capacidad maxima antes del minuto
                        if vector.is_some_vector_full() {
                            debug!("Debug: Flush a DB activado por límite de capacidad");
                            flush_with_retry(&mut vector, &app_context).await;
                            has_uncommitted_data = false;
                            // Reiniciamos el temporizador para tener 1 minuto completo desde esta insercion
                            flush_interval.reset();
                        }
                    }
                    None => {
                        // El canal se cerro
                        if has_uncommitted_data {
                            info!("Info: Guardando últimos datos antes de cerrar");
                            flush_with_retry(&mut vector, &app_context).await;
                        }
                        info!("Info: Canal cerrado, finalizando dba_task");
                        break;
                    }
                }
            }

            // Evento 2: Paso 1 minuto exacto sin que se llenaran los vectores
            _ = flush_interval.tick() => {
                // Condicion B: Insercion por tiempo (solo si hay datos pendientes)
                if has_uncommitted_data {
                    debug!("Debug: Flush a DB activado por tiempo (1 minuto)");
                    flush_with_retry(&mut vector, &app_context).await;
                    has_uncommitted_data = false;
                }
            }
        }
    }
}


async fn flush_with_retry(vector: &mut TableDataVector, app_context: &AppContext) {
    let mut counter: u8 = 1;
    loop {
        match app_context.repo.insert(vector.clone()).await {
            Ok(_) => {
                // Solo vaciamos la memoria si la base de datos confirmo la transaccion
                vector.clear();
                break;
            }
            Err(e) => {
                if counter == 5 {
                    error!("Error crítico: se acabaron los 5 intentos para insertar lote en DB. Borrando datos");
                    vector.clear();
                    break;
                }
                counter += 1;
                error!("Error crítico: Fallo al insertar lote en DB. Intento numero: {counter}. Reintentando en 5s. Detalle: {e}");
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
                 app_context: AppContext) {

    info!("Info: iniciando tarea dba");
    tokio::spawn(async move {
        dba_task(rx_from_msg,
                 app_context
        ).await;
    });
}
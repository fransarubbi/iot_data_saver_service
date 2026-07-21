//! Adaptador de mensajes entre la capa gRPC y el Dominio.
//!
//! Este módulo actúa como una capa de traducción (Mapper/Adapter). Su responsabilidad
//! es convertir los tipos generados automáticamente por `tonic` (Protobuf) a los tipos
//! de dominio internos del sistema, y viceversa.
//!
//! # Arquitectura
//! Funciona mediante dos tareas asíncronas independientes (Actors):
//! * **Upload Task:** Escucha eventos internos (Heartbeats) -> Convierte a Proto -> Envía a gRPC.
//! * **Download Task:** Escucha eventos gRPC -> Desempaqueta `oneof` -> Convierte a Dominio -> Envía a DB/Batcher.

use crate::bucket::logic::BucketData;
use crate::context::domain::AppContext;
use crate::grpc::to_data_saver::Payload;
use crate::grpc::{FromDataSaver, Heartbeat, Metadata, from_data_saver};
use crate::message::domain::{
    AlertAir as AlertAirMessage, AlertTh as AlertThMessage, Measurement as MeasurementMessage,
    Message, Metadata as MetadataMessage, Monitor as MonitorMessage,
    SystemMetrics as MetricsMessage,
};
use crate::system::domain::InternalEvent;
use chrono::{DateTime, Utc};
use chrono_tz::America::Buenos_Aires;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

/// Tarea de subida: Transforma mensajes de dominio en mensajes de transporte gRPC.
///
/// Actualmente se encarga principalmente de enviar los **Heartbeats** generados por el sistema
/// hacia el servidor central (In-Store Service) para mantener la conexión viva.
///
/// # Flujo de Datos
/// 1. Recibe un `Message::Heartbeat` del canal interno.
/// 2. Construye la estructura anidada requerida por el `.proto` (`DataSaverUpload` -> `Payload` -> `Heartbeat`).
/// 3. Envía el mensaje resultante al canal de salida hacia la tarea gRPC.
///
/// # Argumentos
/// * `tx`: Canal de envío hacia la tarea de red (`grpc_service`).
/// * `rx`: Canal de recepción desde el generador de heartbeats.
#[instrument(name = "message_upload_task", skip(tx, rx))]
pub async fn message_upload(tx: mpsc::Sender<FromDataSaver>, mut rx: mpsc::Receiver<Message>) {
    info!("Info: message_upload_task creada");

    while let Some(msg) = rx.recv().await {
        debug!("Debug: ingreso un mensaje de heartbeat para enviar a gRPC");
        match msg {
            Message::Heartbeat(heartbeat) => {
                let grpc_metadata = Metadata {
                    sender_user_id: heartbeat.metadata.sender_user_id,
                    destination_id: heartbeat.metadata.destination_id,
                    timestamp: heartbeat.metadata.timestamp,
                };

                let grpc_heartbeat = Heartbeat {
                    metadata: Some(grpc_metadata),
                    beat: heartbeat.beat,
                };

                let to_edge_payload = from_data_saver::Payload::Heartbeat(grpc_heartbeat);
                let to_edge_msg = FromDataSaver {
                    payload: Some(to_edge_payload),
                };

                if tx.send(to_edge_msg).await.is_err() {
                    error!("Error: no se pudo enviar mensaje Heartbeat a la tarea gRPC");
                }
            }
            _ => {}
        }
    }
    info!("Info: message_upload_task finalizada");
}

/// Tarea de bajada: Transforma mensajes gRPC entrantes en mensajes de dominio.
///
/// Procesa el flujo de datos que llega desde el Edge (vía In-Store Service). Desempaqueta
/// las estructuras `oneof` de Protobuf y mapea los campos a los structs definidos en `message::domain`.
///
/// # Tipos Soportados
/// * `Measurement`: Datos de sensores.
/// * `Monitor`: Diagnósticos de Hubs.
/// * `AlertAir` y `AlertTh`: Alertas ambientales.
/// * `Metrics`: Diagnósticos de Edges.
///
/// # Argumentos
/// * `tx`: Canal de envío hacia la capa de persistencia (Database/Batcher).
/// * `rx`: Canal de recepción de eventos desde la tarea gRPC (`InternalEvent`).
#[instrument(name = "message_download_task", skip(tx, rx))]
pub async fn message_download(
    tx: mpsc::Sender<Message>,
    tx_to_bucket: mpsc::Sender<BucketData>,
    mut rx: mpsc::Receiver<InternalEvent>,
    app_context: AppContext,
) {
    info!("Info: message_download_task creada");

    while let Some(msg) = rx.recv().await {
        debug!("Debug: ingreso un mensaje de datos desde el servicio gRPC");
        match msg {
            InternalEvent::IncomingMessage(msg) => {
                if let Some(payload) = msg.payload {
                    match payload {
                        Payload::Measurement(measurement) => {
                            debug!("Debug: el mensaje entrante es de tipo Measurement");
                            if let Some(metadata) = extract_metadata(measurement.metadata) {
                                let msg = MeasurementMessage {
                                    metadata,
                                    network: measurement.network,
                                    pulse_counter: measurement.pulse_counter,
                                    pulse_max_duration: measurement.pulse_max_duration,
                                    temperature: measurement.temperature,
                                    humidity: measurement.humidity,
                                    air_quality: measurement.air_quality,
                                    sample: measurement.sample,
                                };
                                if tx_to_bucket
                                    .send(BucketData::Measurement(msg))
                                    .await
                                    .is_err()
                                {
                                    error!("Error: no se pudo enviar mensaje a dba_task");
                                }
                            }
                        }
                        Payload::Monitor(monitor) => {
                            debug!("Debug: el mensaje entrante es de tipo Monitor");
                            if let Some(metadata) = extract_metadata(monitor.metadata) {
                                let msg = MonitorMessage {
                                    metadata,
                                    network: monitor.network,
                                    heap_free: monitor.heap_free,
                                    heap_min_free: monitor.heap_min_free,
                                    heap_largest_block: monitor.heap_largest_block,
                                    uptime_sec: monitor.uptime_sec as i64,
                                };
                                if tx.send(Message::Monitor(msg)).await.is_err() {
                                    error!("Error: no se pudo enviar mensaje a dba_task");
                                }
                            }
                        }
                        Payload::AlertAir(alert_air) => {
                            debug!("Debug: el mensaje entrante es de tipo AlertAir");

                            if let Some(metadata) = extract_metadata(alert_air.metadata) {
                                let meta = metadata.clone();
                                let msg = AlertAirMessage {
                                    metadata,
                                    network: alert_air.network.clone(),
                                    initial_air_quality: alert_air.initial_air_quality,
                                    actual_air_quality: alert_air.actual_air_quality,
                                };

                                if tx.send(Message::AlertAir(msg)).await.is_err() {
                                    error!("Error: no se pudo enviar mensaje a dba_task");
                                }

                                let notifier = app_context.telegram_notifier.clone();

                                let msg_to_telegram = format!(
                                    "⚠️ *ALERTA DE AIRE*\n\n\
                                    Red: `{}`\n\
                                    Generada: {}\n\
                                    Recibida: {}\n\
                                    Hub emisor: {}\n\
                                    Calidad de aire inicial: {}\n\
                                    Calidad de aire actual: {}",
                                    alert_air.network,
                                    format_unix_to_argentina(meta.timestamp),
                                    time_now(),
                                    meta.sender_user_id,
                                    alert_air.initial_air_quality,
                                    alert_air.actual_air_quality
                                );

                                tokio::spawn(async move {
                                    notifier.send_alert(&msg_to_telegram).await;
                                });
                            }
                        }
                        Payload::AlertTh(alert_th) => {
                            debug!("Debug: el mensaje entrante es de tipo AlertTh");

                            if let Some(metadata) = extract_metadata(alert_th.metadata) {
                                let meta = metadata.clone();
                                let msg = AlertThMessage {
                                    metadata,
                                    network: alert_th.network.clone(),
                                    initial_temp: alert_th.initial_temp,
                                    actual_temp: alert_th.actual_temp,
                                };

                                if tx.send(Message::AlertTem(msg)).await.is_err() {
                                    error!("Error: no se pudo enviar mensaje a dba_task");
                                }

                                let notifier = app_context.telegram_notifier.clone();

                                let msg_to_telegram = format!(
                                    "⚠️ *ALERTA DE TEMPERATURA*\n\n\
                                    Red: `{}`\n\
                                    Generada: {}\n\
                                    Recibida: {}\n\
                                    Hub emisor: {}\n\
                                    Temperatura inicial: {}\n\
                                    Temperatura actual: {}",
                                    alert_th.network,
                                    format_unix_to_argentina(meta.timestamp),
                                    time_now(),
                                    meta.sender_user_id,
                                    alert_th.initial_temp,
                                    alert_th.actual_temp
                                );

                                tokio::spawn(async move {
                                    notifier.send_alert(&msg_to_telegram).await;
                                });
                            }
                        }
                        Payload::Metric(metrics) => {
                            debug!("Debug: el mensaje entrante es de tipo SystemMetrics");
                            if let Some(metadata) = extract_metadata(metrics.metadata) {
                                let msg = MetricsMessage {
                                    metadata,
                                    uptime_seconds: metrics.uptime_seconds,
                                    cpu_usage_percent: metrics.cpu_usage_percent,
                                    cpu_temp_celsius: metrics.cpu_temp_celsius,
                                    ram_total_mb: metrics.ram_total_mb,
                                    ram_used_mb: metrics.ram_used_mb,
                                    sd_total_gb: metrics.sd_total_gb,
                                    sd_used_gb: metrics.sd_used_gb,
                                    sd_usage_percent: metrics.sd_usage_percent,
                                    network_rx_bytes: metrics.network_rx_bytes,
                                    network_tx_bytes: metrics.network_tx_bytes,
                                    wifi_rssi: Some(metrics.wifi_rssi),
                                    wifi_signal_dbm: Some(metrics.wifi_signal_dbm),
                                };
                                if tx.send(Message::Metrics(msg)).await.is_err() {
                                    error!("Error: no se pudo enviar mensaje a dba_task");
                                }
                            }
                        }
                        Payload::MeasurementBatch(batch) => {
                            debug!("Debug: el mensaje entrante es un MeasurementBatch");

                            let domain_measurements: Vec<MeasurementMessage> = batch
                                .measurements
                                .into_iter()
                                .filter_map(|measurement| {
                                    // filter_map solo conserva los Some(), descartando los None
                                    extract_metadata(measurement.metadata).map(|metadata| {
                                        MeasurementMessage {
                                            metadata,
                                            network: measurement.network,
                                            pulse_counter: measurement.pulse_counter,
                                            pulse_max_duration: measurement.pulse_max_duration,
                                            temperature: measurement.temperature,
                                            humidity: measurement.humidity,
                                            air_quality: measurement.air_quality,
                                            sample: measurement.sample,
                                        }
                                    })
                                })
                                .collect();

                            if !domain_measurements.is_empty() {
                                if tx_to_bucket
                                    .send(BucketData::VecMeasurement(domain_measurements))
                                    .await
                                    .is_err()
                                {
                                    error!("Error: no se pudo enviar MeasurementBatch a dba_task");
                                }
                            }
                        }
                        Payload::MonitorBatch(batch) => {
                            debug!("Debug: el mensaje entrante es un MonitorBatch");

                            let domain_monitors: Vec<MonitorMessage> = batch
                                .monitors
                                .into_iter()
                                .filter_map(|monitor| {
                                    extract_metadata(monitor.metadata).map(|metadata| {
                                        MonitorMessage {
                                            metadata,
                                            network: monitor.network,
                                            heap_free: monitor.heap_free,
                                            heap_min_free: monitor.heap_min_free,
                                            heap_largest_block: monitor.heap_largest_block,
                                            uptime_sec: monitor.uptime_sec as i64,
                                        }
                                    })
                                })
                                .collect();

                            if !domain_monitors.is_empty() {
                                if tx
                                    .send(Message::MonitorBatch(domain_monitors))
                                    .await
                                    .is_err()
                                {
                                    error!("Error: no se pudo enviar MonitorBatch a dba_task");
                                }
                            }
                        }
                        Payload::AlertAirBatch(batch) => {
                            debug!("Debug: el mensaje entrante es un AlertAirBatch");

                            let domain_alerts: Vec<AlertAirMessage> = batch
                                .alerts
                                .into_iter()
                                .filter_map(|alert_air| {
                                    extract_metadata(alert_air.metadata).map(|metadata| {
                                        AlertAirMessage {
                                            metadata,
                                            network: alert_air.network,
                                            initial_air_quality: alert_air.initial_air_quality,
                                            actual_air_quality: alert_air.actual_air_quality,
                                        }
                                    })
                                })
                                .collect();

                            if !domain_alerts.is_empty() {
                                if tx
                                    .send(Message::AlertAirBatch(domain_alerts))
                                    .await
                                    .is_err()
                                {
                                    error!("Error: no se pudo enviar AlertAirBatch a dba_task");
                                }
                            }

                            let notifier = app_context.telegram_notifier.clone();

                            let msg_to_telegram = "⚠️ *BATCH DE ALERTAS DE AIRE*\n\n\
                                 Se recomienda atención."
                                .to_string();

                            tokio::spawn(async move {
                                notifier.send_alert(&msg_to_telegram).await;
                            });
                        }
                        Payload::AlertThBatch(batch) => {
                            debug!("Debug: el mensaje entrante es un AlertThBatch");

                            let domain_alerts: Vec<AlertThMessage> = batch
                                .alerts
                                .into_iter()
                                .filter_map(|alert_th| {
                                    extract_metadata(alert_th.metadata).map(|metadata| {
                                        AlertThMessage {
                                            metadata,
                                            network: alert_th.network,
                                            initial_temp: alert_th.initial_temp,
                                            actual_temp: alert_th.actual_temp,
                                        }
                                    })
                                })
                                .collect();

                            if !domain_alerts.is_empty() {
                                if tx
                                    .send(Message::AlertTemBatch(domain_alerts))
                                    .await
                                    .is_err()
                                {
                                    error!("Error: no se pudo enviar AlertThBatch a dba_task");
                                }
                            }

                            let notifier = app_context.telegram_notifier.clone();

                            let msg_to_telegram = "⚠️ *BATCH DE ALERTAS DE TEMPERATURA*\n\n\
                                 Se recomienda atención."
                                .to_string();

                            tokio::spawn(async move {
                                notifier.send_alert(&msg_to_telegram).await;
                            });
                        }
                    }
                }
            }
        }
    }
    info!("Info: message_download_task finalizada");
}

/// Devuelve la hora actual en Argentina formateada: DD/MM/YYYY HH:MM:SS
pub fn time_now() -> String {
    let argentina_tz = Buenos_Aires;

    let now = Utc::now().with_timezone(&argentina_tz);

    now.format("%d/%m/%Y %H:%M:%S").to_string()
}

/// Convierte un timestamp Unix (segundos) a formato Argentina
pub fn format_unix_to_argentina(unix_seconds: i64) -> String {
    let argentina_tz = Buenos_Aires;

    let datetime = DateTime::from_timestamp(unix_seconds, 0)
        .map(|dt| dt.with_timezone(&argentina_tz))
        .unwrap_or_else(|| Utc::now().with_timezone(&argentina_tz));

    datetime.format("%d/%m/%Y %H:%M:%S").to_string()
}

/// Inicializa y ejecuta la tarea de subida en un hilo de Tokio.
///
/// # Argumentos
/// * `tx_to_grpc`: Canal hacia la capa de transporte.
/// * `rx_from_heartbeat`: Canal desde el generador de eventos de dominio.
pub fn start_message_upload(
    tx_to_grpc: mpsc::Sender<FromDataSaver>,
    rx_from_heartbeat: mpsc::Receiver<Message>,
) {
    info!("Info: iniciando tarea message_upload");
    tokio::spawn(async move {
        message_upload(tx_to_grpc, rx_from_heartbeat).await;
    });
}

/// Inicializa y ejecuta la tarea de bajada en un hilo de Tokio.
///
/// # Argumentos
/// * `tx_to_dba`: Canal hacia la capa de base de datos (Batcher).
/// * `rx_from_grpc`: Canal desde la capa de transporte.
pub fn start_message_download(
    tx_to_dba: mpsc::Sender<Message>,
    tx_to_bucket: mpsc::Sender<BucketData>,
    rx_from_grpc: mpsc::Receiver<InternalEvent>,
    app_context: AppContext,
) {
    info!("Info: iniciando tarea message_download");
    tokio::spawn(async move {
        message_download(tx_to_dba, tx_to_bucket, rx_from_grpc, app_context).await;
    });
}

/// Helper privado para convertir metadatos.
/// Recibe el Option<Metadata> de Protobuf y devuelve Metadata o None si no hay metadatos.
fn extract_metadata(proto_meta: Option<Metadata>) -> Option<MetadataMessage> {
    match proto_meta {
        Some(meta) => Some(MetadataMessage {
            sender_user_id: meta.sender_user_id,
            destination_id: meta.destination_id,
            timestamp: meta.timestamp,
        }),
        None => {
            warn!("Warning: mensaje descartado, llegó sin metadatos obligatorios");
            None
        }
    }
}

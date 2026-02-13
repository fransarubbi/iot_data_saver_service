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


use tokio::sync::{mpsc};
use tracing::{debug, error, info, instrument, warn};
use crate::message::domain::{Measurement as MeasurementMessage, Monitor as MonitorMessage,
                             AlertAir as AlertAirMessage, AlertTh as AlertThMessage,
                             SystemMetrics as MetricsMessage, Message, Metadata as MetadataMessage};
use crate::grpc::{data_saver_download_from_edge, DataSaverUpload, Heartbeat, Metadata};
use crate::grpc::data_saver_download::Payload;
use crate::system::domain::InternalEvent;


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
#[instrument(
    name = "message_upload_task",
    skip(tx, rx)
)]
pub async fn message_upload(tx: mpsc::Sender<DataSaverUpload>,
                            mut rx: mpsc::Receiver<Message>) {

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

                let to_edge_payload = crate::grpc::data_saver_upload_to_edge::Payload::Heartbeat(grpc_heartbeat);
                let to_edge_msg = crate::grpc::DataSaverUploadToEdge {
                    payload: Some(to_edge_payload),
                };

                let upload_payload = crate::grpc::data_saver_upload::Payload::ToEdge(to_edge_msg);

                let grpc_msg = DataSaverUpload {
                    edge_id: "all".to_string(),
                    payload: Some(upload_payload),
                };

                if tx.send(grpc_msg).await.is_err() {
                    error!("Error: no se pudo enviar mensaje Heartbeat a la tarea gRPC");
                }
            },
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
#[instrument(
    name = "message_download_task",
    skip(tx, rx)
)]
pub async fn message_download(tx: mpsc::Sender<Message>,
                              mut rx: mpsc::Receiver<InternalEvent>) {

    info!("Info: message_download_task creada");

    while let Some(msg) = rx.recv().await {
        debug!("Debug: ingreso un mensaje de datos desde el servicio gRPC");
        match msg {
            InternalEvent::IncomingMessage(msg) => {
                if let Some(payload) = msg.payload {
                    match payload { 
                        Payload::FromEdge(msg_from_edge) => {
                            if let Some(inner) = msg_from_edge.payload {
                                match inner {
                                    data_saver_download_from_edge::Payload::Measurement(measurement) => {
                                        debug!("Debug: el mensaje entrante es de tipo Measurement");
                                        if let Some(metadata) = extract_metadata(measurement.metadata) {
                                            let msg = MeasurementMessage {
                                                metadata,
                                                network: measurement.network,
                                                pulse_counter: measurement.pulse_counter,
                                                pulse_max_duration: measurement.pulse_max_duration,
                                                temperature: measurement.temperature,
                                                humidity: measurement.humidity,
                                                co2_ppm: measurement.co2_ppm,
                                                sample: measurement.sample,
                                            };
                                            if tx.send(Message::Report(msg)).await.is_err() {
                                                error!("Error: no se pudo enviar mensaje a dba_task");
                                            }
                                        }
                                    }
                                    data_saver_download_from_edge::Payload::Monitor(monitor) => {
                                        debug!("Debug: el mensaje entrante es de tipo Monitor");
                                        if let Some(metadata) = extract_metadata(monitor.metadata) {
                                            let msg = MonitorMessage {
                                                metadata,
                                                network: monitor.network,
                                                mem_free: monitor.mem_free,
                                                mem_free_hm: monitor.mem_free_hm,
                                                mem_free_block: monitor.mem_free_block,
                                                mem_free_internal: monitor.mem_free_internal,
                                                stack_free_min_coll: monitor.stack_free_min_coll,
                                                stack_free_min_pub: monitor.stack_free_min_pub,
                                                stack_free_min_mic: monitor.stack_free_min_mic,
                                                stack_free_min_th: monitor.stack_free_min_th,
                                                stack_free_min_air: monitor.stack_free_min_air,
                                                stack_free_min_mon: monitor.stack_free_min_mon,
                                                wifi_ssid: monitor.wifi_ssid,
                                                wifi_rssi: monitor.wifi_rssi as i8, // Casting de int32 a i8
                                                active_time: monitor.active_time,
                                            };
                                            if tx.send(Message::Monitor(msg)).await.is_err() {
                                                error!("Error: no se pudo enviar mensaje a dba_task");
                                            }
                                        }
                                    }
                                    data_saver_download_from_edge::Payload::AlertAir(alert_air) => {
                                        debug!("Debug: el mensaje entrante es de tipo AlertAir");
                                        if let Some(metadata) = extract_metadata(alert_air.metadata) {
                                            let msg = AlertAirMessage {
                                                metadata,
                                                network: alert_air.network,
                                                co2_initial_ppm: alert_air.co2_initial_ppm,
                                                co2_actual_ppm: alert_air.co2_actual_ppm,
                                            };
                                            if tx.send(Message::AlertAir(msg)).await.is_err() {
                                                error!("Error: no se pudo enviar mensaje a dba_task");
                                            }
                                        }
                                    }
                                    data_saver_download_from_edge::Payload::AlertTh(alert_th) => {
                                        debug!("Debug: el mensaje entrante es de tipo AlertTh");
                                        if let Some(metadata) = extract_metadata(alert_th.metadata) {
                                            let msg = AlertThMessage {
                                                metadata,
                                                network: alert_th.network,
                                                initial_temp: alert_th.initial_temp,
                                                actual_temp: alert_th.actual_temp,
                                            };
                                            if tx.send(Message::AlertTem(msg)).await.is_err() {
                                                error!("Error: no se pudo enviar mensaje a dba_task");
                                            }
                                        }
                                    }
                                    data_saver_download_from_edge::Payload::Metrics(metrics) => {
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
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    info!("Info: message_download_task finalizada");
}


/// Inicializa y ejecuta la tarea de subida en un hilo de Tokio.
///
/// # Argumentos
/// * `tx_to_grpc`: Canal hacia la capa de transporte.
/// * `rx_from_heartbeat`: Canal desde el generador de eventos de dominio.
pub fn start_message_upload(tx_to_grpc: mpsc::Sender<DataSaverUpload>,
                            rx_from_heartbeat: mpsc::Receiver<Message>) {
    info!("Info: iniciando tarea message_upload");
    tokio::spawn(async move {
        message_upload(tx_to_grpc,
                       rx_from_heartbeat
        ).await;
    });
}


/// Inicializa y ejecuta la tarea de bajada en un hilo de Tokio.
///
/// # Argumentos
/// * `tx_to_dba`: Canal hacia la capa de base de datos (Batcher).
/// * `rx_from_grpc`: Canal desde la capa de transporte.
pub fn start_message_download(tx_to_dba: mpsc::Sender<Message>,
                              rx_from_grpc: mpsc::Receiver<InternalEvent>) {
    info!("Info: iniciando tarea message_download");
    tokio::spawn(async move {
        message_download(tx_to_dba,
                         rx_from_grpc
        ).await;
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
use tokio::sync::{mpsc};
use tracing::error;
use crate::message::domain::{Measurement as MeasurementMessage, Monitor as MonitorMessage, AlertAir as AlertAirMessage, 
                             AlertTh as AlertThMessage, SystemMetrics as MetricsMessage, Message, Metadata};
use crate::grpc::{data_saver_download_from_edge, DataSaverUpload};
use crate::grpc::data_saver_download::Payload;
use crate::system::domain::InternalEvent;

pub async fn message_upload(tx: mpsc::Sender<DataSaverUpload>,
                            mut rx: mpsc::Receiver<Message>) {
    
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Heartbeat(heartbeat) => {
                let grpc_heartbeat = crate::grpc::Heartbeat {
                    beat: heartbeat.beat,
                };

                let to_edge_payload = crate::grpc::data_saver_upload_to_edge::Payload::Heartbeat(grpc_heartbeat);
                let to_edge_msg = crate::grpc::DataSaverUploadToEdge {
                    payload: Some(to_edge_payload),
                };

                let upload_payload = crate::grpc::data_saver_upload::Payload::ToEdge(to_edge_msg);

                let grpc_msg = DataSaverUpload {
                    sender_user_id: heartbeat.metadata.sender_user_id.clone(),
                    destination_id: heartbeat.metadata.destination_id.clone(),
                    timestamp: heartbeat.metadata.timestamp,
                    payload: Some(upload_payload),
                };

                if tx.send(grpc_msg).await.is_err() {
                    error!("Error: enviando mensaje Heartbeat");
                }
            },
            _ => {}
        }
    }
}


pub async fn message_download(tx: mpsc::Sender<Message>,
                               mut rx: mpsc::Receiver<InternalEvent>) {

    while let Some(msg) = rx.recv().await {
        match msg {
            InternalEvent::IncomingMessage(msg) => {
                let metadata = Metadata {
                    sender_user_id: msg.sender_user_id,
                    destination_id: msg.destination_id,
                    timestamp: msg.timestamp,
                };

                if let Some(payload) = msg.payload {
                    match payload { 
                        Payload::FromEdge(msg_from_edge) => {
                            if let Some(inner) = msg_from_edge.payload {
                                match inner {
                                    data_saver_download_from_edge::Payload::Measurement(measurement) => {
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
                                            error!("Error: No se pudo enviar mensaje a dba_task");
                                        }
                                    }
                                    data_saver_download_from_edge::Payload::Monitor(monitor) => {
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
                                            error!("Error: No se pudo enviar mensaje a dba_task");
                                        }
                                    }
                                    data_saver_download_from_edge::Payload::AlertAir(alert_air) => {
                                        let msg = AlertAirMessage {
                                            metadata,
                                            network: alert_air.network,
                                            co2_initial_ppm: alert_air.co2_initial_ppm,
                                            co2_actual_ppm: alert_air.co2_actual_ppm,
                                        };
                                        if tx.send(Message::AlertAir(msg)).await.is_err() {
                                            error!("Error: No se pudo enviar mensaje a dba_task");
                                        }
                                    }
                                    data_saver_download_from_edge::Payload::AlertTh(alert_th) => {
                                        let msg = AlertThMessage {
                                            metadata,
                                            network: alert_th.network,
                                            initial_temp: alert_th.initial_temp,
                                            actual_temp: alert_th.actual_temp,
                                        };
                                        if tx.send(Message::AlertTem(msg)).await.is_err() {
                                            error!("Error: No se pudo enviar mensaje a dba_task");
                                        }
                                    }
                                    data_saver_download_from_edge::Payload::Metrics(metrics) => {
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
                                            error!("Error: No se pudo enviar mensaje a dba_task");
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


pub fn start_message_upload(tx_to_grpc: mpsc::Sender<DataSaverUpload>,
                            rx_from_heartbeat: mpsc::Receiver<Message>) {

    tokio::spawn(async move {
        message_upload(tx_to_grpc,
                       rx_from_heartbeat
        ).await;
    });
}


pub fn start_message_download(tx_to_dba: mpsc::Sender<Message>,
                              rx_from_grpc: mpsc::Receiver<InternalEvent>) {

    tokio::spawn(async move {
        message_download(tx_to_dba,
                         rx_from_grpc
        ).await;
    });
}
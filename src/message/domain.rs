//! Dominio de Mensajería y Modelos de Datos.
//!
//! Este módulo define las estructuras de datos fundamentales que se intercambian
//! entre los distintos componentes del sistema (Mensajes).
//!


use serde::{Serialize, Deserialize};
use sqlx::FromRow;


/// Metadatos estándar para todos los mensajes del sistema.
///
/// Proporciona contexto de trazabilidad, origen y destino para cada paquete de datos.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow, Hash)]
pub struct Metadata {
    pub sender_user_id: String,
    pub destination_id: String,
    pub timestamp: i64,
}


/// Mediciones de sensores ambientales y operativos.
///
/// Representa el paquete de datos principal generado por los nodos.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct Measurement {
    #[sqlx(flatten)]
    pub metadata: Metadata,
    pub network: String,
    pub pulse_counter: i64,
    pub pulse_max_duration: i64,
    pub temperature: f32,
    pub humidity: f32,
    pub co2_ppm: f32,
    pub sample: u32,
}


/// Alerta de calidad de aire.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct AlertAir {
    #[sqlx(flatten)]
    pub metadata: Metadata,
    pub network: String,
    pub co2_initial_ppm: f32,
    pub co2_actual_ppm: f32,
}


/// Alerta de Temperatura y Humedad.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct AlertTh {
    #[sqlx(flatten)]
    pub metadata: Metadata,
    pub network: String,
    pub initial_temp: f32,
    pub actual_temp: f32,
}


/// Datos de telemetría y salud del Hub.
///
/// Incluye información sobre memoria, stack y conectividad para diagnóstico.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct Monitor {
    #[sqlx(flatten)]
    pub metadata: Metadata,
    pub network: String,
    pub mem_free: i64,
    pub mem_free_hm: i64,
    pub mem_free_block: i64,
    pub mem_free_internal: i64,
    pub stack_free_min_coll: i64,
    pub stack_free_min_pub: i64,
    pub stack_free_min_mic: i64,
    pub stack_free_min_th: i64,
    pub stack_free_min_air: i64,
    pub stack_free_min_mon: i64,
    pub wifi_ssid: String,
    pub wifi_rssi: i8,
    pub active_time: i64,
}


/// Mensaje de latido (Heartbeat) para indicar al Edge que la API está viva.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Heartbeat {
    pub metadata: Metadata,
    pub beat: bool,
}


/// Datos de telemetría y salud del Edge.
///
/// Incluye información sobre memoria, cpu, sd y conectividad para diagnóstico.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SystemMetrics {
    pub metadata: Metadata,
    pub uptime_seconds: u64,
    pub cpu_usage_percent: f32,
    pub cpu_temp_celsius: f32,
    pub ram_total_mb: u64,
    pub ram_used_mb: u64,
    pub sd_total_gb: u64,
    pub sd_used_gb: u64,
    pub sd_usage_percent: f32,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub wifi_rssi: Option<i32>,
    pub wifi_signal_dbm: Option<i32>,
}


/// Wrapper que encapsula los tipos de mensajes posibles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Message {
    Report(Measurement),
    Monitor(Monitor),
    AlertAir(AlertAir),
    AlertTem(AlertTh),
    Heartbeat(Heartbeat),
    Metrics(SystemMetrics),
}
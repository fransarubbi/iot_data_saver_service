//! Módulo de persistencia para Métricas del Sistema Edge (Hardware Health).
//!
//! Registra el estado físico de los dispositivos Edge (CPU, RAM, Disco, Red)
//! para monitoreo de infraestructura.

use chrono::DateTime;
use sqlx::{PgPool, Postgres, QueryBuilder};
use crate::message::domain::{SystemMetrics};


/// Inserta métricas del sistema manejando automáticamente los campos opcionales.
///
/// Si los campos `Option` en Rust son `None`, `sqlx` insertará `NULL` en SQL.
pub async fn insert_system_metrics(pool: &PgPool,
                                   data_vec: Vec<SystemMetrics>
) -> Result<(), sqlx::Error> {

    if data_vec.is_empty() {
        return Ok(());
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO metric (
            sender_user_id, destination_id, timestamp,
            uptime_seconds, cpu_usage_percent, cpu_temp_celsius,
            ram_total_mb, ram_used_mb, sd_total_gb, sd_used_gb, sd_usage_percent,
            network_rx_bytes, network_tx_bytes, wifi_rssi, wifi_signal_dbm
        ) "
    );

    query_builder.push_values(data_vec, |mut b, data| {
        b.push_bind(data.metadata.sender_user_id)
            .push_bind(data.metadata.destination_id)
            .push_bind(DateTime::from_timestamp(data.metadata.timestamp, 0).unwrap_or_default())
            .push_bind(data.uptime_seconds as i64)
            .push_bind(data.cpu_usage_percent)
            .push_bind(data.cpu_temp_celsius)
            .push_bind(data.ram_total_mb as i64)
            .push_bind(data.ram_used_mb as i64)
            .push_bind(data.sd_total_gb as i64)
            .push_bind(data.sd_used_gb as i64)
            .push_bind(data.sd_usage_percent)
            .push_bind(data.network_rx_bytes as i64)
            .push_bind(data.network_tx_bytes as i64)
            .push_bind(data.wifi_rssi)
            .push_bind(data.wifi_signal_dbm);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}
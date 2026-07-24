//! Módulo de persistencia para Mediciones (Telemetría Principal).
//!
//! Este módulo maneja la tabla con mayor volumen de escritura del sistema,
//! almacenando los reportes periódicos de los sensores.

use crate::bucket::logic::ProcessedTelemetry;
use chrono::DateTime;
use sqlx::PgPool;

pub async fn insert_measurement(
    pool: &PgPool,
    data: ProcessedTelemetry,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO measurement (timestamp,
                                 network_id,
                                 pulse_counter,
                                 temperature,
                                 humidity,
                                 air_quality)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(DateTime::from_timestamp(data.timestamp, 0).unwrap_or_default())
    .bind(data.network_id)
    .bind(data.pulse_counter_total)
    .bind(data.temperature)
    .bind(data.humidity)
    .bind(data.air_quality)
    .execute(pool)
    .await?;

    Ok(())
}

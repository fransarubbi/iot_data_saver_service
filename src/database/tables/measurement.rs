//! Módulo de persistencia para Mediciones (Telemetría Principal).
//!
//! Este módulo maneja la tabla con mayor volumen de escritura del sistema,
//! almacenando los reportes periódicos de los sensores.

use chrono::DateTime;
use sqlx::{PgPool};
use crate::bucket::logic::ProcessedTelemetry;


pub async fn insert_measurement(pool: &PgPool,
                                data: ProcessedTelemetry
) -> Result<(), sqlx::Error> {

    sqlx::query(
        r#"
        INSERT INTO measurement (timestamp,
                                 network_id,
                                 pulse_counter,
                                 pulse_max_duration,
                                 temperature,
                                 humidity,
                                 co2_ppm)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
        .bind(DateTime::from_timestamp(data.timestamp, 0).unwrap_or_default())
        .bind(data.network_id)
        .bind(data.pulse_counter_total)
        .bind(data.pulse_max_duration)
        .bind(data.temperature)
        .bind(data.humidity)
        .bind(data.co2_ppm)
        .execute(pool)
        .await?;

    Ok(())
}

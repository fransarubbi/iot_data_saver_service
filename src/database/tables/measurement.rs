//! Módulo de persistencia para Mediciones (Telemetría Principal).
//!
//! Este módulo maneja la tabla con mayor volumen de escritura del sistema,
//! almacenando los reportes periódicos de los sensores.

use chrono::DateTime;
use sqlx::{PgPool, Postgres, QueryBuilder};
use crate::message::domain::{Measurement};


/// Ejecuta una inserción masiva de mediciones.
///
/// # Casting
/// Realiza conversiones explícitas (ej. `sample as i64`) para asegurar compatibilidad
/// estricta con los tipos de PostgreSQL.
pub async fn insert_measurement(pool: &PgPool,
                                data_vec: Vec<Measurement>
) -> Result<(), sqlx::Error> {

    if data_vec.is_empty() {
        return Ok(());
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO measurement (
            sender_user_id, destination_id, timestamp,
            network_id, pulse_counter, pulse_max_duration,
            temperature, humidity, co2_ppm, sample
        ) "
    );

    query_builder.push_values(data_vec, |mut b, data| {
        b.push_bind(data.metadata.sender_user_id)
            .push_bind(data.metadata.destination_id)
            .push_bind(DateTime::from_timestamp(data.metadata.timestamp, 0).unwrap_or_default())
            .push_bind(data.network)
            .push_bind(data.pulse_counter)
            .push_bind(data.pulse_max_duration)
            .push_bind(data.temperature)
            .push_bind(data.humidity)
            .push_bind(data.co2_ppm)
            .push_bind(data.sample as i64);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}

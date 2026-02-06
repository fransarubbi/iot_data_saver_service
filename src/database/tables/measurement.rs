//! Módulo de persistencia para Mediciones (Telemetría Principal).
//!
//! Este módulo maneja la tabla con mayor volumen de escritura del sistema,
//! almacenando los reportes periódicos de los sensores.


use sqlx::{Executor, PgPool, Postgres, QueryBuilder};
use crate::message::domain::{Measurement};


/// Inicializa la tabla `measurement`.
///
/// # Tipos de Datos
/// * `id`: Serial (Auto-incremental).
/// * `sender_user_id`: Identificador del dispositivo Hub.
/// * `timestamp`: Marca de tiempo cuando se generó el mensaje.
/// * `network_id`: Identificador de la red a la que está conectado el Hub.
/// * `pulse_counter`: Contador acumulado de pulsos de sonido (BIGINT/i64).
/// * `pulse_max_counter`: Contador acumulado de pulsos de sonido (BIGINT/i64).
/// * `temperature`, `humidity`, `co2_ppm`: Variables ambientales (REAL/f32).
/// * `sample`: Tiempo de sampleo del Hub (BIGINT/i64).
pub async fn create_table_measurement(pool: &PgPool) -> Result<(), sqlx::Error>  {
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS measurement (
            id                   SERIAL PRIMARY KEY,
            sender_user_id       TEXT NOT NULL,
            destination_id       TEXT NOT NULL,
            timestamp            BIGINT NOT NULL,
            network_id           TEXT NOT NULL,
            pulse_counter        BIGINT NOT NULL,
            pulse_max_duration   BIGINT NOT NULL,
            temperature          REAL NOT NULL,
            humidity             REAL NOT NULL,
            co2_ppm              REAL NOT NULL,
            sample               BIGINT NOT NULL
        );
        "#
    )
        .await?;

    Ok(())
}


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
            .push_bind(data.metadata.timestamp)
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

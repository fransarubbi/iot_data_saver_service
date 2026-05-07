//! Módulo de persistencia para Alertas de Calidad de Aire (CO2).
//!
//! Gestiona la creación de la tabla y la inserción eficiente de eventos críticos
//! relacionados con la calidad del aire detectada por los sensores.


use sqlx::{PgPool, Postgres, QueryBuilder};
use chrono::{DateTime};
use crate::message::domain::AlertAir;


/// Realiza una inserción masiva (batch) de alertas de aire usando `QueryBuilder`.
///
/// Utiliza `push_values` para construir una única sentencia SQL con múltiples filas,
/// optimizando el rendimiento de red y base de datos.
///
/// # Argumentos
/// * `pool`: Pool de conexiones a Postgres.
/// * `data_vec`: Vector con las alertas a insertar.
pub async fn insert_alert_air(pool: &PgPool,
                              data_vec: Vec<AlertAir>
) -> Result<(), sqlx::Error> {

    if data_vec.is_empty() {
        return Ok(());
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO alert_air (
            sender_user_id, destination_id, timestamp,
            network_id, co2_initial_ppm, co2_actual_ppm
        ) "
    );

    query_builder.push_values(data_vec, |mut b, data| {
        b.push_bind(data.metadata.sender_user_id)
            .push_bind(data.metadata.destination_id)
            .push_bind(DateTime::from_timestamp(data.metadata.timestamp, 0).unwrap_or_default())
            .push_bind(data.network)
            .push_bind(data.co2_initial_ppm)
            .push_bind(data.co2_actual_ppm);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}





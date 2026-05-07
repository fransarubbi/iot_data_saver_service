//! Módulo de persistencia para Alertas de Temperatura y Humedad.
//!

use chrono::DateTime;
use sqlx::{PgPool, Postgres, QueryBuilder};
use crate::message::domain::{AlertTh};


/// Inserta un lote de alertas de temperatura de forma eficiente.
///
/// # Argumentos
/// * `data_vec`: Vector de alertas (`AlertTh`) acumuladas en memoria.
pub async fn insert_alert_temp(pool: &PgPool,
                               data_vec: Vec<AlertTh>
) -> Result<(), sqlx::Error> {

    if data_vec.is_empty() {
        return Ok(());
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO alert_temp (
            sender_user_id, destination_id, timestamp,
            network_id, initial_temp, actual_temp
        ) "
    );

    query_builder.push_values(data_vec, |mut b, data| {
        b.push_bind(data.metadata.sender_user_id)
            .push_bind(data.metadata.destination_id)
            .push_bind(DateTime::from_timestamp(data.metadata.timestamp, 0).unwrap_or_default())
            .push_bind(data.network)
            .push_bind(data.initial_temp)
            .push_bind(data.actual_temp);
    });
    
    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}

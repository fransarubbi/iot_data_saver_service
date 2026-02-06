//! Módulo de persistencia para Alertas de Temperatura y Humedad.
//!


use sqlx::{Executor, PgPool, Postgres, QueryBuilder};
use crate::message::domain::{AlertTh};


/// Crea la tabla `alert_temp` para almacenar históricos de alertas térmicas.
///
/// # Schema
/// * `id`: Serial (Auto-incremental).
/// * `sender_user_id`: Identificador del dispositivo Hub.
/// * `network_id`: Identificador de la red a la que está conectado el Hub.
/// * `initial_temp`: Temperatura registrada al inicio de la alerta.
/// * `actual_temp`: Temperatura actual de seguimiento.
pub async fn create_table_alert_temp(pool: &PgPool) -> Result<(), sqlx::Error>  {
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS alert_temp (
            id                   SERIAL PRIMARY KEY,
            sender_user_id       TEXT NOT NULL,
            destination_id       TEXT NOT NULL,
            timestamp            BIGINT NOT NULL,
            network_id           TEXT NOT NULL,
            initial_temp         REAL NOT NULL,
            actual_temp          REAL NOT NULL
        );
        "#
    )
        .await?;

    Ok(())
}


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
            .push_bind(data.metadata.timestamp)
            .push_bind(data.network)
            .push_bind(data.initial_temp)
            .push_bind(data.actual_temp);
    });
    
    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}

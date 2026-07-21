//! Módulo de persistencia para Métricas del Hub (Diagnóstico).
//!
//! Almacena estadísticas de bajo nivel sobre el uso de memoria (Stack/Heap)
//! de las tareas FreeRTOS en el microcontrolador.

use chrono::DateTime;
use sqlx::{PgPool, Postgres, QueryBuilder};
use crate::message::domain::Monitor;


/// Batch insert para datos de diagnóstico de firmware.
pub async fn insert_monitor(pool: &PgPool,
                            data_vec: Vec<Monitor>
) -> Result<(), sqlx::Error> {

    if data_vec.is_empty() {
        return Ok(());
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO monitor (
            sender_user_id, destination_id, timestamp, network_id,
            mem_free, mem_free_hm, mem_free_block, active_time
        ) "
    );

    query_builder.push_values(data_vec, |mut b, data| {
        b.push_bind(data.metadata.sender_user_id)
            .push_bind(data.metadata.destination_id)
            .push_bind(DateTime::from_timestamp(data.metadata.timestamp, 0).unwrap_or_default())
            .push_bind(data.network)
            .push_bind(data.heap_free as i64)
            .push_bind(data.heap_min_free as i64)
            .push_bind(data.heap_largest_block as i64)
            .push_bind(data.uptime_sec);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}
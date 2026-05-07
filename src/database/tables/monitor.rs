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
            mem_free, mem_free_hm, mem_free_block, mem_free_internal,
            stack_free_min_coll, stack_free_min_pub, stack_free_min_mic,
            stack_free_min_th, stack_free_min_air, stack_free_min_mon,
            wifi_ssid, wifi_rssi, active_time
        ) "
    );

    query_builder.push_values(data_vec, |mut b, data| {
        b.push_bind(data.metadata.sender_user_id)
            .push_bind(data.metadata.destination_id)
            .push_bind(DateTime::from_timestamp(data.metadata.timestamp, 0).unwrap_or_default())
            .push_bind(data.network)
            .push_bind(data.mem_free)
            .push_bind(data.mem_free_hm)
            .push_bind(data.mem_free_block)
            .push_bind(data.mem_free_internal)
            .push_bind(data.stack_free_min_coll)
            .push_bind(data.stack_free_min_pub)
            .push_bind(data.stack_free_min_mic)
            .push_bind(data.stack_free_min_th)
            .push_bind(data.stack_free_min_air)
            .push_bind(data.stack_free_min_mon)
            .push_bind(data.wifi_ssid)
            .push_bind(data.wifi_rssi as i32)
            .push_bind(data.active_time);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}
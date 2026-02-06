//! Módulo de persistencia para Métricas del Hub (Diagnóstico).
//!
//! Almacena estadísticas de bajo nivel sobre el uso de memoria (Stack/Heap)
//! de las tareas FreeRTOS en el microcontrolador.


use sqlx::{Executor, PgPool, Postgres, QueryBuilder};
use crate::message::domain::Monitor;


/// Crea la tabla `monitor` con columnas para marcas de agua (watermarks) de stack.
///
/// Cada columna `stack_free_min_*` representa el mínimo de memoria libre alcanzado
/// por una tarea específica, vital para detectar desbordamientos de pila.
pub async fn create_table_monitor(pool: &PgPool) -> Result<(), sqlx::Error>  {
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS monitor (
            id                   SERIAL PRIMARY KEY,
            sender_user_id       TEXT NOT NULL,
            destination_id       TEXT NOT NULL,
            timestamp            BIGINT NOT NULL,
            network_id           TEXT NOT NULL,
            mem_free             BIGINT NOT NULL,
            mem_free_hm          BIGINT NOT NULL,
            mem_free_block       BIGINT NOT NULL,
            mem_free_internal    BIGINT NOT NULL,
            stack_free_min_coll  BIGINT NOT NULL,
            stack_free_min_pub   BIGINT NOT NULL,
            stack_free_min_mic   BIGINT NOT NULL,
            stack_free_min_th    BIGINT NOT NULL,
            stack_free_min_air   BIGINT NOT NULL,
            stack_free_min_mon   BIGINT NOT NULL,
            wifi_ssid            TEXT NOT NULL,
            wifi_rssi            INTEGER NOT NULL,
            active_time          BIGINT NOT NULL
        );
        "#
    )
        .await?;

    Ok(())
}


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
            .push_bind(data.metadata.timestamp)
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
            .push_bind(data.wifi_rssi)
            .push_bind(data.active_time);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}
use sqlx::{Executor, PgPool, Postgres, QueryBuilder};
use crate::message::domain::{SystemMetrics};



pub async fn create_table_system_metrics(pool: &PgPool) -> Result<(), sqlx::Error> {
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS metric (
            id                   SERIAL PRIMARY KEY,
            sender_user_id       TEXT NOT NULL,
            destination_id       TEXT NOT NULL,
            timestamp            BIGINT NOT NULL,
            uptime_seconds       BIGINT NOT NULL,
            cpu_usage_percent    REAL NOT NULL,
            cpu_temp_celsius     REAL NOT NULL,
            ram_total_mb         BIGINT NOT NULL,
            ram_used_mb          BIGINT NOT NULL,
            sd_total_gb          BIGINT NOT NULL,
            sd_used_gb           BIGINT NOT NULL,
            sd_usage_percent     REAL NOT NULL,
            network_rx_bytes     BIGINT NOT NULL,
            network_tx_bytes     BIGINT NOT NULL,
            wifi_rssi            INTEGER,
            wifi_signal_dbm      INTEGER
        );
        "#
    )
        .await?;

    Ok(())
}


pub async fn insert_system_metrics(
    pool: &PgPool,
    data_vec: Vec<SystemMetrics>
) -> Result<(), sqlx::Error> {

    if data_vec.is_empty() {
        return Ok(());
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO metric (
            sender_user_id, destination_id, timestamp,
            uptime_seconds, cpu_usage_percent, cpu_temp_celsius,
            ram_total_mb, ram_used_mb, sd_total_gb, sd_used_gb, sd_usage_percent,
            network_rx_bytes, network_tx_bytes, wifi_rssi, wifi_signal_dbm
        ) "
    );

    query_builder.push_values(data_vec, |mut b, data| {
        b.push_bind(data.metadata.sender_user_id)
            .push_bind(data.metadata.destination_id)
            .push_bind(data.metadata.timestamp)
            .push_bind(data.uptime_seconds as i64)
            .push_bind(data.cpu_usage_percent)
            .push_bind(data.cpu_temp_celsius)
            .push_bind(data.ram_total_mb as i64)
            .push_bind(data.ram_used_mb as i64)
            .push_bind(data.sd_total_gb as i64)
            .push_bind(data.sd_used_gb as i64)
            .push_bind(data.sd_usage_percent)
            .push_bind(data.network_rx_bytes as i64)
            .push_bind(data.network_tx_bytes as i64)
            .push_bind(data.wifi_rssi)
            .push_bind(data.wifi_signal_dbm);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}
use sqlx::{Executor, PgPool, Postgres, QueryBuilder};
use crate::message::domain::{Measurement};


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

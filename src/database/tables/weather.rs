use sqlx::{PgPool};
use crate::weather::domain::Weather;


pub async fn insert_weather(pool: &PgPool, data: Weather) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO weather (timestamp, temperature, humidity)
        VALUES ($1, $2, $3)
        "#,
    )
        .bind(data.timestamp)
        .bind(data.temperature_2m)
        .bind(data.relative_humidity_2m)
        .execute(pool)
        .await?;

    Ok(())
}
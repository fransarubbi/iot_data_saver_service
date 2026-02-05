use sqlx::{Executor, PgPool, Postgres, QueryBuilder};
use crate::message::domain::AlertAir;


/// Inicializa la tabla `alert_air` en la base de datos si no existe.
///
/// Esta tabla almacena eventos de calidad de aire (CO2) que superan los umbrales definidos.
pub async fn create_table_alert_air(pool: &PgPool) -> Result<(), sqlx::Error>  {
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS alert_air (
            id                   SERIAL PRIMARY KEY,
            sender_user_id       TEXT NOT NULL,
            destination_id       TEXT NOT NULL,
            timestamp            BIGINT NOT NULL,
            network_id           TEXT NOT NULL,
            co2_initial_ppm      REAL NOT NULL,
            co2_actual_ppm       REAL NOT NULL
        );
        "#
    )
        .await?;
    Ok(())
}


/// Realiza una inserción masiva (batch) de alertas de aire usando `QueryBuilder`.
///
/// Optimizado para reducir el número de round-trips a la base de datos.
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
            .push_bind(data.metadata.timestamp)
            .push_bind(data.network)
            .push_bind(data.co2_initial_ppm)
            .push_bind(data.co2_actual_ppm);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}





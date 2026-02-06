use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing::error;
use tokio::time::sleep;
use crate::config::postgres::{MAX_CONNECTIONS, WAIT_FOR};
use crate::database::domain::TableDataVector;
use crate::database::tables::alert_air::{create_table_alert_air, insert_alert_air};
use crate::database::tables::alert_temp::{create_table_alert_temp, insert_alert_temp};
use crate::database::tables::measurement::{create_table_measurement, insert_measurement};
use crate::database::tables::metrics::{create_table_system_metrics, insert_system_metrics};
use crate::database::tables::monitor::{create_table_monitor, insert_monitor};


#[derive(Clone, Debug)]
pub struct Repository {
    pool: PgPool,
}

impl Repository {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = create_pool(database_url).await?;
        init_schema(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn create_repository(database_url: &str) -> Self {
        loop {
            match Self::new(database_url).await {
                Ok(repo) => return repo,
                Err(e) => {
                    error!("Error inicializando repo: {:?}", e);
                    sleep(WAIT_FOR).await;
                }
            }
        }
    }

    pub async fn insert(&self, tdv: TableDataVector) -> Result<(), sqlx::Error> {
        insert_measurement(&self.pool, tdv.measurement).await?;
        insert_monitor(&self.pool, tdv.monitor).await?;
        insert_alert_temp(&self.pool, tdv.alert_th).await?;
        insert_alert_air(&self.pool, tdv.alert_air).await?;
        insert_system_metrics(&self.pool, tdv.system_metrics).await?;
        Ok(())
    }
}


async fn create_pool(db_path: &str) -> Result<PgPool, sqlx::Error> {
    let database_url = format!("postgres://{}", db_path);

    let pool = PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect(&database_url)
        .await?;

    Ok(pool)
}


async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    create_table_measurement(pool).await?;
    create_table_monitor(pool).await?;
    create_table_alert_temp(pool).await?;
    create_table_alert_air(pool).await?;
    create_table_system_metrics(pool).await?;
    Ok(())
}
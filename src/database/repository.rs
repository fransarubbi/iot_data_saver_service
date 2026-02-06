//! Capa de Abstracción del Repositorio (DAL).
//!
//! Este módulo centraliza todas las interacciones con la base de datos PostgreSQL.
//! Actúa como una fachada que coordina la creación de conexiones, la inicialización
//! del esquema y la delegación de inserciones a los módulos de tabla específicos.
//!
//! # Características
//! * **Pool Management:** Gestiona el ciclo de vida del pool de conexiones `sqlx`.
//! * **Resiliencia:** Implementa lógica de reintento (backoff) durante el inicio.
//! * **Batch Routing:** Despacha los datos acumulados a las tablas correspondientes.


use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing::{debug, error, info};
use tokio::time::sleep;
use crate::database::domain::TableDataVector;
use crate::database::tables::alert_air::{create_table_alert_air, insert_alert_air};
use crate::database::tables::alert_temp::{create_table_alert_temp, insert_alert_temp};
use crate::database::tables::measurement::{create_table_measurement, insert_measurement};
use crate::database::tables::metrics::{create_table_system_metrics, insert_system_metrics};
use crate::database::tables::monitor::{create_table_monitor, insert_monitor};
use crate::system::domain::database::WAIT_FOR;
use crate::system::domain::System;


/// Gestor principal de persistencia.
///
/// Es barato de clonar (`Clone`) ya que envuelve un `Arc<PgPool>` internamente.
/// Está diseñado para ser compartido entre múltiples tareas asíncronas.
#[derive(Clone, Debug)]
pub struct Repository {
    /// Pool de conexiones asíncronas a PostgreSQL.
    pool: PgPool,
}


impl Repository {

    /// Intenta establecer una conexión y preparar la base de datos una sola vez.
    ///
    /// # Pasos
    /// 1. Crea el pool de conexiones según la configuración en `System`.
    /// 2. Ejecuta las sentencias `CREATE TABLE IF NOT EXISTS` para todas las entidades.
    ///
    /// # Retorno
    /// Retorna `Err` si la base de datos no está disponible inmediatamente.
    pub async fn new(system: &System) -> Result<Self, sqlx::Error> {
        let pool = create_pool(system).await?;
        init_schema(&pool).await?;
        Ok(Self { pool })
    }

    /// Constructor resiliente con bucle de reintento infinito.
    ///
    /// Este es el método recomendado para iniciar la aplicación. Si la base de datos
    /// no está lista (ej. contenedor levantándose), bloqueará la tarea actual y
    /// reintentará cada `WAIT_FOR` segundos hasta tener éxito.
    ///
    /// # Argumentos
    /// * `system`: Configuración global del sistema.
    pub async fn create_repository(system: &System) -> Self {
        info!("Info: creando repository");
        loop {
            match Self::new(system).await {
                Ok(repo) => return repo,
                Err(e) => {
                    error!("Error: no se pudo crear repository. Reintentando. {:?}", e);
                    sleep(WAIT_FOR).await;
                }
            }
        }
    }

    /// Persiste un lote heterogéneo de datos en la base de datos.
    ///
    /// Recibe un `TableDataVector` (que actúa como buffer) e inspecciona sus campos.
    /// Si un vector específico no está vacío, delega la inserción a la función correspondiente
    /// del módulo de tablas.
    ///
    /// # Transaccionalidad
    /// Las inserciones se ejecutan secuencialmente. Si ocurre un error a mitad de camino (ej. en `alert_air`),
    /// las inserciones previas (ej. `measurement`) **permanecen confirmadas**.
    ///
    /// # Argumentos
    /// * `tdv`: Estructura que contiene vectores de datos (`Vec<Measurement>`, `Vec<Monitor>`, etc.).
    pub async fn insert(&self, tdv: TableDataVector) -> Result<(), sqlx::Error> {
        debug!("Debug: insertando batch en base de datos");
        if !tdv.measurement.is_empty() {
            insert_measurement(&self.pool, tdv.measurement).await?;
        }
        if !tdv.monitor.is_empty() {
            insert_monitor(&self.pool, tdv.monitor).await?;
        }
        if !tdv.alert_th.is_empty() {
            insert_alert_temp(&self.pool, tdv.alert_th).await?;
        }
        if !tdv.alert_air.is_empty() {
            insert_alert_air(&self.pool, tdv.alert_air).await?;
        }
        if !tdv.system_metrics.is_empty() {
            insert_system_metrics(&self.pool, tdv.system_metrics).await?;
        }
        Ok(())
    }
}


/// Helper privado para configurar el pool de conexiones de sqlx.
///
/// Aplica configuraciones como `max_connections` desde la estructura `System`.
async fn create_pool(system: &System) -> Result<PgPool, sqlx::Error> {
    info!("Info: creando pool");

    let pool = PgPoolOptions::new()
        .max_connections(system.db_pool_size)
        .connect(&system.database_url)
        .await?;

    Ok(pool)
}


/// Helper privado para asegurar que el esquema SQL exista.
///
/// Ejecuta las funciones `create_table_*` de cada módulo en orden secuencial.
async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Info: creando schema de base de datos");
    create_table_measurement(pool).await?;
    create_table_monitor(pool).await?;
    create_table_alert_temp(pool).await?;
    create_table_alert_air(pool).await?;
    create_table_system_metrics(pool).await?;
    Ok(())
}
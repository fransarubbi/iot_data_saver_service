//! Módulo de configuración central y gestión del entorno de ejecución.
//!
//! Este módulo actúa como la fuente única de verdad para la configuración de la aplicación.
//! Se encarga de leer las variables de entorno, establecer valores por defecto seguros
//! y proveer las estructuras necesarias para iniciar los subsistemas (Base de Datos, gRPC, Logging).
//!
//! # Funcionalidades Principales
//! * **Carga de Configuración:** Lee de `.env` en desarrollo y variables de sistema en producción.
//! * **Observabilidad:** Configura `tracing_subscriber` para logs estructurados o legibles.
//! * **Constantes Operativas:** Define timeouts y tamaños de lote para I/O.
//!


use std::env;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};
use crate::grpc::DataSaverDownload;


/// Representa la configuración global del sistema y el estado del entorno.
///
/// Esta estructura centraliza todas las variables de entorno y configuraciones
/// necesarias para iniciar los servicios (Base de datos, gRPC, Logging).
///
#[derive(Debug)]
pub struct System {
    /// URL de conexión a PostgreSQL (ej. `postgres://user:pass@localhost:5432/db`).
    /// **Requerido**.
    pub database_url: String,

    /// Tamaño máximo del pool de conexiones a la base de datos.
    /// Por defecto: `10`.
    pub db_pool_size: u32,

    /// Host donde escuchará o se conectará el servicio gRPC.
    /// Por defecto: `localhost`.
    pub grpc_host: String,

    /// Puerto del servicio gRPC.
    /// Por defecto: `50052`.
    pub grpc_port: u16,

    /// Intervalo en segundos para enviar señales de vida (Heartbeat).
    /// Por defecto: `30` segundos.
    pub heartbeat_interval_secs: u64,

    /// Entorno de ejecución actual (`development`, `staging`, `production`).
    /// Afecta el formato de logs y la carga de archivos `.env`.
    pub environment: String,

    /// Nivel de detalle de los logs (ej. `info`, `debug`, `warn`).
    /// Se autoconfigura según el `environment` si no se especifica.
    pub rust_log: String,
}


impl System {

    /// Carga la configuración desde las variables de entorno.
    ///
    /// # Comportamiento
    /// * Si `ENVIRONMENT` es "development", intenta cargar un archivo `.env`.
    /// * Si falta alguna variable requerida (como `DATABASE_URL`), el programa entrará en pánico (`panic`).
    /// * Establece valores por defecto para variables opcionales.
    ///
    /// # Panics
    /// * Si `DATABASE_URL` no está definida.
    /// * Si las variables numéricas (`PORT`, `POOL_SIZE`) no son números válidos.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {

        info!("Info: creando objeto system");

        let environment = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".into());

        if environment == "development" {
            dotenv::dotenv().ok();
        }

        Ok(System {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL no está configurada"),

            db_pool_size: env::var("DB_POOL_SIZE")
                .unwrap_or("10".to_string())
                .parse()
                .expect("DB_POOL_SIZE debe ser un número"),

            grpc_host: env::var("GRPC_HOST")
                .unwrap_or("localhost".to_string()),

            grpc_port: env::var("GRPC_PORT")
                .unwrap_or("50052".to_string())
                .parse()
                .expect("GRPC_PORT debe ser un número"),

            heartbeat_interval_secs: env::var("HEARTBEAT_INTERVAL_SECS")
                .unwrap_or("30".to_string())
                .parse()
                .expect("HEARTBEAT_INTERVAL_SECS debe ser un número"),

            rust_log: env::var("RUST_LOG")
                .unwrap_or_else(|_| {
                    match environment.as_str() {
                        "development" => "debug".to_string(),
                        "staging" => "info".to_string(),
                        _ => "warn".to_string(),
                    }
                }),

            environment,
        })
    }
}


/// Eventos internos que circulan por los canales del sistema (MPSC).
///
/// Se utiliza para desacoplar la recepción de datos gRPC de su procesamiento.
pub enum InternalEvent {
    IncomingMessage(DataSaverDownload)
}


/// Categorización de errores operativos del sistema.
#[derive(Debug)]
pub enum ErrorType {
    Endpoint,
}


/// Inicializa el sistema de trazabilidad y logs (Tracing).
///
/// Configura el formato de salida basándose en el entorno:
/// * **Production**: Salida JSON (para logs estructurados en la nube).
/// * **Development/Otros**: Salida "Pretty" (colores y formato legible).
///
/// # Argumentos
/// * `system`: Referencia a la configuración cargada para leer el nivel de log (`rust_log`).
pub fn init_tracing(system: &System) {

    let filter = EnvFilter::try_new(&system.rust_log)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let builder = fmt().with_env_filter(filter).with_target(false);

    if system.environment == "production" {
        builder.json().init();
    } else {
        builder.pretty().init();
    }
}


/// Constantes de configuración para la base de datos.
pub mod database {
    use tokio::time::{Duration};
    pub const WAIT_FOR: Duration = Duration::from_secs(5);
    pub const BATCH_SIZE: usize = 100;
}


/// Constantes de configuración para el cliente gRPC.
pub mod grpc_service_const {
    pub const TIMEOUT_SECS: u64 = 10;
    pub const KEEP_ALIVE_TIMEOUT_SECS: u64 = 30;
    pub const KEEP_ALIVE_INTERVAL_SECS: u64 = 15;
}
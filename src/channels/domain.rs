//! Registro central de canales de comunicación (Wiring Harness).
//!
//! Este módulo actúa como el "sistema nervioso" de la aplicación. Su responsabilidad es
//! inicializar todos los pares de canales `mpsc` (Multi-Producer, Single-Consumer) necesarios
//! para interconectar las distintas tareas asíncronas (Actores) del sistema.
//!
//! # Arquitectura
//! La aplicación sigue el Modelo de Actores. Las tareas no comparten memoria, sino que
//! se comunican enviando mensajes. Esta estructura `Channels` se crea al inicio (`main.rs`)
//! y luego se "desmembra" (destructuring), entregando a cada tarea solo los extremos
//! (Sender o Receiver) que necesita para funcionar.


use tokio::sync::mpsc;
use tracing::info;
use crate::bucket::logic::{BucketData, ProcessedTelemetry};
use crate::grpc::{FromDataSaver};
use crate::heartbeat::domain::Event;
use crate::message::domain::Message;
use crate::system::domain::InternalEvent;
use crate::weather::domain::{Weather};


/// Contenedor de todos los canales de comunicación del sistema.
///
/// Agrupa los extremos de envío (`Sender`) y recepción (`Receiver`) para facilitar
/// la inyección de dependencias en las tareas durante el arranque.
pub struct Channels {
    pub heartbeat_to_watchdog: mpsc::Sender<Event>,
    pub watchdog_from_heartbeat: mpsc::Receiver<Event>,
    pub watchdog_to_heartbeat: mpsc::Sender<Event>,
    pub heartbeat_from_watchdog: mpsc::Receiver<Event>,
    pub heartbeat_to_upload_message: mpsc::Sender<Message>,
    pub upload_message_from_heartbeat: mpsc::Receiver<Message>,
    pub upload_message_to_grpc: mpsc::Sender<FromDataSaver>,
    pub grpc_from_upload_message: mpsc::Receiver<FromDataSaver>,
    pub download_message_to_bucket: mpsc::Sender<BucketData>,
    pub bucket_from_download_message: mpsc::Receiver<BucketData>,
    pub grpc_to_download_message: mpsc::Sender<InternalEvent>,
    pub download_message_from_grpc: mpsc::Receiver<InternalEvent>,
    pub weather_to_dba: mpsc::Sender<Weather>,
    pub dba_from_weather: mpsc::Receiver<Weather>,
    pub download_message_to_dba: mpsc::Sender<Message>,
    pub dba_from_download_message: mpsc::Receiver<Message>,
    pub sweeper_to_dba: mpsc::Sender<ProcessedTelemetry>,
    pub dba_from_sweeper: mpsc::Receiver<ProcessedTelemetry>,
}


impl Channels {

    /// Inicializa todos los canales con sus capacidades de buffer específicas.
    ///
    /// # Dimensionamiento de Buffers (Backpressure)
    /// * **Buffer Pequeño (10):** Para señales de control (Heartbeat/Watchdog). Estos eventos son
    ///   poco frecuentes (cada 30s) y críticos. No requieren gran cola.
    /// * **Buffer Grande (200):** Para el flujo de datos principal (gRPC/DB). Permite absorber
    ///   picos de tráfico (bursts) provenientes de la red sin bloquear inmediatamente al emisor,
    ///   mejorando el rendimiento general (throughput).
    pub fn new() -> Channels {
        info!("Info: creando canales de comunicación");
        let (heartbeat_to_watchdog, watchdog_from_heartbeat) = mpsc::channel::<Event>(50);
        let (watchdog_to_heartbeat, heartbeat_from_watchdog) = mpsc::channel::<Event>(50);
        let (heartbeat_to_upload_message, upload_message_from_heartbeat) = mpsc::channel::<Message>(50);
        let (upload_message_to_grpc, grpc_from_upload_message) = mpsc::channel::<FromDataSaver>(200);
        let (download_message_to_bucket, bucket_from_download_message) = mpsc::channel::<BucketData>(200);
        let (grpc_to_download_message, download_message_from_grpc) = mpsc::channel::<InternalEvent>(200);
        let (weather_to_dba, dba_from_weather) = mpsc::channel::<Weather>(10);
        let (sweeper_to_dba, dba_from_sweeper) = mpsc::channel::<ProcessedTelemetry>(10);
        let (download_message_to_dba, dba_from_download_message) = mpsc::channel::<Message>(50);

        Self {
            heartbeat_to_watchdog,
            watchdog_from_heartbeat,
            watchdog_to_heartbeat,
            heartbeat_from_watchdog,
            heartbeat_to_upload_message,
            upload_message_from_heartbeat,
            upload_message_to_grpc,
            grpc_from_upload_message,
            download_message_to_bucket,
            bucket_from_download_message,
            grpc_to_download_message,
            download_message_from_grpc,
            weather_to_dba,
            dba_from_weather,
            sweeper_to_dba,
            dba_from_sweeper,
            download_message_to_dba,
            dba_from_download_message
        }
    }
}
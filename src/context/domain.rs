//! Definición del Contexto de Aplicación (Shared State).
//!
//! Este módulo implementa el patrón de **Estado Compartido** para aplicaciones asíncronas.
//! El `AppContext` actúa como un contenedor de "Inyección de Dependencias" manual,
//! agrupando los recursos que deben ser accesibles por múltiples tareas concurrentes
//! (Base de datos, Configuración, Caché en memoria).


use std::sync::Arc;
use tracing::info;
use crate::alert_issuer::domain::TelegramNotifier;
use crate::database::repository::Repository;
use crate::system::domain::{System};
use dashmap::DashMap;
use crate::message::domain::Measurement;


pub type BucketKey = (String, i64);
pub type SensorDataVector = Vec<Measurement>;
pub type StateMap = Arc<DashMap<BucketKey, SensorDataVector>>;


#[derive(Clone, Debug)]
pub struct AppContext {
    pub repo: Repository,
    pub system: Arc<System>,
    pub telegram_notifier: TelegramNotifier,
    pub bucket_map: Arc<DashMap<BucketKey, SensorDataVector>>,
}


impl AppContext {
    pub async fn new() -> Self {
        
        info!("Info: creando app context");

        let bucket_map: StateMap = Arc::new(DashMap::new());
        
        let system = Arc::new(
            match System::new() {
                Ok(system) => system,
                Err(e) => panic!("Error: no se pudo crear system. {}", e),
            }
        );
        
        let repo = Repository::create_repository(&system).await;
        
        let telegram_notifier = match TelegramNotifier::new() {
            Ok(telegram_notifier) => telegram_notifier,
            Err(e) => panic!("Error: no se pudo crear telegram_notifier. {}", e),
        };
        
        Self { repo, system, telegram_notifier, bucket_map }
    }
}
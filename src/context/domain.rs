//! Definición del Contexto de Aplicación (Shared State).
//!
//! Este módulo implementa el patrón de **Estado Compartido** para aplicaciones asíncronas.
//! El `AppContext` actúa como un contenedor de "Inyección de Dependencias" manual,
//! agrupando los recursos que deben ser accesibles por múltiples tareas concurrentes
//! (Base de datos, Configuración, Caché en memoria).


use std::sync::Arc;
use tracing::info;
use crate::database::repository::Repository;
use crate::system::domain::{System};


#[derive(Clone, Debug)]
pub struct AppContext {
    pub repo: Repository,
    pub system: Arc<System>,
}


impl AppContext {
    pub async fn new() -> Self {
        info!("Info: creando app context");
        let system = Arc::new(
            match System::new() {
                Ok(system) => system,
                Err(e) => panic!("Error: no se pudo crear system. {}", e),
            }
        );
        let repo = Repository::create_repository(&system).await;
        Self { repo, system }
    }
}
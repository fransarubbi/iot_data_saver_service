//! Definición del Contexto de Aplicación (Shared State).
//!
//! Este módulo implementa el patrón de **Estado Compartido** para aplicaciones asíncronas.
//! El `AppContext` actúa como un contenedor de "Inyección de Dependencias" manual,
//! agrupando los recursos que deben ser accesibles por múltiples tareas concurrentes
//! (Base de datos, Configuración, Caché en memoria).


use std::sync::Arc;
use crate::database::repository::Repository;
use crate::system::domain::{System};


#[derive(Clone, Debug)]
pub struct AppContext {
    pub repo: Repository,
    pub system: Arc<System>,
}


impl AppContext {
    pub async fn new() -> Self {
        let repo = Repository::create_repository("amqp://guest:guest@localhost:5672").await;
        let system = Arc::new(System::new("a".to_string()));
        Self { repo, system }
    }
}
use chrono::Utc;
use crate::weather::domain::{OpenMeteoClient, Weather};
use tokio::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};


pub async fn weather_worker(tx_to_dba: mpsc::Sender<Weather>) {
    
    let meteo_client = OpenMeteoClient::new();

    // Configurar un intervalo de 5 min
    let mut interval = tokio::time::interval(Duration::from_secs(300));

    loop {
        interval.tick().await;

        match meteo_client.fetch_weather().await {
            Ok(weather) => {
                let weather = Weather {
                    timestamp: Utc::now(),
                    temperature_2m: weather.temperature_2m,
                    relative_humidity_2m: weather.relative_humidity_2m,
                };
                if tx_to_dba.send(weather).await.is_err() {
                    error!("Error: no se pudo enviar CurrentWeather a dba");
                }
            }
            Err(e) => {
                error!("Error: falló la consulta a Open-Meteo: {e}");
            }
        }
    }
}


pub fn start_weather_worker(tx_to_dba: mpsc::Sender<Weather>) {
    
    info!("Info: iniciando tarea weather_worker");
    tokio::spawn(async move {
        weather_worker(tx_to_dba).await;
    });
}
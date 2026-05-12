use chrono::{DateTime, Utc};
use reqwest::{Client, Error};
use serde::{Deserialize};


#[derive(Deserialize, Debug)]
pub struct WeatherResponse {
    pub current: CurrentWeather,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct CurrentWeather {
    pub temperature_2m: f32,
    pub relative_humidity_2m: f32,
}

#[derive(Debug, Clone)]
pub struct Weather {
    pub timestamp: DateTime<Utc>,
    pub temperature_2m: f32,
    pub relative_humidity_2m: f32,
}


pub struct OpenMeteoClient {
    http_client: Client,
    url: String,
}

impl OpenMeteoClient {
    /// Crea una nueva instancia del cliente.
    /// Se llama una sola vez durante la inicialización del sistema.
    pub fn new() -> Self {
        // Coordenadas fijas de San Luis Capital
        let url = "https://api.open-meteo.com/v1/forecast?latitude=-33.2950&longitude=-66.3356&current=temperature_2m,relative_humidity_2m".to_string();

        Self {
            http_client: Client::new(),
            url,
        }
    }

    /// Realiza la petición a la API utilizando el cliente HTTP interno ya instanciado.
    pub async fn fetch_weather(&self) -> Result<CurrentWeather, Error> {
        let response = self.http_client
            .get(&self.url)
            .send()
            .await?
            .json::<WeatherResponse>()
            .await?;

        Ok(response.current)
    }
}
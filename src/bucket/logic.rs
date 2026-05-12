use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use std::time::{SystemTime, UNIX_EPOCH};
use sqlx::FromRow;
use tracing::{error, info};
use crate::context::domain::{AppContext, BucketKey};
use crate::message::domain::Measurement;


pub enum BucketData {
    Measurement(Measurement),
    VecMeasurement(Vec<Measurement>)
}


#[derive(Debug, Default)]
pub struct ToProcess {
    pub temperature: Vec<f32>,
    pub humidity: Vec<f32>,
    pub co2_ppm: Vec<f32>,
}


#[derive(Default, Debug, Clone, FromRow, PartialEq)]
pub struct ProcessedTelemetry {
    #[sqlx(flatten)]
    pub network_id: String,
    pub timestamp: i64,
    pub temperature: Option<f32>,
    pub humidity: Option<f32>,
    pub co2_ppm: Option<f32>,
    pub pulse_counter_total: i64,
    pub pulse_max_duration: i64,
}


pub async fn bucket_task(mut rx: mpsc::Receiver<BucketData>,
                         app_context: AppContext) {

    while let Some(msg) = rx.recv().await {
        match msg {
            BucketData::Measurement(measurement) => {
                let network_id = measurement.network.clone();
                let raw_timestamp = measurement.metadata.timestamp;
                let bucket_ts = raw_timestamp - (raw_timestamp % 50);
                let key: BucketKey = (network_id, bucket_ts);

                app_context.bucket_map.entry(key)
                    .or_insert_with(Vec::new)
                    .push(measurement);
            },
            BucketData::VecMeasurement(measurements) => {
                for measurement in measurements {
                    let network_id = measurement.network.clone();
                    let raw_timestamp = measurement.metadata.timestamp;
                    let bucket_ts = raw_timestamp - (raw_timestamp % 50);
                    let key: BucketKey = (network_id, bucket_ts);

                    app_context.bucket_map.entry(key)
                        .or_insert_with(Vec::new)
                        .push(measurement);
                }
            }
        }
    }
}


pub async fn sweeper_task(tx_dba: mpsc::Sender<ProcessedTelemetry>,
                          app_context: AppContext) {

    // El temporizador se despierta cada 5 segundos
    let mut ticker = interval(Duration::from_secs(5));
    let window_grace = 50;   // Segundos de espera para los datos

    loop {
        ticker.tick().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut expired_keys = Vec::new();

        // Iteramos solo para encontrar claves vencidas
        for entry in app_context.bucket_map.iter() {
            let bucket_ts = entry.key().1;
            if now > (bucket_ts + window_grace) {
                expired_keys.push(entry.key().clone());
            }
        }

        // Extraemos el vector y lo procesamos
        for key in expired_keys {
            //let network_id = key.clone();
            if let Some((_, vector)) = app_context.bucket_map.remove(&key) {
                // Se lanza un worker independiente para hacer el filtrado sin frenar el bucle del Sweeper
                let tx_worker = tx_dba.clone();
                tokio::spawn(async move {
                    let mut to_process = ToProcess::default();
                    let mut pulse_max_duration: i64 = 0;
                    let mut pulse_counter: i64 = 0;

                    let temperature: Option<f32>;
                    let humidity: Option<f32>;
                    let co2_ppm: Option<f32>;

                    for data in vector {
                        if data.temperature >= 10.0 && data.temperature <= 35.0 {
                            to_process.temperature.push(data.temperature);
                        }
                        if data.humidity >= 20.0 && data.humidity <= 90.0 {
                            to_process.humidity.push(data.humidity);
                        }
                        if data.co2_ppm >= 400.0 && data.co2_ppm <= 1800.0 {
                            to_process.co2_ppm.push(data.co2_ppm);
                        }

                        pulse_counter += data.pulse_counter;

                        if data.pulse_max_duration > pulse_max_duration {
                            pulse_max_duration = data.pulse_max_duration;
                        }
                    }

                    if !to_process.temperature.is_empty() && to_process.temperature.len() >= 4 {
                        to_process.temperature.sort_unstable_by(|a, b| a.total_cmp(b));
                        let quartiles_temp = quartiles(&to_process.temperature);
                        let iqr_temp = quartiles_temp.1 - quartiles_temp.0;
                        to_process.temperature.retain(|&x| {
                            x >= quartiles_temp.0 - 1.5*iqr_temp && x <= quartiles_temp.1 + 1.5*iqr_temp
                        });
                        temperature = Some(to_process.temperature.iter().sum::<f32>() / to_process.temperature.len() as f32);
                    }
                    else if !to_process.temperature.is_empty() && to_process.temperature.len() < 4 {
                        temperature = Some(to_process.temperature.iter().sum::<f32>() / to_process.temperature.len() as f32);
                    }
                    else {
                        temperature = None;
                    }

                    if !to_process.humidity.is_empty() && to_process.humidity.len() >= 4 {
                        to_process.humidity.sort_unstable_by(|a, b| a.total_cmp(b));
                        let quartiles_hum = quartiles(&to_process.humidity);
                        let iqr_hum = quartiles_hum.1 - quartiles_hum.0;
                        to_process.humidity.retain(|&x| {
                            x >= quartiles_hum.0 - 1.5*iqr_hum && x <= quartiles_hum.1 + 1.5*iqr_hum
                        });
                        humidity = Some(to_process.humidity.iter().sum::<f32>() / to_process.humidity.len() as f32);
                    }
                    else if !to_process.humidity.is_empty() && to_process.humidity.len() < 4 {
                        humidity = Some(to_process.humidity.iter().sum::<f32>() / to_process.humidity.len() as f32);
                    }
                    else {
                        humidity = None;
                    }

                    if !to_process.co2_ppm.is_empty() && to_process.co2_ppm.len() >= 4 {
                        to_process.co2_ppm.sort_unstable_by(|a, b| a.total_cmp(b));
                        let quartiles_co2 = quartiles(&to_process.co2_ppm);
                        let iqr_co2 = quartiles_co2.1 - quartiles_co2.0;
                        to_process.co2_ppm.retain(|&x| {
                            x >= quartiles_co2.0 - 1.5*iqr_co2 && x <= quartiles_co2.1 + 1.5*iqr_co2
                        });
                        co2_ppm = Some(to_process.co2_ppm.iter().sum::<f32>() / to_process.co2_ppm.len() as f32);
                    }
                    else if !to_process.co2_ppm.is_empty() && to_process.co2_ppm.len() < 4 {
                        co2_ppm = Some(to_process.co2_ppm.iter().sum::<f32>() / to_process.co2_ppm.len() as f32);
                    }
                    else {
                        co2_ppm = None;
                    }

                    let processed = ProcessedTelemetry {
                        network_id: key.0,
                        timestamp: key.1,
                        temperature,
                        humidity,
                        co2_ppm,
                        pulse_counter_total: pulse_counter,
                        pulse_max_duration
                    };

                    if tx_worker.send(processed).await.is_err() {
                        error!("Error: el receptor (dab task) se ha cerrado o caído.");
                    }
                });
            }
        }
    }
}


fn quartiles(data: &[f32]) -> (f32, f32) {
    let q1 = percentile(data, 25.0);
    let q3 = percentile(data, 75.0);
    (q1, q3)
}


fn percentile(sorted: &[f32], percentile: f32) -> f32 {
    let index = (percentile / 100.0) * (sorted.len() as f32 - 1.0);
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted[lower]
    } else {
        let weight = index - lower as f32;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}


pub fn start_bucket(rx: mpsc::Receiver<BucketData>,
                    app_context: AppContext) {

    info!("Info: iniciando tarea bucket_task");
    tokio::spawn(async move {
        bucket_task(rx,
                    app_context
        ).await;
    });
}


pub fn start_sweeper(tx_dba: mpsc::Sender<ProcessedTelemetry>,
                     app_context: AppContext) {

    info!("Info: iniciando tarea sweeper_task");
    tokio::spawn(async move {
        sweeper_task(tx_dba,
                     app_context
        ).await;
    });
}
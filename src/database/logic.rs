use tokio::sync::mpsc;
use tracing::error;
use crate::context::domain::AppContext;
use crate::database::domain::TableDataVector;
use crate::message::domain::Message;


pub async fn dba_task(mut rx: mpsc::Receiver<Message>,
                      app_context: AppContext) {

    let mut vector = TableDataVector::new();

    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Report(report) => {
                if !vector.is_measurement_full() {
                    vector.measurement.push(report);
                }
            }
            Message::Monitor(monitor) => {
                if !vector.is_monitor_full() {
                    vector.monitor.push(monitor);
                }
            }
            Message::Metrics(metrics) => {
                if !vector.is_metrics_full() {
                    vector.system_metrics.push(metrics);
                }
            }
            Message::AlertAir(alert) => {
                if !vector.is_alert_air_full() {
                    vector.alert_air.push(alert);
                }
            }
            Message::AlertTem(alert) => {
                if !vector.is_alert_th_full() {
                    vector.alert_th.push(alert);
                }
            }
            _ => {}
        }

        if vector.is_some_vector_full() {
            match app_context.repo.insert(vector.clone()).await {
                Ok(_) => {}
                Err(e) => error!("Error: No se pudo insertar batch. {e}")
            }
        }
    }
}


pub fn start_dba(rx_from_msg: mpsc::Receiver<Message>,
                 app_context: AppContext) {

    tokio::spawn(async move {
        dba_task(rx_from_msg,
                 app_context
        ).await;
    });
}
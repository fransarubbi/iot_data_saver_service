use crate::message::domain::{AlertAir, AlertTh, Measurement, Monitor, SystemMetrics};
use crate::system::domain::postgres::BATCH_SIZE;

#[derive(Clone, Debug)]
pub struct TableDataVector {
    pub measurement: Vec<Measurement>,
    pub system_metrics: Vec<SystemMetrics>,
    pub alert_air: Vec<AlertAir>,
    pub alert_th: Vec<AlertTh>,
    pub monitor: Vec<Monitor>,
}


impl TableDataVector {
    pub fn new() -> Self {
        Self {
            measurement: Vec::new(),
            system_metrics: Vec::new(),
            alert_air: Vec::new(),
            alert_th: Vec::new(),
            monitor: Vec::new(),
        }
    }

    pub fn is_measurement_full(&self) -> bool {
        self.measurement.len() == BATCH_SIZE
    }
    pub fn is_monitor_full(&self) -> bool {
        self.monitor.len() == BATCH_SIZE
    }
    pub fn is_alert_air_full(&self) -> bool {
        self.alert_air.len() == BATCH_SIZE
    }
    pub fn is_alert_th_full(&self) -> bool {
        self.alert_th.len() == BATCH_SIZE
    }
    pub fn is_metrics_full(&self) -> bool {
        self.system_metrics.len() == BATCH_SIZE
    }

    pub fn is_some_vector_full(&self) -> bool {
        self.is_measurement_full() || self.is_alert_air_full() ||
            self.is_alert_th_full() || self.is_monitor_full() || self.is_metrics_full()
    }
}




use crate::message::domain::{AlertAir, AlertTh, Measurement, Monitor, SystemMetrics};


#[derive(Clone, Debug)]
pub enum TableDataVector {
    Measurement(Vec<Measurement>),
    Monitor(Vec<Monitor>),
    AlertAir(Vec<AlertAir>),
    AlertTemp(Vec<AlertTh>),
    Metric(Vec<SystemMetrics>),
}






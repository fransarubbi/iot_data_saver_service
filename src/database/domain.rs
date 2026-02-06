//! Estructuras de dominio para la persistencia de datos.
//!
//! Este módulo define los contenedores temporales (Buffers) utilizados para acumular
//! datos en memoria antes de realizar operaciones de escritura masiva (Batch Insert)
//! en la base de datos.


use crate::message::domain::{AlertAir, AlertTh, Measurement, Monitor, SystemMetrics};
use crate::system::domain::database::BATCH_SIZE;


/// Buffer heterogéneo para la acumulación de mensajes de telemetría.
///
/// Esta estructura mantiene vectores separados para cada tipo de dato del sistema.
/// Su propósito es reducir el número de transacciones a la base de datos agrupando
/// múltiples inserciones en una sola operación.
///
#[derive(Clone, Debug)]
pub struct TableDataVector {
    pub measurement: Vec<Measurement>,
    pub system_metrics: Vec<SystemMetrics>,
    pub alert_air: Vec<AlertAir>,
    pub alert_th: Vec<AlertTh>,
    pub monitor: Vec<Monitor>,
}


impl TableDataVector {

    /// Crea un nuevo contenedor con la capacidad pre-reservada.
    ///
    /// Inicializa los vectores internos utilizando `Vec::with_capacity(BATCH_SIZE)`.
    /// Esto evita realocaciones dinámicas de memoria mientras se llena el buffer,
    /// mejorando el rendimiento de inserción.
    pub fn new() -> Self {
        Self {
            measurement: Vec::with_capacity(BATCH_SIZE),
            system_metrics: Vec::with_capacity(BATCH_SIZE),
            alert_air: Vec::with_capacity(BATCH_SIZE),
            alert_th: Vec::with_capacity(BATCH_SIZE),
            monitor: Vec::with_capacity(BATCH_SIZE),
        }
    }

    /// Verifica si alguno de los buffers internos ha alcanzado su capacidad máxima.
    ///
    /// Este método se utiliza como disparador (Trigger) para realizar el volcado (flush)
    /// a la base de datos.
    ///
    /// # Retorno
    /// * `true`: Al menos uno de los vectores tiene longitud igual a `BATCH_SIZE`.
    /// * `false`: Todos los vectores tienen espacio disponible.
    pub fn is_some_vector_full(&self) -> bool {
        self.is_measurement_full() || self.is_alert_air_full() ||
            self.is_alert_th_full() || self.is_monitor_full() || self.is_metrics_full()
    }

    fn is_measurement_full(&self) -> bool {
        self.measurement.len() == BATCH_SIZE
    }
    fn is_monitor_full(&self) -> bool {
        self.monitor.len() == BATCH_SIZE
    }
    fn is_alert_air_full(&self) -> bool {
        self.alert_air.len() == BATCH_SIZE
    }
    fn is_alert_th_full(&self) -> bool {
        self.alert_th.len() == BATCH_SIZE
    }
    fn is_metrics_full(&self) -> bool {
        self.system_metrics.len() == BATCH_SIZE
    }

    /// Reinicia los buffers sin liberar la memoria asignada.
    ///
    /// Establece la longitud de todos los vectores a 0, pero mantiene la capacidad
    /// reservada en el Heap. Esto permite reutilizar la estructura en el siguiente
    /// ciclo de acumulación sin costo de asignación de memoria.
    pub fn clear(&mut self) {
        self.measurement.clear();
        self.system_metrics.clear();
        self.alert_air.clear();
        self.alert_th.clear();
        self.monitor.clear();
    }
}




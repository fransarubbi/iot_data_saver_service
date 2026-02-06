//! Temporizador dedicado para el subsistema de Heartbeat.
//!
//! Este módulo implementa un **Actor Temporizador** (Timer Actor). A diferencia de un
//! `tokio::time::interval` tradicional, este componente espera explícitamente una orden
//! (`InitTimer`) antes de comenzar a contar.
//!
//! Esto permite un control preciso del flujo "Ping-Pong" entre la tarea de lógica y el tiempo,
//! evitando que los ciclos se solapen si el procesamiento toma más tiempo del esperado.


use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, instrument};
use tracing::log::debug;


/// Eventos de control para la coordinación entre el Heartbeat y su Temporizador.
pub enum Event {
    /// Señal emitida por el temporizador cuando el tiempo estipulado ha transcurrido.
    /// Indica que es momento de enviar un nuevo Heartbeat.
    Timeout,

    /// Comando recibido por el temporizador para iniciar una cuenta regresiva.
    /// Contiene la duración de la espera.
    InitTimer(Duration),
}


/// Ejecuta el bucle del temporizador de vigilancia.
///
/// Funciona como un disparador de un solo uso (One-shot trigger) que se rearma en bucle:
/// 1. Se bloquea esperando recibir `Event::InitTimer`.
/// 2. Duerme el hilo asíncronamente por la duración especificada.
/// 3. Envía `Event::Timeout` de vuelta para despertar a la tarea principal.
///
/// # Comportamiento de Bloqueo
/// Una vez iniciado el temporizador (durante el `sleep`), este actor **no procesa**
/// nuevos mensajes hasta que el tiempo expira. Esto garantiza un intervalo mínimo estricto.
///
/// # Argumentos
/// * `tx_to_heartbeat`: Canal para notificar el vencimiento del tiempo (`Timeout`).
/// * `rx_from_heartbeat`: Canal para recibir la orden de inicio (`InitTimer`).
#[instrument(
    name = "watchdog_timer_for_heartbeat_task",
    skip(tx_to_heartbeat, rx_from_heartbeat)
)]
pub async fn watchdog_timer_for_heartbeat(tx_to_heartbeat: mpsc::Sender<Event>,
                                          mut rx_from_heartbeat: mpsc::Receiver<Event>) {
    info!("Info: watchdog timer creada");

    loop {
        let duration = match rx_from_heartbeat.recv().await {
            Some(Event::InitTimer(d)) => d,
            None => break, // Canal cerrado, terminar tarea
            _ => continue,
        };

        sleep(duration).await;
        debug!("Debug: timeout de watchdog completado");

        if tx_to_heartbeat.send(Event::Timeout).await.is_err() {
            error!("Error crítico: no se pudo enviar evento Timeout a heartbeat (canal receptor caído)");
            break;
        }
    }
    info!("Info: watchdog timer finalizada");
}


/// Inicializa y lanza la tarea del temporizador en segundo plano.
///
/// # Argumentos
/// * `tx_to_heartbeat`: Canal de transmisión hacia la tarea principal.
/// * `rx_from_heartbeat`: Canal de recepción desde la tarea principal.
pub fn start_watchdog(tx_to_heartbeat: mpsc::Sender<Event>,
                      rx_from_heartbeat: mpsc::Receiver<Event>) {

    info!("Info: iniciando tarea watchdog timer");
    tokio::spawn(async move {
        watchdog_timer_for_heartbeat(
            tx_to_heartbeat,
            rx_from_heartbeat
        ).await;
    });
}
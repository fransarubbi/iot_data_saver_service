use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};


pub enum Event {
    /// El temporizador de vigilancia (Watchdog) expiró.
    Timeout,
    /// Comando interno para iniciar el temporizador.
    InitTimer(Duration),
}


pub async fn watchdog_timer_for_heartbeat(tx_to_fsm: mpsc::Sender<Event>,
                                          mut cmd_rx: mpsc::Receiver<Event>) {
    loop {
        let duration = match cmd_rx.recv().await {
            Some(Event::InitTimer(d)) => d,
            None => break, // Canal cerrado, terminar tarea
            _ => continue,
        };

        tokio::select! {
            _ = sleep(duration) => {
                // El tiempo se agotó
                let _ = tx_to_fsm.send(Event::Timeout).await;
            }
        }
    }
}
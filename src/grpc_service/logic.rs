//! Cliente gRPC resiliente y gestor de conexión bidireccional.
//!
//! Este módulo encapsula toda la complejidad de la comunicación de red. Implementa un
//! patrón de **Máquina de Estados** para gestionar el ciclo de vida de la conexión
//! (Conexión inicial, Streaming de datos, Reconexión ante fallos).
//!
//! # Características
//! * **Autorecuperación:** Reintenta la conexión automáticamente tras fallos.
//! * **Bidireccional:** Soporta envío y recepción simultánea (Full Duplex).
//! * **Optimizado:** Utiliza compresión Gzip y Keep-Alive HTTP/2.


use tonic::transport::{Channel};
use tonic::codec::CompressionEncoding;
use tonic::Request;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, instrument, warn};
use std::time::Duration;
use crate::context::domain::AppContext;
use crate::grpc::{DataSaverDownload, DataSaverUpload};
use crate::grpc::data_service_client::DataServiceClient;
use crate::system::domain::{InternalEvent, ErrorType, System};
use crate::system::domain::grpc_service_const::{KEEP_ALIVE_INTERVAL_SECS, KEEP_ALIVE_TIMEOUT_SECS, TIMEOUT_SECS};


/// Estados posibles de la máquina de estados del cliente gRPC.
#[derive(Debug, Clone, Copy, PartialEq)]
enum StateClient {
    /// Estado inicial: Intentando establecer conexión TCP/HTTP2.
    Init,
    /// Estado operativo: El stream bidireccional está activo y transfiriendo datos.
    Work,
    /// Estado de fallo: Ocurrió un error y se está esperando antes de reintentar (Backoff).
    Error,
}


/// Crea y configura el canal de transporte gRPC (Channel).
///
/// Aplica configuraciones críticas de red como Timeouts y Keep-Alive para evitar
/// que intermediarios (Firewalls, Load Balancers) cierren la conexión silenciosamente.
///
/// # Argumentos
/// * `system`: Configuración del sistema que contiene host y puerto.
///
/// # Retorno
/// Retorna un `Channel` listo para ser usado por el cliente `IotServiceClient`.
async fn create_channel(system: &System) -> Result<Channel, ErrorType> {

    info!("Info: creando canal gRPC");
    let url = format!("http://{}:{}", system.grpc_host, system.grpc_port);

    let endpoint = Channel::from_shared(url)
        .map_err(|_| ErrorType::Endpoint)?
        .connect_timeout(Duration::from_secs(TIMEOUT_SECS))
        .keep_alive_timeout(Duration::from_secs(KEEP_ALIVE_TIMEOUT_SECS))
        .http2_keep_alive_interval(Duration::from_secs(KEEP_ALIVE_INTERVAL_SECS))
        .keep_alive_while_idle(true);

    endpoint.connect().await.map_err(|e| {
        error!("Error: no se pudo conectar gRPC: {}", e);
        ErrorType::Endpoint
    })
}


/// Tarea principal (Actor) que gestiona el ciclo de vida de la comunicación gRPC.
///
/// Implementa un bucle infinito controlado por una máquina de estados:
/// 1. **Init:** Crea el cliente y establece el `ConnectStream`.
/// 2. **Work:** Usa `tokio::select!` para multiplexar envío y recepción.
/// 3. **Error:** Limpia recursos y espera 5 segundos antes de volver a Init.
///
/// # Flujo de Datos
/// * **Upstream (Subida):** Recibe `DataSaverUpload` de `rx_from_server` y lo envía al servidor.
/// * **Downstream (Bajada):** Recibe `DataSaverDownload` del servidor y lo envía a `tx_to_msg`.
#[instrument(
    name = "grpc_task",
    skip(tx_to_msg, rx_from_server, app_context)
)]
pub async fn grpc_task(tx_to_msg: mpsc::Sender<InternalEvent>,
                       mut rx_from_server: mpsc::Receiver<DataSaverUpload>,
                       app_context: AppContext) {

    info!("Info: grpc task creada");

    let mut state = StateClient::Init;
    let mut tx_session: Option<mpsc::Sender<DataSaverUpload>> = None;
    let mut inbound_stream: Option<tonic::Streaming<DataSaverDownload>> = None;

    loop {
        match state {
            StateClient::Init => {
                match create_channel(&app_context.system).await {
                    Ok(channel) => {
                        info!("Info: canal gRPC creado correctamente");
                        let mut grpc_client = DataServiceClient::new(channel)
                            .send_compressed(CompressionEncoding::Gzip)
                            .accept_compressed(CompressionEncoding::Gzip);

                        let (tx_sess, rx_session) = mpsc::channel::<DataSaverUpload>(100);
                        let request = Request::new(ReceiverStream::new(rx_session));

                        match grpc_client.connect_stream(request).await {
                            Ok(response) => {
                                info!("Info: gRPC Conectado. Stream Bidireccional iniciado");
                                tx_session = Some(tx_sess);
                                inbound_stream = Some(response.into_inner());
                                state = StateClient::Work;
                            }
                            Err(e) => {
                                error!("Error: no se pudo conectar al canal gRPC. {}", e);
                                state = StateClient::Error;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error: canal gRPC no creado. {:?}", e);
                        state = StateClient::Error;
                    }
                }
            }

            StateClient::Work => {
                if let (Some(tx_sess), Some(stream)) = (tx_session.as_ref(), inbound_stream.as_mut()) {
                    tokio::select! {
                        msg_opt = rx_from_server.recv() => {   // Enviar datos (Upstream)
                            debug!("Debug: mensaje entrante a grpc desde message_upload");
                            match msg_opt {
                                Some(msg) => {
                                    if let Err(e) = tx_sess.send(msg).await {
                                        warn!("Warning: stream de envío cerrado {}", e);
                                        state = StateClient::Error;
                                    }
                                }
                                None => {
                                    info!("Info: canal de salida cerrado, terminando tarea");
                                    return;
                                }
                            }
                        }

                        server_msg = stream.next() => {   // Recibir datos (Downstream)
                            debug!("Debug: mensaje entrante a grpc para enviar a message_download");
                            match server_msg {
                                Some(Ok(download_msg)) => {
                                    if tx_to_msg.send(InternalEvent::IncomingMessage(download_msg)).await.is_err() {
                                        error!("Error: no se pudo enviar el mensaje recibido del servidor");
                                    }
                                }
                                Some(Err(e)) => {
                                    error!("Error: stream gRPC {}", e);
                                    state = StateClient::Error;
                                }
                                None => {
                                    warn!("Warning: stream cerrado por el servidor");
                                    state = StateClient::Error;
                                }
                            }
                        }
                    }
                } else {
                    warn!("Warning: estado Work sin stream válido, reiniciando...");
                    state = StateClient::Init;
                }
            }

            StateClient::Error => {
                info!("Info: StateClient Error, limpiando recursos y haciendo backpressure");
                tx_session = None;
                inbound_stream = None;

                tokio::time::sleep(Duration::from_secs(5)).await;
                state = StateClient::Init;
            }
        }
    }
}


/// Inicializa y lanza la tarea gRPC en segundo plano.
///
/// # Argumentos
/// * `tx_to_msg`: Canal para enviar los mensajes recibidos del servidor hacia la lógica de negocio.
/// * `rx_from_msg`: Canal para recibir los mensajes que deben ser enviados al servidor.
/// * `app_context`: Contexto global de la aplicación.
pub fn start_grpc(tx_to_msg: mpsc::Sender<InternalEvent>,
                  rx_from_msg: mpsc::Receiver<DataSaverUpload>,
                  app_context: AppContext) {

    info!("Info: iniciando tarea grpc");
    tokio::spawn(async move {
        grpc_task(tx_to_msg,
                  rx_from_msg,
                  app_context).await;
    });
}
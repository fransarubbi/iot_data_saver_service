use tonic::transport::{Channel};
use tonic::codec::CompressionEncoding;
use tonic::Request;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use std::time::Duration;
use crate::context::domain::AppContext;
use crate::grpc::{DataSaverDownload, DataSaverUpload};
use crate::grpc::iot_service_client::IotServiceClient;
use crate::system::domain::{InternalEvent, ErrorType, System};
use crate::system::domain::grpc_service_const::{KEEP_ALIVE_INTERVAL_SECS, KEEP_ALIVE_TIMEOUT_SECS, TIMEOUT_SECS};

#[derive(Debug, Clone, Copy, PartialEq)]
enum StateClient {
    Init,
    Work,
    Error,
}


async fn create_channel(system: &System) -> Result<Channel, ErrorType> {

    let url = format!("https://{}:{}", system.grpc_host, system.grpc_port);

    let endpoint = Channel::from_shared(url)
        .map_err(|_| ErrorType::Endpoint)?
        .connect_timeout(Duration::from_secs(TIMEOUT_SECS))
        .keep_alive_timeout(Duration::from_secs(KEEP_ALIVE_TIMEOUT_SECS))
        .http2_keep_alive_interval(Duration::from_secs(KEEP_ALIVE_INTERVAL_SECS))
        .keep_alive_while_idle(true);

    endpoint.connect().await.map_err(|e| {
        error!("Error: No se pudo conectar gRPC: {}", e);
        ErrorType::Endpoint
    })
}


pub async fn grpc_task(tx: mpsc::Sender<InternalEvent>,
                       mut rx_outbound: mpsc::Receiver<DataSaverUpload>,
                       app_context: AppContext) {

    let mut state = StateClient::Init;
    let mut tx_session: Option<mpsc::Sender<DataSaverUpload>> = None;
    let mut inbound_stream: Option<tonic::Streaming<DataSaverDownload>> = None;

    loop {
        match state {
            StateClient::Init => {
                match create_channel(&app_context.system).await {
                    Ok(channel) => {
                        // Crear cliente con compresión
                        let mut grpc_client = IotServiceClient::new(channel)
                            .send_compressed(CompressionEncoding::Gzip)
                            .accept_compressed(CompressionEncoding::Gzip);

                        // Configurar stream bidireccional
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
                                error!("{}", e);
                                state = StateClient::Error;
                            }
                        }
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        state = StateClient::Error;
                    }
                }
            }

            StateClient::Work => {
                if let (Some(tx_sess), Some(stream)) = (tx_session.as_ref(), inbound_stream.as_mut()) {
                    tokio::select! {
                        msg_opt = rx_outbound.recv() => {   // Enviar datos (Upstream)
                            match msg_opt {
                                Some(msg) => {
                                    if let Err(e) = tx_sess.send(msg).await {
                                        warn!("Warning: Stream de envío cerrado {}", e);
                                        state = StateClient::Error;
                                    }
                                }
                                None => {
                                    info!("Info: Canal de salida cerrado, terminando tarea");
                                    return;
                                }
                            }
                        }

                        server_msg = stream.next() => {   // Recibir datos (Downstream)
                            match server_msg {
                                Some(Ok(download_msg)) => {
                                    if tx.send(InternalEvent::IncomingMessage(download_msg)).await.is_err() {
                                        error!("Error: No se pudo enviar el mensaje recibido del servidor");
                                    }
                                }
                                Some(Err(e)) => {
                                    error!("Error: stream gRPC {}", e);
                                    state = StateClient::Error;
                                }
                                None => {
                                    warn!("Warning: Stream cerrado por el servidor");
                                    state = StateClient::Error;
                                }
                            }
                        }
                    }
                } else {
                    warn!("Warning: Estado Work sin stream válido, reiniciando...");
                    state = StateClient::Init;
                }
            }

            StateClient::Error => {
                // Limpiar recursos
                tx_session = None;
                inbound_stream = None;

                tokio::time::sleep(Duration::from_secs(5)).await;
                state = StateClient::Init;
            }
        }
    }
}


pub fn start_grpc(tx_to_msg: mpsc::Sender<InternalEvent>,
                  rx_from_msg: mpsc::Receiver<DataSaverUpload>,
                  app_context: AppContext) {

    tokio::spawn(async move {
        grpc_task(tx_to_msg,
                  rx_from_msg,
                  app_context).await;
    });
}
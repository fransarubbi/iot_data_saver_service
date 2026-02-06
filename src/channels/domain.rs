use tokio::sync::mpsc;
use crate::grpc::DataSaverUpload;
use crate::heartbeat::domain::Event;
use crate::message::domain::Message;
use crate::system::domain::InternalEvent;


pub struct Channels {
    pub heartbeat_to_watchdog: mpsc::Sender<Event>,
    pub watchdog_from_heartbeat: mpsc::Receiver<Event>,

    pub watchdog_to_heartbeat: mpsc::Sender<Event>,
    pub heartbeat_from_watchdog: mpsc::Receiver<Event>,

    pub heartbeat_to_upload_message: mpsc::Sender<Message>,
    pub upload_message_from_heartbeat: mpsc::Receiver<Message>,

    pub upload_message_to_grpc: mpsc::Sender<DataSaverUpload>,
    pub grpc_from_upload_message: mpsc::Receiver<DataSaverUpload>,

    pub download_message_to_dba: mpsc::Sender<Message>,
    pub dba_from_download_message: mpsc::Receiver<Message>,

    pub grpc_to_download_message: mpsc::Sender<InternalEvent>,
    pub download_message_from_grpc: mpsc::Receiver<InternalEvent>,
}


impl Channels {
    pub fn new() -> Channels {
        let (h_to_w, w_from_h) = mpsc::channel::<Event>(10);
        let (w_to_h, h_from_w) = mpsc::channel::<Event>(10);
        let (h_to_um, um_from_h) = mpsc::channel::<Message>(10);
        let (um_to_grpc, grpc_from_um) = mpsc::channel::<DataSaverUpload>(200);
        let (dm_to_dba, dba_from_dm) = mpsc::channel::<Message>(200);
        let (grpc_to_dm, dm_from_grpc) = mpsc::channel::<InternalEvent>(200);

        Self {
            heartbeat_to_watchdog: h_to_w,
            watchdog_from_heartbeat: w_from_h,
            watchdog_to_heartbeat: w_to_h,
            heartbeat_from_watchdog: h_from_w,
            heartbeat_to_upload_message: h_to_um,
            upload_message_from_heartbeat: um_from_h,
            upload_message_to_grpc: um_to_grpc,
            grpc_from_upload_message: grpc_from_um,
            download_message_to_dba: dm_to_dba,
            dba_from_download_message: dba_from_dm,
            grpc_to_download_message: grpc_to_dm,
            download_message_from_grpc: dm_from_grpc,
        }
    }
}
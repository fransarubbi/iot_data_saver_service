use crate::grpc::DataSaverDownload;

pub enum InternalEvent {
    IncomingMessage(DataSaverDownload)
}
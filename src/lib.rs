/// Contains definitions of camera commands and some default values.
pub mod consts;

/// Contains enums and structs for the camera settings.
pub mod settings;

/// Contains various convenience methods for interacting with the camera.
pub mod util;

/// Contains the main camera struct.
pub mod cam;

/// Crate-specific error enum. 
/// Every function interacting with the camera returns a Result enum with this error type.
#[derive(thiserror::Error, Debug)]
pub enum CamError {
    #[error("Error while transfering USB data")]
    UsbTransfer(#[from] nusb::transfer::TransferError),

    #[error("Internal I/O error occured")]
    Io(#[from] std::io::Error),

    #[error("Timeout occured during I/O operation")]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error("Invalid response format")]
    InvalidFormat,

    #[error("Invalid response length (expected: {expected}, received: {received})")]
    InvalidLength { expected: usize, received: usize },

    #[error("Unable to initialize connection, attempts: {tries}, status code: {status_code}")]
    ConnInit { tries: u32, status_code: u32 },

    #[error("Unable to send command, attempts: {tries}, status code: {status_code}")]
    SendCommand { tries: u32, status_code: u32 },

    #[error("Error while sending the keepalive command, status code: {status_code}")]
    Keepalive { status_code: u32 },

    #[error("Error while writing data")]
    Write,

    #[error("Couldn't find a device with given VID/PID: {vid:#06X}:{pid:#06X}")]
    NoDeviceFound { vid: u16, pid: u16 },
}

type CamResult<T> = Result<T, CamError>;
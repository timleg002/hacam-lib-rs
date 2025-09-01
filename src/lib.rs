//! A Rust cross-platform userspace driver for interacting with the Huawei EnVizion 360° Camera (Huawei CV60).
//! 
//! Tested on Windows and macOS. Should work on Linux as well. 
//! This library uses the [nusb] library.
//! The camera itself uses a standard LibUSB driver, so it works out of the box on macOS, but you need to select the driver manually on Windows (WinUSB).
//! 
//! [nusb]: https://github.com/kevinmehall/nusb
//! 
//! ## Example
//! 
//! More examples are provided in the `examples/` folder.
//! 
//! ```no_run
//! use hacam_lib_rs::{cam::HaCam, settings::PictureOrientation, util::CamUtil};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut cam = HaCam::new()?;
//! 
//!     cam.initialize_comm().await?;
//! 
//!     println!("Camera info: {:#?}", cam.get_camera_info().await?);
//! 
//!     cam.power_off().await?;
//! 
//!     Ok(())
//! }
//! ```

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
use std::time::Duration;

/// Default timeout for all transfer commands.
pub const DEFAULT_TRANSFER_TIMEOUT: Duration = Duration::from_millis(2000);

/// Timeout for sending firmware data.
pub const FIRMWARE_TRANSFER_TIMEOUT: Duration = Duration::from_millis(5000);

/// If too many keepalive commands fail in the given timeout, the connection is reinitialized.
pub const KEEPALIVE_TIMEOUT: Duration = Duration::from_millis(5000);

/// Interval before sending a keepalive command.
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(500);

/// Interval before attempting to reinitialize connection again due to a failure.
pub const INIT_ATTEMPT_INTERVAL: Duration = Duration::from_millis(100);

/// Receiving buffer size for the keepalive command.
pub const KEEPALIVE_RX_BUF_SIZE: usize = 64;

pub const DEFAULT_MAX_RECV_SIZE: usize = 65536;
pub const DEFAULT_CHUNK_SIZE: usize = 16384;

pub const ENDPOINT_IN_ADDR: u8 = 0x82;
pub const ENDPOINT_OUT_ADDR: u8 = 0x03;

/// Magic number of the messages received from the camera.
pub const RX_HEADER_MAGIC: [u8; 4] = *b"USBS";

/// Magic number of the messages sent to the camera.
pub const TX_HEADER_MAGIC: [u8; 4] = *b"USBC";

pub const DEFAULT_VID: u16 = 0x12D1;
pub const DEFAULT_PID: u16 = 0x109B;

/// Contains "SCSI" camera commands. (for initializing communication, sending keepalives, etc.)
pub mod scsi {
    pub const OPEN_CONN_COMMAND: [i8; 16] = [122, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const KEEP_ALIVE_COMMAND: [i8; 3] = [122, 3, -1];
    pub const APP_CONN_COMMAND: [i8; 16] = [122, 0, 2, 0, 0, 0, 0, 0, 1, 1, 0, 0, 44, 1, 0, 0];
}

/// Contains USB camera commands. (for transferring data, etc.)
pub mod usb {
    pub const GET_CAMERA_STATUS: [i8; 16] = [122, 3, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const GET_THERMAL_STATUS: [i8; 16] = [122, 3, 52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const GET_SCSI_VERSION: [i8; 16] = [122, 3, 2, 0, 0, 0, 0, 0, 118, 50, 46, 48, 48, 48, 48, 0];

    pub const START_LIVE_VIEW: [i8; 16] = [122, 1, 1, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0];
    pub const GET_LIVE_VIEW_FRAME: [i8; 16] = [122, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const CHECK_LIVE_VIEW_STATUS: [i8; 16] = [122, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const STOP_LIVE_VIEW: [i8; 16] = [122, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const CHECK_LIVE_VIEW_STOP_STATUS: [i8; 16] = [122, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    pub const READ_PIC_BUF: [i8; 16] = [122, 5, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const TAKE_PICTURE: [i8; 16] = [122, 1, 5, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
    pub const GET_PIC_THUMBNAIL: [i8; 16] = [122, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const CLEAR_PIC_BUF: [i8; 16] = [122, 1, -123, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const CHECK_CAPTURE_STATUS: [i8; 16] = [122, 2, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const PIC_TRANSFER_STATUS_IS_OK: [i8; 16] =
        [122, 5, -126, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const GET_REMAINING_PIC_NUM: [i8; 16] = [122, 3, 53, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    pub const READ_ALL_SETTINGS: [i8; 16] = [122, 4, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const WRITE_ALL_SETTINGS: [i8; 16] = [123, 4, 96, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    pub const GET_CAMERA_INFO: [i8; 16] = [122, 3, 1, 0, 0, 0, 0, 0, 55, 46, 55, 50, 46, 48, 48, 0];

    pub const POWER_OFF_CAMERA: [i8; 16] = [122, 1, -16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const RESET_CAMERA: [i8; 16] = [122, 1, -126, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const CHECK_CAMERA_RESET_STATUS: [i8; 16] =
        [122, 2, -126, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const CLOSE_CONNECTION: [i8; 16] = [122, 0, 2, 1, 0, 0, 0, 0, 1, 0, 0, 0, 44, 1, 0, 0];

    pub const WRITE_GENERAL_SETTING: [i8; 16] = [123, 4, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const READ_GENERAL_SETTING: [i8; 16] = [122, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    pub const START_RECORDING: [i8; 16] = [122, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const CHECK_START_RECORDING: [i8; 16] = [122, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const STOP_RECORDING: [i8; 16] = [122, 1, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const CHECK_STOP_RECORDING: [i8; 16] = [122, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    pub const THROUGHPUT_READ_TEST: [i8; 16] =
        [122, -16, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    pub const THROUGHPUT_WRITE_TEST: [i8; 16] =
        [123, -16, 16, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
}

use std::{io::Write as _};
use nusb::transfer::{ControlOut, ControlType, Recipient};
use log::*;
use rand::Rng as _;

use crate::{consts::{self, DEFAULT_PID, DEFAULT_VID, ENDPOINT_IN_ADDR, ENDPOINT_OUT_ADDR, RX_HEADER_MAGIC}, settings::*, CamError, CamResult};

/// Struct for interacting with the camera.
pub struct HaCam {
    interface: nusb::Interface,
    in_addr: u8,
    out_addr: u8,

    /// Default amount of tries
    default_tries: u32,
}

/// Enum representing the action taken upon the status byte when receiving data from the camera.
#[derive(Default, PartialEq, Eq)]
pub enum StatusByteAction {
    #[default]
    /// Default action. Evaluates the status byte (usually the first one) and acts accordingly (either tries to send the command again or returns an error)
    Evaluate,
    /// Ignores the status byte, does not attempt to retry.
    Ignore,
    /// Attempts to retry if the status byte indicates that the camera is in power saving mode, but otherwise ignores it.
    IgnoreButRetryIfPowerSaving,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, int_enum::IntEnum)]
/// Represents the thermal status of the camera.
pub enum ThermalStatus {
    Ok = 0,
    OverheatLow = 1,
    OverheatHigh = 2,
    Cold = 3,
}

#[derive(Debug, Clone)]
/// Contains the raw live view frame buffer and the frame duration.
pub struct LiveViewFrame {
    pub duration: std::time::Duration,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
/// Represents the capture status of a picture.
pub enum CaptureStatus {
    ThumbnailAvailable {
        stored_pic_num: u8,
        is_exposure_ready: bool,
        picture_status: u8,
        picture_string: Option<String>,
    },
    TryAgain,
    Captured,
}

impl HaCam {
    /// Opens the USB connection to the camera with default parameters.
    ///
    /// The caller should then use the `initialize_comm` function,
    /// which initializes the data communication to the camera.
    pub fn new() -> CamResult<Self> {
        Self::new_custom(DEFAULT_VID, DEFAULT_PID, 3)
    }

    /// Opens the USB connection to the camera with custom parameters.
    ///
    /// The caller should then use the `initialize_comm` function,
    /// which initializes the data communication to the camera.
    ///
    /// * `vid` - VID of the USB camera.
    /// * `pid` - VID of the USB camera.
    /// * `default_tries` - Specify the default try count for the entire struct.
    fn new_custom(vid: u16, pid: u16, default_tries: u32) -> CamResult<Self> {
        let dev_info = nusb::list_devices()?
            .find(|d| d.vendor_id() == vid && d.product_id() == pid)
            .ok_or(CamError::NoDeviceFound { vid, pid })?;

        let device = dev_info.open()?;

        let interface = device.claim_interface(0)?;

        Ok(Self {
            interface,
            default_tries,
            in_addr: ENDPOINT_IN_ADDR,
            out_addr: ENDPOINT_OUT_ADDR,
        })
    }

    /// Attempts to initialize communication to the camera.
    pub async fn initialize_comm(&mut self) -> CamResult<()> {
        for attempt_no in 0..self.default_tries {
            let out = self
                .read_data_unchecked(&consts::scsi::OPEN_CONN_COMMAND)
                .await?;

            match out[0] {
                0 => {
                    info!("Connection initialized successfully!");
                    return Ok(());
                }
                1 => warn!(
                    "Connection initialized unsuccessfully, trying again... (Attempt {attempt_no}/{})",
                    self.default_tries
                ),
                other => {
                    error!("Unable to initialize connection. Status code: {other}.");

                    return Err(CamError::ConnInit {
                        tries: 1,
                        status_code: other as u32,
                    });
                }
            }

            tokio::time::sleep(consts::INIT_ATTEMPT_INTERVAL).await;
        }

        error!(
            "Unable to initialize connection, reached max attempts ({}).",
            self.default_tries
        );
        Err(CamError::ConnInit {
            tries: self.default_tries,
            status_code: 1,
        })
    }

    /// Sends the keepalive command with the default keepalive timeout.
    /// The keepalive command should be sent every 500ms (the default keepalive interval),
    /// when there are no other transfers.
    pub async fn send_keepalive(&mut self) -> CamResult<()> {
        let max_recv_size = consts::KEEPALIVE_RX_BUF_SIZE;

        let req_buf = nusb::transfer::RequestBuffer::new(max_recv_size);

        let cmd = Self::make_cmd_header(
            &consts::scsi::KEEP_ALIVE_COMMAND,
            max_recv_size as i32,
            true,
            Self::rand_int(),
        )?;

        tokio::time::timeout(
            consts::KEEPALIVE_TIMEOUT,
            self.interface.bulk_out(self.out_addr, cmd),
        )
        .await?
        .into_result()?;

        let res = tokio::time::timeout(
            consts::KEEPALIVE_TIMEOUT,
            self.interface.bulk_in(self.in_addr, req_buf),
        )
        .await?
        .into_result()?;

        let status_byte = res.first().ok_or(CamError::InvalidLength {
            expected: 1,
            received: 0,
        })?;

        match status_byte {
            0 => Ok(()),
            other => {
                error!("Error in keepalive! Received unknown/errorous status code {other}.");
                Err(CamError::Keepalive {
                    status_code: *other as u32,
                })
            }
        }
    }

    /// Resets the USB camera device via an USB control transfer.
    pub async fn reset_usb(&mut self) -> CamResult<()> {
        // The original app uses the bmRequestType value of 0x21 (33),
        // in binary represented as 0b0010_0001.
        //                            ^^^^ ^^^^
        //                            7654 3210
        //
        // The 7th bit represents the data phase transfer direction (in this case 0 for host-to-device direction)
        // The 6th and 5th bits represent the type - standard (0), class (1), vendor (2), reserved (3)
        // Bits 4 to 0 represent the recipient.
        // source: https://www.beyondlogic.org/usbnutshell/usb6.shtml#SetupPacket

        let ctrl = ControlOut {
            control_type: ControlType::Class,
            recipient: Recipient::Interface,
            request: 255,
            value: 0,
            index: self.interface.interface_number() as u16,
            data: &[],
        };

        self.interface
            .control_out(ctrl)
            .await
            .into_result()
            .inspect_err(|e| {
                warn!("An error occured while attempting to reset USB via control transfer ({e})")
            })?;

        Ok(())
    }

    /// Internal message checking function.
    fn is_msg_csw(buf: &[u8], check_int: i32) -> bool {
        let len = buf.len();

        if len < 13 {
            return false;
        }

        let is_first_part_ok = buf[len - 13..=len - 10] == RX_HEADER_MAGIC;

        if check_int == 0 {
            is_first_part_ok
        } else {
            let is_second_part_ok = buf[len - 9..=len - 6] == check_int.to_be_bytes();

            is_first_part_ok && is_second_part_ok
        }
    }

    /// Creates the header for a command.
    ///
    /// * `cmd_bfr` - Command buffer such as TAKE_PICTURE, GET_REMAINING_PIC_NUM, etc. Usually 16 bytes.
    ///   This specifies the type of command, but also sometimes includes other data, such as the length of received data
    ///   when transferring a picture.
    /// * `max_recv_size` - Specifies the maximum receiving size. Usually 65536 (`consts::MAX_RECV_SIZE`) for read commands.
    /// * `is_read` - Specifies if the command reads/queries data (such as taking a picture or transferring it)
    ///   or writes data (such as settings or firmware updates).
    /// * `check_int` - Integer used for checksum. Usually provided by a random integer function.
    fn make_cmd_header(
        cmd_bfr: &[i8],
        max_recv_size: i32,
        is_read: bool,
        check_int: i32,
    ) -> CamResult<Vec<u8>> {
        let mut buf = Vec::<u8>::with_capacity(31);

        let transfer_type = if is_read { -128i8 } else { 0 };

        buf.write_all(&consts::TX_HEADER_MAGIC)?;
        buf.write_all(&check_int.to_be_bytes())?;
        buf.write_all(&max_recv_size.to_le_bytes())?; // reverse byte order
        buf.write_all(&transfer_type.to_be_bytes())?;
        buf.write_all(&0i8.to_be_bytes())?;
        buf.write_all(&16i8.to_be_bytes())?;
        buf.write_all(
            &cmd_bfr
                .iter()
                .map(|signed| *signed as u8)
                .collect::<Vec<u8>>(),
        )?;

        buf.resize(31, 0);

        Ok(buf)
    }

    fn rand_int() -> i32 {
        rand::rng().random()
    }

    /// Sends a read command to the camera, without any checks or chunking.
    ///
    /// * `cmd_bfr` - The command buffer (such as TAKE_PICTURE, GET_CAMERA_INFO). Usually 16 bytes.
    ///
    /// Returns the raw buffer sent by the camera.
    async fn read_data_unchecked(&mut self, cmd_bfr: &[i8]) -> CamResult<Vec<u8>> {
        let req_buf = nusb::transfer::RequestBuffer::new(consts::DEFAULT_MAX_RECV_SIZE);

        let out_buf: Vec<u8> = Self::make_cmd_header(
            cmd_bfr,
            consts::DEFAULT_MAX_RECV_SIZE as i32,
            true,
            Self::rand_int(),
        )?;

        self.interface
            .bulk_out(self.out_addr, out_buf)
            .await
            .into_result()?;

        let in_buf = self
            .interface
            .bulk_in(self.in_addr, req_buf)
            .await
            .into_result()?;

        Ok(in_buf)
    }

    /// Sends a read command to the camera with the specified timeout.
    /// In comparison to the `read_data_unchecked` function, this function
    /// works similar to the original camera code - receiving the data in 16 KiB chunks,
    /// while also using the checksum messages.
    ///
    /// * `cmd_bfr` - The command buffer (such as TAKE_PICTURE, GET_CAMERA_INFO). Usually 16 bytes.
    /// * `timeout` - Specifies the transfer timeout.
    ///
    /// Returns the raw buffer sent by the camera.
    async fn read_data(&mut self, cmd_bfr: &[i8], timeout: std::time::Duration) -> CamResult<Vec<u8>> {
        let mut ret_buf: Vec<u8> = Vec::with_capacity(consts::DEFAULT_MAX_RECV_SIZE);

        let check_int = Self::rand_int();

        let out_buf: Vec<u8> = Self::make_cmd_header(
            cmd_bfr,
            consts::DEFAULT_MAX_RECV_SIZE as i32,
            true,
            check_int,
        )?;

        tokio::time::timeout(timeout, self.interface.bulk_out(self.out_addr, out_buf))
            .await?
            .into_result()?;

        loop {
            let req_buf = nusb::transfer::RequestBuffer::new(consts::DEFAULT_CHUNK_SIZE);

            let in_tmp_buf =
                tokio::time::timeout(timeout, self.interface.bulk_in(self.in_addr, req_buf))
                    .await?
                    .into_result()?;

            if Self::is_msg_csw(&in_tmp_buf, check_int) {
                if in_tmp_buf.len() > 13 {
                    ret_buf.extend(&in_tmp_buf[..in_tmp_buf.len() - 13]);
                }

                break;
            } else {
                if in_tmp_buf.len() + ret_buf.len() > consts::DEFAULT_MAX_RECV_SIZE {
                    error!(
                        "Received too much data! in_tmp_buf: {}, ret_buf: {}, max_recv_size: {}",
                        in_tmp_buf.len(),
                        ret_buf.len(),
                        consts::DEFAULT_MAX_RECV_SIZE
                    );

                    break;
                }

                ret_buf.extend(in_tmp_buf);
            }
        }

        Ok(ret_buf)
    }

    /// Sends a write command to the camera with the specified timeout. This is usually used for firmware update commands
    /// or writing settings.
    ///
    /// * `cmd_bfr` - Command type (for example WRITE_ALL_SETTINGS).
    /// * `data_bfr` - Data buffer sent to the camera.
    /// * `timeout` - Transfer timeout.
    async fn write_data(
        &mut self,
        cmd_bfr: &[i8],
        data_bfr: Vec<u8>,
        timeout: std::time::Duration,
    ) -> CamResult<()> {
        let check_int = Self::rand_int();

        let cmd_header = Self::make_cmd_header(cmd_bfr, data_bfr.len() as i32, false, check_int)?;

        tokio::time::timeout(timeout, self.interface.bulk_out(self.out_addr, cmd_header))
            .await?
            .into_result()?;

        for data_chunk in data_bfr.chunks(consts::DEFAULT_CHUNK_SIZE) {
            tokio::time::timeout(
                timeout,
                self.interface.bulk_out(self.out_addr, data_chunk.to_vec()),
            )
            .await?
            .into_result()?;
        }

        let req_buf = nusb::transfer::RequestBuffer::new(consts::DEFAULT_CHUNK_SIZE);

        let received_buf =
            tokio::time::timeout(timeout, self.interface.bulk_in(self.in_addr, req_buf))
                .await?
                .into_result()?;

        if Self::is_msg_csw(&received_buf, check_int) {
            Ok(())
        } else {
            error!("Couldn't write data: unknown received data (non-CSW)");
            Err(CamError::Write)
        }
    }

    /// Sends a custom read command to the camera, optionally evaluating the status byte.
    ///
    /// * `cmd` - The command buffer (such as TAKE_PICTURE, GET_CAMERA_INFO). Usually 16 bytes.
    /// * `action` - Picks the `StatusByteAction`. This affects if the command is either retried, retried but only if it is in power saving mode
    ///   or if the buffer is returned raw. This is useful for commands such as `GET_REMAINING_PIC_NUM`, which use the status byte
    ///   as the return value.
    /// * `retries` - Number of "soft retries" (retrying only if we fail not due to USB issues)
    ///
    /// Returns the raw buffer returned by the camera.
    pub async fn send_custom_read_command(
        &mut self,
        cmd: &[i8],
        action: StatusByteAction,
        timeout: std::time::Duration,
    ) -> CamResult<Vec<u8>> {
        let tries = 1 + self.default_tries;

        for try_attempt in 0..tries {
            let res = self.read_data(cmd, timeout).await;

            if action == StatusByteAction::Ignore {
                return res;
            }

            let buf = res?;

            let status_byte = buf.first().ok_or(CamError::InvalidLength {
                expected: 1,
                received: 0,
            })?;

            match status_byte {
                0 | 1 => return Ok(buf),
                255 => {
                    warn!("Camera is in power save mode.");
                    info!("Attempting to reinitialize the USB connection...");
                    self.initialize_comm().await?;
                    continue;
                }
                _ if action == StatusByteAction::IgnoreButRetryIfPowerSaving => return Ok(buf),
                2 => warn!("Encountered unrecognized fail signal (2)"),
                3 => warn!(
                    "Received retry signal while attempting to send command. Attempting again ({try_attempt}/{tries})"
                ),
                unknown => warn!("Other/unknown status code received {unknown}"),
            }
        }

        error!("Exhausted retry attempts ({tries}) while sending command");
        Err(CamError::SendCommand {
            tries,
            status_code: 0,
        })
    }

    /// Gets the amount of remaining pictures to be read.
    pub async fn query_remaining_pic_num(&mut self) -> CamResult<u8> {
        let data = self
            .send_custom_read_command(
                &consts::usb::GET_REMAINING_PIC_NUM,
                StatusByteAction::IgnoreButRetryIfPowerSaving,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        Ok(data[0])
    }

    /// Clears the picture buffer of the camera.
    pub async fn clear_camera_pic_buf(&mut self) -> CamResult<()> {
        self.send_custom_read_command(
            &consts::usb::CLEAR_PIC_BUF,
            StatusByteAction::Evaluate,
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }

    /// Powers off the camera.
    pub async fn power_off(&mut self) -> CamResult<()> {
        self.send_custom_read_command(
            &consts::usb::POWER_OFF_CAMERA,
            StatusByteAction::Evaluate,
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }

    /// Starts the live view stream. The caller can then check the status of the stream via the `check_live_view_status`,
    /// receive it with `get_live_view_frame` or stop it via the `stop_live_view` function.
    ///
    /// * `resolution` - Specifies the resolution, which is either high (1920x960) or low (1280x640)
    pub async fn start_live_view(&mut self, resolution: LiveViewResolution) -> CamResult<()> {
        let mut cmd = consts::usb::START_LIVE_VIEW.to_vec();
        cmd[9] = resolution as i8;

        self.send_custom_read_command(
            &cmd,
            StatusByteAction::Evaluate,
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }

    /// Stops the live view stream. The caller than then check the stop status
    /// via the `check_live_view_stop_request_status` function.
    pub async fn stop_live_view(&mut self) -> CamResult<()> {
        self.send_custom_read_command(
            &consts::usb::STOP_LIVE_VIEW,
            StatusByteAction::Evaluate,
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }

    /// Checks the live view status. Returns `true` if the status is OK.
    pub async fn check_live_view_status(&mut self) -> CamResult<bool> {
        let data = self
            .send_custom_read_command(
                &consts::usb::CHECK_LIVE_VIEW_STATUS,
                StatusByteAction::Ignore,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        let status = data.first().ok_or(CamError::InvalidLength {
            expected: 1,
            received: 0,
        })?;

        Ok(*status != 3 && *status != 1)
    }

    /// Returns the live view frame and the camera's thermal status.
    pub async fn get_live_view_frame(&mut self) -> CamResult<(ThermalStatus, LiveViewFrame)> {
        let mut buf: Vec<u8> = Vec::with_capacity(1048576);

        let start = tokio::time::Instant::now();

        let thermal_status = loop {
            let data = self
                .send_custom_read_command(
                    &consts::usb::GET_LIVE_VIEW_FRAME,
                    StatusByteAction::Evaluate,
                    consts::DEFAULT_TRANSFER_TIMEOUT,
                )
                .await?;

            if data.len() < 32 {
                return Err(CamError::InvalidLength {
                    expected: 32,
                    received: data.len(),
                });
            }

            let rx_len = u32::from_le_bytes(data[28..32].try_into().unwrap()) as usize;

            if data.len() < rx_len + 32 {
                return Err(CamError::InvalidLength {
                    expected: rx_len + 32,
                    received: data.len(),
                });
            }

            buf.extend(&data[32..32 + rx_len]);

            if data[1] == 1 {
                // This message contains the last part of the frame.
                break data[20];
            }
        };

        let duration = start.elapsed();

        let frame = LiveViewFrame {
            duration,
            data: buf,
        };

        let thermal_status =
            ThermalStatus::try_from(thermal_status as i8).map_err(|_| CamError::InvalidFormat)?;

        Ok((thermal_status, frame))
    }

    /// Acquires the thumbnail after taking a picture (with the `take_picture` function).
    /// The `check_capture_status` function indicates, whether the thumbnail is ready to be received.
    ///
    /// Returns the raw thumbnail buffer.
    pub async fn get_thumbnail(&mut self) -> CamResult<Vec<u8>> {
        let mut data = self
            .send_custom_read_command(
                &consts::usb::GET_PIC_THUMBNAIL,
                StatusByteAction::Evaluate,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        if data.len() < 20 {
            return Err(CamError::InvalidLength {
                expected: 20,
                received: data.len(),
            });
        }

        let thumb_len = u32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;

        if data.len() < 20 + thumb_len {
            return Err(CamError::InvalidLength {
                expected: 20 + thumb_len,
                received: data.len(),
            });
        }

        let thumb_buf = data.drain(20..20 + thumb_len).collect();

        Ok(thumb_buf)
    }

    /// Gets the partial picture buffer from the camera. This function is used after taking a picture
    /// and checking the capture status.
    ///
    /// * `received_pic_data_len` - Specifies the size of the part of the picture which was already received.
    ///
    /// Returns the picture buffer and a bool signifying if the partial buffer is the last.
    pub async fn get_partial_picture_buffer(
        &mut self,
        received_pic_data_len: u32,
    ) -> CamResult<(Vec<u8>, bool)> {
        let mut cmd = consts::usb::READ_PIC_BUF.to_vec();

        let rpdl_signed_bytes = received_pic_data_len.to_le_bytes().map(|b| b as i8);

        cmd[8..12].copy_from_slice(&rpdl_signed_bytes);

        let mut data = self
            .send_custom_read_command(
                &cmd,
                StatusByteAction::IgnoreButRetryIfPowerSaving,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        if data.len() < 20 {
            return Err(CamError::InvalidLength {
                expected: 20,
                received: data.len(),
            });
        }

        let is_end = data[1] == 1;

        let partial_pic_buf_len = u32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;

        if data.len() < 20 + partial_pic_buf_len {
            return Err(CamError::InvalidLength {
                expected: 20 + partial_pic_buf_len,
                received: data.len(),
            });
        }

        let pic_buf = data.drain(20..20 + partial_pic_buf_len).collect();

        Ok((pic_buf, is_end))
    }

    /// Starts the recording. The caller than then check the stop status
    /// via the `check_start_recording` function.
    pub async fn start_recording(&mut self) -> CamResult<()> {
        self.send_custom_read_command(
            &consts::usb::START_RECORDING,
            StatusByteAction::Evaluate,
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }

    /// Stops the recording. The caller than then check the stop status
    /// via the `check_stop_recording` function.
    pub async fn stop_recording(&mut self) -> CamResult<()> {
        self.send_custom_read_command(
            &consts::usb::STOP_RECORDING,
            StatusByteAction::Evaluate,
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }

    /// Checks the capture status when taking a picture. This function is used right after taking a picture.
    ///
    /// Returns an enum with three possible states: whether the thumbnail is available, the caller should try again
    /// or if the picture is fully captured.
    pub async fn check_capture_status(&mut self) -> CamResult<CaptureStatus> {
        let data = self
            .send_custom_read_command(
                &consts::usb::CHECK_CAPTURE_STATUS,
                StatusByteAction::Ignore,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        let Some(status) = data.first() else {
            return Err(CamError::InvalidLength {
                expected: 1,
                received: 0,
            });
        };

        match *status {
            1 => return Ok(CaptureStatus::TryAgain),
            3 => return Ok(CaptureStatus::Captured),
            0 => {}
            other => {
                warn!(
                    "Received unknown status code ({other}) while attempting to check capture status"
                );
                return Err(CamError::InvalidFormat);
            }
        }

        if data.len() < 9 {
            return Err(CamError::InvalidLength {
                expected: 9,
                received: data.len(),
            });
        }

        let picture_status = data[1]; // This value's purpose is unknown, it isn't used anywhere
        let is_exposure_ready = data[2] == 0;
        let stored_pic_num = data[3];

        let picture_string = if data[8] != 0 {
            let str_buf = data[8..data.len().min(72)]
                .iter()
                .take_while(|&&byte| byte != 0)
                .copied()
                .collect::<Vec<_>>();

            Some(String::from_utf8_lossy(&str_buf).into_owned())
        } else {
            None
        };

        let capture_status = CaptureStatus::ThumbnailAvailable {
            stored_pic_num,
            is_exposure_ready,
            picture_status,
            picture_string,
        };

        Ok(capture_status)
    }

    /// Checks the status of a live view stop request. Returns `true` if the status is OK.
    pub async fn check_live_view_stop_request_status(&mut self) -> CamResult<bool> {
        let data = self
            .send_custom_read_command(
                &consts::usb::CHECK_LIVE_VIEW_STOP_STATUS,
                StatusByteAction::Ignore,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        let status = data.first().ok_or(CamError::InvalidLength {
            expected: 1,
            received: 0,
        })?;

        Ok(*status != 3 && *status != 1)
    }

    /// Checks the status of a recording request. Returns `true` if the status is OK.
    pub async fn check_start_recording_request(&mut self) -> CamResult<bool> {
        let data = self
            .send_custom_read_command(
                &consts::usb::CHECK_START_RECORDING,
                StatusByteAction::Ignore,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        let status = data.first().ok_or(CamError::InvalidLength {
            expected: 1,
            received: 0,
        })?;

        Ok(*status != 3 && *status != 1)
    }

    /// Checks the status of the request for stopping recording. Returns `true` if the status is OK.
    pub async fn check_stop_recording_request(&mut self) -> CamResult<bool> {
        let data = self
            .send_custom_read_command(
                &consts::usb::CHECK_STOP_RECORDING,
                StatusByteAction::Ignore,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        let status = data.first().ok_or(CamError::InvalidLength {
            expected: 1,
            received: 0,
        })?;

        Ok(*status != 3 && *status != 1)
    }

    /// Takes picture using the provided orientation. This function does not return the picture,
    /// thus the caller should then check the capture status,
    /// optionally acquire the thumbnail and then proceed to get the picture.
    ///
    /// * `orientation` - Specifies the orientation of the picture. (0/90/180/270 deg)
    pub async fn take_picture(&mut self, orientation: PictureOrientation) -> CamResult<()> {
        let mut cmd = consts::usb::TAKE_PICTURE.to_vec();
        cmd[8] = orientation as i8;

        self.send_custom_read_command(
            &cmd,
            StatusByteAction::Evaluate,
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }

    /// Gets the camera's execution status and thermal status. The purpose of the execution status
    /// data is unknown.
    pub async fn get_camera_status(&mut self) -> CamResult<(u8, ThermalStatus)> {
        let data = self
            .send_custom_read_command(
                &consts::usb::GET_CAMERA_STATUS,
                StatusByteAction::Evaluate,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        if data.len() < 5 {
            return Err(CamError::InvalidLength {
                expected: 5,
                received: data.len(),
            });
        }

        let execution_status = data[1]; // This value's purpose is unknown, it isn't used anywhere
        let thermal_status = data[4];

        let thermal_status = ThermalStatus::try_from(thermal_status as i8)
            .inspect_err(|_| warn!("Received invalid thermal status value ({thermal_status})"))
            .map_err(|_| CamError::InvalidFormat)?;

        Ok((execution_status, thermal_status))
    }

    /// Returns the camera firmware version. The command returns more data, but its purpose is unknown.
    pub async fn get_camera_info(&mut self) -> CamResult<Option<String>> {
        let data = self
            .send_custom_read_command(
                &consts::usb::GET_CAMERA_INFO,
                StatusByteAction::Evaluate,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        if data.len() < 97 {
            return Err(CamError::InvalidLength {
                expected: 97,
                received: data.len(),
            });
        }

        if data[97] == 118 {
            let str_buf = data[97..data.len().min(129)]
                .iter()
                .take_while(|&&byte| byte != 0)
                .copied()
                .collect::<Vec<_>>();

            let fw_version = String::from_utf8_lossy(&str_buf).into_owned();

            return Ok(Some(fw_version));
        }

        Ok(None)
    }

    /// Returns the "SCSI" version of the camera.
    pub async fn get_scsi_version(&mut self) -> CamResult<Option<String>> {
        let data = self
            .send_custom_read_command(
                &consts::usb::GET_SCSI_VERSION,
                StatusByteAction::Evaluate,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        if data.is_empty() {
            return Err(CamError::InvalidLength {
                expected: 1,
                received: 0,
            });
        }

        if data[1] == 118 {
            let str_buf = data[1..data.len().min(33)]
                .iter()
                .take_while(|&&byte| byte != 0)
                .copied()
                .collect::<Vec<_>>();

            let scsi_version = String::from_utf8_lossy(&str_buf).into_owned();

            return Ok(Some(scsi_version));
        }

        Ok(None)
    }

    /// Writes one setting to the camera.
    /// 
    /// * `setting` - The type of setting.
    /// * `value` - The setting value (one signed byte)
    pub async fn write_setting(&mut self, setting: SettingType, value: u8) -> CamResult<()> {
        let mut cmd = consts::usb::WRITE_GENERAL_SETTING.to_vec();
        cmd[2] = setting as i8;

        self.write_data(
            &cmd,
            vec![value],
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }

    /// Reads one setting from the camera.
    /// 
    /// * `setting` - The type of setting.
    /// 
    /// Returns the value of the setting.
    pub async fn read_setting(&mut self, setting: SettingType) -> CamResult<u8> {
        let mut cmd = consts::usb::READ_GENERAL_SETTING.to_vec();
        cmd[2] = setting as i8;

        let data = self
            .send_custom_read_command(
                &cmd,
                StatusByteAction::IgnoreButRetryIfPowerSaving,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        Ok(data[0])
    }

    /// Returns all settings of the camera.
    pub async fn read_all_settings(&mut self) -> CamResult<CamSettings> {
        let data = self
            .send_custom_read_command(
                &consts::usb::READ_ALL_SETTINGS,
                StatusByteAction::Evaluate,
                consts::DEFAULT_TRANSFER_TIMEOUT,
            )
            .await?;

        if data.is_empty() {
            return Err(CamError::InvalidLength {
                expected: 1,
                received: 0,
            });
        }

        let settings = CamSettings::from_bytes(&data)
            .ok_or(CamError::InvalidFormat)?;

        Ok(settings)
    }

    /// Writes all settings (of the `CamSettings` struct) to the camera.
    /// 
    /// * `settings` - The camera settings.
    pub async fn write_all_settings(&mut self, settings: CamSettings) -> CamResult<()> {
        let data_bfr = settings.to_bytes();

        self.write_data(
            &consts::usb::WRITE_ALL_SETTINGS,
            data_bfr,
            consts::DEFAULT_TRANSFER_TIMEOUT,
        )
        .await?;

        Ok(())
    }
}

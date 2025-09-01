use crate::{settings::{LiveViewResolution, PictureOrientation}, CamResult, cam::CaptureStatus, cam::HaCam};
use std::future::Future;

/// This trait provides convenience functions for the `HaCam` struct.
pub trait CamUtil {
    /// Convenience method for taking a picture and also transferring it.
    ///
    /// * `orientation` - Specifies the orientation of the picture. (0/90/180/270 deg)
    /// * `was_live_view_initialized` - If true, skips initialization of the camera's live view.
    ///   Live view needs to be initialized, otherwise the picture returned is all black.
    /// * `on_thumbnail` - Optional closure which is called when a thumbnail is received. 
    ///   (Rust complains if you just provide `None` as the parameter value, so provide `None::<fn(_)>` as a value)
    ///
    /// Returns the JPG picture as a byte buffer.
    fn take_picture_and_get(
        &mut self,
        orientation: PictureOrientation,
        on_thumbnail: Option<impl FnMut(Vec<u8>) + Send>,
        was_live_view_initialized: bool,
    ) -> impl Future<Output = CamResult<Vec<u8>>> + Send;
}

impl CamUtil for HaCam {
    async fn take_picture_and_get(
        &mut self,
        orientation: PictureOrientation,
        mut on_thumbnail: Option<impl FnMut(Vec<u8>) + Send>,
        was_live_view_initialized: bool,
    ) -> CamResult<Vec<u8>> {
        if !was_live_view_initialized {
            self.start_live_view(LiveViewResolution::Low).await?;

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let live_view_start_status: bool = self.check_live_view_status().await?;

            if !live_view_start_status {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            let _ = self.get_live_view_frame().await?;

            self.stop_live_view().await?;

            let live_view_stop_status: bool = self.check_live_view_stop_request_status().await?;

            if !live_view_stop_status {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        self.clear_camera_pic_buf().await?;

        self.take_picture(orientation).await?;

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let res = self.check_capture_status().await?;

            match res {
                CaptureStatus::ThumbnailAvailable { .. } => {
                    if let Some(ref mut on_thumbnail) = on_thumbnail {
                        let thumbnail = self.get_thumbnail().await?;

                        on_thumbnail(thumbnail);
                    }

                    continue;
                },
                CaptureStatus::TryAgain => continue,
                CaptureStatus::Captured => {
                    let mut buf = Vec::new();
                    loop {
                        let (pbuf, is_end) =
                            self.get_partial_picture_buffer(buf.len() as u32).await?;

                        buf.extend(pbuf);

                        if is_end {
                            break;
                        }
                    }
                    return Ok(buf);
                }
            }
        }
    }
}
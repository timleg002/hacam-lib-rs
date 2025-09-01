use hacam_lib_rs::{
    cam::HaCam,
    settings::{LiveViewResolution},
};
use image::{codecs::{jpeg::JpegEncoder}, ExtendedColorType, ImageEncoder};
use openh264::formats::YUVSource as _;
use tokio::time::sleep;
use yuv::{yuv420_to_rgb, YuvPlanarImage, YuvRange, YuvStandardMatrix};
use std::{fs::File, time::Duration};

#[tokio::main]
/// This example saves a few live view frames as JPG images.
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cam = HaCam::new()?;

    cam.initialize_comm().await?;

    let res = LiveViewResolution::Low;

    cam.start_live_view(res).await?;

    sleep(Duration::from_millis(500)).await;

    println!(
        "Live view started {}successfully!",
        if cam.check_live_view_status().await? {
            ""
        } else {
            "un"
        }
    );

    let mut decoder = openh264::decoder::Decoder::new()?;

    let mut frame_cnt = 0;

    while let Ok((_, frame)) = cam.get_live_view_frame().await {
        for packet in openh264::nal_units(&frame.data) {
            let decoded_pkt = decoder.decode(packet);

            if let Ok(Some(decoded_yuv_frame)) = decoded_pkt {
                let (w, h) = decoded_yuv_frame.dimensions();
                let (y, u, v) = decoded_yuv_frame.strides();

                let yuv_image = YuvPlanarImage {
                    y_plane: decoded_yuv_frame.y(),
                    u_plane: decoded_yuv_frame.u(),
                    v_plane: decoded_yuv_frame.v(),
                    y_stride: y as u32,
                    u_stride: u as u32,
                    v_stride: v as u32,
                    width: w as u32,
                    height: h as u32,
                };

                let column_cnt = 3 * w; // this is the stride - 3 represents the amount of color channels

                let mut rgb_image = vec![0; column_cnt * h]; 

                yuv420_to_rgb(
                    &yuv_image,
                    &mut rgb_image,
                    column_cnt as u32,
                    YuvRange::Full,
                    YuvStandardMatrix::Bt601,
                )?;

                let mut out_file = File::create(format!("frame-{frame_cnt}.jpg"))?;
                let jpg = JpegEncoder::new(&mut out_file);
                jpg.write_image(&rgb_image, w as u32, h as u32, ExtendedColorType::Rgb8)?;
            }
        }

        frame_cnt += 1;

        if frame_cnt > 3 {
            break;
        }
    }

    cam.stop_recording().await?;

    sleep(Duration::from_millis(500)).await;

    println!(
        "Live view ended {}successfully!",
        if cam.check_live_view_stop_request_status().await? {
            ""
        } else {
            "un"
        }
    );

    Ok(())
}

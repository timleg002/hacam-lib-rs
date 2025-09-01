use std::fs::File;
use hacam_lib_rs::{
    cam::HaCam,
    settings::{Resolution as _, SettingType, VideoResolution},
};

#[tokio::main]
/// This example takes a 5-second video and saves it as a MP4 file (using the minimp4 crate)
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cam = HaCam::new()?;

    cam.initialize_comm().await?;

    let res = VideoResolution::High;

    cam.write_setting(SettingType::VideoResolution, VideoResolution::High as u8)
        .await?;

    cam.start_recording().await?;

    println!(
        "Recording started {}successfully!",
        if cam.check_start_recording_request().await? { "" } else { "un" }
    );

    let mut mp4muxer = minimp4::Mp4Muxer::new(File::create("video.mp4")?);

    mp4muxer.init_video(res.w() as i32, res.h() as i32, false, "video");

    let mut frame_cnt = 0;

    while let Ok((_, frame)) = cam.get_live_view_frame().await {
        mp4muxer.write_video_with_fps(&frame.data, 30);

        frame_cnt += 1;

        if frame_cnt > 25_000 {
            break;
        }
    }

    cam.stop_recording().await?;

    println!(
        "Recording ended {}successfully!",
        if cam.check_stop_recording_request().await? { "" } else { "un" }
    );

    mp4muxer.close();

    Ok(())
}

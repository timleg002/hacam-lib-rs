use std::{fs::File, io::Write};

use hacam_lib_rs::{cam::HaCam, settings::PictureOrientation, util::CamUtil};

#[tokio::main]
/// This example takes a photo and saves it in a file.
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cam = HaCam::new()?;

    cam.initialize_comm().await?;

    let img = cam.take_picture_and_get(
        PictureOrientation::Deg0,
        Some(on_thumbnail), 
        false
    ).await?;

    println!("Received an image! Saving it...");

    File::create("image.jpg")
        .unwrap()
        .write_all(&img)
        .unwrap();

    Ok(())
}

fn on_thumbnail(data: Vec<u8>) {
    println!("Thumbnail received! Saving it in a file :3");

    File::create("thumbnail.jpg")
        .unwrap()
        .write_all(&data)
        .unwrap();
}
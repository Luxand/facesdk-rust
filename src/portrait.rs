//!
//! Portrait sample - Rust implementation
//!
//! Usage: portrait <in_file> [out_file]
//!
//! This sample demonstrates face detection and cropping using Luxand FaceSDK.
//!

use std::env;
use std::process;

use fsdk::{FsdkError, Image, FSDK};

const LICENSE_KEY: &str = "<INSERT YOUR LICENSE KEY HERE>";

fn run() -> Result<(), FsdkError> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: portrait <in_file> [out_file]");
        eprintln!("Default out_file name is 'face.<in_file>'");
        process::exit(-1);
    }

    let input_filename = args[1].clone();
    let output_filename = args.get(2).cloned().unwrap_or_else(|| format!("face.{}", input_filename));

    print!("Initializing FSDK... ");
    FSDK::activate_library(LICENSE_KEY)?;
    FSDK::initialize()?;
    println!("OK");

    let license_info = FSDK::get_license_info()?;
    println!("License info: {}", license_info);

    println!("\nLoading file {} ...", input_filename);
    let img = Image::from_file(input_filename)?;

    // HandleArbitraryRotations=false, DetermineFaceRotationAngle=false, InternalResizeWidth=256
    FSDK::set_face_detection_parameters(false, false, 256)?;
    FSDK::set_face_detection_threshold(5)?;

    println!("Detecting face...");
    let face = img.detect_face()?;

    let (x1, y1, x2, y2) = face.rect();

    // Crop and resize face image
    let max_width: f64 = 337.0;
    let max_height: f64 = 450.0;
    let face_w = (x2 - x1) as f64;
    let face_h = (y2 - y1) as f64;

    let cropped = img.crop(x1, y1, x2, y2)?;
    let ratio = f64::min(
        (max_width + 0.4) / (face_w + 1.0),
        (max_height + 0.4) / (face_h + 1.0),
    );
    let resized = cropped.resize(ratio)?;

    // Save face image to file with given compression quality
    resized.save_to_file_with_quality(&output_filename, 85)?;

    println!("File '{}' with detected face is created.", output_filename);

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

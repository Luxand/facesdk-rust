//! Live Face Recognition Sample using Luxand FaceSDK in Rust
//!
//! Uses the native camera via nokhwa crate
//! and displays the video with face detection overlay using minifb.
//! Demonstrates v2 face detection model with liveness detection.
//!
//! Platform-specific liveness:
//! - Windows/Linux: iBeta certified liveness addon
//! - macOS: Built-in liveness detection

use std::sync::OnceLock;
use std::time::Instant;

use tinyfiledialogs::input_box;

use minifb::{Key, Window, WindowOptions};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType, FrameFormat};
use nokhwa::Camera;

use fsdk::{
    Face, Image, Tracker, FSDK, FSDK_IMAGE_COLOR_24BIT,
};

const LICENSE_KEY: &str = "<INSERT YOUR LICENSE KEY HERE>";
const TRACKER_MEMORY_FILE: &str = "tracker.dat";

#[cfg(not(target_os = "macos"))]
const IBETA_DIR: &str = "./fsdk"; // Directory where iBeta data files are located (Windows/Linux)

static FONT: OnceLock<fontdue::Font> = OnceLock::new();
const FONT_DATA: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");

fn init_font() {
    let font = fontdue::Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
        .expect("Failed to parse embedded Inter font");
    let _ = FONT.set(font);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Force X11 on Linux to get window decorations and resize support.
    // minifb's Wayland backend does not provide server-side decorations on many compositors.
    #[cfg(target_os = "linux")]
    if std::env::var("WAYLAND_DISPLAY").is_ok() && std::env::var("DISPLAY").is_ok() {
        std::env::remove_var("WAYLAND_DISPLAY");
    }

    // Initialize FSDK
    print!("Initializing FSDK... ");
    FSDK::activate_library(LICENSE_KEY)?;
    FSDK::initialize()?;
    println!("OK");
    println!("License info: {}", FSDK::get_license_info()?);

    // Configure iBeta liveness on Windows/Linux (must be done before tracker creation)
    // The iBeta data files and plugin DLLs are in the same directory as facesdk.dll
    #[cfg(not(target_os = "macos"))]
    {
        let liveness_model = format!("external:dataDir={}", IBETA_DIR);
        match FSDK::set_parameter("LivenessModel", &liveness_model) {
            Ok(()) => println!("iBeta liveness model loaded from: {}", IBETA_DIR),
            Err(e) => println!("Warning: Could not load iBeta liveness model: {}. Liveness detection may not work.", e),
        }
    }

    // Load or create tracker.
    // Important: DetectionVersion cannot be changed on a non-empty tracker,
    // so do not use set_parameter as a compatibility check for loaded data.
    let tracker = match Tracker::from_file(TRACKER_MEMORY_FILE) {
        Ok(t) => {
            let known_ids = t.get_ids_count().unwrap_or(-1);
            println!("Loaded tracker from {} (known IDs: {})", TRACKER_MEMORY_FILE, known_ids);
            t
        }
        Err(e) => {
            println!("Creating new tracker (load failed: {})", e);
            let t = Tracker::new()?;
            t.set_parameter("DetectionVersion", "2")?;
            t
        }
    };

    // Platform-specific liveness configuration
    #[cfg(not(target_os = "macos"))]
    {
        // Windows/Linux: iBeta certified liveness (single-frame)
        tracker.set_parameters(
            "FaceDetection2PatchSize=256;\
             Threshold=0.8;\
             Threshold2=0.9;\
             DetectLiveness=true;\
             LivenessFramesCount=1;\
             SmoothAttributeLiveness=false"
        )?;
        println!("Liveness: iBeta certified (Windows/Linux)");
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: Built-in liveness detection (multi-frame)
        tracker.set_parameters(
            "FaceDetection2PatchSize=256;\
             Threshold=0.8;\
             Threshold2=0.9;\
             DetectLiveness=true;\
             LivenessFramesCount=6;\
             SmoothAttributeLiveness=true"
        )?;
        println!("Liveness: Built-in (macOS)");
    }

    // Load TrueType font for text rendering
    init_font();

    // Initialize camera
    println!("Opening camera...");
    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut camera = Camera::new(CameraIndex::Index(0), requested)?;
    camera.open_stream()?;

    let resolution = camera.resolution();
    let width = resolution.width() as usize;
    let height = resolution.height() as usize;
    println!("Camera opened: {}x{}", width, height);

    // Create display window (start at half camera resolution, allow resizing)
    let win_width = width / 2;
    let win_height = height / 2;
    let mut window = Window::new(
        "Live Face Recognition - Press ESC to exit, Click to name faces",
        win_width,
        win_height,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window.limit_update_rate(Some(std::time::Duration::from_micros(33333))); // ~30 FPS

    // Display buffer at camera resolution (ARGB format), then scaled for window
    let mut display_buffer: Vec<u32> = vec![0; width * height];
    let mut scaled_buffer: Vec<u32> = Vec::new();

    // Track face names
    let mut fps_counter = FpsCounter::new();

    println!("\nControls:");
    println!("  ESC - Exit and save tracker");
    println!("  Click on a face to name it");
    println!();

    let mut prev_mouse_down = false;
    let mut rgb_data: Vec<u8> = vec![0; width * height * 3];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame = camera.frame()?;
        let raw = frame.buffer();

        // Decode frame, produce RGB for display, and return an FSDK image for tracking.
        let fsdk_image = decode_frame_to_fsdk_image(frame.source_frame_format(), raw, width, height, &mut rgb_data)?;

        for y in 0..height {
            for x in 0..width {
                let src_idx = (y * width + x) * 3;
                let r = rgb_data[src_idx] as u32;
                let g = rgb_data[src_idx + 1] as u32;
                let b = rgb_data[src_idx + 2] as u32;
                display_buffer[y * width + x] = (r << 16) | (g << 8) | b;
            }
        }

        let face_ids = tracker.feed_frame(0, &fsdk_image, 32)?;

        // Compute scaling info for mouse mapping
        let (win_w, win_h) = window.get_size();
        let scale_x = win_w as f32 / width as f32;
        let scale_y = win_h as f32 / height as f32;
        let scale = scale_x.min(scale_y);
        let rw = (width as f32 * scale) as f32;
        let rh = (height as f32 * scale) as f32;
        let ox = (win_w as f32 - rw) / 2.0;
        let oy = (win_h as f32 - rh) / 2.0;

        // Map mouse position from window coordinates to buffer coordinates
        let mouse_buf_pos = window.get_mouse_pos(minifb::MouseMode::Clamp).and_then(|(mx, my)| {
            let bx = (mx - ox) * width as f32 / rw;
            let by = (my - oy) * height as f32 / rh;
            if bx >= 0.0 && bx < width as f32 && by >= 0.0 && by < height as f32 {
                Some((bx, by))
            } else {
                None
            }
        });

        // Draw face rectangles and info
        for &face_id in &face_ids {
            if let Ok(face) = tracker.get_face(0, face_id) {
                let name = tracker.get_all_names(face_id)
                    .ok()
                    .and_then(|names| names.into_iter().find(|n| !n.is_empty()))
                    .unwrap_or_default();

                // Get liveness status
                let (liveness, liveness_error) = get_liveness_info(&tracker, face_id);

                let hovered = mouse_buf_pos.map_or(false, |(mx, my)| {
                    let (x1, y1, x2, y2) = face.rect();
                    mx as i32 >= x1 && mx as i32 <= x2 && my as i32 >= y1 && my as i32 <= y2
                });

                draw_face_overlay(
                    &mut display_buffer, width, height,
                    &face, face_id, &name, liveness, &liveness_error, hovered,
                );
            }
        }

        // Draw FPS
        let fps = fps_counter.tick();
        draw_text_simple(&mut display_buffer, width, 5, 5, &format!("FPS: {:.1}", fps));
        draw_text_simple(&mut display_buffer, width, 5, 30, &format!("Faces: {}", face_ids.len()));

        // Scale to window and update (manual scaling avoids minifb rendering bug)
        scale_buffer_proportional(&display_buffer, width, height, win_w, win_h, &mut scaled_buffer);
        window.update_with_buffer(&scaled_buffer, win_w, win_h)?;

        // Handle mouse click to name a face
        let mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
        if mouse_down && !prev_mouse_down {
            if let Some((mx, my)) = mouse_buf_pos {
                for &face_id in &face_ids {
                    if let Ok(face) = tracker.get_face(0, face_id) {
                        let (x1, y1, x2, y2) = face.rect();
                        if mx as i32 >= x1 && mx as i32 <= x2 && my as i32 >= y1 && my as i32 <= y2 {
                            let current_name = tracker.get_name(face_id).unwrap_or_default();
                            if let Err(e) = tracker.lock_id(face_id) {
                                println!("Warning: failed to lock ID {}: {}", face_id, e);
                            }
                            if let Some(new_name) = show_name_dialog(face_id, &current_name) {
                                match tracker.set_name(face_id, &new_name) {
                                    Ok(()) => {
                                        println!("Named face ID {} -> {}", face_id, new_name);
                                        match tracker.save_to_file(TRACKER_MEMORY_FILE) {
                                            Ok(()) => println!("Tracker memory updated: {}", TRACKER_MEMORY_FILE),
                                            Err(e) => println!("Warning: failed to save tracker after naming: {}", e),
                                        }
                                    }
                                    Err(e) => println!("Warning: failed to set name for ID {}: {}", face_id, e),
                                }
                            }
                            if let Err(e) = tracker.unlock_id(face_id) {
                                println!("Warning: failed to unlock ID {}: {}", face_id, e);
                            }
                            break;
                        }
                    }
                }
            }
        }
        prev_mouse_down = mouse_down;
    }

    // Save tracker memory
    print!("Saving tracker memory... ");
    if let Ok(known_ids) = tracker.get_ids_count() {
        print!("known IDs: {}... ", known_ids);
    }
    tracker.save_to_file(TRACKER_MEMORY_FILE)?;
    println!("OK");

    // Cleanup: stop camera, then exit immediately.
    // We use process::exit to avoid crashes during static destruction,
    // where the DLL cleanup races with nokhwa/minifb destructors.
    camera.stop_stream()?;
    drop(camera);

    println!("Done!");
    std::process::exit(0);
}

/// Show a native input dialog to name a face. Returns None if cancelled or empty.
fn show_name_dialog(face_id: i64, current_name: &str) -> Option<String> {
    let title = format!("Name Face ID {}", face_id);
    // Pass a non-empty default to avoid password mode in tinyfiledialogs
    // (the C library uses --hide-text when the default string is empty)
    let default = if current_name.is_empty() { " " } else { current_name };
    input_box(&title, "Enter a name for this face:", default)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Get liveness information for a face ID.
/// Returns (liveness_confidence, liveness_error_string).
/// liveness_confidence is 0.0 if not yet determined.
fn get_liveness_info(tracker: &Tracker, face_id: i64) -> (f32, String) {
    let mut liveness: f32 = 0.0;
    #[allow(unused_mut)]
    let mut liveness_error = String::new();

    // Get liveness value
    let liveness_attr = tracker.get_facial_attribute(0, face_id, "Liveness");
    if let Ok(liveness_attr) = liveness_attr {
        if !liveness_attr.is_empty() {
            if let Ok(val) = FSDK::get_value_confidence(&liveness_attr, "Liveness") {
                liveness = val;
            }
        }
    }

    // Get liveness error (iBeta on Windows/Linux reports errors separately)
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(error_attr) = tracker.get_facial_attribute(0, face_id, "LivenessError") {
            // Format: "LivenessError=<error_message>;\n"
            if let Some(start) = error_attr.find('=') {
                let val = error_attr[start + 1..].trim_end_matches(|c: char| c == ';' || c.is_control());
                if !val.is_empty() {
                    liveness_error = val.to_string();
                }
            }
        }
    }

    (liveness, liveness_error)
}

/// Draw a face overlay with liveness-colored rectangle
fn draw_face_overlay(
    buffer: &mut [u32], width: usize, height: usize,
    face: &Face, id: i64, name: &str,
    liveness: f32, liveness_error: &str, hovered: bool,
) {
    let (x1, y1, x2, y2) = face.rect();
    let x1 = x1.max(0) as usize;
    let y1 = y1.max(0) as usize;
    let x2_clamped = x2.max(0).min(width.saturating_sub(1) as i32) as usize;
    let y2_clamped = y2.max(0).min(height.saturating_sub(1) as i32) as usize;

    // Color based on liveness and hover state
    let color = if hovered {
        0x4488FFu32 // blue when hovered
    } else if !liveness_error.is_empty() {
        0xFFFF00u32 // yellow for liveness error
    } else if liveness > 0.5 {
        0x00FF00u32 // green for live
    } else if liveness > 0.0 {
        0xFF0000u32 // red for spoof
    } else {
        0x00FF00u32 // green default (liveness not yet determined)
    };

    let thickness = 3usize;

    // Draw top and bottom edges
    for x in x1..=x2_clamped {
        for t in 0..thickness {
            let yt = y1 + t;
            if yt < height {
                buffer[yt * width + x] = color;
            }
            let yb = y2_clamped.wrapping_sub(t);
            if yb < height && yb >= y1 {
                buffer[yb * width + x] = color;
            }
        }
    }

    // Draw left and right edges
    for y in y1..=y2_clamped {
        for t in 0..thickness {
            let xl = x1 + t;
            if xl < width {
                buffer[y * width + xl] = color;
            }
            let xr = x2_clamped.wrapping_sub(t);
            if xr < width && xr >= x1 {
                buffer[y * width + xr] = color;
            }
        }
    }

    // Label above: ID and name
    let label = if name.is_empty() {
        format!("ID:{}", id)
    } else {
        format!("ID:{} {}", id, name)
    };
    draw_text_simple(buffer, width, x1, y1.saturating_sub(25), &label);

    // Label below: liveness status
    let liveness_text = if !liveness_error.is_empty() {
        liveness_error.to_string()
    } else if liveness > 0.5 {
        format!("Live: {:.0}%", liveness * 100.0)
    } else if liveness > 0.0 {
        format!("Spoof: {:.0}%", (1.0 - liveness) * 100.0)
    } else {
        String::new()
    };
    if !liveness_text.is_empty() {
        draw_text_simple(buffer, width, x1, y2_clamped + 5, &liveness_text);
    }
}

/// Render text onto an ARGB buffer using the TrueType font (white with black shadow).
fn draw_text_simple(buffer: &mut [u32], width: usize, x: usize, y: usize, text: &str) {
    let font = FONT.get().expect("Font not initialized — call init_font() first");
    let buf_height = buffer.len() / width;
    let px = 20.0f32;

    let ascent = font.horizontal_line_metrics(px)
        .map(|m| m.ascent as i32)
        .unwrap_or(px as i32);

    // Shadow pass (offset +1,+1, black)
    draw_text_pass(buffer, width, buf_height, x as i32 + 1, y as i32 + 1, text, font, px, ascent, 0x000000);
    // Foreground pass (white)
    draw_text_pass(buffer, width, buf_height, x as i32, y as i32, text, font, px, ascent, 0xFFFFFF);
}

fn draw_text_pass(
    buffer: &mut [u32], buf_width: usize, buf_height: usize,
    x: i32, y: i32, text: &str,
    font: &fontdue::Font, px: f32, ascent: i32, color: u32,
) {
    let cr = (color >> 16) & 0xFF;
    let cg = (color >> 8) & 0xFF;
    let cb = color & 0xFF;

    let mut cursor_x = x as f32;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, px);

        let gx = cursor_x as i32 + metrics.xmin;
        let gy = y + ascent - metrics.ymin - metrics.height as i32;

        for row in 0..metrics.height {
            let py = gy + row as i32;
            if py < 0 || py >= buf_height as i32 { continue; }
            for col in 0..metrics.width {
                let alpha = bitmap[row * metrics.width + col] as u32;
                if alpha == 0 { continue; }
                let px_x = gx + col as i32;
                if px_x < 0 || px_x >= buf_width as i32 { continue; }
                let idx = py as usize * buf_width + px_x as usize;
                if alpha == 255 {
                    buffer[idx] = color;
                } else {
                    let bg = buffer[idx];
                    let bg_r = (bg >> 16) & 0xFF;
                    let bg_g = (bg >> 8) & 0xFF;
                    let bg_b = bg & 0xFF;
                    let inv = 255 - alpha;
                    let out_r = (cr * alpha + bg_r * inv) / 255;
                    let out_g = (cg * alpha + bg_g * inv) / 255;
                    let out_b = (cb * alpha + bg_b * inv) / 255;
                    buffer[idx] = (out_r << 16) | (out_g << 8) | out_b;
                }
            }
        }

        cursor_x += metrics.advance_width;
    }
}

/// Scale ARGB buffer from (sw, sh) into (dw, dh) with aspect ratio preserved.
/// Letterboxes/pillarboxes with black. Returns (offset_x, offset_y, rendered_w, rendered_h)
/// for mouse coordinate mapping.
fn scale_buffer_proportional(
    src: &[u32], sw: usize, sh: usize,
    dw: usize, dh: usize, dst: &mut Vec<u32>,
) -> (usize, usize, usize, usize) {
    dst.clear();
    if dw == 0 || dh == 0 || sw == 0 || sh == 0 {
        return (0, 0, 0, 0);
    }
    dst.resize(dw * dh, 0);

    let scale_x = dw as f64 / sw as f64;
    let scale_y = dh as f64 / sh as f64;
    let scale = scale_x.min(scale_y);
    let rw = (sw as f64 * scale) as usize;
    let rh = (sh as f64 * scale) as usize;
    let ox = (dw - rw) / 2;
    let oy = (dh - rh) / 2;

    for dy in 0..rh {
        let sy = dy * sh / rh;
        let dst_row = (oy + dy) * dw + ox;
        let src_row = sy * sw;
        for dx in 0..rw {
            let sx = dx * sw / rw;
            dst[dst_row + dx] = src[src_row + sx];
        }
    }
    (ox, oy, rw, rh)
}

/// Decode a raw camera frame for both tracking and display.
/// For MJPEG input, tries FSDK JPEG decode first (using the detected SOI..EOI payload)
/// and returns that `Image` directly to avoid an extra Image->buffer->Image roundtrip.
/// If MJPEG decode fails, or for non-MJPEG input, falls back to nokhwa conversion and
/// builds an `Image` from the decoded RGB buffer.
fn decode_frame_to_fsdk_image(
    format: FrameFormat,
    raw: &[u8],
    width: usize,
    height: usize,
    rgb: &mut Vec<u8>,
) -> fsdk::Result<Image> {

    if format == FrameFormat::MJPEG {
        // Linux/V4L2 MJPEG frames may include non-image bytes around payload.
        // First, try robust JPEG decode via FSDK on [SOI..EOI].
        let jpeg_start = raw.windows(2).position(|w| w == [0xFF, 0xD8]).unwrap_or(0);
        let jpeg_end = raw.windows(2)
            .rposition(|w| w == [0xFF, 0xD9])
            .map(|p| p + 2)
            .unwrap_or(raw.len());

        if jpeg_start < jpeg_end {
            if let Ok(img) = Image::from_jpeg_buffer(&raw[jpeg_start..jpeg_end]) {
                if let Ok(mut decoded) = img.to_buffer(FSDK_IMAGE_COLOR_24BIT) {
                    // Keep expected shape for downstream indexing.
                    decoded.resize(width * height * 3, 0);
                    *rgb = decoded;
                    return Ok(img);
                }
            }
        }
    }

    let res = nokhwa::utils::Resolution::new(width as u32, height as u32);
    let buf = nokhwa::Buffer::new(res, raw, format);

    match buf.decode_image::<RgbFormat>() {
        Ok(decoded) => {
            *rgb = decoded.into_raw();
        }
        Err(_) => {
            // Keep expected buffer shape to avoid downstream out-of-bounds indexing.
            rgb.clear();
            rgb.resize(width * height * 3, 0);
        }
    }

    let scan_line = (width * 3) as i32;
    Image::from_buffer(rgb, width as i32, height as i32, scan_line, FSDK_IMAGE_COLOR_24BIT)
}


/// FPS counter
struct FpsCounter {
    last_time: Instant,
    frame_count: u32,
    fps: f64,
}

impl FpsCounter {
    fn new() -> Self {
        FpsCounter {
            last_time: Instant::now(),
            frame_count: 0,
            fps: 0.0,
        }
    }

    fn tick(&mut self) -> f64 {
        self.frame_count += 1;
        let elapsed = self.last_time.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.fps = self.frame_count as f64 / elapsed;
            self.frame_count = 0;
            self.last_time = Instant::now();
        }
        self.fps
    }
}

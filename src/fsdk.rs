//!
//! Luxand FaceSDK Library - Rust Wrapper
//!
//! Copyright(c) 2026 Luxand, Inc.
//!         http://www.luxand.com
//!
//! Safe wrapper types and functions for FaceSDK
//!

pub mod consts;
#[path = "fsdk_bindings.rs"]
mod ffi;

use std::ffi::c_int;
use std::path::Path;
use std::sync::Mutex;

use thiserror::Error;

pub use consts::*;

// ============================================================================
// FFI types
// ============================================================================

pub type HImage = c_int;
pub type HTracker = c_int;
pub type HCamera = c_int;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: c_int,
    pub y: c_int,
}

/// A point with floating-point coordinates (used in facial features)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct BBox {
    pub p0: Point,
    pub p1: Point,
}

/// Detected face (TFace)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Face {
    /// Detector confidence
    pub score: f32,
    /// In-plane rotation angle in degrees
    pub angle: f32,
    /// Bounding box with top-left (p0) and bottom-right (p1) corners
    pub bbox: BBox,
    /// 5 key points: eye centers, nose tip, mouth corners
    pub features: [Point; 5],
}

impl Face {
    /// Returns the bounding rectangle (x1, y1, x2, y2) for the face
    pub fn rect(&self) -> (i32, i32, i32, i32) {
        (self.bbox.p0.x, self.bbox.p0.y, self.bbox.p1.x, self.bbox.p1.y)
    }

    pub fn width(&self) -> i32 {
        self.bbox.p1.x - self.bbox.p0.x
    }

    pub fn height(&self) -> i32 {
        self.bbox.p1.y - self.bbox.p0.y
    }

    pub fn center(&self) -> Point {
        Point {
            x: (self.bbox.p0.x + self.bbox.p1.x) / 2,
            y: (self.bbox.p0.y + self.bbox.p1.y) / 2,
        }
    }
}

/// Array of 70 facial feature points
pub type Features = [PointF; FSDK_FACIAL_FEATURE_COUNT];

/// Result of matching a face template against the tracker memory
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct IDSimilarity {
    pub id: i64,
    pub similarity: f32,
}

/// Face template for matching
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FaceTemplate {
    pub data: [u8; FSDK_FACE_TEMPLATE_SIZE],
}

impl Default for FaceTemplate {
    fn default() -> Self {
        FaceTemplate {
            data: [0u8; FSDK_FACE_TEMPLATE_SIZE],
        }
    }
}

impl std::fmt::Debug for FaceTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FaceTemplate")
            .field("size", &self.data.len())
            .finish()
    }
}

// ============================================================================
// Error types
// ============================================================================

#[derive(Error, Debug)]
pub enum FsdkError {
    #[error("FSDK library loading error: {0}")]
    LibraryLoadError(String),
    #[error("FSDK function {func} failed with error {code} ({name})")]
    FsdkError {
        func: &'static str,
        code: i32,
        name: &'static str,
    },
    #[error("Invalid string: {0}")]
    InvalidString(String),
}

impl FsdkError {
    pub fn from_code(func: &'static str, code: i32) -> Self {
        FsdkError::FsdkError {
            func,
            code,
            name: error_name(code),
        }
    }

    /// Returns true if this error indicates face was not found
    pub fn is_face_not_found(&self) -> bool {
        matches!(self, FsdkError::FsdkError { code, .. } if *code == FSDKE_FACE_NOT_FOUND)
    }
}

pub type Result<T> = std::result::Result<T, FsdkError>;

// ============================================================================
// FSDK main interface
// ============================================================================
pub struct FSDK;

impl FSDK {
    /// Activates the FaceSDK library with the given license key
    pub fn activate_library(license_key: &str) -> Result<()> {
        ffi::activate_library(license_key)
    }

    /// Returns the hardware ID for license activation
    pub fn get_hardware_id() -> Result<String> {
        ffi::get_hardware_id()
    }

    /// Initializes the FaceSDK library
    pub fn initialize() -> Result<()> {
        ffi::initialize()
    }

    /// Finalizes the FaceSDK library
    pub fn finalize() -> Result<()> {
        ffi::finalize()
    }

    /// Returns license information
    pub fn get_license_info() -> Result<String> {
        ffi::get_license_info()
    }

    /// Returns library version information
    pub fn get_version_info() -> Result<String> {
        ffi::get_version_info()
    }

    /// Gets the number of threads used by FaceSDK
    pub fn get_num_threads() -> Result<i32> {
        ffi::get_num_threads()
    }

    /// Sets the number of threads used by FaceSDK
    pub fn set_num_threads(num: i32) -> Result<()> {
        ffi::set_num_threads(num)
    }

    /// Sets multiple parameters at once (format: "param1=value1;param2=value2")
    pub fn set_parameters(params: &str) -> Result<()> {
        ffi::set_parameters(params)
    }

    /// Sets a single parameter
    pub fn set_parameter(name: &str, value: &str) -> Result<()> {
        ffi::set_parameter(name, value)
    }

    /// Sets JPEG compression quality (0-100)
    pub fn set_jpeg_compression_quality(quality: i32) -> Result<()> {
        ffi::set_jpeg_compression_quality(quality)
    }

    /// Matches two face templates and returns similarity (0.0-1.0)
    pub fn match_faces(template1: &FaceTemplate, template2: &FaceTemplate) -> Result<f32> {
        ffi::match_faces(template1, template2)
    }

    /// Parses attribute string and returns confidence for a specific value
    pub fn get_value_confidence(attribute_values: &str, value: &str) -> Result<f32> {
        ffi::get_value_confidence(attribute_values, value)
    }

    /// Initializes video capturing subsystem
    pub fn initialize_capturing() -> Result<()> {
        ffi::initialize_capturing()
    }

    /// Finalizes video capturing subsystem
    pub fn finalize_capturing() -> Result<()> {
        ffi::finalize_capturing()
    }

    /// Sets HTTP proxy for IP cameras
    pub fn set_http_proxy(server: &str, port: u16, username: &str, password: &str) -> Result<()> {
        ffi::set_http_proxy(server, port, username, password)
    }
}

// ============================================================================
// Image wrapper
// ============================================================================

/// Safe wrapper for FSDK Image handle
pub struct Image {
    handle: HImage,
}

impl Image {
    /// Creates a new empty image
    pub fn new() -> Result<Self> {
        let handle = ffi::create_empty_image()?;
        Ok(Image { handle })
    }

    /// Creates an Image from a raw handle (takes ownership)
    pub(crate) fn from_handle(handle: HImage) -> Self {
        Image { handle }
    }

    /// Returns the raw handle
    pub fn handle(&self) -> HImage {
        self.handle
    }

    /// Loads an image from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| FsdkError::InvalidString("Invalid path".to_string()))?;
        let handle = ffi::load_image_from_file(path_str)?;
        Ok(Image { handle })
    }

    /// Loads an image from raw pixel buffer
    pub fn from_buffer(buffer: &[u8], width: i32, height: i32, scan_line: i32, image_mode: i32) -> Result<Self> {
        let handle = ffi::load_image_from_buffer(buffer, width, height, scan_line, image_mode)?;
        Ok(Image { handle })
    }

    /// Loads an image from JPEG buffer
    pub fn from_jpeg_buffer(buffer: &[u8]) -> Result<Self> {
        let handle = ffi::load_image_from_jpeg_buffer(buffer)?;
        Ok(Image { handle })
    }

    /// Loads an image from PNG buffer
    pub fn from_png_buffer(buffer: &[u8]) -> Result<Self> {
        let handle = ffi::load_image_from_png_buffer(buffer)?;
        Ok(Image { handle })
    }

    /// Saves the image to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| FsdkError::InvalidString("Invalid path".to_string()))?;
        ffi::save_image_to_file(self.handle, path_str)
    }

    /// Saves the image to a file with specified JPEG quality.
    /// Uses a lock to prevent concurrent threads from changing the global quality
    /// between the set_jpeg_compression_quality and save_to_file calls.
    pub fn save_to_file_with_quality<P: AsRef<Path>>(&self, path: P, quality: i32) -> Result<()> {
        static JPEG_QUALITY_LOCK: Mutex<()> = Mutex::new(());
        let _guard = JPEG_QUALITY_LOCK.lock().unwrap();
        FSDK::set_jpeg_compression_quality(quality)?;
        self.save_to_file(path)
    }

    /// Gets the buffer size needed to save image in given mode
    pub fn get_buffer_size(&self, image_mode: i32) -> Result<i32> {
        ffi::get_image_buffer_size(self.handle, image_mode)
    }

    /// Saves image to a buffer
    pub fn to_buffer(&self, image_mode: i32) -> Result<Vec<u8>> {
        let size = self.get_buffer_size(image_mode)?;
        let mut buffer = vec![0u8; size as usize];
        ffi::save_image_to_buffer(self.handle, &mut buffer, image_mode)?;
        Ok(buffer)
    }

    /// Returns the width of the image
    pub fn width(&self) -> Result<i32> {
        ffi::get_image_width(self.handle)
    }

    /// Returns the height of the image
    pub fn height(&self) -> Result<i32> {
        ffi::get_image_height(self.handle)
    }

    /// Returns the size (width, height) of the image
    pub fn size(&self) -> Result<(i32, i32)> {
        Ok((self.width()?, self.height()?))
    }

    /// Creates a copy of the image
    pub fn copy(&self) -> Result<Image> {
        let dest = Image::new()?;
        ffi::copy_image(self.handle, dest.handle)?;
        Ok(dest)
    }

    /// Resizes the image by the given ratio
    pub fn resize(&self, ratio: f64) -> Result<Image> {
        let dest = Image::new()?;
        ffi::resize_image(self.handle, ratio, dest.handle)?;
        Ok(dest)
    }

    /// Resizes the image by separate X and Y ratios
    pub fn resize_xy(&self, ratio_x: f64, ratio_y: f64) -> Result<Image> {
        let dest = Image::new()?;
        ffi::resize_image_xy(self.handle, ratio_x, ratio_y, dest.handle)?;
        Ok(dest)
    }

    /// Rotates the image by the given angle (degrees)
    pub fn rotate(&self, angle: f64) -> Result<Image> {
        let dest = Image::new()?;
        ffi::rotate_image(self.handle, angle, dest.handle)?;
        Ok(dest)
    }

    /// Rotates the image by 90 degrees * multiplier
    pub fn rotate_90(&self, multiplier: i32) -> Result<Image> {
        let dest = Image::new()?;
        ffi::rotate_image_90(self.handle, multiplier, dest.handle)?;
        Ok(dest)
    }

    /// Rotates the image around a center point
    pub fn rotate_center(&self, angle: f64, x_center: f64, y_center: f64) -> Result<Image> {
        let dest = Image::new()?;
        ffi::rotate_image_center(self.handle, angle, x_center, y_center, dest.handle)?;
        Ok(dest)
    }

    /// Copies a rectangular region of the image
    pub fn crop(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<Image> {
        let dest = Image::new()?;
        ffi::copy_rect(self.handle, x1, y1, x2, y2, dest.handle)?;
        Ok(dest)
    }

    /// Copies a rectangular region with replicated border
    pub fn crop_replicate_border(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<Image> {
        let dest = Image::new()?;
        ffi::copy_rect_replicate_border(self.handle, x1, y1, x2, y2, dest.handle)?;
        Ok(dest)
    }

    /// Mirrors the image (horizontal by default, vertical if use_vertical is true)
    pub fn mirror(&mut self, use_vertical: bool) -> Result<()> {
        ffi::mirror_image(self.handle, use_vertical)
    }

    /// Detects the face with the highest confidence in the image
    pub fn detect_face(&self) -> Result<Face> {
        ffi::detect_face(self.handle)
    }

    /// Detects all faces in the image, sorted by confidence in descending order
    pub fn detect_multiple_faces(&self, max_faces: usize) -> Result<Vec<Face>> {
        ffi::detect_multiple_faces(self.handle, max_faces)
    }

    /// Detects a face and its 70 facial features
    pub fn detect_facial_features(&self) -> Result<Features> {
        ffi::detect_facial_features(self.handle)
    }

    /// Detects 70 facial features of a face found by `detect_face` / `detect_multiple_faces`
    pub fn detect_facial_features_in_region(&self, face: &Face) -> Result<Features> {
        ffi::detect_facial_features_in_region(self.handle, face)
    }

    /// Extracts a normalized face image of the given size.
    /// Returns the face image and the facial features in its coordinates.
    pub fn extract_face_image(&self, features: &Features, width: i32, height: i32) -> Result<(Image, Features)> {
        let (handle, resized) = ffi::extract_face_image(self.handle, features, width, height)?;
        Ok((Image::from_handle(handle), resized))
    }

    /// Extracts face template from the image (detects the face automatically)
    pub fn get_face_template(&self) -> Result<FaceTemplate> {
        ffi::get_face_template(self.handle)
    }

    /// Extracts face template for a face found by `detect_face` / `detect_multiple_faces`
    pub fn get_face_template_in_region(&self, face: &Face) -> Result<FaceTemplate> {
        ffi::get_face_template_in_region(self.handle, face)
    }

    /// Detects facial attributes using 70 facial features
    /// Returns string like "Male=0.95;Female=0.05" for Gender attribute
    pub fn detect_facial_attribute(&self, features: &Features, attribute_name: &str) -> Result<String> {
        ffi::detect_facial_attribute_using_features(self.handle, features, attribute_name)
    }

    /// Detects facial attributes using a face found by `detect_face` / `detect_multiple_faces`
    pub fn detect_facial_attribute_using_face(&self, face: &Face, attribute_name: &str) -> Result<String> {
        ffi::detect_facial_attribute_using_face(self.handle, face, attribute_name)
    }
}

impl Default for Image {
    fn default() -> Self {
        Image { handle: -1 }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if self.handle != -1 {
            ffi::free_image(self.handle);
            self.handle = -1;
        }
    }
}

// ============================================================================
// Camera wrapper
// ============================================================================

/// Safe wrapper for FSDK Camera handle
pub struct Camera {
    handle: HCamera,
}

impl Camera {
    /// Opens an IP video camera
    pub fn open_ip(compression: i32, url: &str, username: &str, password: &str, timeout_seconds: i32) -> Result<Self> {
        let handle = ffi::open_ip_video_camera(compression, url, username, password, timeout_seconds)?;
        Ok(Camera { handle })
    }

    /// Grabs a frame from the camera
    pub fn grab_frame(&self) -> Result<Image> {
        let handle = ffi::grab_frame(self.handle)?;
        Ok(Image::from_handle(handle))
    }

    /// Closes the camera
    pub fn close(&mut self) -> Result<()> {
        let h = std::mem::replace(&mut self.handle, -1);
        if h != -1 {
            ffi::close_video_camera(h)?;
        }
        Ok(())
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        if self.handle != -1 {
            let _ = ffi::close_video_camera(self.handle);
            self.handle = -1;
        }
    }
}

// ============================================================================
// Tracker wrapper
// ============================================================================

/// Safe wrapper for FSDK Tracker handle
pub struct Tracker {
    handle: HTracker,
}

impl Tracker {
    /// Creates a new tracker
    pub fn new() -> Result<Self> {
        let handle = ffi::create_tracker()?;
        Ok(Tracker { handle })
    }

    /// Loads tracker from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| FsdkError::InvalidString("Invalid path".to_string()))?;
        let handle = ffi::load_tracker_memory_from_file(path_str)?;
        Ok(Tracker { handle })
    }

    /// Loads tracker from buffer
    pub fn from_buffer(buffer: &[u8]) -> Result<Self> {
        let handle = ffi::load_tracker_memory_from_buffer(buffer)?;
        Ok(Tracker { handle })
    }

    /// Clears all tracker memory
    pub fn clear(&mut self) -> Result<()> {
        ffi::clear_tracker(self.handle)
    }

    /// Sets a tracker parameter
    pub fn set_parameter(&self, name: &str, value: &str) -> Result<()> {
        ffi::set_tracker_parameter(self.handle, name, value)
    }

    /// Sets multiple tracker parameters
    pub fn set_parameters(&self, params: &str) -> Result<()> {
        ffi::set_tracker_multiple_parameters(self.handle, params)
    }

    /// Gets a tracker parameter value
    pub fn get_parameter(&self, name: &str) -> Result<String> {
        ffi::get_tracker_parameter(self.handle, name)
    }

    /// Processes a video frame and returns face IDs
    pub fn feed_frame(&self, camera_idx: i64, image: &Image, max_ids: usize) -> Result<Vec<i64>> {
        ffi::feed_frame(self.handle, camera_idx, image.handle, max_ids)
    }

    /// Gets eye positions for a tracked face (the first two points of Features)
    pub fn get_eyes(&self, camera_idx: i64, id: i64) -> Result<Features> {
        ffi::get_tracker_eyes(self.handle, camera_idx, id)
    }

    /// Gets facial features for a tracked face
    pub fn get_facial_features(&self, camera_idx: i64, id: i64) -> Result<Features> {
        ffi::get_tracker_facial_features(self.handle, camera_idx, id)
    }

    /// Gets face (bounding box, score, angle and 5 key points) for a tracked face
    pub fn get_face(&self, camera_idx: i64, id: i64) -> Result<Face> {
        ffi::get_tracker_face(self.handle, camera_idx, id)
    }

    /// Gets facial attribute for a tracked face
    pub fn get_facial_attribute(&self, camera_idx: i64, id: i64, attribute_name: &str) -> Result<String> {
        ffi::get_tracker_facial_attribute(self.handle, camera_idx, id, attribute_name)
    }

    /// Locks an ID to prevent purging
    pub fn lock_id(&self, id: i64) -> Result<()> {
        ffi::lock_id(self.handle, id)
    }

    /// Unlocks an ID
    pub fn unlock_id(&self, id: i64) -> Result<()> {
        ffi::unlock_id(self.handle, id)
    }

    /// Purges all data for an ID
    pub fn purge_id(&self, id: i64) -> Result<()> {
        ffi::purge_id(self.handle, id)
    }

    /// Sets the name for an ID
    pub fn set_name(&self, id: i64, name: &str) -> Result<()> {
        ffi::set_name(self.handle, id, name)
    }

    /// Gets the name for an ID
    pub fn get_name(&self, id: i64) -> Result<String> {
        ffi::get_name(self.handle, id)
    }

    /// Gets all names for an ID and similar IDs
    pub fn get_all_names(&self, id: i64) -> Result<Vec<String>> {
        ffi::get_all_names(self.handle, id)
    }

    /// Gets reassigned ID after merger
    pub fn get_id_reassignment(&self, id: i64) -> Result<i64> {
        ffi::get_id_reassignment(self.handle, id)
    }

    /// Gets count of similar IDs
    pub fn get_similar_id_count(&self, id: i64) -> Result<i64> {
        ffi::get_similar_id_count(self.handle, id)
    }

    /// Gets list of similar IDs
    pub fn get_similar_id_list(&self, id: i64) -> Result<Vec<i64>> {
        ffi::get_similar_id_list(self.handle, id)
    }

    /// Saves tracker memory to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| FsdkError::InvalidString("Invalid path".to_string()))?;
        ffi::save_tracker_memory_to_file(self.handle, path_str)
    }

    /// Gets buffer size needed to save tracker memory
    pub fn get_buffer_size(&self) -> Result<i64> {
        ffi::get_tracker_memory_buffer_size(self.handle)
    }

    /// Saves tracker memory to buffer
    pub fn to_buffer(&self) -> Result<Vec<u8>> {
        let size = self.get_buffer_size()?;
        let mut buffer = vec![0u8; size as usize];
        ffi::save_tracker_memory_to_buffer(self.handle, &mut buffer)?;
        Ok(buffer)
    }

    /// Gets count of all known IDs
    pub fn get_ids_count(&self) -> Result<i64> {
        ffi::get_tracker_ids_count(self.handle)
    }

    /// Gets all known IDs
    pub fn get_all_ids(&self) -> Result<Vec<i64>> {
        ffi::get_tracker_all_ids(self.handle)
    }

    /// Gets face IDs (stored face templates) of an ID
    pub fn get_face_ids_for_id(&self, id: i64) -> Result<Vec<i64>> {
        ffi::get_tracker_face_ids_for_id(self.handle, id)
    }

    /// Gets the ID a face ID belongs to
    pub fn get_id_by_face_id(&self, face_id: i64) -> Result<i64> {
        ffi::get_tracker_id_by_face_id(self.handle, face_id)
    }

    /// Gets the face template stored under a face ID
    pub fn get_face_template(&self, face_id: i64) -> Result<FaceTemplate> {
        ffi::get_tracker_face_template(self.handle, face_id)
    }

    /// Gets the face image stored under a face ID
    pub fn get_face_image(&self, face_id: i64) -> Result<Image> {
        Ok(Image::from_handle(ffi::get_tracker_face_image(self.handle, face_id)?))
    }

    /// Stores a face image under a face ID (the image must be 112x112 pixels)
    pub fn set_face_image(&self, face_id: i64, image: &Image) -> Result<()> {
        ffi::set_tracker_face_image(self.handle, face_id, image.handle)
    }

    /// Deletes the face image stored under a face ID
    pub fn delete_face_image(&self, face_id: i64) -> Result<()> {
        ffi::delete_tracker_face_image(self.handle, face_id)
    }

    /// Deletes a face (template and image) from the tracker memory
    pub fn delete_face(&self, face_id: i64) -> Result<()> {
        ffi::delete_tracker_face(self.handle, face_id)
    }

    /// Creates a new ID from a face template. Returns (ID, face ID)
    pub fn create_id(&self, template: &FaceTemplate) -> Result<(i64, i64)> {
        ffi::tracker_create_id(self.handle, template)
    }

    /// Adds a face template to an existing ID. Returns the new face ID
    pub fn add_face_template(&self, id: i64, template: &FaceTemplate) -> Result<i64> {
        ffi::add_tracker_face_template(self.handle, id, template)
    }

    /// Matches a face template against the tracker memory.
    /// Returns up to `max_count` IDs with similarity above `threshold`, best match first
    pub fn match_faces(&self, template: &FaceTemplate, threshold: f32, max_count: usize) -> Result<Vec<IDSimilarity>> {
        ffi::tracker_match_faces(self.handle, template, threshold, max_count)
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        if self.handle != -1 {
            ffi::free_tracker(self.handle);
            self.handle = -1;
        }
    }
}

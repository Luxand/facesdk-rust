//! Low-level FFI bindings for FSDK
//!
//! This module contains all unsafe code for interacting with the FSDK C library.
//! All unsafe operations are encapsulated here to provide safe wrappers in the main module.

use std::ffi::{c_char, c_double, c_float, c_int, c_longlong, c_uchar, c_uint, c_ushort, c_void, CStr, CString};
use std::mem::ManuallyDrop;
use std::sync::OnceLock;

use libloading::{Library, Symbol};
#[cfg(target_os = "windows")]
use libloading::os::windows::Library as WinLibrary;

use crate::consts::*;
use crate::{Eyes, Face, FacePosition, FaceTemplate, Features, FsdkError, HCamera, HImage, HTracker, Point, Result};

// Function pointer types - Initialization
type FnActivateLibrary = unsafe extern "C" fn(*const c_char) -> c_int;
type FnGetHardwareID = unsafe extern "C" fn(*mut c_char) -> c_int;
type FnGetLicenseInfo = unsafe extern "C" fn(*mut c_char) -> c_int;
type FnInitialize = unsafe extern "C" fn(*const c_char) -> c_int;
type FnFinalize = unsafe extern "C" fn() -> c_int;
type FnGetNumThreads = unsafe extern "C" fn(*mut c_int) -> c_int;
type FnSetNumThreads = unsafe extern "C" fn(c_int) -> c_int;
type FnSetParameters = unsafe extern "C" fn(*const c_char, *mut c_int) -> c_int;
type FnSetParameter = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;

// Function pointer types - Image
type FnCreateEmptyImage = unsafe extern "C" fn(*mut HImage) -> c_int;
type FnFreeImage = unsafe extern "C" fn(HImage) -> c_int;
type FnLoadImageFromFile = unsafe extern "C" fn(*mut HImage, *const c_char) -> c_int;
type FnSaveImageToFile = unsafe extern "C" fn(HImage, *const c_char) -> c_int;
type FnLoadImageFromBuffer = unsafe extern "C" fn(*mut HImage, *const c_uchar, c_int, c_int, c_int, c_int) -> c_int;
type FnLoadImageFromJpegBuffer = unsafe extern "C" fn(*mut HImage, *const c_uchar, c_uint) -> c_int;
type FnLoadImageFromPngBuffer = unsafe extern "C" fn(*mut HImage, *const c_uchar, c_uint) -> c_int;
type FnGetImageBufferSize = unsafe extern "C" fn(HImage, *mut c_int, c_int) -> c_int;
type FnSaveImageToBuffer = unsafe extern "C" fn(HImage, *mut c_uchar, c_int) -> c_int;
type FnGetImageWidth = unsafe extern "C" fn(HImage, *mut c_int) -> c_int;
type FnGetImageHeight = unsafe extern "C" fn(HImage, *mut c_int) -> c_int;
type FnCopyImage = unsafe extern "C" fn(HImage, HImage) -> c_int;
type FnResizeImage = unsafe extern "C" fn(HImage, c_double, HImage) -> c_int;
type FnResizeImageXY = unsafe extern "C" fn(HImage, c_double, c_double, HImage) -> c_int;
type FnRotateImage = unsafe extern "C" fn(HImage, c_double, HImage) -> c_int;
type FnRotateImage90 = unsafe extern "C" fn(HImage, c_int, HImage) -> c_int;
type FnRotateImageCenter = unsafe extern "C" fn(HImage, c_double, c_double, c_double, HImage) -> c_int;
type FnCopyRect = unsafe extern "C" fn(HImage, c_int, c_int, c_int, c_int, HImage) -> c_int;
type FnCopyRectReplicateBorder = unsafe extern "C" fn(HImage, c_int, c_int, c_int, c_int, HImage) -> c_int;
type FnMirrorImage = unsafe extern "C" fn(HImage, c_int) -> c_int;
type FnSetJpegCompressionQuality = unsafe extern "C" fn(c_int) -> c_int;

// Function pointer types - Face Detection
type FnSetFaceDetectionParameters = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type FnSetFaceDetectionThreshold = unsafe extern "C" fn(c_int) -> c_int;
type FnGetDetectedFaceConfidence = unsafe extern "C" fn(*mut c_int) -> c_int;
type FnDetectFace = unsafe extern "C" fn(HImage, *mut FacePosition) -> c_int;
type FnDetectMultipleFaces = unsafe extern "C" fn(HImage, *mut c_int, *mut FacePosition, c_int) -> c_int;
type FnDetectEyes = unsafe extern "C" fn(HImage, *mut Eyes) -> c_int;
type FnDetectEyesInRegion = unsafe extern "C" fn(HImage, *const FacePosition, *mut Eyes) -> c_int;

// Function pointer types - Face Detection v2
type FnDetectFace2 = unsafe extern "C" fn(HImage, *mut Face) -> c_int;
type FnDetectMultipleFaces2 = unsafe extern "C" fn(HImage, *mut c_int, *mut Face, c_int) -> c_int;
type FnDetectFaceAndFeatures2 = unsafe extern "C" fn(HImage, *mut Face, *mut Features) -> c_int;
type FnDetectMultipleFacesAndFeatures2 = unsafe extern "C" fn(HImage, *mut c_int, *mut Face, *mut Features, c_int) -> c_int;

// Function pointer types - Facial Features
type FnDetectFacialFeatures = unsafe extern "C" fn(HImage, *mut Features) -> c_int;
type FnDetectFacialFeaturesInRegion = unsafe extern "C" fn(HImage, *const FacePosition, *mut Features) -> c_int;
type FnDetectFacialFeaturesEx = unsafe extern "C" fn(HImage, *mut Features, *mut c_float) -> c_int;
type FnDetectFacialFeaturesInRegionEx = unsafe extern "C" fn(HImage, *const FacePosition, *mut Features, *mut c_float) -> c_int;

// Function pointer types - Templates and Matching
type FnGetFaceTemplate = unsafe extern "C" fn(HImage, *mut FaceTemplate) -> c_int;
type FnGetFaceTemplateInRegion = unsafe extern "C" fn(HImage, *const FacePosition, *mut FaceTemplate) -> c_int;
type FnGetFaceTemplateUsingFeatures = unsafe extern "C" fn(HImage, *const Features, *mut FaceTemplate) -> c_int;
type FnGetFaceTemplateUsingEyes = unsafe extern "C" fn(HImage, *const Eyes, *mut FaceTemplate) -> c_int;
type FnMatchFaces = unsafe extern "C" fn(*const FaceTemplate, *const FaceTemplate, *mut c_float) -> c_int;
type FnGetMatchingThresholdAtFAR = unsafe extern "C" fn(c_float, *mut c_float) -> c_int;
type FnGetMatchingThresholdAtFRR = unsafe extern "C" fn(c_float, *mut c_float) -> c_int;

// Function pointer types - Templates v2
type FnGetFaceTemplate2 = unsafe extern "C" fn(HImage, *mut FaceTemplate) -> c_int;
type FnGetFaceTemplateInRegion2 = unsafe extern "C" fn(HImage, *const Face, *mut FaceTemplate) -> c_int;

// Function pointer types - Facial Attributes
type FnDetectFacialAttributeUsingFeatures = unsafe extern "C" fn(HImage, *const Features, *const c_char, *mut c_char, c_longlong) -> c_int;
type FnGetValueConfidence = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_float) -> c_int;

// Function pointer types - Camera
type FnInitializeCapturing = unsafe extern "C" fn() -> c_int;
type FnFinalizeCapturing = unsafe extern "C" fn() -> c_int;
type FnOpenIPVideoCamera = unsafe extern "C" fn(c_int, *const c_char, *const c_char, *const c_char, c_int, *mut HCamera) -> c_int;
type FnCloseVideoCamera = unsafe extern "C" fn(HCamera) -> c_int;
type FnGrabFrame = unsafe extern "C" fn(HCamera, *mut HImage) -> c_int;
type FnSetHTTPProxy = unsafe extern "C" fn(*const c_char, c_ushort, *const c_char, *const c_char) -> c_int;

// Function pointer types - Tracker
type FnCreateTracker = unsafe extern "C" fn(*mut HTracker) -> c_int;
type FnFreeTracker = unsafe extern "C" fn(HTracker) -> c_int;
type FnClearTracker = unsafe extern "C" fn(HTracker) -> c_int;
type FnSetTrackerParameter = unsafe extern "C" fn(HTracker, *const c_char, *const c_char) -> c_int;
type FnSetTrackerMultipleParameters = unsafe extern "C" fn(HTracker, *const c_char, *mut c_int) -> c_int;
type FnGetTrackerParameter = unsafe extern "C" fn(HTracker, *const c_char, *mut c_char, c_longlong) -> c_int;
type FnFeedFrame = unsafe extern "C" fn(HTracker, c_longlong, HImage, *mut c_longlong, *mut c_longlong, c_longlong) -> c_int;
type FnGetTrackerEyes = unsafe extern "C" fn(HTracker, c_longlong, c_longlong, *mut Eyes) -> c_int;
type FnGetTrackerFacialFeatures = unsafe extern "C" fn(HTracker, c_longlong, c_longlong, *mut Features) -> c_int;
type FnGetTrackerFacePosition = unsafe extern "C" fn(HTracker, c_longlong, c_longlong, *mut FacePosition) -> c_int;
type FnGetTrackerFacialAttribute = unsafe extern "C" fn(HTracker, c_longlong, c_longlong, *const c_char, *mut c_char, c_longlong) -> c_int;
type FnGetTrackerFace = unsafe extern "C" fn(HTracker, c_longlong, c_longlong, *mut Face) -> c_int;
type FnLockID = unsafe extern "C" fn(HTracker, c_longlong) -> c_int;
type FnUnlockID = unsafe extern "C" fn(HTracker, c_longlong) -> c_int;
type FnPurgeID = unsafe extern "C" fn(HTracker, c_longlong) -> c_int;
type FnSetName = unsafe extern "C" fn(HTracker, c_longlong, *const c_char) -> c_int;
type FnGetName = unsafe extern "C" fn(HTracker, c_longlong, *mut c_char, c_longlong) -> c_int;
type FnGetAllNames = unsafe extern "C" fn(HTracker, c_longlong, *mut c_char, c_longlong) -> c_int;
type FnGetIDReassignment = unsafe extern "C" fn(HTracker, c_longlong, *mut c_longlong) -> c_int;
type FnGetSimilarIDCount = unsafe extern "C" fn(HTracker, c_longlong, *mut c_longlong) -> c_int;
type FnGetSimilarIDList = unsafe extern "C" fn(HTracker, c_longlong, *mut c_longlong, c_longlong) -> c_int;
type FnSaveTrackerMemoryToFile = unsafe extern "C" fn(HTracker, *const c_char) -> c_int;
type FnLoadTrackerMemoryFromFile = unsafe extern "C" fn(*mut HTracker, *const c_char) -> c_int;
type FnGetTrackerMemoryBufferSize = unsafe extern "C" fn(HTracker, *mut c_longlong) -> c_int;
type FnSaveTrackerMemoryToBuffer = unsafe extern "C" fn(HTracker, *mut c_uchar, c_longlong) -> c_int;
type FnLoadTrackerMemoryFromBuffer = unsafe extern "C" fn(*mut HTracker, *const c_uchar) -> c_int;
type FnGetTrackerIDsCount = unsafe extern "C" fn(HTracker, *mut c_longlong) -> c_int;
type FnGetTrackerAllIDs = unsafe extern "C" fn(HTracker, *mut c_longlong, c_longlong) -> c_int;

struct FsdkLibrary {
    // ManuallyDrop prevents FreeLibrary on process exit, avoiding heap corruption
    // from DLL cleanup racing with static destruction.
    _lib: ManuallyDrop<Library>,
    // Initialization
    activate_library: FnActivateLibrary,
    get_hardware_id: FnGetHardwareID,
    get_license_info: FnGetLicenseInfo,
    initialize: FnInitialize,
    finalize: FnFinalize,
    get_num_threads: FnGetNumThreads,
    set_num_threads: FnSetNumThreads,
    set_parameters: FnSetParameters,
    set_parameter: FnSetParameter,
    // Image
    create_empty_image: FnCreateEmptyImage,
    free_image: FnFreeImage,
    load_image_from_file: FnLoadImageFromFile,
    save_image_to_file: FnSaveImageToFile,
    load_image_from_buffer: FnLoadImageFromBuffer,
    load_image_from_jpeg_buffer: FnLoadImageFromJpegBuffer,
    load_image_from_png_buffer: FnLoadImageFromPngBuffer,
    get_image_buffer_size: FnGetImageBufferSize,
    save_image_to_buffer: FnSaveImageToBuffer,
    get_image_width: FnGetImageWidth,
    get_image_height: FnGetImageHeight,
    copy_image: FnCopyImage,
    resize_image: FnResizeImage,
    resize_image_xy: FnResizeImageXY,
    rotate_image: FnRotateImage,
    rotate_image_90: FnRotateImage90,
    rotate_image_center: FnRotateImageCenter,
    copy_rect: FnCopyRect,
    copy_rect_replicate_border: FnCopyRectReplicateBorder,
    mirror_image: FnMirrorImage,
    set_jpeg_compression_quality: FnSetJpegCompressionQuality,
    // Face Detection
    set_face_detection_parameters: FnSetFaceDetectionParameters,
    set_face_detection_threshold: FnSetFaceDetectionThreshold,
    get_detected_face_confidence: FnGetDetectedFaceConfidence,
    detect_face: FnDetectFace,
    detect_multiple_faces: FnDetectMultipleFaces,
    detect_eyes: FnDetectEyes,
    detect_eyes_in_region: FnDetectEyesInRegion,
    // Face Detection v2
    detect_face2: FnDetectFace2,
    detect_multiple_faces2: FnDetectMultipleFaces2,
    detect_face_and_features2: FnDetectFaceAndFeatures2,
    detect_multiple_faces_and_features2: FnDetectMultipleFacesAndFeatures2,
    // Facial Features
    detect_facial_features: FnDetectFacialFeatures,
    detect_facial_features_in_region: FnDetectFacialFeaturesInRegion,
    detect_facial_features_ex: FnDetectFacialFeaturesEx,
    detect_facial_features_in_region_ex: FnDetectFacialFeaturesInRegionEx,
    // Templates and Matching
    get_face_template: FnGetFaceTemplate,
    get_face_template_in_region: FnGetFaceTemplateInRegion,
    get_face_template_using_features: FnGetFaceTemplateUsingFeatures,
    get_face_template_using_eyes: FnGetFaceTemplateUsingEyes,
    match_faces: FnMatchFaces,
    get_matching_threshold_at_far: FnGetMatchingThresholdAtFAR,
    get_matching_threshold_at_frr: FnGetMatchingThresholdAtFRR,
    // Templates v2
    get_face_template2: FnGetFaceTemplate2,
    get_face_template_in_region2: FnGetFaceTemplateInRegion2,
    // Facial Attributes
    detect_facial_attribute_using_features: FnDetectFacialAttributeUsingFeatures,
    get_value_confidence: FnGetValueConfidence,
    // Camera
    initialize_capturing: FnInitializeCapturing,
    finalize_capturing: FnFinalizeCapturing,
    open_ip_video_camera: FnOpenIPVideoCamera,
    close_video_camera: FnCloseVideoCamera,
    grab_frame: FnGrabFrame,
    set_http_proxy: FnSetHTTPProxy,
    // Tracker
    create_tracker: FnCreateTracker,
    free_tracker: FnFreeTracker,
    clear_tracker: FnClearTracker,
    set_tracker_parameter: FnSetTrackerParameter,
    set_tracker_multiple_parameters: FnSetTrackerMultipleParameters,
    get_tracker_parameter: FnGetTrackerParameter,
    feed_frame: FnFeedFrame,
    get_tracker_eyes: FnGetTrackerEyes,
    get_tracker_facial_features: FnGetTrackerFacialFeatures,
    get_tracker_face_position: FnGetTrackerFacePosition,
    get_tracker_face: FnGetTrackerFace,
    get_tracker_facial_attribute: FnGetTrackerFacialAttribute,
    lock_id: FnLockID,
    unlock_id: FnUnlockID,
    purge_id: FnPurgeID,
    set_name: FnSetName,
    get_name: FnGetName,
    get_all_names: FnGetAllNames,
    get_id_reassignment: FnGetIDReassignment,
    get_similar_id_count: FnGetSimilarIDCount,
    get_similar_id_list: FnGetSimilarIDList,
    save_tracker_memory_to_file: FnSaveTrackerMemoryToFile,
    load_tracker_memory_from_file: FnLoadTrackerMemoryFromFile,
    get_tracker_memory_buffer_size: FnGetTrackerMemoryBufferSize,
    save_tracker_memory_to_buffer: FnSaveTrackerMemoryToBuffer,
    load_tracker_memory_from_buffer: FnLoadTrackerMemoryFromBuffer,
    get_tracker_ids_count: FnGetTrackerIDsCount,
    get_tracker_all_ids: FnGetTrackerAllIDs,
}

// SAFETY: The FSDK library functions are thread-safe according to documentation
unsafe impl Send for FsdkLibrary {}
unsafe impl Sync for FsdkLibrary {}

static FSDK_LIB: OnceLock<Result<FsdkLibrary>> = OnceLock::new();

fn get_library_path() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        if cfg!(target_arch = "aarch64") {
            "fsdk/osx_arm64/libfsdk.dylib"
        } else {
            "fsdk/osx_x86_64/libfsdk.dylib"
        }
    }
    #[cfg(target_os = "linux")]
    {
        if cfg!(target_pointer_width = "64") {
            "fsdk/linux64/libfsdk.so"
        } else {
            "fsdk/linux32/libfsdk.so"
        }
    }
    #[cfg(target_os = "windows")]
    {
        if cfg!(target_pointer_width = "64") {
            "fsdk/win64/facesdk.dll"
        } else {
            "fsdk/win32/facesdk.dll"
        }
    }
}

impl FsdkLibrary {
    fn load() -> Result<Self> {
        let lib_path = get_library_path();

        let paths_to_try = [
            lib_path.to_string(),
            format!("../{}", lib_path),
        ];

        let mut last_err: Option<libloading::Error> = None;
        let lib = paths_to_try
            .iter()
            .find_map(|path| {
                // On Windows, use LOAD_WITH_ALTERED_SEARCH_PATH with an absolute path
                // so that dependency DLLs (openvino.dll, tbb12.dll, etc.) are found
                // in the same directory as facesdk.dll.
                #[cfg(target_os = "windows")]
                {
                    if let Ok(abs) = std::path::absolute(path) {
                        if abs.exists() {
                            return match unsafe { WinLibrary::load_with_flags(&abs, 0x00000008) } {
                                Ok(wlib) => {
                                    // Also set the DLL directory so that plugins loaded
                                    // at runtime by facesdk.dll (e.g. IBetaPlugin.dll)
                                    // can be found in the same directory.
                                    if let Some(dir) = abs.parent() {
                                        use std::os::windows::ffi::OsStrExt;
                                        let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                                        unsafe {
                                            extern "system" {
                                                fn SetDllDirectoryW(path: *const u16) -> i32;
                                            }
                                            SetDllDirectoryW(wide.as_ptr());
                                        }
                                    }
                                    Some(Library::from(wlib))
                                }
                                Err(e) => {
                                    last_err = Some(e);
                                    None
                                }
                            };
                        }
                    }
                    None
                }
                #[cfg(not(target_os = "windows"))]
                match unsafe { Library::new(path) } {
                    Ok(l) => Some(l),
                    Err(e) => {
                        last_err = Some(e);
                        None
                    }
                }
            })
            .ok_or_else(|| {
                FsdkError::LibraryLoadError(format!(
                    "Could not load FSDK library. Last error: {:?}",
                    last_err
                ))
            })?;

        // SAFETY: We're loading known function symbols from the FSDK library.
        // The function signatures match the C API documentation.
        unsafe {
            macro_rules! load_fn {
                ($lib:expr, $name:expr) => {{
                    let sym: Symbol<*const c_void> = $lib.get($name.as_bytes()).map_err(|e| {
                        FsdkError::LibraryLoadError(format!("Failed to load {}: {}", $name, e))
                    })?;
                    std::mem::transmute(*sym)
                }};
            }

            Ok(FsdkLibrary {
                // Initialization
                activate_library: load_fn!(lib, "FSDK_ActivateLibrary"),
                get_hardware_id: load_fn!(lib, "FSDK_GetHardware_ID"),
                get_license_info: load_fn!(lib, "FSDK_GetLicenseInfo"),
                initialize: load_fn!(lib, "FSDK_Initialize"),
                finalize: load_fn!(lib, "FSDK_Finalize"),
                get_num_threads: load_fn!(lib, "FSDK_GetNumThreads"),
                set_num_threads: load_fn!(lib, "FSDK_SetNumThreads"),
                set_parameters: load_fn!(lib, "FSDK_SetParameters"),
                set_parameter: load_fn!(lib, "FSDK_SetParameter"),
                // Image
                create_empty_image: load_fn!(lib, "FSDK_CreateEmptyImage"),
                free_image: load_fn!(lib, "FSDK_FreeImage"),
                load_image_from_file: load_fn!(lib, "FSDK_LoadImageFromFile"),
                save_image_to_file: load_fn!(lib, "FSDK_SaveImageToFile"),
                load_image_from_buffer: load_fn!(lib, "FSDK_LoadImageFromBuffer"),
                load_image_from_jpeg_buffer: load_fn!(lib, "FSDK_LoadImageFromJpegBuffer"),
                load_image_from_png_buffer: load_fn!(lib, "FSDK_LoadImageFromPngBuffer"),
                get_image_buffer_size: load_fn!(lib, "FSDK_GetImageBufferSize"),
                save_image_to_buffer: load_fn!(lib, "FSDK_SaveImageToBuffer"),
                get_image_width: load_fn!(lib, "FSDK_GetImageWidth"),
                get_image_height: load_fn!(lib, "FSDK_GetImageHeight"),
                copy_image: load_fn!(lib, "FSDK_CopyImage"),
                resize_image: load_fn!(lib, "FSDK_ResizeImage"),
                resize_image_xy: load_fn!(lib, "FSDK_ResizeImageXY"),
                rotate_image: load_fn!(lib, "FSDK_RotateImage"),
                rotate_image_90: load_fn!(lib, "FSDK_RotateImage90"),
                rotate_image_center: load_fn!(lib, "FSDK_RotateImageCenter"),
                copy_rect: load_fn!(lib, "FSDK_CopyRect"),
                copy_rect_replicate_border: load_fn!(lib, "FSDK_CopyRectReplicateBorder"),
                mirror_image: load_fn!(lib, "FSDK_MirrorImage"),
                set_jpeg_compression_quality: load_fn!(lib, "FSDK_SetJpegCompressionQuality"),
                // Face Detection
                set_face_detection_parameters: load_fn!(lib, "FSDK_SetFaceDetectionParameters"),
                set_face_detection_threshold: load_fn!(lib, "FSDK_SetFaceDetectionThreshold"),
                get_detected_face_confidence: load_fn!(lib, "FSDK_GetDetectedFaceConfidence"),
                detect_face: load_fn!(lib, "FSDK_DetectFace"),
                detect_multiple_faces: load_fn!(lib, "FSDK_DetectMultipleFaces"),
                detect_eyes: load_fn!(lib, "FSDK_DetectEyes"),
                detect_eyes_in_region: load_fn!(lib, "FSDK_DetectEyesInRegion"),
                // Face Detection v2
                detect_face2: load_fn!(lib, "FSDK_DetectFace2"),
                detect_multiple_faces2: load_fn!(lib, "FSDK_DetectMultipleFaces2"),
                detect_face_and_features2: load_fn!(lib, "FSDK_DetectFaceAndFeatures2"),
                detect_multiple_faces_and_features2: load_fn!(lib, "FSDK_DetectMultipleFacesAndFeatures2"),
                // Facial Features
                detect_facial_features: load_fn!(lib, "FSDK_DetectFacialFeatures"),
                detect_facial_features_in_region: load_fn!(lib, "FSDK_DetectFacialFeaturesInRegion"),
                detect_facial_features_ex: load_fn!(lib, "FSDK_DetectFacialFeaturesEx"),
                detect_facial_features_in_region_ex: load_fn!(lib, "FSDK_DetectFacialFeaturesInRegionEx"),
                // Templates and Matching
                get_face_template: load_fn!(lib, "FSDK_GetFaceTemplate"),
                get_face_template_in_region: load_fn!(lib, "FSDK_GetFaceTemplateInRegion"),
                get_face_template_using_features: load_fn!(lib, "FSDK_GetFaceTemplateUsingFeatures"),
                get_face_template_using_eyes: load_fn!(lib, "FSDK_GetFaceTemplateUsingEyes"),
                match_faces: load_fn!(lib, "FSDK_MatchFaces"),
                get_matching_threshold_at_far: load_fn!(lib, "FSDK_GetMatchingThresholdAtFAR"),
                get_matching_threshold_at_frr: load_fn!(lib, "FSDK_GetMatchingThresholdAtFRR"),
                // Templates v2
                get_face_template2: load_fn!(lib, "FSDK_GetFaceTemplate2"),
                get_face_template_in_region2: load_fn!(lib, "FSDK_GetFaceTemplateInRegion2"),
                // Facial Attributes
                detect_facial_attribute_using_features: load_fn!(lib, "FSDK_DetectFacialAttributeUsingFeatures"),
                get_value_confidence: load_fn!(lib, "FSDK_GetValueConfidence"),
                // Camera
                initialize_capturing: load_fn!(lib, "FSDK_InitializeCapturing"),
                finalize_capturing: load_fn!(lib, "FSDK_FinalizeCapturing"),
                open_ip_video_camera: load_fn!(lib, "FSDK_OpenIPVideoCamera"),
                close_video_camera: load_fn!(lib, "FSDK_CloseVideoCamera"),
                grab_frame: load_fn!(lib, "FSDK_GrabFrame"),
                set_http_proxy: load_fn!(lib, "FSDK_SetHTTPProxy"),
                // Tracker
                create_tracker: load_fn!(lib, "FSDK_CreateTracker"),
                free_tracker: load_fn!(lib, "FSDK_FreeTracker"),
                clear_tracker: load_fn!(lib, "FSDK_ClearTracker"),
                set_tracker_parameter: load_fn!(lib, "FSDK_SetTrackerParameter"),
                set_tracker_multiple_parameters: load_fn!(lib, "FSDK_SetTrackerMultipleParameters"),
                get_tracker_parameter: load_fn!(lib, "FSDK_GetTrackerParameter"),
                feed_frame: load_fn!(lib, "FSDK_FeedFrame"),
                get_tracker_eyes: load_fn!(lib, "FSDK_GetTrackerEyes"),
                get_tracker_facial_features: load_fn!(lib, "FSDK_GetTrackerFacialFeatures"),
                get_tracker_face_position: load_fn!(lib, "FSDK_GetTrackerFacePosition"),
                get_tracker_face: load_fn!(lib, "FSDK_GetTrackerFace"),
                get_tracker_facial_attribute: load_fn!(lib, "FSDK_GetTrackerFacialAttribute"),
                lock_id: load_fn!(lib, "FSDK_LockID"),
                unlock_id: load_fn!(lib, "FSDK_UnlockID"),
                purge_id: load_fn!(lib, "FSDK_PurgeID"),
                set_name: load_fn!(lib, "FSDK_SetName"),
                get_name: load_fn!(lib, "FSDK_GetName"),
                get_all_names: load_fn!(lib, "FSDK_GetAllNames"),
                get_id_reassignment: load_fn!(lib, "FSDK_GetIDReassignment"),
                get_similar_id_count: load_fn!(lib, "FSDK_GetSimilarIDCount"),
                get_similar_id_list: load_fn!(lib, "FSDK_GetSimilarIDList"),
                save_tracker_memory_to_file: load_fn!(lib, "FSDK_SaveTrackerMemoryToFile"),
                load_tracker_memory_from_file: load_fn!(lib, "FSDK_LoadTrackerMemoryFromFile"),
                get_tracker_memory_buffer_size: load_fn!(lib, "FSDK_GetTrackerMemoryBufferSize"),
                save_tracker_memory_to_buffer: load_fn!(lib, "FSDK_SaveTrackerMemoryToBuffer"),
                load_tracker_memory_from_buffer: load_fn!(lib, "FSDK_LoadTrackerMemoryFromBuffer"),
                get_tracker_ids_count: load_fn!(lib, "FSDK_GetTrackerIDsCount"),
                get_tracker_all_ids: load_fn!(lib, "FSDK_GetTrackerAllIDs"),
                _lib: ManuallyDrop::new(lib),
            })
        }
    }
}

fn get_lib() -> Result<&'static FsdkLibrary> {
    let result = FSDK_LIB.get_or_init(|| FsdkLibrary::load());
    match result {
        Ok(lib) => Ok(lib),
        Err(e) => Err(FsdkError::LibraryLoadError(format!("{}", e))),
    }
}

fn check_result(func: &'static str, result: c_int) -> Result<()> {
    if result == FSDKE_OK {
        Ok(())
    } else {
        Err(FsdkError::from_code(func, result))
    }
}

// ============================================================================
// Safe wrappers for all FFI calls
// ============================================================================

// --- Initialization ---

pub fn activate_library(license_key: &str) -> Result<()> {
    let lib = get_lib()?;
    let key = CString::new(license_key).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let result = unsafe { (lib.activate_library)(key.as_ptr()) };
    check_result("ActivateLibrary", result)
}

pub fn get_hardware_id() -> Result<String> {
    let lib = get_lib()?;
    let mut buffer = vec![0u8; 4096];
    let result = unsafe { (lib.get_hardware_id)(buffer.as_mut_ptr() as *mut c_char) };
    check_result("GetHardware_ID", result)?;
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    Ok(cstr.to_string_lossy().into_owned())
}

pub fn get_license_info() -> Result<String> {
    let lib = get_lib()?;
    let mut buffer = vec![0u8; 4096];
    let result = unsafe { (lib.get_license_info)(buffer.as_mut_ptr() as *mut c_char) };
    check_result("GetLicenseInfo", result)?;
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    Ok(cstr.to_string_lossy().into_owned())
}

pub fn initialize() -> Result<()> {
    let lib = get_lib()?;
    let empty = CString::new("").unwrap();
    let result = unsafe { (lib.initialize)(empty.as_ptr()) };
    check_result("Initialize", result)
}

pub fn finalize() -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.finalize)() };
    check_result("Finalize", result)
}

pub fn get_num_threads() -> Result<i32> {
    let lib = get_lib()?;
    let mut num: c_int = 0;
    let result = unsafe { (lib.get_num_threads)(&mut num) };
    check_result("GetNumThreads", result)?;
    Ok(num)
}

pub fn set_num_threads(num: i32) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.set_num_threads)(num) };
    check_result("SetNumThreads", result)
}

pub fn set_parameters(params: &str) -> Result<()> {
    let lib = get_lib()?;
    let c_params = CString::new(params).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut err_pos: c_int = 0;
    let result = unsafe { (lib.set_parameters)(c_params.as_ptr(), &mut err_pos) };
    check_result("SetParameters", result)
}

pub fn set_parameter(name: &str, value: &str) -> Result<()> {
    let lib = get_lib()?;
    let c_name = CString::new(name).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let c_value = CString::new(value).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let result = unsafe { (lib.set_parameter)(c_name.as_ptr(), c_value.as_ptr()) };
    check_result("SetParameter", result)
}

// --- Image ---

pub fn create_empty_image() -> Result<HImage> {
    let lib = get_lib()?;
    let mut handle: HImage = -1;
    let result = unsafe { (lib.create_empty_image)(&mut handle) };
    check_result("CreateEmptyImage", result)?;
    Ok(handle)
}

pub fn free_image(handle: HImage) {
    if handle != -1 {
        if let Ok(lib) = get_lib() {
            unsafe {
                (lib.free_image)(handle);
            }
        }
    }
}

pub fn load_image_from_file(path: &str) -> Result<HImage> {
    let lib = get_lib()?;
    let c_path = CString::new(path).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut handle: HImage = -1;
    let result = unsafe { (lib.load_image_from_file)(&mut handle, c_path.as_ptr()) };
    check_result("LoadImageFromFile", result)?;
    Ok(handle)
}

pub fn save_image_to_file(handle: HImage, path: &str) -> Result<()> {
    let lib = get_lib()?;
    let c_path = CString::new(path).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let result = unsafe { (lib.save_image_to_file)(handle, c_path.as_ptr()) };
    check_result("SaveImageToFile", result)
}

pub fn load_image_from_buffer(buffer: &[u8], width: i32, height: i32, scan_line: i32, image_mode: i32) -> Result<HImage> {
    let lib = get_lib()?;
    let mut handle: HImage = -1;
    let result = unsafe { (lib.load_image_from_buffer)(&mut handle, buffer.as_ptr(), width, height, scan_line, image_mode) };
    check_result("LoadImageFromBuffer", result)?;
    Ok(handle)
}

pub fn load_image_from_jpeg_buffer(buffer: &[u8]) -> Result<HImage> {
    let lib = get_lib()?;
    let mut handle: HImage = -1;
    let result = unsafe { (lib.load_image_from_jpeg_buffer)(&mut handle, buffer.as_ptr(), buffer.len() as c_uint) };
    check_result("LoadImageFromJpegBuffer", result)?;
    Ok(handle)
}

pub fn load_image_from_png_buffer(buffer: &[u8]) -> Result<HImage> {
    let lib = get_lib()?;
    let mut handle: HImage = -1;
    let result = unsafe { (lib.load_image_from_png_buffer)(&mut handle, buffer.as_ptr(), buffer.len() as c_uint) };
    check_result("LoadImageFromPngBuffer", result)?;
    Ok(handle)
}

pub fn get_image_buffer_size(handle: HImage, image_mode: i32) -> Result<i32> {
    let lib = get_lib()?;
    let mut size: c_int = 0;
    let result = unsafe { (lib.get_image_buffer_size)(handle, &mut size, image_mode) };
    check_result("GetImageBufferSize", result)?;
    Ok(size)
}

pub fn save_image_to_buffer(handle: HImage, buffer: &mut [u8], image_mode: i32) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.save_image_to_buffer)(handle, buffer.as_mut_ptr(), image_mode) };
    check_result("SaveImageToBuffer", result)
}

pub fn get_image_width(handle: HImage) -> Result<i32> {
    let lib = get_lib()?;
    let mut width: c_int = 0;
    let result = unsafe { (lib.get_image_width)(handle, &mut width) };
    check_result("GetImageWidth", result)?;
    Ok(width)
}

pub fn get_image_height(handle: HImage) -> Result<i32> {
    let lib = get_lib()?;
    let mut height: c_int = 0;
    let result = unsafe { (lib.get_image_height)(handle, &mut height) };
    check_result("GetImageHeight", result)?;
    Ok(height)
}

pub fn copy_image(src: HImage, dst: HImage) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.copy_image)(src, dst) };
    check_result("CopyImage", result)
}

pub fn resize_image(src: HImage, ratio: f64, dst: HImage) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.resize_image)(src, ratio, dst) };
    check_result("ResizeImage", result)
}

pub fn resize_image_xy(src: HImage, ratio_x: f64, ratio_y: f64, dst: HImage) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.resize_image_xy)(src, ratio_x, ratio_y, dst) };
    check_result("ResizeImageXY", result)
}

pub fn rotate_image(src: HImage, angle: f64, dst: HImage) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.rotate_image)(src, angle, dst) };
    check_result("RotateImage", result)
}

pub fn rotate_image_90(src: HImage, multiplier: i32, dst: HImage) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.rotate_image_90)(src, multiplier, dst) };
    check_result("RotateImage90", result)
}

pub fn rotate_image_center(src: HImage, angle: f64, x_center: f64, y_center: f64, dst: HImage) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.rotate_image_center)(src, angle, x_center, y_center, dst) };
    check_result("RotateImageCenter", result)
}

pub fn copy_rect(src: HImage, x1: i32, y1: i32, x2: i32, y2: i32, dst: HImage) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.copy_rect)(src, x1, y1, x2, y2, dst) };
    check_result("CopyRect", result)
}

pub fn copy_rect_replicate_border(src: HImage, x1: i32, y1: i32, x2: i32, y2: i32, dst: HImage) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.copy_rect_replicate_border)(src, x1, y1, x2, y2, dst) };
    check_result("CopyRectReplicateBorder", result)
}

pub fn mirror_image(handle: HImage, use_vertical: bool) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.mirror_image)(handle, use_vertical as c_int) };
    check_result("MirrorImage", result)
}

pub fn set_jpeg_compression_quality(quality: i32) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.set_jpeg_compression_quality)(quality) };
    check_result("SetJpegCompressionQuality", result)
}

// --- Face Detection ---

pub fn set_face_detection_parameters(
    handle_arbitrary_rotations: bool,
    determine_face_rotation_angle: bool,
    internal_resize_width: i32,
) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe {
        (lib.set_face_detection_parameters)(
            handle_arbitrary_rotations as c_int,
            determine_face_rotation_angle as c_int,
            internal_resize_width,
        )
    };
    check_result("SetFaceDetectionParameters", result)
}

pub fn set_face_detection_threshold(threshold: i32) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.set_face_detection_threshold)(threshold) };
    check_result("SetFaceDetectionThreshold", result)
}

pub fn get_detected_face_confidence() -> Result<i32> {
    let lib = get_lib()?;
    let mut confidence: c_int = 0;
    let result = unsafe { (lib.get_detected_face_confidence)(&mut confidence) };
    check_result("GetDetectedFaceConfidence", result)?;
    Ok(confidence)
}

pub fn detect_face(handle: HImage) -> Result<FacePosition> {
    let lib = get_lib()?;
    let mut face_pos = FacePosition::default();
    let result = unsafe { (lib.detect_face)(handle, &mut face_pos) };
    check_result("DetectFace", result)?;
    Ok(face_pos)
}

pub fn detect_multiple_faces(handle: HImage, max_faces: usize) -> Result<Vec<FacePosition>> {
    let lib = get_lib()?;
    let mut count: c_int = 0;
    let mut faces: Vec<FacePosition> = vec![FacePosition::default(); max_faces];
    let buffer_size = (max_faces * std::mem::size_of::<FacePosition>()) as c_int;
    let result = unsafe { (lib.detect_multiple_faces)(handle, &mut count, faces.as_mut_ptr(), buffer_size) };
    if result == FSDKE_FACE_NOT_FOUND {
        return Ok(Vec::new());
    }
    check_result("DetectMultipleFaces", result)?;
    faces.truncate(count as usize);
    Ok(faces)
}

pub fn detect_eyes(handle: HImage) -> Result<Eyes> {
    let lib = get_lib()?;
    let mut eyes = Eyes::default();
    let result = unsafe { (lib.detect_eyes)(handle, &mut eyes) };
    check_result("DetectEyes", result)?;
    Ok(eyes)
}

pub fn detect_eyes_in_region(handle: HImage, face_position: &FacePosition) -> Result<Eyes> {
    let lib = get_lib()?;
    let mut eyes = Eyes::default();
    let result = unsafe { (lib.detect_eyes_in_region)(handle, face_position, &mut eyes) };
    check_result("DetectEyesInRegion", result)?;
    Ok(eyes)
}

// --- Face Detection v2 ---

pub fn detect_face2(handle: HImage) -> Result<Face> {
    let lib = get_lib()?;
    let mut face = Face::default();
    let result = unsafe { (lib.detect_face2)(handle, &mut face) };
    check_result("DetectFace2", result)?;
    Ok(face)
}

pub fn detect_multiple_faces2(handle: HImage, max_faces: usize) -> Result<Vec<Face>> {
    let lib = get_lib()?;
    let mut count: c_int = 0;
    let mut faces: Vec<Face> = vec![Face::default(); max_faces];
    let buffer_size = (max_faces * std::mem::size_of::<Face>()) as c_int;
    let result = unsafe { (lib.detect_multiple_faces2)(handle, &mut count, faces.as_mut_ptr(), buffer_size) };
    if result == FSDKE_FACE_NOT_FOUND {
        return Ok(Vec::new());
    }
    check_result("DetectMultipleFaces2", result)?;
    faces.truncate(count as usize);
    Ok(faces)
}

pub fn detect_face_and_features2(handle: HImage) -> Result<(Face, Features)> {
    let lib = get_lib()?;
    let mut face = Face::default();
    let mut features: Features = [Point::default(); FSDK_FACIAL_FEATURE_COUNT];
    let result = unsafe { (lib.detect_face_and_features2)(handle, &mut face, &mut features) };
    check_result("DetectFaceAndFeatures2", result)?;
    Ok((face, features))
}

pub fn detect_multiple_faces_and_features2(handle: HImage, max_faces: usize) -> Result<(Vec<Face>, Vec<Features>)> {
    let lib = get_lib()?;
    let mut count: c_int = 0;
    let mut faces: Vec<Face> = vec![Face::default(); max_faces];
    let mut features: Vec<Features> = vec![[Point::default(); FSDK_FACIAL_FEATURE_COUNT]; max_faces];
    let buffer_size = (max_faces * std::mem::size_of::<Face>()) as c_int;
    let result = unsafe { (lib.detect_multiple_faces_and_features2)(handle, &mut count, faces.as_mut_ptr(), features.as_mut_ptr(), buffer_size) };
    if result == FSDKE_FACE_NOT_FOUND {
        return Ok((Vec::new(), Vec::new()));
    }
    check_result("DetectMultipleFacesAndFeatures2", result)?;
    faces.truncate(count as usize);
    features.truncate(count as usize);
    Ok((faces, features))
}

// --- Facial Features ---

pub fn detect_facial_features(handle: HImage) -> Result<Features> {
    let lib = get_lib()?;
    let mut features: Features = [Point::default(); FSDK_FACIAL_FEATURE_COUNT];
    let result = unsafe { (lib.detect_facial_features)(handle, &mut features) };
    check_result("DetectFacialFeatures", result)?;
    Ok(features)
}

pub fn detect_facial_features_in_region(handle: HImage, face_position: &FacePosition) -> Result<Features> {
    let lib = get_lib()?;
    let mut features: Features = [Point::default(); FSDK_FACIAL_FEATURE_COUNT];
    let result = unsafe { (lib.detect_facial_features_in_region)(handle, face_position, &mut features) };
    check_result("DetectFacialFeaturesInRegion", result)?;
    Ok(features)
}

pub fn detect_facial_features_ex(handle: HImage) -> Result<(Features, [f32; FSDK_FACIAL_FEATURE_COUNT])> {
    let lib = get_lib()?;
    let mut features: Features = [Point::default(); FSDK_FACIAL_FEATURE_COUNT];
    let mut confidence = [0.0f32; FSDK_FACIAL_FEATURE_COUNT];
    let result = unsafe { (lib.detect_facial_features_ex)(handle, &mut features, confidence.as_mut_ptr()) };
    check_result("DetectFacialFeaturesEx", result)?;
    Ok((features, confidence))
}

pub fn detect_facial_features_in_region_ex(handle: HImage, face_position: &FacePosition) -> Result<(Features, [f32; FSDK_FACIAL_FEATURE_COUNT])> {
    let lib = get_lib()?;
    let mut features: Features = [Point::default(); FSDK_FACIAL_FEATURE_COUNT];
    let mut confidence = [0.0f32; FSDK_FACIAL_FEATURE_COUNT];
    let result = unsafe { (lib.detect_facial_features_in_region_ex)(handle, face_position, &mut features, confidence.as_mut_ptr()) };
    check_result("DetectFacialFeaturesInRegionEx", result)?;
    Ok((features, confidence))
}

// --- Templates and Matching ---

pub fn get_face_template(handle: HImage) -> Result<FaceTemplate> {
    let lib = get_lib()?;
    let mut template = FaceTemplate::default();
    let result = unsafe { (lib.get_face_template)(handle, &mut template) };
    check_result("GetFaceTemplate", result)?;
    Ok(template)
}

pub fn get_face_template_in_region(handle: HImage, face_position: &FacePosition) -> Result<FaceTemplate> {
    let lib = get_lib()?;
    let mut template = FaceTemplate::default();
    let result = unsafe { (lib.get_face_template_in_region)(handle, face_position, &mut template) };
    check_result("GetFaceTemplateInRegion", result)?;
    Ok(template)
}

pub fn get_face_template_using_features(handle: HImage, features: &Features) -> Result<FaceTemplate> {
    let lib = get_lib()?;
    let mut template = FaceTemplate::default();
    let result = unsafe { (lib.get_face_template_using_features)(handle, features, &mut template) };
    check_result("GetFaceTemplateUsingFeatures", result)?;
    Ok(template)
}

pub fn get_face_template_using_eyes(handle: HImage, eyes: &Eyes) -> Result<FaceTemplate> {
    let lib = get_lib()?;
    let mut template = FaceTemplate::default();
    let result = unsafe { (lib.get_face_template_using_eyes)(handle, eyes, &mut template) };
    check_result("GetFaceTemplateUsingEyes", result)?;
    Ok(template)
}

pub fn match_faces(template1: &FaceTemplate, template2: &FaceTemplate) -> Result<f32> {
    let lib = get_lib()?;
    let mut similarity: c_float = 0.0;
    let result = unsafe { (lib.match_faces)(template1, template2, &mut similarity) };
    check_result("MatchFaces", result)?;
    Ok(similarity)
}

pub fn get_matching_threshold_at_far(far_value: f32) -> Result<f32> {
    let lib = get_lib()?;
    let mut threshold: c_float = 0.0;
    let result = unsafe { (lib.get_matching_threshold_at_far)(far_value, &mut threshold) };
    check_result("GetMatchingThresholdAtFAR", result)?;
    Ok(threshold)
}

pub fn get_matching_threshold_at_frr(frr_value: f32) -> Result<f32> {
    let lib = get_lib()?;
    let mut threshold: c_float = 0.0;
    let result = unsafe { (lib.get_matching_threshold_at_frr)(frr_value, &mut threshold) };
    check_result("GetMatchingThresholdAtFRR", result)?;
    Ok(threshold)
}

// --- Templates v2 ---

pub fn get_face_template2(handle: HImage) -> Result<FaceTemplate> {
    let lib = get_lib()?;
    let mut template = FaceTemplate::default();
    let result = unsafe { (lib.get_face_template2)(handle, &mut template) };
    check_result("GetFaceTemplate2", result)?;
    Ok(template)
}

pub fn get_face_template_in_region2(handle: HImage, face: &Face) -> Result<FaceTemplate> {
    let lib = get_lib()?;
    let mut template = FaceTemplate::default();
    let result = unsafe { (lib.get_face_template_in_region2)(handle, face, &mut template) };
    check_result("GetFaceTemplateInRegion2", result)?;
    Ok(template)
}

// --- Facial Attributes ---

pub fn detect_facial_attribute_using_features(handle: HImage, features: &Features, attribute_name: &str) -> Result<String> {
    let lib = get_lib()?;
    let c_attr = CString::new(attribute_name).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut buffer = vec![0u8; 4096];
    let result = unsafe {
        (lib.detect_facial_attribute_using_features)(
            handle,
            features,
            c_attr.as_ptr(),
            buffer.as_mut_ptr() as *mut c_char,
            buffer.len() as c_longlong
        )
    };
    check_result("DetectFacialAttributeUsingFeatures", result)?;
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    Ok(cstr.to_string_lossy().into_owned())
}

pub fn get_value_confidence(attribute_values: &str, value: &str) -> Result<f32> {
    let lib = get_lib()?;
    let c_attr_values = CString::new(attribute_values).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let c_value = CString::new(value).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut confidence: c_float = 0.0;
    let result = unsafe { (lib.get_value_confidence)(c_attr_values.as_ptr(), c_value.as_ptr(), &mut confidence) };
    check_result("GetValueConfidence", result)?;
    Ok(confidence)
}

// --- Camera ---

pub fn initialize_capturing() -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.initialize_capturing)() };
    check_result("InitializeCapturing", result)
}

pub fn finalize_capturing() -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.finalize_capturing)() };
    check_result("FinalizeCapturing", result)
}

pub fn set_http_proxy(server: &str, port: u16, username: &str, password: &str) -> Result<()> {
    let lib = get_lib()?;
    let c_server = CString::new(server).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let c_username = CString::new(username).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let c_password = CString::new(password).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let result = unsafe { (lib.set_http_proxy)(c_server.as_ptr(), port as c_ushort, c_username.as_ptr(), c_password.as_ptr()) };
    check_result("SetHTTPProxy", result)
}

pub fn open_ip_video_camera(compression: i32, url: &str, username: &str, password: &str, timeout_seconds: i32) -> Result<HCamera> {
    let lib = get_lib()?;
    let c_url = CString::new(url).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let c_username = CString::new(username).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let c_password = CString::new(password).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut handle: HCamera = -1;
    let result = unsafe {
        (lib.open_ip_video_camera)(
            compression,
            c_url.as_ptr(),
            c_username.as_ptr(),
            c_password.as_ptr(),
            timeout_seconds,
            &mut handle
        )
    };
    check_result("OpenIPVideoCamera", result)?;
    Ok(handle)
}

pub fn close_video_camera(handle: HCamera) -> Result<()> {
    if handle != -1 {
        let lib = get_lib()?;
        let result = unsafe { (lib.close_video_camera)(handle) };
        check_result("CloseVideoCamera", result)?;
    }
    Ok(())
}

pub fn grab_frame(camera: HCamera) -> Result<HImage> {
    let lib = get_lib()?;
    let mut handle: HImage = -1;
    let result = unsafe { (lib.grab_frame)(camera, &mut handle) };
    check_result("GrabFrame", result)?;
    Ok(handle)
}

// --- Tracker ---

pub fn create_tracker() -> Result<HTracker> {
    let lib = get_lib()?;
    let mut handle: HTracker = -1;
    let result = unsafe { (lib.create_tracker)(&mut handle) };
    check_result("CreateTracker", result)?;
    Ok(handle)
}

pub fn free_tracker(handle: HTracker) {
    if handle != -1 {
        if let Ok(lib) = get_lib() {
            unsafe {
                (lib.free_tracker)(handle);
            }
        }
    }
}

pub fn clear_tracker(handle: HTracker) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.clear_tracker)(handle) };
    check_result("ClearTracker", result)
}

pub fn set_tracker_parameter(handle: HTracker, name: &str, value: &str) -> Result<()> {
    let lib = get_lib()?;
    let c_name = CString::new(name).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let c_value = CString::new(value).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let result = unsafe { (lib.set_tracker_parameter)(handle, c_name.as_ptr(), c_value.as_ptr()) };
    check_result("SetTrackerParameter", result)
}

pub fn set_tracker_multiple_parameters(handle: HTracker, params: &str) -> Result<()> {
    let lib = get_lib()?;
    let c_params = CString::new(params).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut err_pos: c_int = 0;
    let result = unsafe { (lib.set_tracker_multiple_parameters)(handle, c_params.as_ptr(), &mut err_pos) };
    check_result("SetTrackerMultipleParameters", result)
}

pub fn get_tracker_parameter(handle: HTracker, name: &str) -> Result<String> {
    let lib = get_lib()?;
    let c_name = CString::new(name).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut buffer = vec![0u8; 4096];
    let result = unsafe { (lib.get_tracker_parameter)(handle, c_name.as_ptr(), buffer.as_mut_ptr() as *mut c_char, buffer.len() as c_longlong) };
    check_result("GetTrackerParameter", result)?;
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    Ok(cstr.to_string_lossy().into_owned())
}

pub fn feed_frame(handle: HTracker, camera_idx: i64, image: HImage, max_ids: usize) -> Result<Vec<i64>> {
    let lib = get_lib()?;
    let mut face_count: c_longlong = 0;
    let mut ids: Vec<c_longlong> = vec![0; max_ids];
    let buffer_size = (max_ids * std::mem::size_of::<c_longlong>()) as c_longlong;
    let result = unsafe { (lib.feed_frame)(handle, camera_idx, image, &mut face_count, ids.as_mut_ptr(), buffer_size) };
    check_result("FeedFrame", result)?;
    ids.truncate(face_count as usize);
    Ok(ids)
}

pub fn get_tracker_eyes(handle: HTracker, camera_idx: i64, id: i64) -> Result<Eyes> {
    let lib = get_lib()?;
    let mut eyes = Eyes::default();
    let result = unsafe { (lib.get_tracker_eyes)(handle, camera_idx, id, &mut eyes) };
    check_result("GetTrackerEyes", result)?;
    Ok(eyes)
}

pub fn get_tracker_facial_features(handle: HTracker, camera_idx: i64, id: i64) -> Result<Features> {
    let lib = get_lib()?;
    let mut features: Features = [Point::default(); FSDK_FACIAL_FEATURE_COUNT];
    let result = unsafe { (lib.get_tracker_facial_features)(handle, camera_idx, id, &mut features) };
    check_result("GetTrackerFacialFeatures", result)?;
    Ok(features)
}

pub fn get_tracker_face_position(handle: HTracker, camera_idx: i64, id: i64) -> Result<FacePosition> {
    let lib = get_lib()?;
    let mut face_pos = FacePosition::default();
    let result = unsafe { (lib.get_tracker_face_position)(handle, camera_idx, id, &mut face_pos) };
    check_result("GetTrackerFacePosition", result)?;
    Ok(face_pos)
}

pub fn get_tracker_face(handle: HTracker, camera_idx: i64, id: i64) -> Result<Face> {
    let lib = get_lib()?;
    let mut face = Face::default();
    let result = unsafe { (lib.get_tracker_face)(handle, camera_idx, id, &mut face) };
    check_result("GetTrackerFace", result)?;
    Ok(face)
}

pub fn get_tracker_facial_attribute(handle: HTracker, camera_idx: i64, id: i64, attribute_name: &str) -> Result<String> {
    let lib = get_lib()?;
    let c_attr = CString::new(attribute_name).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut buffer = vec![0u8; 4096];
    let result = unsafe {
        (lib.get_tracker_facial_attribute)(
            handle,
            camera_idx,
            id,
            c_attr.as_ptr(),
            buffer.as_mut_ptr() as *mut c_char,
            buffer.len() as c_longlong
        )
    };
    check_result("GetTrackerFacialAttribute", result)?;
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    Ok(cstr.to_string_lossy().into_owned())
}

pub fn lock_id(handle: HTracker, id: i64) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.lock_id)(handle, id) };
    check_result("LockID", result)
}

pub fn unlock_id(handle: HTracker, id: i64) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.unlock_id)(handle, id) };
    check_result("UnlockID", result)
}

pub fn purge_id(handle: HTracker, id: i64) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.purge_id)(handle, id) };
    check_result("PurgeID", result)
}

pub fn set_name(handle: HTracker, id: i64, name: &str) -> Result<()> {
    let lib = get_lib()?;
    let c_name = CString::new(name).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let result = unsafe { (lib.set_name)(handle, id, c_name.as_ptr()) };
    check_result("SetName", result)
}

pub fn get_name(handle: HTracker, id: i64) -> Result<String> {
    let lib = get_lib()?;
    let mut buffer = vec![0u8; 4096];
    let result = unsafe { (lib.get_name)(handle, id, buffer.as_mut_ptr() as *mut c_char, buffer.len() as c_longlong) };
    check_result("GetName", result)?;
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    Ok(cstr.to_string_lossy().into_owned())
}

pub fn get_all_names(handle: HTracker, id: i64) -> Result<Vec<String>> {
    let lib = get_lib()?;
    let mut buffer = vec![0u8; 4096];
    let result = unsafe { (lib.get_all_names)(handle, id, buffer.as_mut_ptr() as *mut c_char, buffer.len() as c_longlong) };
    check_result("GetAllNames", result)?;
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    let names_str = cstr.to_string_lossy();
    Ok(names_str.split(';').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect())
}

pub fn get_id_reassignment(handle: HTracker, id: i64) -> Result<i64> {
    let lib = get_lib()?;
    let mut reassigned_id: c_longlong = 0;
    let result = unsafe { (lib.get_id_reassignment)(handle, id, &mut reassigned_id) };
    check_result("GetIDReassignment", result)?;
    Ok(reassigned_id)
}

pub fn get_similar_id_count(handle: HTracker, id: i64) -> Result<i64> {
    let lib = get_lib()?;
    let mut count: c_longlong = 0;
    let result = unsafe { (lib.get_similar_id_count)(handle, id, &mut count) };
    check_result("GetSimilarIDCount", result)?;
    Ok(count)
}

pub fn get_similar_id_list(handle: HTracker, id: i64) -> Result<Vec<i64>> {
    let count = get_similar_id_count(handle, id)?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let lib = get_lib()?;
    let mut ids: Vec<c_longlong> = vec![0; count as usize];
    let buffer_size = (count as usize * std::mem::size_of::<c_longlong>()) as c_longlong;
    let result = unsafe { (lib.get_similar_id_list)(handle, id, ids.as_mut_ptr(), buffer_size) };
    check_result("GetSimilarIDList", result)?;
    Ok(ids)
}

pub fn save_tracker_memory_to_file(handle: HTracker, filename: &str) -> Result<()> {
    let lib = get_lib()?;
    let c_filename = CString::new(filename).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let result = unsafe { (lib.save_tracker_memory_to_file)(handle, c_filename.as_ptr()) };
    check_result("SaveTrackerMemoryToFile", result)
}

pub fn load_tracker_memory_from_file(filename: &str) -> Result<HTracker> {
    let lib = get_lib()?;
    let c_filename = CString::new(filename).map_err(|e| FsdkError::InvalidString(e.to_string()))?;
    let mut handle: HTracker = -1;
    let result = unsafe { (lib.load_tracker_memory_from_file)(&mut handle, c_filename.as_ptr()) };
    check_result("LoadTrackerMemoryFromFile", result)?;
    Ok(handle)
}

pub fn get_tracker_memory_buffer_size(handle: HTracker) -> Result<i64> {
    let lib = get_lib()?;
    let mut size: c_longlong = 0;
    let result = unsafe { (lib.get_tracker_memory_buffer_size)(handle, &mut size) };
    check_result("GetTrackerMemoryBufferSize", result)?;
    Ok(size)
}

pub fn save_tracker_memory_to_buffer(handle: HTracker, buffer: &mut [u8]) -> Result<()> {
    let lib = get_lib()?;
    let result = unsafe { (lib.save_tracker_memory_to_buffer)(handle, buffer.as_mut_ptr(), buffer.len() as c_longlong) };
    check_result("SaveTrackerMemoryToBuffer", result)
}

pub fn load_tracker_memory_from_buffer(buffer: &[u8]) -> Result<HTracker> {
    let lib = get_lib()?;
    let mut handle: HTracker = -1;
    let result = unsafe { (lib.load_tracker_memory_from_buffer)(&mut handle, buffer.as_ptr()) };
    check_result("LoadTrackerMemoryFromBuffer", result)?;
    Ok(handle)
}

pub fn get_tracker_ids_count(handle: HTracker) -> Result<i64> {
    let lib = get_lib()?;
    let mut count: c_longlong = 0;
    let result = unsafe { (lib.get_tracker_ids_count)(handle, &mut count) };
    check_result("GetTrackerIDsCount", result)?;
    Ok(count)
}

pub fn get_tracker_all_ids(handle: HTracker) -> Result<Vec<i64>> {
    let count = get_tracker_ids_count(handle)?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let lib = get_lib()?;
    let mut ids: Vec<c_longlong> = vec![0; count as usize];
    let buffer_size = (count as usize * std::mem::size_of::<c_longlong>()) as c_longlong;
    let result = unsafe { (lib.get_tracker_all_ids)(handle, ids.as_mut_ptr(), buffer_size) };
    check_result("GetTrackerAllIDs", result)?;
    Ok(ids)
}

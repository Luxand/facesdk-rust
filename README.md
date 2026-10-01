## [FaceSDK](https://www.luxand.com/facesdk/?utm_source=github&utm_medium=readmd&utm_campaign=header) · [CloudAPI](https://luxand.cloud/?utm_source=github&utm_medium=readmd&utm_campaign=header) · [LinkedIn](https://www.linkedin.com/company/luxand-inc.) · [Contact](mailto:support@luxand.com)

### NIST-approved

Luxand's FaceSDK ranked within the top 21.8% by the National Institute of Standards and Technology (NIST) during the Face Recognition Vendor Test (FRVT).

### iBeta Certified Liveness

The iBeta certified Liveness add-on for FaceSDK aced Level 1 Presentation Attack Detection (PAD) testing, following ISO/IEC 30107-3 standards.

---

# FaceSDK - Rust, Cross-Platform (macOS, Linux, Windows)

Cross-platform Rust examples demonstrating face detection, recognition, and liveness verification using Luxand FaceSDK 9.0. Includes a Rust wrapper with dynamic library loading — no build-time linking required.

> Before running examples, replace `INSERT THE LICENSE KEY HERE` with your license key in `src/liverecognition.rs` and `src/portrait.rs`.

## Examples

### Live Face Recognition (`liverecognition`)

Real-time face detection, recognition, and liveness verification from a webcam feed.

- Real-time face detection and tracking
- Face recognition with persistent identity across sessions (saved to `tracker90.dat`)
- Liveness detection to prevent spoofing attacks
- Click-to-name face identification via native dialog
- FPS overlay in the live window
- Resizable window with aspect-ratio-preserving scaling

### Portrait (`portrait`)

Face detection and cropping from a static image.
 - Detects the most prominent face in the input image, crops it to a square around the face, and saves to the output file.
 - If `output_file` is omitted, the output is saved as `face.<input_file>`.

## Prerequisites

- **Rust** toolchain (1.70+) installed via `rustup`: https://rustup.rs/
- **Luxand FaceSDK 9.0** native library placed in the `fsdk/` directory (already included in this repository):
  - macOS ARM64: `fsdk/osx_arm64/libfsdk.dylib`
  - Linux 64-bit: `fsdk/linux64/libfsdk.so`
  - Windows 64-bit: `fsdk/win64/facesdk.dll`
- **Camera** — A webcam accessible by the OS (for `liverecognition`)
- **Video4Linux / V4L2** development packages on Linux (for webcam access in `liverecognition`)
    - Search your distribution packages for `Video4Linux`, `V4L2`, `libv4l`, or `v4l-utils`
    - Common package names include `libv4l-dev` (Ubuntu/Debian), `libv4l-devel` (Fedora/RHEL), and `v4l-utils` (Arch)
- **Clang / libclang** development libraries on Linux — required by `bindgen` through the `nokhwa -> v4l2-sys-mit` dependency chain when generating V4L2 FFI bindings
    - Common package names include `clang` and `libclang-dev` (Ubuntu/Debian), `clang` and `clang-devel` (Fedora/RHEL), and `clang` (Arch)
- **iBeta liveness add-on** (Windows/Linux only) — the plugin libraries and the `data/` model directory are included in `fsdk/win64`, `fsdk/linux64` and `fsdk/data`. By default, `liverecognition` uses `IBETA_DIR = "./fsdk"` as `LivenessModel` data directory. The add-on also requires a FaceSDK license key that permits it and the iBeta license installed on the system (see [iBeta License](#ibeta-license)).

### Install Rust

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

```powershell
# Windows (PowerShell)
winget install Rustlang.Rustup
```

### Ubuntu Example

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Linux build dependencies for webcam support and bindgen
sudo apt-get update
sudo apt-get install -y libv4l-dev clang libclang-dev
```

> On Linux, the build and run commands below are shown with Ubuntu-compatible examples. For other distributions, install the equivalent Video4Linux / V4L2 and libclang development packages for your package manager.

## Building and Running

The commands below are cross-platform (`cargo`), including Linux (Ubuntu).

```bash
# Build all
cargo build --release

# Run live recognition
cargo run --release --bin liverecognition

# Run portrait face detection
cargo run --release --bin portrait -- <input_file> [output_file]

# Example
cargo run --release --bin portrait -- photo.jpg
# Creates "face.photo.jpg" with the detected face cropped and resized
```

Run the examples from the repository root: the FaceSDK library is loaded from `fsdk/<platform>/` relative to the current directory (or its parent), and the tracker memory is saved to `tracker90.dat` in the current directory.

## FaceSDK 9.0 API

FaceSDK 9.0 uses new neural network models for face detection and recognition. The face template size is 1040 bytes.
Templates and tracker memory files created with FaceSDK 8.x are not compatible with 9.0.

### Face Structure

```rust
pub struct Face {
    pub score: f32,           // Detector confidence
    pub angle: f32,           // In-plane rotation angle in degrees
    pub bbox: BBox,           // Bounding box with top-left (p0) and bottom-right (p1)
    pub features: [Point; 5], // Eye centers, nose tip, mouth corners
}
```

`Face` replaces `FacePosition` from previous versions. Use `face.rect()`, `face.width()`, `face.height()` and `face.center()` to work with the bounding box.
Facial features (`Features`) are 70 points with floating-point coordinates (`PointF`).

### Detection and Recognition Functions

```rust
let face: Face = image.detect_face()?;                          // face with the highest confidence
let faces: Vec<Face> = image.detect_multiple_faces(max_count)?; // sorted by confidence, descending
let features: Features = image.detect_facial_features_in_region(&face)?;
let template: FaceTemplate = image.get_face_template_in_region(&face)?;
let similarity: f32 = FSDK::match_faces(&template1, &template2)?;
```

### Face Detection Parameters

Parameters are set with `FSDK::set_parameter` / `FSDK::set_parameters`, or with `tracker.set_parameter` / `tracker.set_parameters` for the Tracker API.

| Parameter | Description | Default | Accepted Values |
| :--- | :--- | :---: | :--- |
| FaceDetectionThreshold | Minimum detection score for a face to be reported | 0.64 (Tracker: 0.4) | Float in [0, 1] |
| FaceDetectionPatchSize | Size of the square patch the detector works with | 640 (Tracker: 256) | Divisible by 32, minimum 64 (higher = slower but detects smaller faces) |
| FaceDetectionPatchMode | Image patching algorithm | fast | `"fast"`, `"mixed"`, `"full"` |
| FaceDetectionBigFaceSize | Size of the whole-image pass used to find faces too large for a single patch | 384 | Positive integer |
| FaceDetectionBatchSize | Image patches processed simultaneously | 1 | Positive integer |
| TrimOutOfScreenFaces | Discard faces crossing the edges of the image | true | `"true"`, `"false"` |
| FaceDetectionModel | Path to the face detection model file | default | File path or `"default"` |

The examples use `FaceDetectionPatchSize=128` for live webcam video and `256` for still photos.

### Face Recognition Parameters

| Parameter | Description | Default | Accepted Values |
| :--- | :--- | :---: | :--- |
| FaceRecognitionModel | Path to the face recognition model file | default | File path or `"default"` |
| FaceRecognitionUseFlipTest | Also use the mirrored face when creating a template | false | `"false"` or `"true"` |
| FaceRecognitionBatchSize | Faces processed in one inference call | 1 | Positive integer |
| ComputationDelegate | Computation backend for all models | cpu | `"none"`, `"cpu"`, `"gpu"` |

## Liveness Detection

### Windows and Linux: iBeta Certified Liveness

On Windows and Linux, the examples use the [iBeta Certified Liveness Addon](https://www.luxand.com/facesdk/documentation/certifiedliveness.php) for robust single-frame presentation attack detection.

#### iBeta License

The iBeta add-on requires a license file (`.v2c`) installed on the target system before starting the application. The license file and the `install_license` utilities are in the `INSTALL_LICENSE` directory. To install it, run:

```bash
# Windows
cd INSTALL_LICENSE
run_to_install_license.bat

# Linux
cd INSTALL_LICENSE
sh INSTALL.sh
```

If the add-on cannot be loaded, `liverecognition` prints a warning and continues without iBeta liveness. `FSDKE_PLUGIN_NO_PERMISSION` (-31) means that your FaceSDK license key does not permit the iBeta add-on.

```rust
// Load iBeta liveness model (before tracker creation)
const IBETA_DIR: &str = "./fsdk";
FSDK::set_parameter("LivenessModel", &format!("external:dataDir={}", IBETA_DIR))?;

// Configure tracker
tracker.set_parameters(
    "FaceDetectionPatchSize=128; FaceDetectionThreshold=0.4; \
     DetectLiveness=true; LivenessFramesCount=1; SmoothAttributeLiveness=false"
)?;
```

If your iBeta files are stored elsewhere, update `IBETA_DIR` in `src/liverecognition.rs`.

### macOS: Built-in Liveness Detection

> **Note:** The iBeta certified liveness addon is not supported on macOS. The example uses the built-in liveness detection method instead, which requires multiple frames for assessment.

```rust
tracker.set_parameters(
    "FaceDetectionPatchSize=128; FaceDetectionThreshold=0.4; \
     DetectLiveness=true; LivenessFramesCount=6; SmoothAttributeLiveness=true"
)?;
```

### Liveness UI Indicators

| Color | Meaning |
| :--- | :--- |
| Green | Live face detected (liveness > 50%) |
| Red | Possible spoof detected (liveness <= 50%) |
| Yellow | Liveness error (e.g., model not loaded) |
| Blue | Mouse hovering over face (click to name) |

## Controls

| Key / Action | Effect |
| :--- | :--- |
| ESC | Exit and save tracker memory |
| Click on a face | Assign or change a name for the face ID |

## Project Structure

```
├── src/
│   ├── fsdk.rs             # Rust wrapper for FaceSDK (Image, Tracker, FSDK)
│   ├── fsdk_bindings.rs    # Low-level FFI bindings (dynamic library loading)
│   ├── consts.rs           # Constants and error codes
│   ├── liverecognition.rs  # Live face recognition example (webcam)
│   └── portrait.rs         # Static face detection and cropping example
├── assets/
│   └── Inter-Regular.ttf   # Embedded TrueType font for overlay text
├── fsdk/                   # Native FaceSDK libraries (per-platform)
│   ├── data/               # iBeta liveness model and configuration files (Windows/Linux)
│   │   ├── detection/
│   │   ├── pipelines/
│   │   ├── preprocessing/
│   │   └── quality/
│   ├── linux64/            # libfsdk.so and iBeta plugin libraries
│   ├── osx_arm64/          # libfsdk.dylib
│   └── win64/              # facesdk.dll and iBeta plugin libraries
├── INSTALL_LICENSE/        # iBeta license file and install utilities (Windows/Linux)
├── input.png               # Sample input image for portrait example
├── Cargo.toml
└── README.md
```

## [FaceSDK](https://www.luxand.com/facesdk/?utm_source=github&utm_medium=readmd&utm_campaign=header) · [CloudAPI](https://luxand.cloud/?utm_source=github&utm_medium=readmd&utm_campaign=header) · [LinkedIn](https://www.linkedin.com/company/luxand-inc.) · [Contact](mailto:support@luxand.com)

### NIST-approved

Luxand's FaceSDK ranked within the top 21.8% by the National Institute of Standards and Technology (NIST) during the Face Recognition Vendor Test (FRVT).

### iBeta Certified Liveness

The iBeta certified Liveness add-on for FaceSDK aced Level 1 Presentation Attack Detection (PAD) testing, following ISO/IEC 30107-3 standards.

---

# FaceSDK - Rust, Cross-Platform (macOS, Linux, Windows)

Cross-platform Rust examples demonstrating face detection, recognition, and liveness verification using Luxand FaceSDK. Includes a Rust wrapper with dynamic library loading — no build-time linking required.

> Before running examples, set `LICENSE_KEY` in `src/liverecognition.rs` and `src/portrait.rs`.

## Examples

### Live Face Recognition (`liverecognition`)

Real-time face detection, recognition, and liveness verification from a webcam feed.

- Real-time face detection and tracking with the improved v2 model
- Face recognition with persistent identity across sessions (saved to `tracker.dat`)
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
- **Luxand FaceSDK** native library placed in the `fsdk/` directory:
  - macOS ARM64: `fsdk/osx_arm64/libfsdk.dylib`
  - Linux 64-bit: `fsdk/linux64/libfsdk.so`
  - Windows 64-bit: `fsdk/win64/facesdk.dll`
- **Camera** — A webcam accessible by the OS (for `liverecognition`)
- **Video4Linux / V4L2** development packages on Linux (for webcam access in `liverecognition`)
    - Search your distribution packages for `Video4Linux`, `V4L2`, `libv4l`, or `v4l-utils`
    - Common package names include `libv4l-dev` (Ubuntu/Debian), `libv4l-devel` (Fedora/RHEL), and `v4l-utils` (Arch)
- **Clang / libclang** development libraries on Linux — required by `bindgen` through the `nokhwa -> v4l2-sys-mit` dependency chain when generating V4L2 FFI bindings
    - Common package names include `clang` and `libclang-dev` (Ubuntu/Debian), `clang` and `clang-devel` (Fedora/RHEL), and `clang` (Arch)
- **iBeta liveness model files** (Windows/Linux only) — By default, `liverecognition` uses `IBETA_DIR = "./fsdk"` as `LivenessModel` data directory

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

On Windows (via Git Bash or MSYS2), if your user profile path contains non-ASCII characters, use `./build.sh` instead of `cargo build` directly. It relocates `CARGO_HOME` to `C:/cargo-home` so that NASM can handle the mozjpeg SIMD build without Unicode path errors.

## Improved v2 Face Detection

The v2 model significantly improves face detection and recognition accuracy. The face template size is 2068 bytes and the recognition threshold is lower — values as low as 0.8 provide good results.

### Face Structure

```rust
pub struct Face {
    pub bbox: BBox,           // Bounding box with top-left (p0) and bottom-right (p1)
    pub features: [Point; 5], // Eye centers, nose tip, mouth corners
}
```

Use `Face` instead of `FacePosition` when working with v2 detection. The `bbox` field provides direct top-left and bottom-right coordinates. The `features` array contains 5 key facial landmarks.

### v2 Detection Functions

```rust
let face: Face = image.detect_face2()?;
let faces: Vec<Face> = image.detect_multiple_faces2(max_count)?;
let template: FaceTemplate = image.get_face_template2()?;
let template: FaceTemplate = image.get_face_template_in_region2(&face)?;
```

`detect_face2` returns the face with the highest confidence. `detect_multiple_faces2` returns faces sorted by confidence in descending order.

### Activating v2 Detection in Tracker

```rust
tracker.set_parameter("DetectionVersion", "2")?;
```

This parameter must be set before the first call to `tracker.feed_frame()`. It cannot be set on a non-empty Tracker (one loaded from a file with existing face data).

### v2 Detection Parameters

| Parameter | Description | Default | Accepted Values |
| :--- | :--- | :---: | :--- |
| FaceDetection2Model | Path to the face detection model file | default | File path or `"default"` |
| FaceDetection2Threshold | Face detection threshold | 0.64 | Float in [0, 1] |
| FaceDetection2BatchSize | Image patches processed simultaneously | 1 | Positive integer |
| FaceDetection2PatchSize | Size of a single image patch | 640 | Positive integer (higher = slower but detects smaller faces) |
| FaceDetection2PatchMode | Image patching algorithm | fast | `"fast"`, `"full"`, `"mixed"` |
| FaceDetection2ComputationDelegate | Computation backend | cpu | `"none"`, `"cpu"`, `"gpu"` |

### v2 Recognition Parameters

| Parameter | Description | Default | Accepted Values |
| :--- | :--- | :---: | :--- |
| FaceRecognition2Model | Path to the face recognition model file | default | File path or `"default"` |
| FaceRecognition2UseFlipTest | Use mirrored image when creating template | false | `"false"` or `"true"` |
| FaceRecognition2ComputationDelegate | Computation backend | cpu | `"none"`, `"cpu"`, `"gpu"` |

## Liveness Detection

### Windows and Linux: iBeta Certified Liveness

On Windows and Linux, the examples use the [iBeta Certified Liveness Addon](https://www.luxand.com/facesdk/documentation/certifiedliveness.php) for robust single-frame presentation attack detection.

```rust
// Load iBeta liveness model (before tracker creation)
const IBETA_DIR: &str = "./fsdk";
FSDK::set_parameter("LivenessModel", &format!("external:dataDir={}", IBETA_DIR))?;

// Configure tracker
tracker.set_parameters(
    "DetectLiveness=true; LivenessFramesCount=1; SmoothAttributeLiveness=false"
)?;
```

If your iBeta files are stored elsewhere, update `IBETA_DIR` in `src/liverecognition.rs`.

### macOS: Built-in Liveness Detection

> **Note:** The iBeta certified liveness addon is not supported on macOS. The example uses the built-in liveness detection method instead, which requires multiple frames for assessment.

```rust
tracker.set_parameters(
    "DetectLiveness=true; LivenessFramesCount=6; SmoothAttributeLiveness=true"
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
│   ├── data/               # iBeta liveness model and configuration files
│   │   ├── detection/
│   │   ├── pipelines/
│   │   ├── preprocessing/
│   │   └── quality/
│   ├── linux64/
│   ├── osx_arm64/
│   └── win64/
├── build.sh                # Build wrapper for Windows/Git Bash (NASM Unicode path workaround)
├── input.png               # Sample input image for portrait example
├── Cargo.toml
└── README.md
```

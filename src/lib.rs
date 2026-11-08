//! Rust bindings for [CharLS](https://github.com/team-charls/charls),
//! the implementation of the JPEG-LS standard
//! for lossless and near-lossless image compression and decompression.
//!
//! # Targets
//!
//! - **native**: the CharLS C library via `charls-sys`.
//! - **wasm32**: `@team-charls/charls-wasm` (npm) via `wasm-bindgen`. [`init`] must
//!   be awaited once before the first decode. Encode is unsupported.
//!
//! # Building
//!
//! On native targets, `charls-sys` compiles the bundled CharLS sources with CMake,
//! so CMake and a C++ compiler are required. CharLS does not need to be installed.
//!
//! # Cargo features
//!
//! - `static`: ask rustc to link CharLS as a static library (native only).
//! - `vcpkg`: do not build CharLS; find an installed one through vcpkg (native only).
//!
//! # Example
//!
//! ```no_run
//! use charls::CharLS;
//!
//! // Read a JPEG-LS file
//! let data = std::fs::read("test.jls")?;
//! let mut charls = CharLS::default();
//! let decoded = charls.decode(&data)?;
//! // The same instance can decode or encode any number of images.
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
use native as backend;
#[cfg(target_arch = "wasm32")]
use wasm as backend;

#[cfg(target_arch = "wasm32")]
pub use wasm::init;

pub type CharlsResult<T> = Result<T, Error>;

/// The error code CharLS reports, as it reports it.
pub type CharlsErrc = i32;

#[cfg(not(target_arch = "wasm32"))]
pub type CharlsEncoder = native::CharlsEncoder;
#[cfg(not(target_arch = "wasm32"))]
pub type CharlsDecoder = native::CharlsDecoder;

/// Error type for all high-level CharLS operations
#[derive(Debug, Copy, Clone, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Unable to start the codec
    InitCodec,
    /// Unable to compute decompressed size
    ComputeSize,
    /// CharLS error
    JpegLsError {
        /// the native error code from CharLS
        code: CharlsErrc,
    },
    /// wasm only: [`init`] was never awaited.
    Uninitialised,
    /// The operation is not available on this target.
    Unsupported,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InitCodec => write!(f, "Unable to start the codec"),
            Error::ComputeSize => write!(f, "Unable to compute decompressed size"),
            Error::JpegLsError { code } => write!(f, "{}", backend::error_message(*code)),
            Error::Uninitialised => write!(f, "The CharLS wasm module has not been loaded"),
            Error::Unsupported => write!(f, "Not supported on this target"),
        }
    }
}

impl std::error::Error for Error {}

/// CharLS codec instance
#[derive(Default)]
pub struct CharLS {
    codec: backend::Codec,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct FrameInfo {
    pub width: u32,
    pub height: u32,
    pub bits_per_sample: i32,
    pub component_count: i32,
}

/// Interleave modes for encoding/decoding.
///
/// Defines how color components are arranged in memory for both native and encoded formats.
/// See more: https://github.com/team-charls/charls/wiki/How-to-use-the-API#multi-component-image-typical-rgb-color
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InterleaveMode {
    /// No interleave mode.
    ///
    /// *Native Format:* by plane: `RRRRGGGGBBBB`
    /// *Encoded Format:* scan 1: `rrr`, scan 2: `ggg`, scan 3: `bbb`
    #[default]
    None,
    /// Line interleave mode.
    ///
    /// *Native Format:* by pixel: `RGBRGBRGBRGB`
    /// *Encoded Format:* scan 1: `rrrgggbbb`
    Line,
    /// Sample interleave mode.
    ///
    /// *Native Format:* by pixel: `RGBRGBRGBRGB`
    /// *Encoded Format:* scan 1: `rgbrgbrgb`
    Sample,
}

impl CharLS {
    pub fn decode_with_stride(&mut self, src: &[u8], stride: u32) -> CharlsResult<Vec<u8>> {
        self.codec.decode_with_stride(src, stride)
    }

    pub fn decode(&mut self, src: &[u8]) -> CharlsResult<Vec<u8>> {
        self.decode_with_stride(src, 0)
    }

    pub fn encode(&mut self, frame_info: FrameInfo, near: i32, src: &[u8]) -> CharlsResult<Vec<u8>> {
        self.codec.encode(frame_info, near, src)
    }

    pub fn get_frame_info(&mut self, src: &[u8]) -> CharlsResult<FrameInfo> {
        self.codec.frame_info(src)
    }

    /// Sets the interleave mode for the encoder.
    pub fn set_interleave_mode(&mut self, interleave_mode: InterleaveMode) -> CharlsResult<()> {
        self.codec.set_interleave_mode(interleave_mode)
    }
}

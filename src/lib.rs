//! Rust bindings for [CharLS](https://github.com/team-charls/charls),
//! the implementation of the JPEG-LS standard
//! for lossless and near-lossless image compression and decompression.
//!
//! # Cargo features
//!
//! - `static`: statically link CharLS.
//!   If this is not enabled,
//!   you need to install the CharLS (e.g. `libcharls.so`) into your system
//!   or add it to your library path (`LD_LIBRARY_PATH`).
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
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use charls_sys::*;
use std::ffi::CStr;

pub type CharlsResult<T> = Result<T, Error>;
pub type CharlsEncoder = *mut charls_jpegls_encoder;
pub type CharlsDecoder = *mut charls_jpegls_decoder;

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
        code: charls_jpegls_errc
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InitCodec => write!(f, "Unable to start the codec"),
            Error::ComputeSize => write!(f, "Unable to compute decompressed size"),
            Error::JpegLsError { code } => {
                let message = unsafe {
                    let msg = charls_get_error_message(*code);
                    CStr::from_ptr(msg)
                };
                write!(f, "{}", message.to_string_lossy())
            }
        }
    }
}

impl std::error::Error for Error {}

/// CharLS codec instance
#[derive(Default)]
pub struct CharLS {
    encoder: Option<CharlsEncoder>,
    decoder: Option<CharlsDecoder>,
}

#[derive(Default, Debug)]
pub struct FrameInfo {
    pub width: u32,
    pub height: u32,
    pub bits_per_sample: i32,
    pub component_count: i32,
}

impl CharLS {
    pub fn decode_with_stride(&mut self, src: &[u8], stride: u32) -> CharlsResult<Vec<u8>> {
        let decoder = self.decoder.unwrap_or_else(|| {
            self.decoder = Some(unsafe { charls_jpegls_decoder_create() });
            self.decoder.unwrap()
        });

        if decoder.is_null() {
            return Err(Error::InitCodec);
        }

        let err = unsafe {
            charls_jpegls_decoder_set_source_buffer(decoder, src.as_ptr() as _, src.len())
        };

        translate_error(err)?;

        let err = unsafe { charls_jpegls_decoder_read_header(decoder) };

        translate_error(err)?;

        let size = unsafe {
            let mut computed_size: usize = 0;
            let size_err =
                charls_jpegls_decoder_get_destination_size(decoder, stride, &mut computed_size);

            if size_err == 0 {
                Some(computed_size)
            } else {
                None
            }
        };

        match size {
            Some(size) => {
                let mut dst = vec![0; size];
                let err = unsafe {
                    charls_jpegls_decoder_decode_to_buffer(
                        decoder,
                        dst.as_mut_ptr() as _,
                        size,
                        stride,
                    )
                };

                translate_error(err)?;
                Ok(dst)
            }
            None => Err(Error::ComputeSize),
        }
    }

    pub fn decode(&mut self, src: &[u8]) -> CharlsResult<Vec<u8>> {
        self.decode_with_stride(src, 0)
    }

    pub fn encode(
        &mut self,
        frame_info: FrameInfo,
        near: i32,
        src: &[u8],
    ) -> CharlsResult<Vec<u8>> {
        let encoder = self.encoder.unwrap_or_else(|| {
            self.encoder = Some(unsafe { charls_jpegls_encoder_create() });
            self.encoder.unwrap()
        });

        if encoder.is_null() {
            return Err(Error::InitCodec);
        }

        let frame_info = charls_frame_info {
            width: frame_info.width,
            height: frame_info.height,
            bits_per_sample: frame_info.bits_per_sample,
            component_count: frame_info.component_count,
        };

        let err = unsafe {
            charls_jpegls_encoder_set_frame_info(encoder, &frame_info as *const charls_frame_info)
        };

        translate_error(err)?;

        let mut size = 0;
        let err =
            unsafe { charls_jpegls_encoder_get_estimated_destination_size(encoder, &mut size) };

        translate_error(err)?;

        let mut dst = vec![0; size];
        let err = unsafe {
            charls_jpegls_encoder_set_destination_buffer(
                encoder,
                dst.as_mut_ptr() as *mut std::os::raw::c_void,
                size,
            )
        };

        translate_error(err)?;

        let err = unsafe { charls_jpegls_encoder_set_near_lossless(encoder, near) };

        translate_error(err)?;

        let mut data = src.to_vec();
        let err = unsafe {
            charls_jpegls_encoder_encode_from_buffer(
                encoder,
                data.as_mut_ptr() as *mut std::os::raw::c_void,
                data.len(),
                0,
            )
        };

        translate_error(err)?;

        let err = unsafe { charls_jpegls_encoder_get_bytes_written(encoder, &mut size) };

        translate_error(err)?;

        dst.truncate(size);
        Ok(dst)
    }

    pub fn get_frame_info(&mut self, src: &[u8]) -> CharlsResult<FrameInfo>{
        let decoder = self.decoder.unwrap_or_else(|| {
            self.decoder = Some(unsafe { charls_jpegls_decoder_create() });
            self.decoder.unwrap()
        });

        if decoder.is_null() {
            return Err(Error::InitCodec);
        }

        let err = unsafe {
            charls_jpegls_decoder_set_source_buffer(decoder, src.as_ptr() as _, src.len())
        };
        translate_error(err)?;

        let err = unsafe { charls_jpegls_decoder_read_header(decoder) };
        translate_error(err)?;

        let mut frame_info = charls_frame_info {
            width: 0,
            height: 0,
            bits_per_sample: 0,
            component_count: 0,
        };
        let err = unsafe {
            charls_jpegls_decoder_get_frame_info(decoder, &mut frame_info)
        };
        translate_error(err)?;

        Ok(FrameInfo {
            width: frame_info.width,
            height: frame_info.height,
            bits_per_sample: frame_info.bits_per_sample,
            component_count: frame_info.component_count
        })
    }
}

impl Drop for CharLS {
    fn drop(&mut self) {
        if let Some(decoder) = self.decoder {
            unsafe {
                charls_jpegls_decoder_destroy(decoder);
            }
        }

        if let Some(encoder) = self.encoder {
            unsafe {
                charls_jpegls_encoder_destroy(encoder);
            }
        }
    }
}

fn translate_error(code: i32) -> CharlsResult<()> {
    if code != 0 {
        return Err(Error::JpegLsError { code });
    }

    Ok(())
}

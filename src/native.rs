//! The CharLS C library via `charls-sys`. Native only (builds with CMake).

use std::ffi::CStr;
use std::ptr::NonNull;

use charls_sys::*;

use crate::{CharlsResult, Error, FrameInfo, InterleaveMode};

pub type CharlsEncoder = *mut charls_jpegls_encoder;
pub type CharlsDecoder = *mut charls_jpegls_decoder;

pub fn error_message(code: crate::CharlsErrc) -> String {
    let message = unsafe {
        let msg = charls_get_error_message(code);
        CStr::from_ptr(msg)
    };
    message.to_string_lossy().into_owned()
}

/// CharLS encoders and decoders are single-use: after one stream they reject
/// further calls with `InvalidOperation`, and the 2.4 C API has no reset. So a
/// fresh handle is created for every operation; only the settings live here.
#[derive(Default)]
pub struct Codec {
    interleave_mode: InterleaveMode,
}

/// Owns a `charls_jpegls_decoder` and destroys it on drop.
struct Decoder(NonNull<charls_jpegls_decoder>);

impl Decoder {
    fn new() -> CharlsResult<Self> {
        NonNull::new(unsafe { charls_jpegls_decoder_create() }).map(Self).ok_or(Error::InitCodec)
    }

    fn as_ptr(&self) -> CharlsDecoder {
        self.0.as_ptr()
    }

    /// Points the decoder at `src` and parses the JPEG-LS header.
    fn read_header(&self, src: &[u8]) -> CharlsResult<()> {
        let err = unsafe { charls_jpegls_decoder_set_source_buffer(self.as_ptr(), src.as_ptr() as _, src.len()) };
        translate_error(err)?;
        let err = unsafe { charls_jpegls_decoder_read_header(self.as_ptr()) };
        translate_error(err)
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { charls_jpegls_decoder_destroy(self.as_ptr()) };
    }
}

/// Owns a `charls_jpegls_encoder` and destroys it on drop.
struct Encoder(NonNull<charls_jpegls_encoder>);

impl Encoder {
    fn new() -> CharlsResult<Self> {
        NonNull::new(unsafe { charls_jpegls_encoder_create() }).map(Self).ok_or(Error::InitCodec)
    }

    fn as_ptr(&self) -> CharlsEncoder {
        self.0.as_ptr()
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe { charls_jpegls_encoder_destroy(self.as_ptr()) };
    }
}

impl Codec {
    pub fn decode_with_stride(&mut self, src: &[u8], stride: u32) -> CharlsResult<Vec<u8>> {
        let guard = Decoder::new()?;
        guard.read_header(src)?;
        let decoder = guard.as_ptr();

        let size = unsafe {
            let mut computed_size: usize = 0;
            let size_err = charls_jpegls_decoder_get_destination_size(decoder, stride, &mut computed_size);
            if size_err == 0 { Some(computed_size) } else { None }
        };

        match size {
            Some(size) => {
                let mut dst = vec![0; size];
                let err = unsafe { charls_jpegls_decoder_decode_to_buffer(decoder, dst.as_mut_ptr() as _, size, stride) };
                translate_error(err)?;
                Ok(dst)
            }
            None => Err(Error::ComputeSize),
        }
    }

    pub fn encode(&mut self, frame_info: FrameInfo, near: i32, src: &[u8]) -> CharlsResult<Vec<u8>> {
        let guard = Encoder::new()?;
        let encoder = guard.as_ptr();

        let frame_info = charls_frame_info {
            width: frame_info.width,
            height: frame_info.height,
            bits_per_sample: frame_info.bits_per_sample,
            component_count: frame_info.component_count,
        };

        let err = unsafe { charls_jpegls_encoder_set_frame_info(encoder, &frame_info as *const charls_frame_info) };
        translate_error(err)?;

        let mut size = 0;
        let err = unsafe { charls_jpegls_encoder_get_estimated_destination_size(encoder, &mut size) };
        translate_error(err)?;

        let mut dst = vec![0; size];
        let err = unsafe {
            charls_jpegls_encoder_set_destination_buffer(encoder, dst.as_mut_ptr() as *mut std::os::raw::c_void, size)
        };
        translate_error(err)?;

        let err = unsafe { charls_jpegls_encoder_set_near_lossless(encoder, near) };
        translate_error(err)?;

        let interleave_mode = match self.interleave_mode {
            InterleaveMode::None => charls_interleave_mode_none,
            InterleaveMode::Line => charls_interleave_mode_line,
            InterleaveMode::Sample => charls_interleave_mode_sample,
        };
        let err = unsafe { charls_jpegls_encoder_set_interleave_mode(encoder, interleave_mode) };
        translate_error(err)?;

        let mut data = src.to_vec();
        let err = unsafe {
            charls_jpegls_encoder_encode_from_buffer(encoder, data.as_mut_ptr() as *mut std::os::raw::c_void, data.len(), 0)
        };
        translate_error(err)?;

        let err = unsafe { charls_jpegls_encoder_get_bytes_written(encoder, &mut size) };
        translate_error(err)?;

        dst.truncate(size);
        Ok(dst)
    }

    pub fn frame_info(&mut self, src: &[u8]) -> CharlsResult<FrameInfo> {
        let decoder = Decoder::new()?;
        decoder.read_header(src)?;

        let mut frame_info = charls_frame_info {
            width: 0,
            height: 0,
            bits_per_sample: 0,
            component_count: 0,
        };
        let err = unsafe { charls_jpegls_decoder_get_frame_info(decoder.as_ptr(), &mut frame_info) };
        translate_error(err)?;

        Ok(FrameInfo {
            width: frame_info.width,
            height: frame_info.height,
            bits_per_sample: frame_info.bits_per_sample,
            component_count: frame_info.component_count,
        })
    }

    /// Applies to every later `encode`.
    pub fn set_interleave_mode(&mut self, interleave_mode: InterleaveMode) -> CharlsResult<()> {
        self.interleave_mode = interleave_mode;
        Ok(())
    }
}

fn translate_error(code: i32) -> CharlsResult<()> {
    if code != 0 {
        return Err(Error::JpegLsError { code });
    }
    Ok(())
}

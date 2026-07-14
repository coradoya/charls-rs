//! `@team-charls/charls-wasm` (npm) via `wasm-bindgen`. The bundler resolves the
//! package and its `.wasm`.
//!
//! The module instantiates asynchronously but `CharLS::default()` is sync and
//! infallible, so it is loaded once via [`init`]; decoding is sync afterwards,
//! keeping the public API identical on both targets. Decoding before [`init`] is
//! [`Error::Uninitialised`] — a wasm main thread cannot block on a promise.

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use crate::{CharlsResult, Error, FrameInfo, InterleaveMode};

#[wasm_bindgen(module = "/src/charls_wasm.js")]
extern "C" {
    #[wasm_bindgen(js_name = "initCharls", catch)]
    async fn js_init() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = "isInitialised")]
    fn js_is_initialised() -> bool;

    #[wasm_bindgen(js_name = "decodeJpegLs", catch)]
    fn js_decode(source: &[u8]) -> Result<Uint8Array, JsValue>;

    #[wasm_bindgen(js_name = "frameInfo", catch)]
    fn js_frame_info(source: &[u8]) -> Result<Vec<i32>, JsValue>;
}

/// Loads the codec. Await once before any decode.
pub async fn init() -> CharlsResult<()> {
    js_init().await.map(|_| ()).map_err(|_| Error::InitCodec)
}

/// The wasm build throws JS exceptions rather than returning CharLS error codes.
pub fn error_message(code: crate::CharlsErrc) -> String {
    format!("JPEG-LS error {code}")
}

#[derive(Default)]
pub struct Codec;

impl Codec {
    pub fn decode_with_stride(&mut self, src: &[u8], stride: u32) -> CharlsResult<Vec<u8>> {
        // The wasm wrapper always decodes tightly packed; honouring a stride is
        // not possible and ignoring one would corrupt the image.
        if stride != 0 {
            return Err(Error::Unsupported);
        }
        if !js_is_initialised() {
            return Err(Error::Uninitialised);
        }
        js_decode(src).map(|decoded| decoded.to_vec()).map_err(|_| Error::ComputeSize)
    }

    pub fn encode(&mut self, _frame_info: FrameInfo, _near: i32, _src: &[u8]) -> CharlsResult<Vec<u8>> {
        Err(Error::Unsupported)
    }

    pub fn frame_info(&mut self, src: &[u8]) -> CharlsResult<FrameInfo> {
        if !js_is_initialised() {
            return Err(Error::Uninitialised);
        }
        let info = js_frame_info(src).map_err(|_| Error::InitCodec)?;
        let [width, height, bits_per_sample, component_count] = info[..] else {
            return Err(Error::InitCodec);
        };
        Ok(FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample,
            component_count,
        })
    }

    pub fn set_interleave_mode(&mut self, _interleave_mode: InterleaveMode) -> CharlsResult<()> {
        Err(Error::Unsupported)
    }
}

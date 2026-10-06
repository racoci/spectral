//! Adaptadores de Fronteira WebAssembly (FFI) para o Codec RDO-Jet
//!
//! Encapsula a compressão por zona morta e descompressão binária compacta.

use wasm_bindgen::prelude::*;
use super::super::infrastructure::audio_decoding::RateDistortionOptimizedCodec;

#[wasm_bindgen]
pub fn wasm_rdo_jet_compress_frame(samples: &[f32], rate_distortion_lambda: f32) -> Vec<u8> {
    RateDistortionOptimizedCodec::compress_frame(samples, rate_distortion_lambda)
}

#[wasm_bindgen]
pub fn wasm_rdo_jet_decompress_frame(packet: &[u8]) -> Vec<f32> {
    RateDistortionOptimizedCodec::decompress_frame(packet)
}

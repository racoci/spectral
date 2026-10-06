//! Adaptadores de Fronteira WebAssembly (FFI)
//!
//! Encapsula estritamente a conversão de tipos de ponteiros JavaScript para
//! requisições de casos de uso sem nenhuma lógica matemática interna.

use wasm_bindgen::prelude::*;
use super::super::application_use_cases::{GenerateHolomorphicTransformUseCase, HolomorphicTransformRequest};

#[wasm_bindgen]
pub fn wasm_new_architecture_generate_holomorphic_spectrogram(
    audio_bytes: &[u8],
    height_custom: usize,
    minimum_frequency_custom: f32,
    maximum_frequency_custom: f32,
    scale_type: &str,
    palette_type: &str,
    column_start_index: usize,
    column_end_index: usize,
    shape_parameter_q_custom: f32,
    enable_reassignment: bool,
    enable_tight_frame: bool,
) -> Vec<u8> {
    GenerateHolomorphicTransformUseCase::execute(HolomorphicTransformRequest {
        audio_bytes,
        height: height_custom,
        minimum_frequency: minimum_frequency_custom,
        maximum_frequency: maximum_frequency_custom,
        scale_type,
        palette_type,
        column_start: column_start_index,
        column_end: column_end_index,
        shape_parameter_q: shape_parameter_q_custom,
        enable_reassignment,
        enable_tight_frame,
    })
}

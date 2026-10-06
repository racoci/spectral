//! Adaptadores de Fronteira WebAssembly (FFI) para Geração de Espectrogramas
//!
//! Encapsula estritamente a conversão de tipos de ponteiros JavaScript para
//! requisições de casos de uso sem nenhuma lógica matemática interna.

use wasm_bindgen::prelude::*;
use super::super::application_use_cases::{
    GenerateHolomorphicTransformUseCase, HolomorphicTransformRequest,
    GenerateShortTimeFourierTransformUseCase, ShortTimeFourierTransformRequest,
    GenerateConstantQTransformUseCase, ConstantQTransformRequest,
};

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

#[wasm_bindgen]
pub fn wasm_new_architecture_generate_short_time_fourier_transform(
    audio_bytes: &[u8],
    height_custom: usize,
    window_size: usize,
    zero_padding_factor: usize,
    minimum_frequency: f32,
    maximum_frequency: f32,
    is_reassigned_mode: bool,
    palette_type: &str,
    view_start_ratio: f32,
    view_end_ratio: f32,
    point_radius: f32,
    frequency_scale_type: &str,
    horizontal_resolution_factor: f32,
    level_of_detail: usize,
    enable_time_reassignment: bool,
    enable_frequency_reassignment: bool,
    maximum_derivative_order: usize,
) -> Vec<u8> {
    GenerateShortTimeFourierTransformUseCase::execute(ShortTimeFourierTransformRequest {
        audio_bytes,
        height: height_custom,
        window_size,
        zero_padding_factor,
        minimum_frequency,
        maximum_frequency,
        is_reassigned_mode,
        palette_type,
        view_start_ratio,
        view_end_ratio,
        point_radius,
        frequency_scale_type,
        horizontal_resolution_factor,
        level_of_detail,
        enable_time_reassignment,
        enable_frequency_reassignment,
        maximum_derivative_order,
    })
}

#[wasm_bindgen]
pub fn wasm_new_architecture_generate_constant_q_transform(
    audio_bytes: &[u8],
    height_custom: usize,
    minimum_frequency: f32,
    maximum_frequency: f32,
    palette_type: &str,
    view_start_ratio: f32,
    view_end_ratio: f32,
    point_radius: f32,
    horizontal_resolution_factor: f32,
    level_of_detail: usize,
    enable_time_reassignment: bool,
    enable_frequency_reassignment: bool,
    maximum_derivative_order: usize,
) -> Vec<u8> {
    GenerateConstantQTransformUseCase::execute(ConstantQTransformRequest {
        audio_bytes,
        height: height_custom,
        minimum_frequency,
        maximum_frequency,
        palette_type,
        view_start_ratio,
        view_end_ratio,
        point_radius,
        horizontal_resolution_factor,
        level_of_detail,
        enable_time_reassignment,
        enable_frequency_reassignment,
        maximum_derivative_order,
    })
}

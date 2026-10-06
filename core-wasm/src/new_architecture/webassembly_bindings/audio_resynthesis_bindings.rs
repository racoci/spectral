//! Adaptadores de Fronteira WebAssembly (FFI) para Ressíntese de Áudio
//!
//! Encapsula a invocação do caso de uso de reconstrução acústica bit-perfect.

use wasm_bindgen::prelude::*;
use super::super::application_use_cases::{BitPerfectAudioSynthesisUseCase, AudioSynthesisRequest};

#[wasm_bindgen]
pub fn wasm_synthesize_hybrid_spectrogram_to_wav(
    original_audio_bytes: &[u8],
    rgba_grid: &[u8],
    width: usize,
    height: usize,
    minimum_frequency: f32,
    maximum_frequency: f32,
    frequency_scale_type: &str,
    window_size: usize,
    zero_padding_factor: usize,
    start_column: usize,
    end_column: usize,
    hop_size: usize,
    sampling_rate_hz: u32,
    is_region_edited: bool,
) -> Vec<u8> {
    BitPerfectAudioSynthesisUseCase::execute(AudioSynthesisRequest {
        original_audio_bytes,
        rgba_grid,
        width,
        height,
        minimum_frequency,
        maximum_frequency,
        frequency_scale_type,
        window_size,
        zero_padding_factor,
        start_column,
        end_column,
        hop_size,
        sampling_rate_hz,
        is_region_edited,
    })
}

#[wasm_bindgen]
pub fn wasm_synthesize_spectrogram_to_wav(
    rgba_grid: &[u8],
    width: usize,
    height: usize,
    minimum_frequency: f32,
    maximum_frequency: f32,
    frequency_scale_type: &str,
    window_size: usize,
    zero_padding_factor: usize,
    start_column: usize,
    end_column: usize,
    hop_size: usize,
    sampling_rate_hz: u32,
) -> Vec<u8> {
    BitPerfectAudioSynthesisUseCase::execute(AudioSynthesisRequest {
        original_audio_bytes: &[],
        rgba_grid,
        width,
        height,
        minimum_frequency,
        maximum_frequency,
        frequency_scale_type,
        window_size,
        zero_padding_factor,
        start_column,
        end_column,
        hop_size,
        sampling_rate_hz,
        is_region_edited: true,
    })
}

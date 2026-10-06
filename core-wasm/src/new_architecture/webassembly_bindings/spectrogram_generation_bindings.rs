//! Adaptadores de Fronteira WebAssembly (FFI) para Geração de Espectrogramas
//!
//! Encapsula estritamente a conversão de tipos de ponteiros JavaScript para
//! requisições de casos de uso sem nenhuma lógica matemática interna.

use wasm_bindgen::prelude::*;
use super::super::application_use_cases::{
    GenerateHolomorphicTransformUseCase, HolomorphicTransformRequest,
    GenerateShortTimeFourierTransformUseCase, ShortTimeFourierTransformRequest,
    GenerateConstantQTransformUseCase, ConstantQTransformRequest,
    HigherOrderDerivativesUseCase, HigherOrderDerivativesRequest,
    SlidingDifferentialJetUseCase, SlidingDifferentialJetRequest,
};
use super::super::infrastructure::audio_decoding::WaveAudioParser;

#[wasm_bindgen]
pub fn wasm_get_spectrogram_dimensions(
    data: &[u8],
    height_custom: usize,
    horizontal_resolution_factor: usize,
    quality_level_of_detail: usize,
    time_start_ratio: f32,
    time_end_ratio: f32,
) -> Vec<usize> {
    let is_draft = quality_level_of_detail >= 1;
    let mut h = if is_draft { 256 } else { height_custom };
    if !h.is_power_of_two() || h < 4 { h = 512; }

    let wav_info = match WaveAudioParser::parse_pcm_16bit(data) {
        Some(info) => info,
        None => return vec![0, 0, 0, 0, 0],
    };

    let num_samples = wav_info.samples.len();
    let start_sample = ((time_start_ratio.clamp(0.0, 1.0) * num_samples as f32) as usize).min(num_samples.saturating_sub(2));
    let end_sample = ((time_end_ratio.clamp(0.0, 1.0) * num_samples as f32) as usize).clamp(start_sample + 2, num_samples);
    let sliced_samples = end_sample - start_sample;

    let target_cols = if is_draft {
        512usize
    } else {
        let k_clamped = horizontal_resolution_factor.min(4);
        (1024usize << k_clamped).max(512)
    };
    let hop = (sliced_samples / target_cols).max(1);
    let w = (sliced_samples / hop).max(2);

    vec![w, h, hop, wav_info.sampling_rate_hz as usize, num_samples]
}

#[inline(always)]
fn cubic_hermite_interpolation(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c = -0.5 * p0 + 0.5 * p2;
    let d = p1;
    a * t * t * t + b * t * t + c * t + d
}

#[wasm_bindgen]
pub fn wasm_bicubic_resample_spectrogram(
    source_grid: &[u8],
    source_width: usize,
    source_height: usize,
    source_time_start: f32,
    source_time_end: f32,
    source_frequency_min: f32,
    source_frequency_max: f32,
    target_width: usize,
    target_height: usize,
    target_time_start: f32,
    target_time_end: f32,
    target_frequency_min: f32,
    target_frequency_max: f32,
) -> Vec<u8> {
    if source_width < 2 || source_height < 2 || target_width == 0 || target_height == 0 || source_grid.len() < source_width * source_height * 4 {
        return vec![0u8; target_width * target_height * 4];
    }

    let log_src_fmin = source_frequency_min.max(1.0).log2();
    let log_src_fmax = source_frequency_max.max(source_frequency_min + 1.0).log2();
    let log_src_span = (log_src_fmax - log_src_fmin).max(1e-6);

    let log_dst_fmin = target_frequency_min.max(1.0).log2();
    let log_dst_fmax = target_frequency_max.max(target_frequency_min + 1.0).log2();
    let log_dst_span = (log_dst_fmax - log_dst_fmin).max(1e-6);

    let src_t_span = (source_time_end - source_time_start).max(1e-6);
    let dst_t_span = (target_time_end - target_time_start).max(1e-6);

    let mut x_int_lut = Vec::with_capacity(target_width);
    let mut x_frac_lut = Vec::with_capacity(target_width);

    for c in 0..target_width {
        let t_ratio = if target_width > 1 { c as f32 / (target_width - 1) as f32 } else { 0.0 };
        let t = target_time_start + t_ratio * dst_t_span;
        let norm_x = (t - source_time_start) / src_t_span;
        let x_f = norm_x * (source_width - 1) as f32;
        let x_clamped = x_f.clamp(0.0, (source_width - 1) as f32);
        let xi = x_clamped.floor() as isize;
        let xf = x_clamped - xi as f32;
        x_int_lut.push(xi);
        x_frac_lut.push(xf);
    }

    let mut destination_grid = vec![0u8; target_width * target_height * 4];

    for r in 0..target_height {
        let freq_ratio = if target_height > 1 { r as f32 / (target_height - 1) as f32 } else { 0.0 };
        let log_f = log_dst_fmin + freq_ratio * log_dst_span;
        let norm_y = (log_f - log_src_fmin) / log_src_span;
        let y_f = norm_y * (source_height - 1) as f32;
        let y_clamped = y_f.clamp(0.0, (source_height - 1) as f32);
        let yi = y_clamped.floor() as isize;
        let yf = y_clamped - yi as f32;

        let y_m1 = (yi - 1).clamp(0, (source_height - 1) as isize) as usize * source_width * 4;
        let y_0 = yi.clamp(0, (source_height - 1) as isize) as usize * source_width * 4;
        let y_1 = (yi + 1).clamp(0, (source_height - 1) as isize) as usize * source_width * 4;
        let y_2 = (yi + 2).clamp(0, (source_height - 1) as isize) as usize * source_width * 4;

        let dst_row = r * target_width * 4;

        for c in 0..target_width {
            let xi = x_int_lut[c];
            let u = x_frac_lut[c];

            let x_m1 = (xi - 1).clamp(0, (source_width - 1) as isize) as usize * 4;
            let x_0 = xi.clamp(0, (source_width - 1) as isize) as usize * 4;
            let x_1 = (xi + 1).clamp(0, (source_width - 1) as isize) as usize * 4;
            let x_2 = (xi + 2).clamp(0, (source_width - 1) as isize) as usize * 4;

            let dst_idx = dst_row + c * 4;

            for ch in 0..3 {
                let p00 = source_grid[y_m1 + x_m1 + ch] as f32;
                let p01 = source_grid[y_m1 + x_0 + ch] as f32;
                let p02 = source_grid[y_m1 + x_1 + ch] as f32;
                let p03 = source_grid[y_m1 + x_2 + ch] as f32;
                let row0 = cubic_hermite_interpolation(p00, p01, p02, p03, u);

                let p10 = source_grid[y_0 + x_m1 + ch] as f32;
                let p11 = source_grid[y_0 + x_0 + ch] as f32;
                let p12 = source_grid[y_0 + x_1 + ch] as f32;
                let p13 = source_grid[y_0 + x_2 + ch] as f32;
                let row1 = cubic_hermite_interpolation(p10, p11, p12, p13, u);

                let p20 = source_grid[y_1 + x_m1 + ch] as f32;
                let p21 = source_grid[y_1 + x_0 + ch] as f32;
                let p22 = source_grid[y_1 + x_1 + ch] as f32;
                let p23 = source_grid[y_1 + x_2 + ch] as f32;
                let row2 = cubic_hermite_interpolation(p20, p21, p22, p23, u);

                let p30 = source_grid[y_2 + x_m1 + ch] as f32;
                let p31 = source_grid[y_2 + x_0 + ch] as f32;
                let p32 = source_grid[y_2 + x_1 + ch] as f32;
                let p33 = source_grid[y_2 + x_2 + ch] as f32;
                let row3 = cubic_hermite_interpolation(p30, p31, p32, p33, u);

                let val = cubic_hermite_interpolation(row0, row1, row2, row3, yf);
                destination_grid[dst_idx + ch] = val.clamp(0.0, 255.0).round() as u8;
            }
            destination_grid[dst_idx + 3] = 255;
        }
    }

    destination_grid
}

#[wasm_bindgen]
pub fn wasm_generate_complex_reassigned_ycbcr_spectrogram(
    data: &[u8],
    h_custom: usize,
    _window_type: &str,
    window_size: usize,
    zero_padding: usize,
    fmin_custom: f32,
    fmax_custom: f32,
    algorithm_type: &str,
    palette_type: &str,
    t_start: f32,
    t_end: f32,
    point_radius: f32,
    scale_type: &str,
    horizontal_res_k: usize,
    quality_lod: usize,
    _chunk_start: usize,
    _chunk_count: usize,
    enable_time_reassign: Option<bool>,
    enable_freq_reassign: Option<bool>,
    max_derivative_order: Option<usize>,
) -> Vec<u8> {
    let enable_time = enable_time_reassign.unwrap_or(true);
    let enable_freq = enable_freq_reassign.unwrap_or(true);
    let max_order = max_derivative_order.unwrap_or(1);

    if algorithm_type.starts_with("higher_order") {
        let parts: Vec<&str> = algorithm_type.split(':').collect();
        let o = if parts.len() > 1 { parts[1].parse::<usize>().unwrap_or(2).clamp(1, 4) } else { 2 };
        let mode = if parts.len() > 2 {
            match parts[2] {
                "ridge" => 0,
                "anisotropy" => 1,
                "curvature" => 2,
                _ => 0,
            }
        } else {
            0
        };
        HigherOrderDerivativesUseCase::execute(HigherOrderDerivativesRequest {
            audio_bytes: data,
            height: h_custom,
            window_size,
            zero_padding_factor: zero_padding,
            minimum_frequency: fmin_custom,
            maximum_frequency: fmax_custom,
            derivative_order: o,
            visualization_mode: mode,
            palette_type,
            view_start_ratio: t_start,
            view_end_ratio: t_end,
            frequency_scale_type: scale_type,
        })
    } else if algorithm_type.starts_with("sliding_jet") {
        let parts: Vec<&str> = algorithm_type.split(':').collect();
        let o = if parts.len() > 1 { parts[1].parse::<usize>().unwrap_or(2).clamp(1, 4) } else { 2 };
        let mode = if parts.len() > 2 {
            match parts[2] {
                "ridge" => 0,
                "anisotropy" => 1,
                "curvature" => 2,
                _ => 0,
            }
        } else {
            0
        };
        SlidingDifferentialJetUseCase::execute(SlidingDifferentialJetRequest {
            audio_bytes: data,
            height: h_custom,
            window_size,
            minimum_frequency: fmin_custom,
            maximum_frequency: fmax_custom,
            derivative_order: o,
            visualization_mode: mode,
            palette_type,
            view_start_ratio: t_start,
            view_end_ratio: t_end,
            frequency_scale_type: scale_type,
        })
    } else if algorithm_type == "cqt" {
        GenerateConstantQTransformUseCase::execute(ConstantQTransformRequest {
            audio_bytes: data,
            height: h_custom,
            minimum_frequency: fmin_custom,
            maximum_frequency: fmax_custom,
            palette_type,
            view_start_ratio: t_start,
            view_end_ratio: t_end,
            point_radius,
            horizontal_resolution_factor: horizontal_res_k as f32,
            level_of_detail: quality_lod,
            enable_time_reassignment: enable_time,
            enable_frequency_reassignment: enable_freq,
            maximum_derivative_order: max_order,
        })
    } else if algorithm_type == "holomorphic" {
        GenerateHolomorphicTransformUseCase::execute(HolomorphicTransformRequest {
            audio_bytes: data,
            height: h_custom,
            minimum_frequency: fmin_custom,
            maximum_frequency: fmax_custom,
            scale_type,
            palette_type,
            column_start: 0,
            column_end: 1000,
            shape_parameter_q: 2.0,
            enable_reassignment: enable_time || enable_freq,
            enable_tight_frame: true,
        })
    } else {
        let is_reassign = algorithm_type == "reassignment";
        GenerateShortTimeFourierTransformUseCase::execute(ShortTimeFourierTransformRequest {
            audio_bytes: data,
            height: h_custom,
            window_size,
            zero_padding_factor: zero_padding,
            minimum_frequency: fmin_custom,
            maximum_frequency: fmax_custom,
            is_reassigned_mode: is_reassign,
            palette_type,
            view_start_ratio: t_start,
            view_end_ratio: t_end,
            point_radius,
            frequency_scale_type: scale_type,
            horizontal_resolution_factor: horizontal_res_k as f32,
            level_of_detail: quality_lod,
            enable_time_reassignment: enable_time,
            enable_frequency_reassignment: enable_freq,
            maximum_derivative_order: max_order,
        })
    }
}

#[wasm_bindgen]
pub fn wasm_generate_holomorphic_exploration_spectrogram(
    data: &[u8],
    h_custom: usize,
    fmin_custom: f32,
    fmax_custom: f32,
    scale_type: &str,
    palette_type: &str,
    c_start_in: usize,
    c_end_in: usize,
    q_param_custom: f32,
    enable_reassignment: bool,
    enable_tight_frame: bool,
) -> Vec<u8> {
    GenerateHolomorphicTransformUseCase::execute(HolomorphicTransformRequest {
        audio_bytes: data,
        height: h_custom,
        minimum_frequency: fmin_custom,
        maximum_frequency: fmax_custom,
        scale_type,
        palette_type,
        column_start: c_start_in,
        column_end: c_end_in,
        shape_parameter_q: q_param_custom,
        enable_reassignment,
        enable_tight_frame,
    })
}

//! Módulo de Exploração Holomorfa Dedicado (WASM)
//!
//! Implementação 100% isolada para exploração de campos holomorfos multiescala:
//! - Escalas: CQT (lambda=0), Mel (lambda=700), Bark (lambda=1960), Linear
//! - Filtros com centralização logarítmica e normalização L2 unitária
//! - Densidade analítica de frame tight rho(y)
//! - Convenção de visualização YCbCr com preservação estrita de fase

use wasm_bindgen::prelude::*;
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::{PI, TAU};

#[wasm_bindgen]
pub fn wasm_generate_holomorphic_exploration_spectrogram(
    data: &[u8],
    h_custom: usize,
    fmin_custom: f32,
    fmax_custom: f32,
    scale_type: &str,       // "cqt", "mel", "bark", "linear"
    palette_type: &str,     // "ycbcr", "snake"
    c_start_in: usize,
    c_end_in: usize,
    q_param_custom: f32,
    enable_reassignment: bool,
    enable_tight_frame: bool,
) -> Vec<u8> {
    if data.len() < 44 {
        return Vec::new();
    }

    // 1. Extração segura de parâmetros do cabeçalho WAV (PCM 16-bit)
    let num_channels = u16::from_le_bytes([data[22], data[23]]) as usize;
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let fs = if sample_rate > 0 { sample_rate as f32 } else { 48000.0 };

    let pcm_bytes = &data[44..];
    let total_samples = pcm_bytes.len() / (2 * num_channels.max(1));
    if total_samples == 0 {
        return Vec::new();
    }

    // Média de canais para sinal mono f32
    let mut signal = Vec::with_capacity(total_samples);
    for i in 0..total_samples {
        let mut sample_acc = 0.0f32;
        for ch in 0..num_channels {
            let offset = (i * num_channels + ch) * 2;
            if offset + 1 < pcm_bytes.len() {
                let s16 = i16::from_le_bytes([pcm_bytes[offset], pcm_bytes[offset + 1]]) as f32;
                sample_acc += s16 / 32768.0;
            }
        }
        signal.push(sample_acc / num_channels as f32);
    }

    let h = if h_custom > 0 { h_custom } else { 600 };
    let fmin = if fmin_custom > 0.0 { fmin_custom } else { 50.0 };
    let fmax = if fmax_custom > fmin { fmax_custom } else { (fs * 0.5).min(12000.0) };
    let q_param = if q_param_custom > 0.1 { q_param_custom } else { 2.0 };

    // Geometria da FFT e Janela
    let n_stft = 2048usize;
    let hop = 232usize;
    let w = (total_samples.saturating_sub(n_stft) / hop).max(1);

    let c_start = c_start_in.min(w.saturating_sub(1));
    let c_end = c_end_in.clamp(c_start + 1, w);
    let chunk_w = c_end - c_start;

    // 2. Parâmetros da Escala Perceptual e Polo Deslocado lambda
    let is_linear = scale_type == "linear";
    let is_mel = scale_type == "mel";
    let is_bark = scale_type == "bark";

    let lambda = if is_mel {
        700.0f32
    } else if is_bark {
        1960.0f32
    } else {
        0.0f32
    };

    let step_lin = (fmax - fmin) / (h as f32 - 1.0).max(1.0);
    let mel_min = 2595.0 * (1.0 + fmin / 700.0).log2();
    let mel_max = 2595.0 * (1.0 + fmax / 700.0).log2();
    let mel_step = (mel_max - mel_min) / (h as f32 - 1.0).max(1.0);

    let bark_min = 26.81 * (fmin / (1960.0 + fmin)) - 0.53;
    let bark_max = 26.81 * (fmax / (1960.0 + fmax)) - 0.53;
    let bark_step = (bark_max - bark_min) / (h as f32 - 1.0).max(1.0);

    let cqt_step = (fmax / fmin).log2() / (h as f32 - 1.0).max(1.0);

    // 3. Pré-cálculo da LUT de Filtros com Normalização L2 Unitária e Densidade de Frame
    let df = fs / n_stft as f32;
    let mut fc_lut = vec![0.0f32; h];
    let mut k_low_lut = vec![0usize; h];
    let mut k_high_lut = vec![0usize; h];
    let mut kernels_lut: Vec<Vec<(f32, f32)>> = vec![Vec::new(); h];

    for j in 0..h {
        let fc = if is_linear {
            fmin + j as f32 * step_lin
        } else if is_mel {
            let mel_val = mel_min + j as f32 * mel_step;
            700.0 * (2.0f32.powf(mel_val / 2595.0) - 1.0)
        } else if is_bark {
            let bark_val = bark_min + j as f32 * bark_step;
            (1960.0 * (bark_val + 0.53) / (26.28 - bark_val)).max(fmin)
        } else {
            fmin * 2.0f32.powf(j as f32 * cqt_step)
        };
        fc_lut[j] = fc;

        let f_low = (((fc + lambda) * 0.15) - lambda).max(fmin);
        let f_high = (((fc + lambda) * 3.5) - lambda).min(fs * 0.5);

        let k_low = (f_low * n_stft as f32 / fs).round() as isize;
        let k_high = (f_high * n_stft as f32 / fs).round() as isize;
        let k_low_u = k_low.clamp(1, (n_stft / 2 - 2) as isize) as usize;
        let k_high_u = k_high.clamp(k_low_u as isize + 1, (n_stft / 2 - 1) as isize) as usize;

        k_low_lut[j] = k_low_u;
        k_high_lut[j] = k_high_u;

        let mut kernel = Vec::with_capacity(k_high_u - k_low_u + 1);
        let mut energy_sum = 0.0f32;

        for curr_k in k_low_u..=k_high_u {
            let f_k = curr_k as f32 * df;
            let u = (f_k + lambda) / (fc + lambda);
            if u > 1e-4 {
                // Centralização logarítmica: base u * exp(1 - u) possui pico exato 1.0 em u = 1
                let base = u * (1.0 - u).exp();
                let h0_val = base.powf(q_param);
                let h1_val = u * h0_val;
                kernel.push((h0_val, h1_val));
                energy_sum += h0_val * h0_val * df;
            } else {
                kernel.push((0.0, 0.0));
            }
        }

        // Normalização L^2 unitária
        let n2 = if energy_sum > 1e-30 {
            1.0 / energy_sum.sqrt()
        } else {
            1.0
        };

        // Ponderação analítica de frame tight
        let rho_factor = if enable_tight_frame {
            let eta = q_param / (fc + lambda);
            if is_mel {
                let log_rho = (4.0 * PI * q_param) * eta.ln() - (2800.0 * PI * eta);
                (log_rho * 0.5).exp().clamp(0.01, 10.0)
            } else if is_bark {
                let log_rho = (4.0 * PI * q_param - 1.0) * eta.ln() - (7840.0 * PI * eta);
                (log_rho * 0.5).exp().clamp(0.01, 10.0)
            } else {
                (fc / fmin).sqrt().clamp(0.5, 15.0)
            }
        } else {
            1.0f32
        };

        for k in 0..kernel.len() {
            kernel[k].0 *= n2 * rho_factor;
            kernel[k].1 *= n2 * rho_factor;
        }

        kernels_lut[j] = kernel;
    }

    // 4. Execução da Transformada de Fourier
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);

    let mut fft_buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
    let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

    let mut reassigned_grid_re = vec![0.0f32; h * w];
    let mut reassigned_grid_im = vec![0.0f32; h * w];
    let mut max_abs_z = 1e-12f32;

    // Janela Gaussiana de Análise com dispersão sigma = 0.25 * win_len
    let win_len = 1024usize.min(n_stft);
    let sigma = 0.25f32 * (win_len as f32 * 0.5);
    let mut gauss_win = vec![0.0f32; win_len];
    for i in 0..win_len {
        let t = (i as f32) - (win_len as f32 * 0.5);
        gauss_win[i] = (-0.5 * (t * t) / (sigma * sigma)).exp();
    }

    for c in c_start..c_end {
        let center_sample = c * hop + n_stft / 2;
        let half_win = (win_len as isize) / 2;

        for (i, slot) in fft_buffer.iter_mut().enumerate() {
            let s_idx = center_sample as isize - half_win + i as isize;
            if i < win_len && s_idx >= 0 && (s_idx as usize) < signal.len() {
                *slot = Complex::new(signal[s_idx as usize] * gauss_win[i], 0.0);
            } else {
                *slot = Complex::new(0.0, 0.0);
            }
        }

        fft.process_with_scratch(&mut fft_buffer, &mut scratch);

        for j in 0..h {
            let fc = fc_lut[j];
            let k_low = k_low_lut[j];
            let k_high = k_high_lut[j];
            let kernel = &kernels_lut[j];

            let mut w0_re = 0.0f32;
            let mut w0_im = 0.0f32;
            let mut w1_re = 0.0f32;
            let mut w1_im = 0.0f32;

            for (i, curr_k) in (k_low..=k_high).enumerate() {
                let (h0, h1) = kernel[i];
                if h0 > 1e-6 {
                    let bin = fft_buffer[curr_k];
                    w0_re += bin.re * h0;
                    w0_im += bin.im * h0;

                    w1_re += bin.re * h1;
                    w1_im += bin.im * h1;
                }
            }

            let abs_sq = w0_re * w0_re + w0_im * w0_im;
            if abs_sq > 1e-12 {
                let w0 = Complex::new(w0_re, w0_im);
                let w1 = Complex::new(w1_re, w1_im);

                if enable_reassignment {
                    let w1_w0_conj = w1 * w0.conj();
                    let re_r = w1_w0_conj.re / abs_sq;
                    let im_r = w1_w0_conj.im / abs_sq;

                    let f_reassigned = (((fc + lambda) * re_r) - lambda).clamp(fmin * 0.5, fmax * 1.5);
                    let p_period = 1.0 / (fc + lambda);
                    let t_shift_s = -(q_param * p_period / (2.0 * PI)) * im_r;
                    let c_reassigned_f = (c as f32) + (t_shift_s * fs) / hop as f32;

                    let j_reassigned_f = if is_linear {
                        ((f_reassigned - fmin) / (fmax - fmin)) * (h as f32 - 1.0)
                    } else if is_mel {
                        let mel_val = 2595.0 * (1.0 + f_reassigned.max(0.0) / 700.0).log2();
                        ((mel_val - mel_min) / (mel_max - mel_min)) * (h as f32 - 1.0)
                    } else if is_bark {
                        let bark_val = 26.81 * (f_reassigned.max(0.0) / (1960.0 + f_reassigned.max(0.0))) - 0.53;
                        ((bark_val - bark_min) / (bark_max - bark_min)) * (h as f32 - 1.0)
                    } else {
                        (f_reassigned / fmin).log2() / cqt_step
                    };

                    let c_idx = c_reassigned_f.round() as isize;
                    let j_idx = j_reassigned_f.round() as isize;

                    if c_idx >= 0 && c_idx < w as isize && j_idx >= 0 && j_idx < h as isize {
                        let target = j_idx as usize * w + c_idx as usize;
                        reassigned_grid_re[target] += w0.re;
                        reassigned_grid_im[target] += w0.im;
                    }
                } else {
                    let target = j * w + c;
                    reassigned_grid_re[target] = w0.re;
                    reassigned_grid_im[target] = w0.im;
                }

                let mag = abs_sq.sqrt();
                if mag > max_abs_z {
                    max_abs_z = mag;
                }
            }
        }
    }

    // 5. Renderização de Cores
    let mut rgba_buffer = vec![0u8; chunk_w * h * 4];

    for row in 0..h {
        for col in c_start..c_end {
            let grid_idx = row * w + col;
            let out_idx = (row * chunk_w + (col - c_start)) * 4;

            let re = reassigned_grid_re[grid_idx];
            let im = reassigned_grid_im[grid_idx];
            let abs_z = (re * re + im * im).sqrt();

            if abs_z < 1e-12 {
                rgba_buffer[out_idx] = 0;
                rgba_buffer[out_idx + 1] = 0;
                rgba_buffer[out_idx + 2] = 0;
                rgba_buffer[out_idx + 3] = 255;
                continue;
            }

            let abs_norm = (abs_z / max_abs_z).clamp(0.0, 1.0);

            if palette_type == "snake" {
                // Geodesic Snake Térmica 24-bit
                let level = (abs_norm * 65535.0).round() as usize;
                let r = (level % 256) as u8;
                let g = ((level / 256) % 256) as u8;
                let b = (r as i32 - g as i32).abs() as u8;
                rgba_buffer[out_idx] = r;
                rgba_buffer[out_idx + 1] = g;
                rgba_buffer[out_idx + 2] = b;
                rgba_buffer[out_idx + 3] = 255;
            } else {
                // Convenção YCbCr Exata: preservação estrita de fase sem ponto neutro
                let db = (20.0 * (abs_norm + 1e-15).log10()).clamp(-96.0, 0.0);
                let y_val = ((db + 96.0) / 96.0 * 255.0).clamp(0.0, 255.0);
                let y_u = y_val.round() as u8;

                let aq_db = (y_u as f32 / 255.0) * 96.0 - 96.0;
                let aq = 10.0f32.powf(aq_db / 20.0);

                let res = (abs_norm - aq).abs();
                let sat = (res / (aq + 1e-12)).clamp(1.0 / 255.0, 1.0);

                let phase = im.atan2(re);
                let cb = 128.0 + 127.0 * sat * phase.sin();
                let cr = 128.0 + 127.0 * sat * phase.cos();

                // BT.601 full-range
                let r_val = y_val + 1.402 * (cr - 128.0);
                let g_val = y_val - 0.344136 * (cb - 128.0) - 0.714136 * (cr - 128.0);
                let b_val = y_val + 1.772 * (cb - 128.0);

                rgba_buffer[out_idx] = r_val.clamp(0.0, 255.0).round() as u8;
                rgba_buffer[out_idx + 1] = g_val.clamp(0.0, 255.0).round() as u8;
                rgba_buffer[out_idx + 2] = b_val.clamp(0.0, 255.0).round() as u8;
                rgba_buffer[out_idx + 3] = 255;
            }
        }
    }

    rgba_buffer
}

// =========================================================================
// RENDERIZADOR DO CAMPO HOLOMORFO FUNDAMENTAL E RESÍDUOS ANALÍTICOS
// =========================================================================

/// Renderiza o campo holomorfo fundamental E(t, y) = a(t, y) + i * phi(t, y)
/// nos seus campos derivados (Log-Amplitude, Fase, Frequência de Fase, Crescimento
/// de Envelope, Resíduo de Cauchy-Riemann e Resíduo Harmônico do Laplaciano).
#[wasm_bindgen]
pub fn wasm_render_holomorphic_field(
    data: &[u8],
    h_custom: usize,
    fmin_custom: f32,
    fmax_custom: f32,
    scale_type: &str, // "cqt", "mel", "bark", "linear"
    field_mode: &str, // "log_amplitude", "phase", "phase_frequency", "envelope_growth", "cr_residual", "harmonicity_residual"
    show_contours: bool,
    show_ridge_candidates: bool,
    c_start_in: usize,
    c_end_in: usize,
    q_param_custom: f32,
) -> Vec<u8> {
    if data.len() < 44 {
        return Vec::new();
    }

    let num_channels = u16::from_le_bytes([data[22], data[23]]) as usize;
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let fs = if sample_rate > 0 { sample_rate as f32 } else { 48000.0 };

    let pcm_bytes = &data[44..];
    let total_samples = pcm_bytes.len() / (2 * num_channels.max(1));
    if total_samples == 0 {
        return Vec::new();
    }

    let mut signal = Vec::with_capacity(total_samples);
    for i in 0..total_samples {
        let mut sample_acc = 0.0f32;
        for ch in 0..num_channels {
            let offset = (i * num_channels + ch) * 2;
            if offset + 1 < pcm_bytes.len() {
                let s16 = i16::from_le_bytes([pcm_bytes[offset], pcm_bytes[offset + 1]]) as f32;
                sample_acc += s16 / 32768.0;
            }
        }
        signal.push(sample_acc / num_channels as f32);
    }

    let h = if h_custom > 0 { h_custom } else { 600 };
    let fmin = if fmin_custom > 0.0 { fmin_custom } else { 50.0 };
    let fmax = if fmax_custom > fmin { fmax_custom } else { (fs * 0.5).min(12000.0) };
    let q_param = if q_param_custom > 0.1 { q_param_custom } else { 2.0 };

    let n_stft = 2048usize;
    let hop = 232usize;
    let dt = hop as f32 / fs;
    let w = (total_samples.saturating_sub(n_stft) / hop).max(1);

    let c_start = c_start_in.min(w.saturating_sub(1));
    let c_end = c_end_in.clamp(c_start + 1, w);
    let chunk_w = c_end - c_start;

    let is_linear = scale_type == "linear";
    let is_mel = scale_type == "mel";
    let is_bark = scale_type == "bark";

    let lambda = if is_mel {
        700.0f32
    } else if is_bark {
        1960.0f32
    } else {
        0.0f32
    };

    let step_lin = (fmax - fmin) / (h as f32 - 1.0).max(1.0);
    let mel_min = 2595.0 * (1.0 + fmin / 700.0).log2();
    let mel_max = 2595.0 * (1.0 + fmax / 700.0).log2();
    let mel_step = (mel_max - mel_min) / (h as f32 - 1.0).max(1.0);

    let bark_min = 26.81 * (fmin / (1960.0 + fmin)) - 0.53;
    let bark_max = 26.81 * (fmax / (1960.0 + fmax)) - 0.53;
    let bark_step = (bark_max - bark_min) / (h as f32 - 1.0).max(1.0);

    let cqt_step = (fmax / fmin).log2() / (h as f32 - 1.0).max(1.0);

    let df = fs / n_stft as f32;
    let mut fc_lut = vec![0.0f32; h];
    let mut eta_lut = vec![0.0f32; h];
    let mut eta_prime_lut = vec![0.0f32; h];
    let mut eta_double_prime_lut = vec![0.0f32; h];
    let mut k_low_lut = vec![0usize; h];
    let mut k_high_lut = vec![0usize; h];
    let mut kernels_lut: Vec<Vec<f32>> = vec![Vec::new(); h];

    let dy = 1.0 / (h as f32 - 1.0).max(1.0);

    for j in 0..h {
        let fc = if is_linear {
            fmin + j as f32 * step_lin
        } else if is_mel {
            let mel_val = mel_min + j as f32 * mel_step;
            700.0 * (2.0f32.powf(mel_val / 2595.0) - 1.0)
        } else if is_bark {
            let bark_val = bark_min + j as f32 * bark_step;
            (1960.0 * (bark_val + 0.53) / (26.28 - bark_val)).max(fmin)
        } else {
            fmin * 2.0f32.powf(j as f32 * cqt_step)
        };
        fc_lut[j] = fc;

        // eta(y) = q / (2 * pi * (fc + lambda))
        let eta = q_param / (2.0 * PI * (fc + lambda));
        eta_lut[j] = eta;

        let f_low = (((fc + lambda) * 0.15) - lambda).max(fmin);
        let f_high = (((fc + lambda) * 3.5) - lambda).min(fs * 0.5);

        let k_low = ((f_low * n_stft as f32 / fs).round() as isize).clamp(1, (n_stft / 2 - 2) as isize) as usize;
        let k_high = ((f_high * n_stft as f32 / fs).round() as isize).clamp(k_low as isize + 1, (n_stft / 2 - 1) as isize) as usize;

        k_low_lut[j] = k_low;
        k_high_lut[j] = k_high;

        let mut kernel = Vec::with_capacity(k_high - k_low + 1);
        let mut energy_sum = 0.0f32;

        for curr_k in k_low..=k_high {
            let f_k = curr_k as f32 * df;
            let u = (f_k + lambda) / (fc + lambda);
            if u > 1e-4 {
                let base = u * (1.0 - u).exp();
                let h0_val = base.powf(q_param);
                kernel.push(h0_val);
                energy_sum += h0_val * h0_val * df;
            } else {
                kernel.push(0.0);
            }
        }

        let n2 = if energy_sum > 1e-30 { 1.0 / energy_sum.sqrt() } else { 1.0 };
        for k in 0..kernel.len() {
            kernel[k] *= n2;
        }

        kernels_lut[j] = kernel;
    }

    // Derivadas verticais de eta(y) com respeito a y
    for j in 0..h {
        let eta_prev = if j > 0 { eta_lut[j - 1] } else { eta_lut[j] };
        let eta_next = if j < h - 1 { eta_lut[j + 1] } else { eta_lut[j] };
        let eta_curr = eta_lut[j];

        let ep = (eta_next - eta_prev) / (2.0 * dy);
        let epp = (eta_next - 2.0 * eta_curr + eta_prev) / (dy * dy);

        eta_prime_lut[j] = ep;
        eta_double_prime_lut[j] = epp;
    }

    // Avaliação do campo complexo E(t, y)
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    let mut fft_buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
    let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

    let win_len = 1024usize.min(n_stft);
    let sigma = 0.25f32 * (win_len as f32 * 0.5);
    let mut gauss_win = vec![0.0f32; win_len];
    for i in 0..win_len {
        let t = (i as f32) - (win_len as f32 * 0.5);
        gauss_win[i] = (-0.5 * (t * t) / (sigma * sigma)).exp();
    }

    // Matrizes completas do campo complexo no chunk: E_re e E_im
    // Inclui 2 colunas de margem temporal para derivadas centrais t+1, t-1, t+2, t-2
    let c_pad_start = c_start.saturating_sub(2);
    let c_pad_end = (c_end + 2).min(w);
    let pad_w = c_pad_end - c_pad_start;

    let mut e_re_grid = vec![0.0f32; h * pad_w];
    let mut e_im_grid = vec![0.0f32; h * pad_w];

    for c in c_pad_start..c_pad_end {
        let center_sample = c * hop + n_stft / 2;
        let half_win = (win_len as isize) / 2;

        for (i, slot) in fft_buffer.iter_mut().enumerate() {
            let s_idx = center_sample as isize - half_win + i as isize;
            if i < win_len && s_idx >= 0 && (s_idx as usize) < signal.len() {
                *slot = Complex::new(signal[s_idx as usize] * gauss_win[i], 0.0);
            } else {
                *slot = Complex::new(0.0, 0.0);
            }
        }

        fft.process_with_scratch(&mut fft_buffer, &mut scratch);

        let local_c = c - c_pad_start;
        for j in 0..h {
            let k_low = k_low_lut[j];
            let k_high = k_high_lut[j];
            let kernel = &kernels_lut[j];

            let mut w0_re = 0.0f32;
            let mut w0_im = 0.0f32;

            for (i, curr_k) in (k_low..=k_high).enumerate() {
                let h0 = kernel[i];
                if h0 > 1e-6 {
                    let bin = fft_buffer[curr_k];
                    w0_re += bin.re * h0;
                    w0_im += bin.im * h0;
                }
            }

            let idx = j * pad_w + local_c;
            e_re_grid[idx] = w0_re;
            e_im_grid[idx] = w0_im;
        }
    }

    // Cálculo dos campos escalares a = ln|E| e phi = arg(E)
    let mut a_grid = vec![0.0f32; h * pad_w];
    let mut phi_grid = vec![0.0f32; h * pad_w];
    let mut max_abs_e = 1e-12f32;

    for idx in 0..(h * pad_w) {
        let re = e_re_grid[idx];
        let im = e_im_grid[idx];
        let mag = (re * re + im * im).sqrt();
        if mag > max_abs_e {
            max_abs_e = mag;
        }
        a_grid[idx] = (mag + 1e-12).ln();
        phi_grid[idx] = im.atan2(re);
    }

    // Buffer de saída RGBA
    let mut rgba_buffer = vec![0u8; chunk_w * h * 4];

    for row in 0..h {
        let j = row;
        let ep = eta_prime_lut[j];
        let epp = eta_double_prime_lut[j];
        let fc = fc_lut[j];

        let j_prev = if j > 0 { j - 1 } else { j };
        let j_next = if j < h - 1 { j + 1 } else { j };

        for col in c_start..c_end {
            let local_c = col - c_pad_start;
            let out_col = col - c_start;
            let out_idx = (row * chunk_w + out_col) * 4;

            let idx_c = j * pad_w + local_c;
            let idx_cp = j * pad_w + (local_c + 1).min(pad_w - 1);
            let idx_cm = j * pad_w + local_c.saturating_sub(1);

            let a_curr = a_grid[idx_c];
            let phi_curr = phi_grid[idx_c];

            // 1. Derivadas temporais centrais com desmodulação de portadora fc
            let a_t = (a_grid[idx_cp] - a_grid[idx_cm]) / (2.0 * dt);
            let expected_dphi = 2.0 * PI * fc * (2.0 * dt);
            let raw_dphi = phi_grid[idx_cp] - phi_grid[idx_cm];
            let delta_phi = wrap_phase(raw_dphi - expected_dphi);
            let phi_t = 2.0 * PI * fc + delta_phi / (2.0 * dt);
            let f_phi = phi_t / (2.0 * PI);

            let a_tt = (a_grid[idx_cp] - 2.0 * a_curr + a_grid[idx_cm]) / (dt * dt);
            let _phi_tt = (wrap_phase(phi_grid[idx_cp] - phi_curr) - wrap_phase(phi_curr - phi_grid[idx_cm])) / (dt * dt);

            // 2. Derivadas verticais centrais
            let idx_jp = j_next * pad_w + local_c;
            let idx_jm = j_prev * pad_w + local_c;

            let a_y = (a_grid[idx_jp] - a_grid[idx_jm]) / (2.0 * dy);
            let a_yy = (a_grid[idx_jp] - 2.0 * a_curr + a_grid[idx_jm]) / (dy * dy);

            // 3. Frequências e resíduos holomorfos
            let f_a = -a_y / (2.0 * PI * ep);
            let cr_residual = (f_phi - f_a).abs();

            let laplacian_error = (a_tt + a_yy / (ep * ep).max(1e-12) - (epp * a_y) / (ep * ep * ep).abs().max(1e-12)).abs();

            // Identificação de candidato a ridge: a_y cruza zero com concavidade negativa (a_yy < 0)
            let is_ridge_candidate = a_yy < -0.5 && a_y.abs() < 1.2;

            match field_mode {
                "phase" => {
                    // Mapa Cíclico de Fase com saturação 1.0 (sem transparência)
                    let norm_phi = (phi_curr / (2.0 * PI)).rem_euclid(1.0);
                    let (r, g, b) = hsv_to_rgb_solid(norm_phi * 360.0, 0.9, 0.95);

                    if show_contours && (norm_phi < 0.02 || norm_phi > 0.98) {
                        rgba_buffer[out_idx] = 255;
                        rgba_buffer[out_idx + 1] = 255;
                        rgba_buffer[out_idx + 2] = 255;
                    } else {
                        rgba_buffer[out_idx] = r;
                        rgba_buffer[out_idx + 1] = g;
                        rgba_buffer[out_idx + 2] = b;
                    }
                }
                "phase_frequency" => {
                    // Frequência de fase f_phi mapeada no intervalo [fmin, fmax]
                    let u = ((f_phi - fmin) / (fmax - fmin)).clamp(0.0, 1.0);
                    let (r, g, b) = spectral_colormap_solid(u);
                    rgba_buffer[out_idx] = r;
                    rgba_buffer[out_idx + 1] = g;
                    rgba_buffer[out_idx + 2] = b;
                }
                "envelope_growth" => {
                    // Crescimento/Decaimento temporal a_t (Divergente: azul < 0, ardósia = 0, âmbar > 0)
                    let (r, g, b) = diverging_colormap_solid(a_t * 0.05);
                    rgba_buffer[out_idx] = r;
                    rgba_buffer[out_idx + 1] = g;
                    rgba_buffer[out_idx + 2] = b;
                }
                "cr_residual" => {
                    // Resíduo de Cauchy-Riemann Delta f = |f_phi - f_a|
                    // Preto = 0.0 Hz (Perfeição Holomorfa), Vermelho = Violação
                    let err_norm = (cr_residual / 40.0).clamp(0.0, 1.0);
                    let r = (err_norm * 255.0).round() as u8;
                    let g = (err_norm * err_norm * 180.0).round() as u8;
                    rgba_buffer[out_idx] = r;
                    rgba_buffer[out_idx + 1] = g;
                    rgba_buffer[out_idx + 2] = 0;
                }
                "harmonicity_residual" => {
                    // Resíduo Harmônico do Laplaciano |H_a|
                    let err_norm = (laplacian_error / 500.0).clamp(0.0, 1.0);
                    let r = (err_norm * 220.0).round() as u8;
                    let b = (err_norm * 255.0).round() as u8;
                    rgba_buffer[out_idx] = r;
                    rgba_buffer[out_idx + 1] = 0;
                    rgba_buffer[out_idx + 2] = b;
                }
                _ => {
                    // "log_amplitude": a(t, y) = ln|E| com escala monotônica em dB
                    let re = e_re_grid[idx_c];
                    let im = e_im_grid[idx_c];
                    let mag = (re * re + im * im).sqrt();
                    let norm_mag = (mag / max_abs_e).clamp(1e-5, 1.0);
                    let db = (20.0 * norm_mag.log10()).clamp(-96.0, 0.0);
                    let u = ((db + 96.0) / 96.0).clamp(0.0, 1.0);

                    if show_ridge_candidates && is_ridge_candidate {
                        // Linha sólida verde esmeralda para candidatos a ridge
                        rgba_buffer[out_idx] = 16;
                        rgba_buffer[out_idx + 1] = 240;
                        rgba_buffer[out_idx + 2] = 130;
                    } else if show_contours && (db % 12.0).abs() < 0.6 {
                        // Isolinhas de contorno a cada 12 dB
                        rgba_buffer[out_idx] = 255;
                        rgba_buffer[out_idx + 1] = 255;
                        rgba_buffer[out_idx + 2] = 255;
                    } else {
                        // Monotônico ouro/magma com fundo escuro sólido
                        let r = (u * 255.0).round() as u8;
                        let g = (u * u * 215.0).round() as u8;
                        let b = (u * u * u * 160.0).round() as u8;
                        rgba_buffer[out_idx] = r;
                        rgba_buffer[out_idx + 1] = g;
                        rgba_buffer[out_idx + 2] = b;
                    }
                }
            }
            rgba_buffer[out_idx + 3] = 255;
        }
    }

    rgba_buffer
}

/// Sonda o campo holomorfo no ponto pontual exato (t_sec, f_hz), extraindo
/// todos os invariantes analíticos e diferenciais para o cursor do mouse
#[wasm_bindgen]
pub fn wasm_probe_holomorphic_analytic_point(
    data: &[u8],
    t_sec: f32,
    f_hz: f32,
    scale_type: &str,
    q_param_custom: f32,
) -> String {
    if data.len() < 44 {
        return "{}".to_string();
    }

    let num_channels = u16::from_le_bytes([data[22], data[23]]) as usize;
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let fs = if sample_rate > 0 { sample_rate as f32 } else { 48000.0 };

    let pcm_bytes = &data[44..];
    let total_samples = pcm_bytes.len() / (2 * num_channels.max(1));
    if total_samples == 0 {
        return "{}".to_string();
    }

    let mut signal = Vec::with_capacity(total_samples);
    for i in 0..total_samples {
        let mut sample_acc = 0.0f32;
        for ch in 0..num_channels {
            let offset = (i * num_channels + ch) * 2;
            if offset + 1 < pcm_bytes.len() {
                let s16 = i16::from_le_bytes([pcm_bytes[offset], pcm_bytes[offset + 1]]) as f32;
                sample_acc += s16 / 32768.0;
            }
        }
        signal.push(sample_acc / num_channels as f32);
    }

    let q_param = if q_param_custom > 0.1 { q_param_custom } else { 2.0 };
    let n_stft = 2048usize;
    let hop = 232usize;
    let dt = hop as f32 / fs;

    let target_sample = ((t_sec * fs).round() as isize).clamp(0, total_samples as isize - 1) as usize;
    let c = (target_sample.saturating_sub(n_stft / 2) / hop).max(2).min((total_samples.saturating_sub(n_stft) / hop).saturating_sub(2));

    let lambda = if scale_type == "mel" {
        700.0f32
    } else if scale_type == "bark" {
        1960.0f32
    } else {
        0.0f32
    };

    let fc = f_hz.clamp(20.0, fs * 0.48);
    let eta = q_param / (2.0 * PI * (fc + lambda));
    let eta_prime = -q_param / (2.0 * PI * (fc + lambda).powi(2));
    let eta_double_prime = 2.0 * q_param / (2.0 * PI * (fc + lambda).powi(3));

    // Janela Gaussiana
    let win_len = 1024usize.min(n_stft);
    let sigma = 0.25f32 * (win_len as f32 * 0.5);
    let mut gauss_win = vec![0.0f32; win_len];
    for i in 0..win_len {
        let t = (i as f32) - (win_len as f32 * 0.5);
        gauss_win[i] = (-0.5 * (t * t) / (sigma * sigma)).exp();
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

    // Avalia nos 5 quadros temporais adjacentes [c-2, c-1, c, c+1, c+2]
    let df = fs / n_stft as f32;
    let f_low = (((fc + lambda) * 0.15) - lambda).max(20.0);
    let f_high = (((fc + lambda) * 3.5) - lambda).min(fs * 0.5);
    let k_low = ((f_low * n_stft as f32 / fs).round() as isize).clamp(1, (n_stft / 2 - 2) as isize) as usize;
    let k_high = ((f_high * n_stft as f32 / fs).round() as isize).clamp(k_low as isize + 1, (n_stft / 2 - 1) as isize) as usize;

    let mut kernel = Vec::with_capacity(k_high - k_low + 1);
    let mut energy_sum = 0.0f32;
    for curr_k in k_low..=k_high {
        let f_k = curr_k as f32 * df;
        let u = (f_k + lambda) / (fc + lambda);
        if u > 1e-4 {
            let base = u * (1.0 - u).exp();
            let h0_val = base.powf(q_param);
            kernel.push(h0_val);
            energy_sum += h0_val * h0_val * df;
        } else {
            kernel.push(0.0);
        }
    }
    let n2 = if energy_sum > 1e-30 { 1.0 / energy_sum.sqrt() } else { 1.0 };

    let mut e_vals = Vec::with_capacity(5);
    for offset in [-2, -1, 0, 1, 2] {
        let curr_c = (c as isize + offset).clamp(0, (total_samples / hop) as isize - 1) as usize;
        let center_sample = curr_c * hop + n_stft / 2;
        let half_win = (win_len as isize) / 2;

        let mut fft_buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        for (i, slot) in fft_buffer.iter_mut().enumerate() {
            let s_idx = center_sample as isize - half_win + i as isize;
            if i < win_len && s_idx >= 0 && (s_idx as usize) < signal.len() {
                *slot = Complex::new(signal[s_idx as usize] * gauss_win[i], 0.0);
            }
        }
        fft.process_with_scratch(&mut fft_buffer, &mut scratch);

        let mut w0_re = 0.0f32;
        let mut w0_im = 0.0f32;
        for (i, curr_k) in (k_low..=k_high).enumerate() {
            let h0 = kernel[i] * n2;
            let bin = fft_buffer[curr_k];
            w0_re += bin.re * h0;
            w0_im += bin.im * h0;
        }
        e_vals.push(Complex::new(w0_re, w0_im));
    }

    let e_center = e_vals[2];
    let mag = e_center.norm();
    let a_val = (mag + 1e-12).ln();
    let phi_val = e_center.im.atan2(e_center.re);

    // Derivadas temporais centrais com desmodulação da frequência portadora fc
    let raw_phi_3 = e_vals[3].im.atan2(e_vals[3].re);
    let raw_phi_1 = e_vals[1].im.atan2(e_vals[1].re);
    let expected_dphi = 2.0 * PI * fc * (2.0 * dt);
    let delta_phi = wrap_phase(raw_phi_3 - raw_phi_1 - expected_dphi);
    let phi_t = 2.0 * PI * fc + delta_phi / (2.0 * dt);
    let f_phi = phi_t / (2.0 * PI);

    let e_t = (e_vals[3] - e_vals[1]) / (2.0 * dt);
    let e_tt = (e_vals[3] - 2.0 * e_center + e_vals[1]) / (dt * dt);

    let mag_sq = mag * mag;
    let a_t = if mag_sq > 1e-12 {
        let q_t = (e_t * e_center.conj()) / mag_sq;
        q_t.re
    } else {
        0.0
    };

    let a_y = -eta_prime * phi_t;
    let f_a = -a_y / (2.0 * PI * eta_prime);
    let phi_y = eta_prime * a_t;

    let (a_tt, phi_tt) = if mag_sq > 1e-12 {
        let q_tt = (e_tt * e_center.conj()) / mag_sq - ((e_t * e_center.conj()) / mag_sq).powu(2);
        (q_tt.re, q_tt.im)
    } else {
        (0.0, 0.0)
    };

    let f_dot = phi_tt / (2.0 * PI);
    let cr_residual = (f_phi - f_a).abs();
    let a_yy = -eta_double_prime * phi_t - eta_prime * phi_tt;
    let laplacian_error = (a_tt + a_yy / (eta_prime * eta_prime).max(1e-12) - (eta_double_prime * a_y) / (eta_prime * eta_prime * eta_prime).abs().max(1e-12)).abs();

    let is_ridge = a_yy < -0.1 && a_y.abs() < 2.0;
    let a_ty = -eta_prime * phi_tt - eta_double_prime * phi_t;
    let ridge_velocity = if is_ridge && a_yy.abs() > 1e-6 {
        -a_ty / a_yy
    } else {
        0.0
    };

    let payload = serde_json::json!({
        "t_sec": t_sec,
        "f_hz": fc,
        "E_re": e_center.re,
        "E_im": e_center.im,
        "magnitude": mag,
        "log_amplitude": a_val,
        "log_amplitude_db": 20.0 * (mag.max(1e-6)).log10(),
        "phase_rad": phi_val,
        "phase_deg": phi_val * 180.0 / PI,
        "phi_t": phi_t,
        "f_phi_hz": f_phi,
        "f_a_hz": f_a,
        "cr_residual_hz": cr_residual,
        "a_t": a_t,
        "a_y": a_y,
        "phi_y": phi_y,
        "a_tt": a_tt,
        "phi_tt": phi_tt,
        "f_dot_hz_per_sec": f_dot,
        "eta": eta,
        "eta_prime": eta_prime,
        "laplacian_residual": laplacian_error,
        "is_ridge_candidate": is_ridge,
        "ridge_velocity": ridge_velocity,
    });

    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
}

#[inline(always)]
fn wrap_phase(p: f32) -> f32 {
    (p + PI).rem_euclid(TAU) - PI
}

#[inline(always)]
fn hsv_to_rgb_solid(h_deg: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = (h_deg / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (h_prime.rem_euclid(2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

#[inline(always)]
fn spectral_colormap_solid(u: f32) -> (u8, u8, u8) {
    // Colormap espectral contínuo (turbo-like)
    let r = (9.0 * (1.0 - (2.0 * u - 1.0).abs()).max(0.0) + 0.1).clamp(0.0, 1.0);
    let g = (4.0 * (1.0 - (2.0 * u - 0.7).abs()).max(0.0)).clamp(0.0, 1.0);
    let b = (4.0 * (1.0 - (2.0 * u - 0.3).abs()).max(0.0)).clamp(0.0, 1.0);
    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

#[inline(always)]
fn diverging_colormap_solid(val: f32) -> (u8, u8, u8) {
    let v = val.clamp(-1.0, 1.0);
    if v < 0.0 {
        // Azul para decaimento
        let f = -v;
        (
            ((15.0 + f * 40.0)).round() as u8,
            ((23.0 + f * 120.0)).round() as u8,
            ((42.0 + f * 213.0)).round() as u8,
        )
    } else {
        // Ouro/âmbar para crescimento
        let f = v;
        (
            ((15.0 + f * 240.0)).round() as u8,
            ((23.0 + f * 155.0)).round() as u8,
            ((42.0 * (1.0 - f))).round() as u8,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_wav(freq_hz: f32, duration_s: f32, sr: u32) -> Vec<u8> {
        let n = (duration_s * sr as f32) as usize;
        let mut pcm = Vec::with_capacity(n);
        let dt = 1.0 / sr as f32;
        for i in 0..n {
            let t = i as f32 * dt;
            pcm.push(0.8 * (2.0 * PI * freq_hz * t).sin());
        }
        crate::synthesis::encode_pcm_to_wav_bytes(&pcm, sr)
    }

    #[test]
    fn test_holomorphic_field_rendering_all_modes() {
        let wav = generate_test_wav(440.0, 0.2, 12000);
        let modes = [
            "log_amplitude",
            "phase",
            "phase_frequency",
            "envelope_growth",
            "cr_residual",
            "harmonicity_residual",
        ];

        for mode in modes {
            let rgba = wasm_render_holomorphic_field(
                &wav, 128, 50.0, 4000.0, "cqt", mode, true, true, 0, 10, 2.0,
            );
            assert!(!rgba.is_empty(), "Modo de campo {} deve produzir buffer RGBA não vazio", mode);
            assert_eq!(rgba.len() % 4, 0, "Buffer RGBA deve ter comprimento múltiplo de 4");
            // Verifica canal alpha opaco (255) - sem transparência!
            for (idx, &a) in rgba.iter().skip(3).step_by(4).enumerate() {
                assert_eq!(a, 255, "Alpha em pixel {} deve ser estritamente 255 (sem transparência)", idx);
            }
        }
    }

    #[test]
    fn test_holomorphic_analytic_probe_identities() {
        let wav = generate_test_wav(440.0, 0.4, 12000);
        let probe_json = wasm_probe_holomorphic_analytic_point(&wav, 0.2, 440.0, "cqt", 2.0);
        let parsed: serde_json::Value = serde_json::from_str(&probe_json).expect("JSON de sonda deve ser válido");

        let f_phi = parsed["f_phi_hz"].as_f64().expect("f_phi_hz deve ser float");
        let f_a = parsed["f_a_hz"].as_f64().expect("f_a_hz deve ser float");
        let cr_res = parsed["cr_residual_hz"].as_f64().expect("cr_residual_hz deve ser float");

        assert!((f_phi - 440.0).abs() < 15.0, "f_phi deve aproximar 440 Hz, obteve {}", f_phi);
        assert!((f_a - 440.0).abs() < 15.0, "f_a deve aproximar 440 Hz por Cauchy-Riemann, obteve {}", f_a);
        assert!(cr_res < 10.0, "Resíduo de Cauchy-Riemann deve ser pequeno, obteve {}", cr_res);
    }
}


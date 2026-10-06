//! Módulo de Exploração Holomorfa Dedicado (WASM)
//!
//! Implementação 100% isolada para exploração de campos holomorfos multiescala:
//! - Escalas: CQT (lambda=0), Mel (lambda=700), Bark (lambda=1960), Linear
//! - Filtros com centralização logarítmica e normalização L2 unitária
//! - Densidade analítica de frame tight rho(y)
//! - Convenção de visualização YCbCr com preservação estrita de fase

use wasm_bindgen::prelude::*;
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

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

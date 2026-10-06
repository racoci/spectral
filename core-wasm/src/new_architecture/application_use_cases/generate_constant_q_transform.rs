//! Caso de Uso: Transformada de Fator de Qualidade Constante (CQT)
//!
//! Avaliação por matriz esparsa de wavelets de Cauchy com escada monomial diferencial
//! W_0 e W_1 = u * W_0, reatribuição conforme via quociente R e normalização unitária L^2.

use super::super::domain_mathematics::numerical_types::complex_number::ComplexNumber32;
use super::super::domain_mathematics::analysis_windows::CauchyWaveletLadder;
use super::super::domain_mathematics::color_spaces::{LuminanceChrominanceYCbCr, GeodesicSnakePalette};
use super::super::infrastructure::audio_decoding::WaveAudioParser;
use super::super::infrastructure::fast_fourier_transform::FastFourierTransformPlannerCache;
use rustfft::num_complex::Complex;
use std::f32::consts::PI;

pub struct ConstantQTransformRequest<'a> {
    pub audio_bytes: &'a [u8],
    pub height: usize,
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
    pub palette_type: &'a str,
    pub view_start_ratio: f32,
    pub view_end_ratio: f32,
    pub point_radius: f32,
    pub horizontal_resolution_factor: f32,
    pub level_of_detail: usize,
    pub enable_time_reassignment: bool,
    pub enable_frequency_reassignment: bool,
    pub maximum_derivative_order: usize,
}

pub struct GenerateConstantQTransformUseCase;

impl GenerateConstantQTransformUseCase {
    pub fn execute(request: ConstantQTransformRequest) -> Vec<u8> {
        let audio_info = match WaveAudioParser::parse_pcm_16bit(request.audio_bytes) {
            Some(info) => info,
            None => return Vec::new(),
        };

        let fs = audio_info.sampling_rate_hz as f32;
        let signal = audio_info.samples;
        let total_samples = signal.len();

        let h = if request.level_of_detail == 1 { 256 } else if request.height > 0 { request.height } else { 512 };
        let win_len = 1024usize;
        let pad_factor = 2usize;
        let n_stft = win_len * pad_factor;

        let base_hop = 232usize;
        let hop = if request.level_of_detail == 1 {
            base_hop * 2
        } else if request.horizontal_resolution_factor > 0.05 {
            ((base_hop as f32) / request.horizontal_resolution_factor).round().max(16.0) as usize
        } else {
            base_hop
        };

        let w = (total_samples.saturating_sub(n_stft) / hop).max(1);
        let c_start = ((request.view_start_ratio * (w as f32)).floor() as usize).min(w.saturating_sub(1));
        let c_end = ((request.view_end_ratio * (w as f32)).ceil() as usize).clamp(c_start + 1, w);
        let chunk_w = c_end - c_start;

        let fmin = if request.minimum_frequency > 0.0 { request.minimum_frequency } else { 20.0 };
        let fmax = if request.maximum_frequency > fmin { request.maximum_frequency } else { (fs * 0.5).min(20000.0) };

        let q_cauchy = 2.0f32;
        let ladder = CauchyWaveletLadder::new(q_cauchy, 0.0);

        let df = fs / (n_stft as f32);
        let step = (fmax / fmin).log2() / ((h as f32 - 1.0).max(1.0));

        let mut fc_lut = vec![0.0f32; h];
        let mut k_low_lut = vec![0usize; h];
        let mut k_high_lut = vec![0usize; h];
        let mut kernels_lut: Vec<Vec<(f32, f32)>> = vec![Vec::new(); h];

        for j in 0..h {
            let fc = fmin * 2.0f32.powf((j as f32) * step);
            fc_lut[j] = fc;

            let f_low = (fc * 0.2).max(fmin);
            let f_high = (fc * 3.0).min(fs * 0.5);

            let k_low = ((f_low * (n_stft as f32) / fs).round() as isize).clamp(1, (n_stft / 2 - 2) as isize) as usize;
            let k_high = ((f_high * (n_stft as f32) / fs).round() as isize).clamp(k_low as isize + 1, (n_stft / 2 - 1) as isize) as usize;

            k_low_lut[j] = k_low;
            k_high_lut[j] = k_high;

            let mut kernel = Vec::with_capacity(k_high - k_low + 1);
            let mut energy_sum = 0.0f32;

            for curr_k in k_low..=k_high {
                let f_k = (curr_k as f32) * df;
                let (h0, h1) = ladder.evaluate_ladder_pair(f_k, fc);
                kernel.push((h0, h1));
                energy_sum += h0 * h0 * df;
            }

            let n2 = if energy_sum > 1e-30 { 1.0 / energy_sum.sqrt() } else { 1.0 };
            for k in 0..kernel.len() {
                kernel[k].0 *= n2;
                kernel[k].1 *= n2;
            }

            kernels_lut[j] = kernel;
        }

        let mut planner_cache = FastFourierTransformPlannerCache::new();
        let fft = planner_cache.plan_forward(n_stft);

        let mut fft_buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

        let mut reassigned_grid_re = vec![0.0f32; h * w];
        let mut reassigned_grid_im = vec![0.0f32; h * w];

        let half_win = (win_len as f32) * 0.5;
        let sigma_samples = 0.25f32 * half_win;
        let inv_two_sigma_sq = 0.5f32 / (sigma_samples * sigma_samples);
        let mut win_h = vec![0.0f32; win_len];
        for i in 0..win_len {
            let t = (i as f32) - half_win;
            win_h[i] = (-t * t * inv_two_sigma_sq).exp();
        }

        for c in c_start..c_end {
            let start = c * hop;

            for i in 0..n_stft {
                fft_buffer[i] = Complex::new(0.0, 0.0);
            }

            for i in 0..win_len {
                let idx = (start as isize) + (i as isize) - (win_len as isize / 2);
                if idx >= 0 && (idx as usize) < total_samples {
                    fft_buffer[i] = Complex::new(signal[idx as usize] * win_h[i], 0.0);
                }
            }

            fft.process_with_scratch(&mut fft_buffer, &mut scratch);

            let phase_multiplier = 2.0 * PI * (c * hop) as f32 / (n_stft as f32);

            for j in 0..h {
                let fc = fc_lut[j];
                let k_low = k_low_lut[j];
                let k_high = k_high_lut[j];
                let kernel = &kernels_lut[j];

                let mut w0_re = 0.0f32;
                let mut w0_im = 0.0f32;
                let mut w1_re = 0.0f32;
                let mut w1_im = 0.0f32;
                let mut weight_sum = 0.0f32;

                for (i, curr_k) in (k_low..=k_high).enumerate() {
                    let (h0, h1) = kernel[i];
                    if h0 > 1e-6 {
                        let y_k = fft_buffer[curr_k];
                        let y_sym = fft_buffer[n_stft - curr_k];
                        let fft_bin = Complex::new(0.5 * (y_k.re + y_sym.re), 0.5 * (y_k.im - y_sym.im));

                        let angle = (curr_k as f32) * phase_multiplier;
                        let phase_shifter = Complex::new(angle.cos(), angle.sin());
                        let shifted_bin = fft_bin * phase_shifter;

                        w0_re += shifted_bin.re * h0;
                        w0_im += shifted_bin.im * h0;
                        w1_re += shifted_bin.re * h1;
                        w1_im += shifted_bin.im * h1;
                        weight_sum += h0;
                    }
                }

                if weight_sum > 1e-15 {
                    let w0 = ComplexNumber32::new(w0_re / (n_stft as f32), w0_im / (n_stft as f32));
                    let w1 = ComplexNumber32::new(w1_re / (n_stft as f32), w1_im / (n_stft as f32));
                    let abs_w0_sq = w0.norm_squared();

                    if abs_w0_sq > 1e-12 {
                        let r = w1 / w0;
                        let f_reassigned = if request.enable_frequency_reassignment && request.maximum_derivative_order >= 1 {
                            (fc * r.real_part).clamp(fmin * 0.5, fmax * 1.5)
                        } else {
                            fc
                        };

                        let p_period = 1.0 / fc;
                        let t_shift_s = if request.enable_time_reassignment && request.maximum_derivative_order >= 1 {
                            -(q_cauchy * p_period / (2.0 * PI)) * r.imaginary_part
                        } else {
                            0.0
                        };

                        let t_reassigned_samples = (c * hop) as f32 + t_shift_s * fs;
                        let c_reassigned_f = t_reassigned_samples / (hop as f32);
                        let j_reassigned_f = (f_reassigned / fmin).log2() / step;

                        let c_reassigned_i = c_reassigned_f.round() as isize;
                        let j_reassigned_i = j_reassigned_f.round() as isize;

                        let dx = c_reassigned_f - (c_reassigned_i as f32);
                        let dy = j_reassigned_f - (j_reassigned_i as f32);

                        let d_log_a_dy = (std::f32::consts::LN_2 * (q_cauchy * r.real_part - (q_cauchy + 0.5))).abs();
                        let sig_f = (request.point_radius * 0.5 / (1.0 + d_log_a_dy)).clamp(0.1, 4.0);
                        let sig_t = (request.point_radius * 0.5).clamp(0.1, 4.0);

                        let inv_2_sig_t_sq = 0.5 / (sig_t * sig_t);
                        let inv_2_sig_f_sq = 0.5 / (sig_f * sig_f);

                        let wx_m1 = (-(-1.0 - dx) * (-1.0 - dx) * inv_2_sig_t_sq).exp();
                        let wx_0  = (-(-dx * dx) * inv_2_sig_t_sq).exp();
                        let wx_p1 = (-( 1.0 - dx) * ( 1.0 - dx) * inv_2_sig_t_sq).exp();

                        let wy_m1 = (-(-1.0 - dy) * (-1.0 - dy) * inv_2_sig_f_sq).exp();
                        let wy_0  = (-(-dy * dy) * inv_2_sig_f_sq).exp();
                        let wy_p1 = (-( 1.0 - dy) * ( 1.0 - dy) * inv_2_sig_f_sq).exp();

                        let inv_sum = 1.0 / (((wx_m1 + wx_0 + wx_p1) * (wy_m1 + wy_0 + wy_p1)).max(1e-12));
                        let wx = [wx_m1, wx_0, wx_p1];
                        let wy = [wy_m1, wy_0, wy_p1];

                        for ox in -1isize..=1isize {
                            let curr_c = c_reassigned_i + ox;
                            if curr_c < 0 || curr_c >= (w as isize) { continue; }
                            let w_x_val = wx[(ox + 1) as usize];

                            for oy in -1isize..=1isize {
                                let curr_j = j_reassigned_i + oy;
                                if curr_j >= 0 && curr_j < (h as isize) {
                                    let target_idx_cqt = (curr_j as usize) * w + (curr_c as usize);
                                    let norm_w = (w_x_val * wy[(oy + 1) as usize]) * inv_sum;
                                    reassigned_grid_re[target_idx_cqt] += w0.real_part * norm_w * 4194304.0;
                                    reassigned_grid_im[target_idx_cqt] += w0.imaginary_part * norm_w * 4194304.0;
                                }
                            }
                        }
                    } else {
                        let target_idx = j * w + c;
                        reassigned_grid_re[target_idx] = w0.real_part * 4194304.0;
                        reassigned_grid_im[target_idx] = w0.imaginary_part * 4194304.0;
                    }
                }
            }
        }

        let mut rgba_buffer = vec![0u8; chunk_w * h * 4];
        let snake_palette = GeodesicSnakePalette::new();
        let normalizer = 4194304.0f32;

        for row in 0..h {
            for col in c_start..c_end {
                let grid_idx = row * w + col;
                let out_idx = (row * chunk_w + (col - c_start)) * 4;

                let re = reassigned_grid_re[grid_idx];
                let im = reassigned_grid_im[grid_idx];
                let sample = ComplexNumber32::new(re, im);
                let abs_z = sample.absolute_value();

                if abs_z < 1e-12 {
                    rgba_buffer[out_idx] = 0;
                    rgba_buffer[out_idx + 1] = 0;
                    rgba_buffer[out_idx + 2] = 0;
                    rgba_buffer[out_idx + 3] = 255;
                    continue;
                }

                if request.palette_type == "snake" {
                    let norm_mag = (abs_z / normalizer).clamp(0.0, 1.0);
                    let (r, g, b) = snake_palette.map_normalized_magnitude(norm_mag);
                    rgba_buffer[out_idx] = r;
                    rgba_buffer[out_idx + 1] = g;
                    rgba_buffer[out_idx + 2] = b;
                    rgba_buffer[out_idx + 3] = 255;
                } else {
                    let ycbcr = LuminanceChrominanceYCbCr::from_complex_sample(sample, normalizer);
                    let (r, g, b) = ycbcr.to_rgb_u8();
                    rgba_buffer[out_idx] = r;
                    rgba_buffer[out_idx + 1] = g;
                    rgba_buffer[out_idx + 2] = b;
                    rgba_buffer[out_idx + 3] = 255;
                }
            }
        }

        rgba_buffer
    }
}

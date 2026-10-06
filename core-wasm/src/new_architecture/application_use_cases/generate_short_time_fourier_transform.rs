//! Caso de Uso: Transformada de Fourier de Tempo Curto (STFT) e Reatribuição de Auger-Flandrin
//!
//! Executa a decomposição tempo-frequência linear ou logarítmica com empacotamento 2-em-1
//! de FFTs reais, reatribuição vetorial canônica e renderização de cores sem alocações no laço.

use super::super::domain_mathematics::numerical_types::complex_number::ComplexNumber32;
use super::super::domain_mathematics::frequency_scales::{
    PerceptualFrequencyScale, LinearFrequencyScale, CauchyLogarithmicFrequencyScale,
    AuditoryMelFrequencyScale, PsychoacousticBarkFrequencyScale,
};
use super::super::domain_mathematics::color_spaces::{LuminanceChrominanceYCbCr, GeodesicSnakePalette};
use super::super::infrastructure::audio_decoding::WaveAudioParser;
use super::super::infrastructure::fast_fourier_transform::FastFourierTransformPlannerCache;
use rustfft::num_complex::Complex;
use std::f32::consts::PI;

pub struct ShortTimeFourierTransformRequest<'a> {
    pub audio_bytes: &'a [u8],
    pub height: usize,
    pub window_size: usize,
    pub zero_padding_factor: usize,
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
    pub is_reassigned_mode: bool,
    pub palette_type: &'a str,
    pub view_start_ratio: f32,
    pub view_end_ratio: f32,
    pub point_radius: f32,
    pub frequency_scale_type: &'a str,
    pub horizontal_resolution_factor: f32,
    pub level_of_detail: usize,
    pub enable_time_reassignment: bool,
    pub enable_frequency_reassignment: bool,
    pub maximum_derivative_order: usize,
}

pub struct GenerateShortTimeFourierTransformUseCase;

impl GenerateShortTimeFourierTransformUseCase {
    pub fn execute(request: ShortTimeFourierTransformRequest) -> Vec<u8> {
        let audio_info = match WaveAudioParser::parse_pcm_16bit(request.audio_bytes) {
            Some(info) => info,
            None => return Vec::new(),
        };

        let fs = audio_info.sampling_rate_hz as f32;
        let signal = audio_info.samples;
        let total_samples = signal.len();

        let h = if request.level_of_detail == 1 {
            256
        } else if request.height > 0 {
            request.height
        } else {
            512
        };

        let win_len = if request.window_size > 0 { request.window_size } else { 1024 };
        let pad_factor = if request.zero_padding_factor > 0 { request.zero_padding_factor } else { 2 };
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

        let scale_box: Box<dyn PerceptualFrequencyScale> = match request.frequency_scale_type {
            "linear" => Box::new(LinearFrequencyScale { minimum_frequency: fmin, maximum_frequency: fmax }),
            "mel" => Box::new(AuditoryMelFrequencyScale::new(fmin, fmax)),
            "bark" => Box::new(PsychoacousticBarkFrequencyScale::new(fmin, fmax)),
            _ => Box::new(CauchyLogarithmicFrequencyScale { minimum_frequency: fmin, maximum_frequency: fmax }),
        };

        // Janelas Gaussianas e derivadas analíticas Hermiteanas
        let mut win_h = vec![0.0f32; win_len];
        let mut win_th = vec![0.0f32; win_len];
        let mut win_dh = vec![0.0f32; win_len];

        let half_win = (win_len as f32) * 0.5;
        let sigma_samples = 0.25f32 * half_win;
        let inv_two_sigma_sq = 0.5f32 / (sigma_samples * sigma_samples);

        for i in 0..win_len {
            let t = (i as f32) - half_win;
            let g = (-t * t * inv_two_sigma_sq).exp();
            win_h[i] = g;
            win_th[i] = t * g;
            win_dh[i] = -(t / (sigma_samples * sigma_samples)) * g;
        }

        // Tabela de frequências centrais e bins FFT correspondentes
        let mut fc_lut = vec![0.0f32; h];
        let mut k_f_lut = vec![0.0f32; h];
        for j in 0..h {
            let y = (j as f32) / ((h as f32 - 1.0).max(1.0));
            let fc = scale_box.frequency_from_coordinate(y);
            fc_lut[j] = fc;
            k_f_lut[j] = fc * (n_stft as f32) / fs;
        }

        let mut planner_cache = FastFourierTransformPlannerCache::new();
        let fft = planner_cache.plan_forward(n_stft);

        let mut buffer_h_th = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buffer_dh = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

        let mut reassigned_grid_re = vec![0.0f32; h * w];
        let mut reassigned_grid_im = vec![0.0f32; h * w];

        for c in c_start..c_end {
            let start = c * hop;

            for i in 0..n_stft {
                buffer_h_th[i] = Complex::new(0.0, 0.0);
                buffer_dh[i] = Complex::new(0.0, 0.0);
            }

            for i in 0..win_len {
                let idx = (start as isize) + (i as isize) - (win_len as isize / 2);
                if idx >= 0 && (idx as usize) < total_samples {
                    let sample_val = signal[idx as usize];
                    // Otimização 2-em-1: empacota h na parte real e t*h na parte imaginária!
                    buffer_h_th[i] = Complex::new(sample_val * win_h[i], sample_val * win_th[i]);
                    buffer_dh[i] = Complex::new(sample_val * win_dh[i], 0.0);
                }
            }

            fft.process_with_scratch(&mut buffer_h_th, &mut scratch);
            fft.process_with_scratch(&mut buffer_dh, &mut scratch);

            for j in 0..h {
                let fc = fc_lut[j];
                let k_f = k_f_lut[j];
                let k_floor = ((k_f.floor() as usize).clamp(1, n_stft / 2 - 2)) as usize;
                let k_ceil = k_floor + 1;
                let delta_k = k_f - (k_floor as f32);

                // Desempacotamento analítico O(1) de duas FFTs reais a partir de uma complexa
                let y_floor = buffer_h_th[k_floor];
                let y_sym_floor = buffer_h_th[n_stft - k_floor];
                let s_h_floor = Complex::new(0.5 * (y_floor.re + y_sym_floor.re), 0.5 * (y_floor.im - y_sym_floor.im));

                let y_ceil = buffer_h_th[k_ceil];
                let y_sym_ceil = buffer_h_th[n_stft - k_ceil];
                let s_h_ceil = Complex::new(0.5 * (y_ceil.re + y_sym_ceil.re), 0.5 * (y_ceil.im - y_sym_ceil.im));

                let s_h = s_h_floor * (1.0 - delta_k) + s_h_ceil * delta_k;
                let s_dh = buffer_dh[k_floor] * (1.0 - delta_k) + buffer_dh[k_ceil] * delta_k;

                let mag_sq = s_h.re * s_h.re + s_h.im * s_h.im;
                if mag_sq < 1.0 {
                    if mag_sq > 1e-4 {
                        let target_idx = j * w + c;
                        reassigned_grid_re[target_idx] += s_h.re;
                        reassigned_grid_im[target_idx] += s_h.im;
                    }
                    continue;
                }

                if request.is_reassigned_mode {
                    let s_v1_v0_conj = s_dh * s_h.conj();
                    let re_r = s_v1_v0_conj.re / mag_sq;
                    let im_r = s_v1_v0_conj.im / mag_sq;

                    let t_shift_samples = if request.enable_time_reassignment && request.maximum_derivative_order >= 1 {
                        -(sigma_samples * sigma_samples) * re_r
                    } else {
                        0.0
                    };
                    let c_reassigned_f = (c as f32) + t_shift_samples / (hop as f32);

                    let omega_shift = if request.enable_frequency_reassignment && request.maximum_derivative_order >= 1 {
                        -im_r
                    } else {
                        0.0
                    };
                    let f_reassigned = fc - (omega_shift * fs / (2.0 * PI));
                    let y_reassigned = scale_box.coordinate_from_frequency(f_reassigned);
                    let j_reassigned_f = y_reassigned * (h as f32 - 1.0);

                    let c_reassigned_i = c_reassigned_f.round() as isize;
                    let j_reassigned_i = j_reassigned_f.round() as isize;

                    let dx = c_reassigned_f - (c_reassigned_i as f32);
                    let dy = j_reassigned_f - (j_reassigned_i as f32);

                    // Atalho de subpixel: se o ponto cair a menos de 0.15 px do centro, deposita diretamente
                    if (dx * dx + dy * dy) < 0.0225 {
                        if c_reassigned_i >= 0 && c_reassigned_i < (w as isize) && j_reassigned_i >= 0 && j_reassigned_i < (h as isize) {
                            let target = (j_reassigned_i as usize) * w + (c_reassigned_i as usize);
                            reassigned_grid_re[target] += s_h.re;
                            reassigned_grid_im[target] += s_h.im;
                        }
                        continue;
                    }

                    let d_log_a_dt = -re_r;
                    let d_log_a_dw = (sigma_samples * sigma_samples) * im_r;

                    let sig_t = (request.point_radius * 0.5 / (1.0 + d_log_a_dt.abs())).clamp(0.1, 4.0);
                    let sig_f = (request.point_radius * 0.5 / (1.0 + d_log_a_dw.abs())).clamp(0.1, 4.0);

                    let inv_2_sig_t_sq = 0.5 / (sig_t * sig_t);
                    let inv_2_sig_f_sq = 0.5 / (sig_f * sig_f);

                    // Gaussiana separável rápida (6 exponenciais)
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
                                let target = (curr_j as usize) * w + (curr_c as usize);
                                let norm_w = (w_x_val * wy[(oy + 1) as usize]) * inv_sum;
                                reassigned_grid_re[target] += s_h.re * norm_w;
                                reassigned_grid_im[target] += s_h.im * norm_w;
                            }
                        }
                    }
                } else {
                    // Modo Smooth Log Spectrogram
                    let target_idx = j * w + c;
                    reassigned_grid_re[target_idx] = s_h.re;
                    reassigned_grid_im[target_idx] = s_h.im;
                }
            }
        }

        // Renderização de Cores
        let mut rgba_buffer = vec![0u8; chunk_w * h * 4];
        let snake_palette = GeodesicSnakePalette::new();

        // Escala canônica de normalização para 24-bits
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

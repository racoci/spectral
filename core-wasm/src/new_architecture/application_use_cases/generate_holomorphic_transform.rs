//! Caso de Uso: Geração de Espectrograma Holomorfo Multiescala
//!
//! Orquestra a avaliação do campo holomorfo analítico nas escalas Mel, Bark, CQT e Linear,
//! aplicando normalização unitária L^2 e ponderação de frame tight.

use super::super::domain_mathematics::numerical_types::complex_number::ComplexNumber32;
use super::super::domain_mathematics::frequency_scales::{
    PerceptualFrequencyScale, LinearFrequencyScale, CauchyLogarithmicFrequencyScale,
    AuditoryMelFrequencyScale, PsychoacousticBarkFrequencyScale,
};
use super::super::domain_mathematics::analysis_windows::CauchyWaveletLadder;
use super::super::domain_mathematics::color_spaces::{LuminanceChrominanceYCbCr, GeodesicSnakePalette};
use super::super::infrastructure::audio_decoding::WaveAudioParser;
use super::super::infrastructure::fast_fourier_transform::FastFourierTransformPlannerCache;
use rustfft::num_complex::Complex;
use std::f32::consts::PI;

pub struct HolomorphicTransformRequest<'a> {
    pub audio_bytes: &'a [u8],
    pub height: usize,
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
    pub scale_type: &'a str,
    pub palette_type: &'a str,
    pub column_start: usize,
    pub column_end: usize,
    pub shape_parameter_q: f32,
    pub enable_reassignment: bool,
    pub enable_tight_frame: bool,
}

pub struct GenerateHolomorphicTransformUseCase;

impl GenerateHolomorphicTransformUseCase {
    pub fn execute(request: HolomorphicTransformRequest) -> Vec<u8> {
        let audio_info = match WaveAudioParser::parse_pcm_16bit(request.audio_bytes) {
            Some(info) => info,
            None => return Vec::new(),
        };

        let fs = audio_info.sampling_rate_hz as f32;
        let signal = audio_info.samples;
        let total_samples = signal.len();

        let h = if request.height > 0 { request.height } else { 600 };
        let fmin = if request.minimum_frequency > 0.0 { request.minimum_frequency } else { 50.0 };
        let fmax = if request.maximum_frequency > fmin { request.maximum_frequency } else { (fs * 0.5).min(12000.0) };
        let q_param = if request.shape_parameter_q > 0.1 { request.shape_parameter_q } else { 2.0 };

        let n_stft = 2048usize;
        let hop = 232usize;
        let w = (total_samples.saturating_sub(n_stft) / hop).max(1);

        let c_start = request.column_start.min(w.saturating_sub(1));
        let c_end = request.column_end.clamp(c_start + 1, w);
        let chunk_w = c_end - c_start;

        let scale_box: Box<dyn PerceptualFrequencyScale> = match request.scale_type {
            "linear" => Box::new(LinearFrequencyScale { minimum_frequency: fmin, maximum_frequency: fmax }),
            "mel" => Box::new(AuditoryMelFrequencyScale::new(fmin, fmax)),
            "bark" => Box::new(PsychoacousticBarkFrequencyScale::new(fmin, fmax)),
            _ => Box::new(CauchyLogarithmicFrequencyScale { minimum_frequency: fmin, maximum_frequency: fmax }),
        };

        let lambda = scale_box.shifted_pole_lambda();
        let ladder = CauchyWaveletLadder::new(q_param, lambda);

        let df = fs / (n_stft as f32);
        let mut fc_lut = vec![0.0f32; h];
        let mut k_low_lut = vec![0usize; h];
        let mut k_high_lut = vec![0usize; h];
        let mut kernels_lut: Vec<Vec<(f32, f32)>> = vec![Vec::new(); h];

        for j in 0..h {
            let y = (j as f32) / ((h as f32 - 1.0).max(1.0));
            let fc = scale_box.frequency_from_coordinate(y);
            fc_lut[j] = fc;

            let f_low = (((fc + lambda) * 0.15) - lambda).max(fmin);
            let f_high = (((fc + lambda) * 3.5) - lambda).min(fs * 0.5);

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
            let rho_factor = if request.enable_tight_frame {
                let eta = scale_box.imaginary_embedding_eta(y, q_param);
                if request.scale_type == "mel" {
                    let log_rho = (4.0 * PI * q_param) * eta.ln() - (2800.0 * PI * eta);
                    (log_rho * 0.5).exp().clamp(0.01, 10.0)
                } else if request.scale_type == "bark" {
                    let log_rho = (4.0 * PI * q_param - 1.0) * eta.ln() - (7840.0 * PI * eta);
                    (log_rho * 0.5).exp().clamp(0.01, 10.0)
                } else {
                    (fc / fmin).sqrt().clamp(0.5, 15.0)
                }
            } else {
                1.0
            };

            for k in 0..kernel.len() {
                kernel[k].0 *= n2 * rho_factor;
                kernel[k].1 *= n2 * rho_factor;
            }

            kernels_lut[j] = kernel;
        }

        let mut planner_cache = FastFourierTransformPlannerCache::new();
        let fft = planner_cache.plan_forward(n_stft);

        let mut fft_buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

        let mut grid_re = vec![0.0f32; h * w];
        let mut grid_im = vec![0.0f32; h * w];
        let mut max_abs = 1e-12f32;

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
                    let w0 = ComplexNumber32::new(w0_re, w0_im);
                    let w1 = ComplexNumber32::new(w1_re, w1_im);

                    if request.enable_reassignment {
                        let r = w1 / w0;
                        let f_reassigned = (((fc + lambda) * r.real_part) - lambda).clamp(fmin * 0.5, fmax * 1.5);
                        let p_period = 1.0 / (fc + lambda);
                        let t_shift_s = -(q_param * p_period / (2.0 * PI)) * r.imaginary_part;
                        let c_f = (c as f32) + (t_shift_s * fs) / hop as f32;

                        let y_reassigned = scale_box.coordinate_from_frequency(f_reassigned);
                        let j_f = y_reassigned * (h as f32 - 1.0);

                        let c_idx = c_f.round() as isize;
                        let j_idx = j_f.round() as isize;

                        if c_idx >= 0 && c_idx < w as isize && j_idx >= 0 && j_idx < h as isize {
                            let target = j_idx as usize * w + c_idx as usize;
                            grid_re[target] += w0.real_part;
                            grid_im[target] += w0.imaginary_part;
                        }
                    } else {
                        let target = j * w + c;
                        grid_re[target] = w0.real_part;
                        grid_im[target] = w0.imaginary_part;
                    }

                    let mag = abs_sq.sqrt();
                    if mag > max_abs {
                        max_abs = mag;
                    }
                }
            }
        }

        let mut rgba_buffer = vec![0u8; chunk_w * h * 4];
        let snake_palette = GeodesicSnakePalette::new();

        for row in 0..h {
            for col in c_start..c_end {
                let grid_idx = row * w + col;
                let out_idx = (row * chunk_w + (col - c_start)) * 4;

                let re = grid_re[grid_idx];
                let im = grid_im[grid_idx];
                let sample = ComplexNumber32::new(re, im);

                if sample.norm_squared() < 1e-24 {
                    rgba_buffer[out_idx] = 0;
                    rgba_buffer[out_idx + 1] = 0;
                    rgba_buffer[out_idx + 2] = 0;
                    rgba_buffer[out_idx + 3] = 255;
                    continue;
                }

                if request.palette_type == "snake" {
                    let norm_mag = sample.absolute_value() / max_abs;
                    let (r, g, b) = snake_palette.map_normalized_magnitude(norm_mag);
                    rgba_buffer[out_idx] = r;
                    rgba_buffer[out_idx + 1] = g;
                    rgba_buffer[out_idx + 2] = b;
                    rgba_buffer[out_idx + 3] = 255;
                } else {
                    let ycbcr = LuminanceChrominanceYCbCr::from_complex_sample(sample, max_abs);
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

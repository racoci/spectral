//! Caso de Uso: Derivadas de Ordem Superior e Rastreamento de Cristas (Hermite Jet)
//!
//! Avaliação por múltiplas janelas polinomiais de Hermite para cálculo do tensor Hessiano,
//! detecção de cristas espectrais, anisotropia e taxa de variação de frequência (chirp rate).

use super::super::domain_mathematics::color_spaces::{convert_hsv_to_rgb_u8, GeodesicSnakePalette};
use super::super::domain_mathematics::frequency_scales::{
    PerceptualFrequencyScale, LinearFrequencyScale, CauchyLogarithmicFrequencyScale,
    AuditoryMelFrequencyScale, PsychoacousticBarkFrequencyScale,
};
use super::super::infrastructure::audio_decoding::WaveAudioParser;
use super::super::infrastructure::fast_fourier_transform::FastFourierTransformPlannerCache;
use rustfft::num_complex::Complex;
use std::f32::consts::PI;

pub struct HigherOrderDerivativesRequest<'a> {
    pub audio_bytes: &'a [u8],
    pub height: usize,
    pub window_size: usize,
    pub zero_padding_factor: usize,
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
    pub derivative_order: usize,
    pub visualization_mode: usize, // 0 = ridge, 1 = anisotropy, 2 = curvature
    pub palette_type: &'a str,
    pub view_start_ratio: f32,
    pub view_end_ratio: f32,
    pub frequency_scale_type: &'a str,
}

pub struct HigherOrderDerivativesUseCase;

impl HigherOrderDerivativesUseCase {
    pub fn execute(request: HigherOrderDerivativesRequest) -> Vec<u8> {
        let audio_info = match WaveAudioParser::parse_pcm_16bit(request.audio_bytes) {
            Some(info) => info,
            None => return Vec::new(),
        };

        let fs = audio_info.sampling_rate_hz as f32;
        let signal = audio_info.samples;
        let total_samples = signal.len();

        let h = if request.height > 0 { request.height } else { 512 };
        let win_len = if request.window_size > 0 { request.window_size } else { 1024 };
        let pad_factor = if request.zero_padding_factor > 0 { request.zero_padding_factor } else { 2 };
        let n_stft = win_len * pad_factor;

        let hop = 232usize;
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

        // Geração das janelas de Hermite (g0, g1 = g', g2 = g'')
        let half_win = (win_len as f32) * 0.5;
        let sigma = 0.25f32 * half_win;
        let s2 = sigma * sigma;
        let inv_two_s2 = 0.5f32 / s2;

        let mut win_g0 = vec![0.0f32; win_len];
        let mut win_g1 = vec![0.0f32; win_len];
        let mut win_g2 = vec![0.0f32; win_len];

        for i in 0..win_len {
            let t = (i as f32) - half_win;
            let g = (-t * t * inv_two_s2).exp();
            win_g0[i] = g;
            win_g1[i] = -(t / s2) * g;
            win_g2[i] = ((t * t - s2) / (s2 * s2)) * g;
        }

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

        let mut buf_g0 = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buf_g1 = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buf_g2 = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

        let mut rgba_buffer = vec![0u8; chunk_w * h * 4];
        let snake_palette = GeodesicSnakePalette::new();

        for c in c_start..c_end {
            let start = c * hop;

            for i in 0..n_stft {
                buf_g0[i] = Complex::new(0.0, 0.0);
                buf_g1[i] = Complex::new(0.0, 0.0);
                buf_g2[i] = Complex::new(0.0, 0.0);
            }

            for i in 0..win_len {
                let idx = (start as isize) + (i as isize) - (win_len as isize / 2);
                if idx >= 0 && (idx as usize) < total_samples {
                    let s = signal[idx as usize];
                    buf_g0[i] = Complex::new(s * win_g0[i], 0.0);
                    buf_g1[i] = Complex::new(s * win_g1[i], 0.0);
                    buf_g2[i] = Complex::new(s * win_g2[i], 0.0);
                }
            }

            fft.process_with_scratch(&mut buf_g0, &mut scratch);
            fft.process_with_scratch(&mut buf_g1, &mut scratch);
            fft.process_with_scratch(&mut buf_g2, &mut scratch);

            for j in 0..h {
                let k_f = k_f_lut[j];
                let k_floor = ((k_f.floor() as usize).clamp(1, n_stft / 2 - 2)) as usize;
                let k_ceil = k_floor + 1;
                let delta_k = k_f - (k_floor as f32);

                let v0 = buf_g0[k_floor] * (1.0 - delta_k) + buf_g0[k_ceil] * delta_k;
                let v1 = buf_g1[k_floor] * (1.0 - delta_k) + buf_g1[k_ceil] * delta_k;
                let v2 = buf_g2[k_floor] * (1.0 - delta_k) + buf_g2[k_ceil] * delta_k;

                let mag_sq = v0.re * v0.re + v0.im * v0.im;
                let mag = mag_sq.sqrt();
                let norm_db = ((20.0 * (mag / 4194304.0).max(1e-6).log10() + 100.0) / 100.0).clamp(0.0, 1.0);

                let out_idx = (j * chunk_w + (c - c_start)) * 4;

                if mag < 1e-4 {
                    rgba_buffer[out_idx] = 0;
                    rgba_buffer[out_idx + 1] = 0;
                    rgba_buffer[out_idx + 2] = 0;
                    rgba_buffer[out_idx + 3] = 255;
                    continue;
                }

                // Cálculo do tensor Hessiano de segunda ordem via quocientes conformes
                let r = (v1 * v0.conj()) / mag_sq.max(1e-12);
                let q2 = (v2 * v0.conj()) / mag_sq.max(1e-12);
                let s_curvature = q2 - (r * r);

                // Autovalores e anisotropia do tensor de curvatura
                let trace_h = s_curvature.re;
                let det_h = s_curvature.re * s_curvature.re + s_curvature.im * s_curvature.im;
                let discriminant = (trace_h * trace_h - 4.0 * det_h).abs().sqrt();
                let lambda1 = 0.5 * (trace_h + discriminant);
                let lambda2 = 0.5 * (trace_h - discriminant);

                let anisotropy = ((lambda1.abs() - lambda2.abs()).abs() / (lambda1.abs() + lambda2.abs() + 1e-6)).clamp(0.0, 1.0);
                let is_ridge = lambda2.abs() > lambda1.abs() * 1.5 && lambda2 < 0.0;
                let ridge_angle = 0.5 * s_curvature.im.atan2(s_curvature.re);

                let (r_val, g_val, b_val) = if request.palette_type == "snake" {
                    let (mut r, mut g, mut b) = snake_palette.map_normalized_magnitude(norm_db);
                    if is_ridge {
                        r = r.saturating_add(60);
                        g = g.saturating_add(60);
                        b = b.saturating_add(60);
                    }
                    (r, g, b)
                } else {
                    match request.visualization_mode {
                        0 => {
                            // Cristas da Hessiana e Orientação Direcional
                            let theta_norm = (ridge_angle / PI + 0.5).clamp(0.0, 1.0);
                            let hue = theta_norm * 6.0;
                            let sat = (0.35 + 0.65 * anisotropy).clamp(0.0, 1.0);
                            let val = norm_db;
                            let (mut r, mut g, mut b) = convert_hsv_to_rgb_u8(hue, sat, val);
                            if is_ridge {
                                r = r.saturating_add(50);
                                g = g.saturating_add(50);
                                b = b.saturating_add(50);
                            }
                            (r, g, b)
                        },
                        1 => {
                            // Anisotropia Espectral e Chirp Rate
                            let chirp_norm = (s_curvature.im * 0.0005).clamp(-1.0, 1.0);
                            let hue = if chirp_norm >= 0.0 { 3.0 + chirp_norm } else { 0.5 + chirp_norm * 0.5 };
                            let sat = anisotropy.clamp(0.25, 1.0);
                            let val = norm_db * (0.2 + 0.8 * anisotropy);
                            convert_hsv_to_rgb_u8(hue.clamp(0.0, 6.0), sat, val)
                        },
                        _ => {
                            // Curvatura Espectral
                            let curv_norm = (s_curvature.norm_sqr().sqrt() * 0.01).clamp(0.0, 1.0);
                            let hue = curv_norm * 4.5;
                            let sat = 0.85;
                            let val = norm_db;
                            convert_hsv_to_rgb_u8(hue, sat, val)
                        }
                    }
                };

                rgba_buffer[out_idx] = r_val;
                rgba_buffer[out_idx + 1] = g_val;
                rgba_buffer[out_idx + 2] = b_val;
                rgba_buffer[out_idx + 3] = 255;
            }
        }

        rgba_buffer
    }
}

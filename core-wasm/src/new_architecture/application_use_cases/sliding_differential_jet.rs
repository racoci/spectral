//! Caso de Uso: Jato Diferencial Deslizante em Tempo Real (Sliding Jet DFT O(1))
//!
//! Avaliação por recorrência recursiva do jato de Taylor sem alocação ou cálculo de FFTs
//! durante o avanço do laço temporal, garantindo tempo constante O(1) por amostra.

use super::super::domain_mathematics::color_spaces::{convert_hsv_to_rgb_u8, GeodesicSnakePalette};
use super::super::domain_mathematics::frequency_scales::{
    PerceptualFrequencyScale, LinearFrequencyScale, CauchyLogarithmicFrequencyScale,
    AuditoryMelFrequencyScale, PsychoacousticBarkFrequencyScale,
};
use super::super::infrastructure::audio_decoding::WaveAudioParser;
use std::f32::consts::PI;

pub struct SlidingDifferentialJetRequest<'a> {
    pub audio_bytes: &'a [u8],
    pub height: usize,
    pub window_size: usize,
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
    pub derivative_order: usize,
    pub visualization_mode: usize,
    pub palette_type: &'a str,
    pub view_start_ratio: f32,
    pub view_end_ratio: f32,
    pub frequency_scale_type: &'a str,
}

pub struct SlidingDifferentialJetUseCase;

impl SlidingDifferentialJetUseCase {
    pub fn execute(request: SlidingDifferentialJetRequest) -> Vec<u8> {
        let audio_info = match WaveAudioParser::parse_pcm_16bit(request.audio_bytes) {
            Some(info) => info,
            None => return Vec::new(),
        };

        let fs = audio_info.sampling_rate_hz as f32;
        let signal = audio_info.samples;
        let total_samples = signal.len();

        let h = if request.height > 0 { request.height } else { 512 };
        let win_len = if request.window_size > 0 { request.window_size } else { 1024 };
        let sj_n = win_len.min(512);

        let hop = 232usize;
        let w = (total_samples.saturating_sub(sj_n) / hop).max(1);

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

        let mut fc_lut = vec![0.0f32; h];
        let mut sliding_channels = Vec::with_capacity(h);

        for j in 0..h {
            let y = (j as f32) / ((h as f32 - 1.0).max(1.0));
            let fc = scale_box.frequency_from_coordinate(y);
            fc_lut[j] = fc;

            let theta_0 = 2.0 * PI * fc / fs;
            let mut ch = crate::analysis::higher_order::SlidingJetDftChannel::new(sj_n, theta_0, request.derivative_order, 0.99999);
            if signal.len() >= sj_n {
                ch.init(&signal[0..sj_n]);
            }
            sliding_channels.push(ch);
        }

        let sigma_s = 0.4 * (sj_n as f32 * 0.5) / fs;
        let mut current_m = 0usize;

        let mut rgba_buffer = vec![0u8; chunk_w * h * 4];
        let snake_palette = GeodesicSnakePalette::new();

        for c in c_start..c_end {
            let target_m = c * hop;
            let t_c = (target_m as f32) / fs;

            if target_m > current_m && target_m + sj_n <= total_samples {
                if target_m - current_m <= hop * 2 && target_m - current_m < sj_n {
                    for m in current_m..target_m {
                        let x_out = signal[m];
                        let x_in = signal[m + sj_n];
                        for j in 0..h {
                            sliding_channels[j].update(x_out, x_in);
                        }
                    }
                } else {
                    for j in 0..h {
                        sliding_channels[j].init(&signal[target_m..target_m + sj_n]);
                    }
                }
                current_m = target_m;
            }

            for j in 0..h {
                let fc = fc_lut[j];
                let ch = &sliding_channels[j];
                let derivs = ch.extract_derivatives(sigma_s, fc, t_c, fs);

                let mag = if derivs.peak_magnitude > 0.0 { derivs.peak_magnitude } else { derivs.magnitude };
                let norm_db = ((20.0 * (mag / 4194304.0).max(1e-6).log10() + 100.0) / 100.0).clamp(0.0, 1.0);

                let out_idx = (j * chunk_w + (c - c_start)) * 4;

                let (r_val, g_val, b_val) = if request.palette_type == "snake" {
                    let (mut r, mut g, mut b) = snake_palette.map_normalized_magnitude(norm_db);
                    if derivs.is_ridge {
                        r = r.saturating_add(60);
                        g = g.saturating_add(60);
                        b = b.saturating_add(60);
                    }
                    (r, g, b)
                } else {
                    match request.visualization_mode {
                        0 => {
                            let theta_norm = (derivs.ridge_angle_rad / PI + 0.5).clamp(0.0, 1.0);
                            let hue = theta_norm * 6.0;
                            let sat = (0.35 + 0.65 * derivs.anisotropy).clamp(0.0, 1.0);
                            let val = norm_db;
                            let (mut r, mut g, mut b) = convert_hsv_to_rgb_u8(hue, sat, val);
                            if derivs.is_ridge {
                                r = r.saturating_add(50);
                                g = g.saturating_add(50);
                                b = b.saturating_add(50);
                            }
                            (r, g, b)
                        },
                        1 => {
                            let chirp_norm = (derivs.d2_phi_dt2 * 0.0005).clamp(-1.0, 1.0);
                            let hue = if chirp_norm >= 0.0 { 3.0 + chirp_norm } else { 0.5 + chirp_norm * 0.5 };
                            let sat = derivs.anisotropy.clamp(0.25, 1.0);
                            let val = norm_db * (0.2 + 0.8 * derivs.anisotropy);
                            convert_hsv_to_rgb_u8(hue.clamp(0.0, 6.0), sat, val)
                        },
                        _ => {
                            let curv_norm = (-derivs.lambda_1 * 0.0002).clamp(0.0, 1.0);
                            let hue = 4.5 - 2.5 * curv_norm;
                            let sat = 0.85;
                            let val = norm_db * curv_norm.sqrt();
                            let (mut r, mut g, mut b) = convert_hsv_to_rgb_u8(hue, sat, val);
                            if derivs.is_ridge {
                                r = 255;
                                g = g.saturating_add(100);
                                b = 255;
                            }
                            (r, g, b)
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

//! Motor de Síntese Analítica e Renderizador de Espectrograma STN (Sines + Transients + Noise)

use super::stn_model::*;
use std::f32::consts::{PI, TAU};
use rustfft::{FftPlanner, num_complex::Complex};

#[inline(always)]
fn db_to_amp(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

#[inline(always)]
fn clamp_f32(v: f32, min: f32, max: f32) -> f32 {
    v.max(min).min(max)
}

/// Síntese completa de áudio estéreo do modelo STN
pub fn synthesize_stn_pcm(cfg: &StnConfig) -> (Vec<f32>, Vec<f32>) {
    let fs = if cfg.sample_rate > 1000.0 { cfg.sample_rate } else { 48000.0 };
    let total_samples = (fs * cfg.duration_s.max(0.05)).round() as usize;

    let mut left = vec![0.0f32; total_samples];
    let mut right = vec![0.0f32; total_samples];

    // 1. Componente Senoides (S)
    if cfg.enable_s {
        let s_master = db_to_amp(cfg.s_gain_db);
        let pan = cfg.pan_s.clamp(-1.0, 1.0);
        let pan_gl = ((1.0 - pan) * 0.5).sqrt();
        let pan_gr = ((1.0 + pan) * 0.5).sqrt();

        for s in &cfg.sines {
            if !s.enabled {
                continue;
            }
            let part_gain = db_to_amp(s.db) * s_master;
            let start_sample = (s.start_s * fs).round() as usize;
            let end_sample = ((s.end_s * fs).round() as usize).min(total_samples);

            if start_sample >= end_sample || start_sample >= total_samples {
                continue;
            }

            let fade_in_s = (s.fade_in_ms * 0.001 * fs).max(1.0);
            let fade_out_s = (s.fade_out_ms * 0.001 * fs).max(1.0);
            let dur_samples = (end_sample - start_sample) as f32;

            for i in start_sample..end_sample {
                let local_i = (i - start_sample) as f32;
                let t_sec = local_i / fs;

                // Envelope suave
                let mut env = 1.0f32;
                if local_i < fade_in_s {
                    env = env.min(local_i / fade_in_s);
                }
                let rem = dur_samples - local_i;
                if rem < fade_out_s {
                    env = env.min(rem / fade_out_s);
                }

                // Modulação de frequência por vibrato e inarmonicidade
                let vib_mod = s.vibrato_hz * (TAU * s.vib_rate_hz * t_sec).sin();
                let inharm_factor = 1.0 + s.inharmonicity * (s.id * s.id) as f32 * 0.001;
                let f_inst = s.f_hz * inharm_factor + vib_mod;

                let phase = TAU * f_inst * t_sec + s.phase_rad;
                let sample_val = phase.sin() * part_gain * env.clamp(0.0, 1.0);

                left[i] += sample_val * pan_gl;
                right[i] += sample_val * pan_gr;
            }
        }
    }

    // 2. Componente Transientes (T)
    if cfg.enable_t {
        let t_master = db_to_amp(cfg.t_gain_db);
        let pan = cfg.pan_t.clamp(-1.0, 1.0);
        let pan_gl = ((1.0 - pan) * 0.5).sqrt();
        let pan_gr = ((1.0 + pan) * 0.5).sqrt();

        for e in &cfg.events {
            if !e.enabled {
                continue;
            }
            let ev_gain = db_to_amp(e.db) * t_master;
            let start_sample = (e.start_s * fs).round() as usize;
            let dur_samples = ((e.dur_ms * 0.001 * fs).round() as usize).max(4);
            let end_sample = (start_sample + dur_samples).min(total_samples);

            if start_sample >= total_samples || start_sample >= end_sample {
                continue;
            }

            for i in start_sample..end_sample {
                let j = (i - start_sample) as f32;
                let norm_j = j / dur_samples as f32;

                // Janela temporal
                let win = match e.window.as_str() {
                    "Blackman" => 0.42 - 0.5 * (TAU * norm_j).cos() + 0.08 * (4.0 * PI * norm_j).cos(),
                    "Retangular" => 1.0,
                    _ => 0.5 - 0.5 * (TAU * norm_j).cos(), // Hann
                };

                // Decaimento do impulso
                let decay = match e.decay.as_str() {
                    "Impulso curto" => (-j / 8.0).exp(),
                    "Duplo exponencial" => ((-norm_j * 12.0).exp() - (-norm_j * 40.0).exp()) * 2.0,
                    "Ressonante" => (-norm_j * 6.0).exp() * (TAU * 1400.0 * (j / fs)).sin(),
                    _ => (-norm_j * 5.0).exp(), // Exponencial
                };

                // Modelo de representação (PCM / DCT / Ruído de ataque)
                let carrier = match e.type_repr.as_str() {
                    "dct" => {
                        let c0 = (PI * norm_j * 2.0).cos();
                        let c1 = (PI * norm_j * 6.0).cos() * 0.5;
                        c0 + c1
                    }
                    "noise" => {
                        let mut seed = ((i * 1103515245 + 12345) & 0x7fffffff) as f32;
                        (seed / 1073741824.0) - 1.0
                    }
                    _ => (TAU * 1100.0 * (j / fs)).sin(), // PCM
                };

                let val = carrier * decay * win * ev_gain;
                left[i] += val * pan_gl;
                right[i] += val * pan_gr;
            }
        }
    }

    // 3. Componente Ruído Estocástico (N)
    if cfg.enable_n {
        let n_master = db_to_amp(cfg.n_gain_db + cfg.noise.env_gain_db);
        let pan = cfg.pan_n.clamp(-1.0, 1.0);
        let pan_gl = ((1.0 - pan) * 0.5).sqrt();
        let pan_gr = ((1.0 + pan) * 0.5).sqrt();

        let mut seed = cfg.noise.seed.max(1);
        let mut xor_rand = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            ((seed & 0x7fffffff) as f32 / 1073741824.0) - 1.0
        };

        // Banco de 8 filtros passa-faixa com frequências centrais
        let band_freqs = [80.0f32, 250.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 16000.0];
        let mut band_states = [0.0f32; 8];
        let mut band_coeffs = [0.0f32; 8];

        for (b, &fc) in band_freqs.iter().enumerate() {
            let fc_clamped = fc.min(fs * 0.45);
            band_coeffs[b] = (-TAU * fc_clamped / fs).exp();
        }

        let bands_db = &cfg.noise.bands_db;
        let corr = cfg.noise.correlation.clamp(0.0, 0.95);
        let mut prev_sample = 0.0f32;

        for i in 0..total_samples {
            let white = xor_rand();
            let mut noise_sample = 0.0f32;
            let mut prev_band = 0.0f32;

            for b in 0..8 {
                let a = band_coeffs[b];
                band_states[b] = (1.0 - a) * white + a * band_states[b];
                let band_out = band_states[b] - prev_band;
                prev_band = band_states[b];

                let gain = if b < bands_db.len() {
                    db_to_amp(bands_db[b])
                } else {
                    1.0
                };
                noise_sample += band_out * gain;
            }

            // Correlação temporal
            let smoothed = (1.0 - corr) * noise_sample + corr * prev_sample;
            prev_sample = smoothed;

            let val = smoothed * n_master * 0.35;
            left[i] += val * pan_gl;
            right[i] += val * pan_gr;
        }
    }

    // 4. Binauralização Espacial ITD (Interaural Time Difference) e ILD
    let itd_samples = (cfg.itd_ms * 0.001 * fs).round() as isize;
    let ild_factor = db_to_amp(cfg.ild_db * 0.5);

    if itd_samples > 0 {
        let d = itd_samples as usize;
        for i in (d..total_samples).rev() {
            right[i] = right[i - d];
        }
        for i in 0..d.min(total_samples) {
            right[i] = 0.0;
        }
    } else if itd_samples < 0 {
        let d = (-itd_samples) as usize;
        for i in (d..total_samples).rev() {
            left[i] = left[i - d];
        }
        for i in 0..d.min(total_samples) {
            left[i] = 0.0;
        }
    }

    // Aplicação de ILD e Master Volume
    let master = db_to_amp(cfg.master_gain_db);
    for i in 0..total_samples {
        left[i] *= ild_factor * master;
        right[i] *= (1.0 / ild_factor) * master;
    }

    // 5. Normalização de Pico
    if cfg.normalize {
        let mut peak = 1e-9f32;
        for i in 0..total_samples {
            peak = peak.max(left[i].abs()).max(right[i].abs());
        }
        let target = db_to_amp(cfg.peak_limit_db);
        if peak > 0.0 {
            let scale = target / peak;
            for i in 0..total_samples {
                left[i] = (left[i] * scale).clamp(-1.0, 1.0);
                right[i] = (right[i] * scale).clamp(-1.0, 1.0);
            }
        }
    }

    (left, right)
}

/// Converte o sinal estéreo PCM float32 para arquivo WAV de 16-bit
pub fn encode_stn_wav(left: &[f32], right: &[f32], sample_rate: u32) -> Vec<u8> {
    let n = left.len().min(right.len());
    let mut wav = Vec::with_capacity(44 + n * 4);

    // Cabeçalho RIFF
    wav.extend_from_slice(b"RIFF");
    let total_chunk_size = (36 + n * 4) as u32;
    wav.extend_from_slice(&total_chunk_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // Subchunk fmt
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // Tamanho do subchunk fmt
    wav.extend_from_slice(&1u16.to_le_bytes());  // PCM
    wav.extend_from_slice(&2u16.to_le_bytes());  // 2 Canais (Estéreo)
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * 4;
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());  // Block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample

    // Subchunk data
    wav.extend_from_slice(b"data");
    let data_size = (n * 4) as u32;
    wav.extend_from_slice(&data_size.to_le_bytes());

    for i in 0..n {
        let s_l = (left[i].clamp(-1.0, 1.0) * 32767.0).round() as i16;
        let s_r = (right[i].clamp(-1.0, 1.0) * 32767.0).round() as i16;
        wav.extend_from_slice(&s_l.to_le_bytes());
        wav.extend_from_slice(&s_r.to_le_bytes());
    }

    wav
}

/// Renderiza o espectrograma real com reatribuição (Reassignment) em alta performance em Rust
pub fn render_stn_spectrogram_rgba(
    cfg: &StnConfig,
    width: usize,
    height: usize,
    fmin_in: f32,
    fmax_in: f32,
    view_mode: &str,
    palette_name: &str,
) -> Vec<u8> {
    let (left, _) = synthesize_stn_pcm(cfg);
    let fs = cfg.sample_rate;
    let total_samples = left.len();

    let w = width.max(64);
    let h = height.max(32);
    let fmin = fmin_in.max(10.0);
    let fmax = fmax_in.min(fs * 0.49).max(fmin + 50.0);

    let n_fft = 2048usize;
    let hop = (total_samples / w).max(32);
    let num_frames = (total_samples.saturating_sub(n_fft) / hop).max(1);

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_fft);

    let mut window = vec![0.0f32; n_fft];
    for i in 0..n_fft {
        window[i] = 0.5 - 0.5 * (TAU * i as f32 / (n_fft - 1) as f32).cos();
    }

    let mut fft_buffer = vec![Complex::<f32>::new(0.0, 0.0); n_fft];
    let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

    // Matriz de magnitude acumulada [h, w]
    let mut energy_grid = vec![0.0f32; w * h];
    let mut max_energy = 1e-9f32;

    let log_min = fmin.ln();
    let log_max = fmax.ln();

    let mut prev_phases = vec![0.0f32; n_fft / 2];

    for f_idx in 0..num_frames {
        let center = f_idx * hop;
        if center + n_fft > total_samples {
            break;
        }

        for i in 0..n_fft {
            fft_buffer[i] = Complex::new(left[center + i] * window[i], 0.0);
        }

        fft.process_with_scratch(&mut fft_buffer, &mut scratch);

        let out_col = (f_idx * w / num_frames).min(w - 1);

        for k in 1..(n_fft / 2) {
            let bin_f = k as f32 * fs / n_fft as f32;
            if bin_f < fmin || bin_f > fmax {
                continue;
            }

            let bin = fft_buffer[k];
            let mag = (bin.re * bin.re + bin.im * bin.im).sqrt();
            let phase = bin.im.atan2(bin.re);

            let mut target_f = bin_f;
            let mut target_col = out_col;

            // Reatribuição (Reassignment) analítica tempo-frequência
            if view_mode == "reassignment" && f_idx > 0 {
                let dphi = phase - prev_phases[k] - TAU * k as f32 * (hop as f32 / n_fft as f32);
                let wrapped_dphi = (dphi + PI).rem_euclid(TAU) - PI;
                let reass_f = bin_f + wrapped_dphi * fs / (TAU * hop as f32);
                target_f = clamp_f32(reass_f, fmin, fmax);

                if k > 1 && k < n_fft / 2 - 1 {
                    let dphi_k = fft_buffer[k + 1].im.atan2(fft_buffer[k + 1].re) - fft_buffer[k - 1].im.atan2(fft_buffer[k - 1].re);
                    let wrapped_k = (dphi_k + PI).rem_euclid(TAU) - PI;
                    let dt_offset = -wrapped_k / (2.0 * TAU / n_fft as f32);
                    let reass_sample = (center as f32 + dt_offset).clamp(0.0, total_samples as f32 - 1.0);
                    target_col = ((reass_sample / total_samples as f32) * w as f32) as usize;
                    target_col = target_col.min(w - 1);
                }
            }
            prev_phases[k] = phase;

            let y_norm = (target_f.ln() - log_min) / (log_max - log_min);
            let row = ((1.0 - y_norm.clamp(0.0, 1.0)) * (h as f32 - 1.0)).round() as usize;
            let row_idx = row.min(h - 1);

            let idx = row_idx * w + target_col;
            energy_grid[idx] += mag;
            if energy_grid[idx] > max_energy {
                max_energy = energy_grid[idx];
            }
        }
    }

    // Mapeamento para buffer RGBA sólido conforme a paleta
    let mut rgba = vec![255u8; w * h * 4];

    for idx in 0..(w * h) {
        let e = energy_grid[idx];
        let db = if e > 1e-9 {
            20.0 * (e / max_energy).log10()
        } else {
            -120.0
        };
        let u = ((db + 100.0) / 100.0).clamp(0.0, 1.0);

        let out_idx = idx * 4;
        let (r, g, b) = match palette_name {
            "viridis" => {
                let h_deg = 260.0 - 260.0 * u;
                hsl_to_rgb(h_deg, 0.75 + u * 0.25, 0.12 + u * 0.55)
            }
            "magma" => {
                let h_deg = (290.0 - 300.0 * u).rem_euclid(360.0);
                hsl_to_rgb(h_deg, 0.95, 0.08 + u * 0.65)
            }
            "mono" => {
                let lum = (u * 255.0).round() as u8;
                (lum, lum, lum)
            }
            _ => {
                // "inferno" clássico
                let r_val = (u * 2.2).clamp(0.0, 1.0);
                let g_val = (u.powf(2.0) * 1.5).clamp(0.0, 1.0);
                let b_val = (u.powf(4.0) * 2.5).clamp(0.0, 1.0);
                (
                    (r_val * 255.0).round() as u8,
                    (g_val * 255.0).round() as u8,
                    (b_val * 255.0).round() as u8,
                )
            }
        };

        rgba[out_idx] = r;
        rgba[out_idx + 1] = g;
        rgba[out_idx + 2] = b;
        rgba[out_idx + 3] = 255;
    }

    rgba
}

#[inline(always)]
fn hsl_to_rgb(h_deg: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = (h_deg / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (h_prime.rem_euclid(2.0) - 1.0).abs());
    let m = l - c * 0.5;

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
        ((r1 + m).clamp(0.0, 1.0) * 255.0).round() as u8,
        ((g1 + m).clamp(0.0, 1.0) * 255.0).round() as u8,
        ((b1 + m).clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stn_synthesis_and_wav_generation() {
        let mut cfg = StnConfig::default();
        cfg.duration_s = 0.2;
        cfg.sample_rate = 16000.0;

        let (left, right) = synthesize_stn_pcm(&cfg);
        assert_eq!(left.len(), right.len());
        assert_eq!(left.len(), (16000.0 * 0.2) as usize);

        // Verifica que o sinal não é puramente silencioso
        let mut energy = 0.0f32;
        for &s in &left {
            energy += s * s;
        }
        assert!(energy > 1e-4, "Energia do sinal STN sintetizado deve ser positiva");

        // Verifica geração de WAV
        let wav = encode_stn_wav(&left, &right, 16000);
        assert!(wav.len() > 44);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_stn_spectrogram_render_fast() {
        let mut cfg = StnConfig::default();
        cfg.duration_s = 0.15;
        cfg.sample_rate = 12000.0;

        let rgba = render_stn_spectrogram_rgba(&cfg, 120, 64, 40.0, 5000.0, "reassignment", "inferno");
        assert_eq!(rgba.len(), 120 * 64 * 4);
        // Garante opacidade 255
        for (i, &a) in rgba.iter().skip(3).step_by(4).enumerate() {
            assert_eq!(a, 255, "Alpha do pixel {} deve ser estritamente 255", i);
        }
    }
}

//! Analisador Analítico DDSP (Problema Inverso em Forma Fechada)
//!
//! Implementa os estimadores de parâmetros estabelecidos nos estágios E00-E18:
//! - E03/E04: Frequência fundamental f0 e amplitude A com interpolação parabólica sub-bin
//! - E05: Envelope temporal ADSR (tau_A, tau_D, S, tau_R)
//! - E06: Estrutura harmônica H_k com gauge H1 = 1.0 e inclinação alpha
//! - E07: Envelope espectral sob condições de gauge por mínimos quadrados
//! - E08: Coeficiente de inarmonicidade B de cordas por regressão polinomial
//! - E09: Taxa e profundidade de LFO vibrato
//! - E13: Piso de ruído estocástico residual

use std::f32::consts::PI;
use rustfft::{FftPlanner, num_complex::Complex};
use super::ddsp_model::*;

/// Analisa um buffer de amostras de áudio e extrai a configuração DDSP completa correspondente
pub fn analyze_audio_to_ddsp(pcm: &[f32], sample_rate: f32, f0_hint: Option<f32>) -> DdspConfig {
    let mut config = DdspConfig::default();
    config.sample_rate = sample_rate;
    config.duration_s = pcm.len() as f32 / sample_rate;

    if pcm.len() < 256 {
        return config;
    }

    // 1. Extração de F0 e Amplitude por FFT com interpolação parabólica de alta precisão
    let (f0_est, amp_peak) = extract_f0_and_amplitude(pcm, sample_rate, f0_hint);
    config.f0 = f0_est;
    config.amplitude = amp_peak.clamp(0.05, 1.0);

    // 2. Extração do Envelope ADSR C1
    config.adsr = extract_adsr_envelope(pcm, sample_rate);

    // 3. Extração da Série Harmônica e Inarmonicidade B (E06, E08)
    let (harmonics_cfg, measured_partial_freqs) = extract_harmonics_and_inharmonicity(pcm, sample_rate, f0_est);
    config.harmonics = harmonics_cfg;

    // 4. Extração do Envelope Espectral sob Gauge (E07)
    config.spectral_envelope = extract_gauge_spectral_envelope(
        &config.harmonics.amplitudes,
        &measured_partial_freqs,
    );

    // 5. Extração de LFO Vibrato (E09)
    config.lfo = extract_lfo_parameters(pcm, sample_rate, f0_est);

    // 6. Estimativa de Ruído Residual (E13)
    config.noise = estimate_noise_floor(pcm, sample_rate, f0_est, &config.harmonics.amplitudes);

    config
}

/// Extrai f0 fundamental e amplitude através do pico espectral mais saliente
fn extract_f0_and_amplitude(pcm: &[f32], sr: f32, f0_hint: Option<f32>) -> (f32, f32) {
    let n = pcm.len().min(8192);
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);

    let mut buffer = Vec::with_capacity(n);
    for i in 0..n {
        let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / n as f32).cos()); // Hann window
        buffer.push(Complex::new(pcm[i] * w, 0.0));
    }
    fft.process(&mut buffer);

    let half = n / 2;
    let mut mags = vec![0.0f32; half];
    for i in 0..half {
        mags[i] = buffer[i].norm();
    }

    let min_bin = (40.0 * n as f32 / sr).ceil() as usize;
    let max_bin = ((4000.0 * n as f32 / sr).floor() as usize).min(half - 2);

    let mut best_bin = min_bin;
    let mut best_mag = 0.0f32;

    if let Some(hint) = f0_hint {
        if hint > 30.0 {
            let center_bin = (hint * n as f32 / sr).round() as usize;
            let lo = center_bin.saturating_sub(12).max(min_bin);
            let hi = (center_bin + 12).min(max_bin);
            for b in lo..=hi {
                if mags[b] > best_mag {
                    best_mag = mags[b];
                    best_bin = b;
                }
            }
        }
    }

    if best_mag < 1e-6 {
        for b in min_bin..max_bin {
            if mags[b] > best_mag {
                best_mag = mags[b];
                best_bin = b;
            }
        }
    }

    // Interpolação parabólica sub-bin no log-magnitude
    let delta = if best_bin > 0 && best_bin < half - 1 {
        let a = (mags[best_bin - 1] + 1e-12).ln();
        let b = (mags[best_bin] + 1e-12).ln();
        let c = (mags[best_bin + 1] + 1e-12).ln();
        let denom = a - 2.0 * b + c;
        if denom.abs() > 1e-9 {
            0.5 * (a - c) / denom
        } else {
            0.0
        }
    } else {
        0.0
    };

    let f0 = (best_bin as f32 + delta) * sr / n as f32;
    let amplitude = (best_mag / (n as f32 * 0.25)).clamp(0.01, 1.0);

    (f0.max(20.0), amplitude)
}

/// Extrai tempos de ataque, decay, sustain e release a partir da envoltória RMS
fn extract_adsr_envelope(pcm: &[f32], sr: f32) -> AdsrEnvelopeConfig {
    let frame_size = ((sr * 0.010) as usize).max(64); // frames de 10 ms
    let num_frames = pcm.len() / frame_size;
    if num_frames < 4 {
        return AdsrEnvelopeConfig::default();
    }

    let mut env = Vec::with_capacity(num_frames);
    for m in 0..num_frames {
        let slice = &pcm[m * frame_size..(m + 1) * frame_size];
        let rms = (slice.iter().map(|&s| s * s).sum::<f32>() / slice.len() as f32).sqrt();
        env.push(rms);
    }

    let peak_val = env.iter().cloned().fold(0.0f32, f32::max);
    if peak_val < 1e-5 {
        return AdsrEnvelopeConfig::default();
    }

    let peak_idx = env.iter().position(|&v| (v - peak_val).abs() < 1e-6).unwrap_or(0);
    let attack_s = ((peak_idx + 1) as f32 * frame_size as f32 / sr).clamp(0.005, 0.5);

    // Achar sustain na região média (entre 40% e 75% da duração)
    let mid_start = (num_frames * 4 / 10).max(peak_idx + 1).min(num_frames - 1);
    let mid_end = (num_frames * 75 / 100).max(mid_start + 1).min(num_frames);
    let sustain_val = if mid_end > mid_start {
        let mut mid_slice: Vec<f32> = env[mid_start..mid_end].to_vec();
        mid_slice.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        mid_slice[mid_slice.len() / 2]
    } else {
        peak_val * 0.7
    };
    let sustain_norm = (sustain_val / peak_val).clamp(0.1, 0.95);

    let decay_frames = mid_start.saturating_sub(peak_idx).max(1);
    let decay_s = (decay_frames as f32 * frame_size as f32 / sr).clamp(0.01, 0.8);

    // Release do final para o início
    let mut end_idx = num_frames - 1;
    while end_idx > mid_end && env[end_idx] < peak_val * 0.08 {
        end_idx -= 1;
    }
    let release_frames = (num_frames - end_idx).max(2);
    let release_s = (release_frames as f32 * frame_size as f32 / sr).clamp(0.02, 0.8);

    AdsrEnvelopeConfig {
        attack_s,
        decay_s,
        sustain: sustain_norm,
        release_s,
        curve: EnvCurveKind::CubicSmooth,
    }
}

/// Extrai amplitudes parciais H_k e inarmonicidade B através de regressão analítica
fn extract_harmonics_and_inharmonicity(pcm: &[f32], sr: f32, f0: f32) -> (HarmonicsConfig, Vec<f32>) {
    let n = pcm.len().min(16384);
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);

    let mut buffer = Vec::with_capacity(n);
    for i in 0..n {
        let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / n as f32).cos());
        buffer.push(Complex::new(pcm[i] * w, 0.0));
    }
    fft.process(&mut buffer);

    let max_k = 16;
    let mut raw_amps = Vec::with_capacity(max_k);
    let mut measured_freqs = Vec::with_capacity(max_k);

    for k in 1..=max_k {
        let nominal_f = k as f32 * f0;
        if nominal_f >= sr * 0.45 {
            break;
        }

        let target_bin = (nominal_f * n as f32 / sr).round() as usize;
        let lo = target_bin.saturating_sub(6).max(1);
        let hi = (target_bin + 6).min(n / 2 - 2);

        let mut best_m = 0.0f32;
        let mut best_b = target_bin;
        for b in lo..=hi {
            let m = buffer[b].norm();
            if m > best_m {
                best_m = m;
                best_b = b;
            }
        }

        // Sub-bin peak
        let a = (buffer[best_b - 1].norm() + 1e-12).ln();
        let b = (buffer[best_b].norm() + 1e-12).ln();
        let c = (buffer[best_b + 1].norm() + 1e-12).ln();
        let delta = if (a - 2.0 * b + c).abs() > 1e-9 {
            0.5 * (a - c) / (a - 2.0 * b + c)
        } else {
            0.0
        };

        let exact_f = (best_b as f32 + delta) * sr / n as f32;
        measured_freqs.push(exact_f);
        raw_amps.push(best_m);
    }

    if raw_amps.is_empty() {
        return (HarmonicsConfig::default(), vec![f0]);
    }

    // Normalização de gauge H1 = 1.0 (Estágio E06)
    let h1 = raw_amps[0].max(1e-6);
    let normalized_amps: Vec<f32> = raw_amps.iter().map(|&a| (a / h1).clamp(0.0001, 1.0)).collect();

    // Estimativa de inclinação de roll-off alpha (regressão linear log-log)
    let mut sum_ln_k_sq = 0.0f32;
    let mut sum_ln_k_ln_h = 0.0f32;
    for k in 2..=normalized_amps.len().min(6) {
        let ln_k = (k as f32).ln();
        let ln_h = (normalized_amps[k - 1] + 1e-9).ln();
        sum_ln_k_sq += ln_k * ln_k;
        sum_ln_k_ln_h += ln_k * ln_h;
    }
    let alpha = if sum_ln_k_sq > 1e-6 {
        (-sum_ln_k_ln_h / sum_ln_k_sq).clamp(0.2, 3.5)
    } else {
        1.1
    };

    // Estimativa de Inarmonicidade B (Estágio E08)
    // (fk / k)^2 ~ f0^2 + (B * f0^2) * k^2
    let mut inharmonicity_b = 0.0f32;
    let num_pts = measured_freqs.len().min(8);
    if num_pts >= 4 {
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        let mut sum_xx = 0.0f32;
        let mut sum_xy = 0.0f32;

        for k_idx in 1..=num_pts {
            let k_f = k_idx as f32;
            let x = k_f * k_f; // k^2
            let y = (measured_freqs[k_idx - 1] / k_f).powi(2); // (fk / k)^2

            sum_x += x;
            sum_y += y;
            sum_xx += x * x;
            sum_xy += x * y;
        }

        let n_f = num_pts as f32;
        let denom = n_f * sum_xx - sum_x * sum_x;
        if denom.abs() > 1e-6 {
            let slope = (n_f * sum_xy - sum_x * sum_y) / denom;
            let intercept = (sum_y - slope * sum_x) / n_f;
            if intercept > 10.0 && slope > 0.0 {
                inharmonicity_b = (slope / intercept).clamp(0.0, 0.015);
            }
        }
    }

    let cfg = HarmonicsConfig {
        amplitudes: normalized_amps,
        phases: vec![0.0; raw_amps.len()],
        inharmonicity_b,
        roll_off_alpha: alpha,
    };

    (cfg, measured_freqs)
}

/// Extrai coeficientes w0, w1, w2, w3 da base contínua sob gauge (Estágio E07)
fn extract_gauge_spectral_envelope(
    harmonic_amps: &[f32],
    measured_freqs: &[f32],
) -> SpectralEnvelopeConfig {
    let mut config = SpectralEnvelopeConfig::default();
    let n = harmonic_amps.len().min(measured_freqs.len());
    if n < 3 {
        return config;
    }

    // Regressão simples no termo quadrático principal w0 (curvatura formântica centrada em 440 Hz)
    let mut num = 0.0f32;
    let mut den = 0.0f32;

    for i in 0..n {
        let f = measured_freqs[i];
        let u = (f / 440.0).log2();
        let u2 = u * u;
        let amp_db = 20.0 * harmonic_amps[i].max(1e-4).log10();

        num += u2 * amp_db;
        den += u2 * u2;
    }

    let w0 = if den > 1e-6 { (num / den).clamp(-4.0, 4.0) } else { 0.0 };
    config.gauge_weights = [w0, 0.0, 0.0, 0.0];
    config
}

/// Estima taxa fm e profundidade d de modulação periódica LFO (Estágio E09)
fn extract_lfo_parameters(pcm: &[f32], sr: f32, f0: f32) -> LfoConfig {
    let hop = ((sr * 0.005) as usize).max(32); // 5 ms por passo de pitch
    let win = hop * 4;
    let num_steps = (pcm.len().saturating_sub(win)) / hop;

    if num_steps < 32 {
        return LfoConfig::default();
    }

    let mut pitch_track = Vec::with_capacity(num_steps);
    for m in 0..num_steps {
        let start = m * hop;
        let (f_local, _) = extract_f0_and_amplitude(&pcm[start..start + win], sr, Some(f0));
        let cents = 1200.0 * (f_local / f0).log2();
        pitch_track.push(cents);
    }

    // Remover DC
    let mean_cents = pitch_track.iter().sum::<f32>() / pitch_track.len() as f32;
    for c in &mut pitch_track {
        *c -= mean_cents;
    }

    // Achar frequência de pico no espectro da trajetória de cents (entre 1.5 e 12 Hz)
    let nfft = pitch_track.len().next_power_of_two();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(nfft);

    let mut buf = vec![Complex::new(0.0f32, 0.0); nfft];
    for (i, &c) in pitch_track.iter().enumerate() {
        buf[i] = Complex::new(c, 0.0);
    }
    fft.process(&mut buf);

    let fs_lfo = sr / hop as f32;
    let bin_lo = (1.5 * nfft as f32 / fs_lfo).ceil() as usize;
    let bin_hi = ((12.0 * nfft as f32 / fs_lfo).floor() as usize).min(nfft / 2);

    let mut best_b = bin_lo;
    let mut max_mag = 0.0f32;
    for b in bin_lo..=bin_hi {
        let m = buf[b].norm();
        if m > max_mag {
            max_mag = m;
            best_b = b;
        }
    }

    let fm_est = best_b as f32 * fs_lfo / nfft as f32;
    let depth_cents = (max_mag * 2.0 / pitch_track.len() as f32).clamp(0.0, 80.0);

    LfoConfig {
        enabled: depth_cents > 3.0,
        rate_hz: fm_est.clamp(1.5, 12.0),
        depth_cents,
        phase_rad: 0.0,
        waveform: LfoWaveform::Sine,
    }
}

/// Estima nível de ruído residual subtraindo a reconstrução harmônica (Estágio E13)
fn estimate_noise_floor(pcm: &[f32], sr: f32, f0: f32, harmonic_amps: &[f32]) -> NoiseConfig {
    let n = pcm.len().min(4096);
    let dt = 1.0 / sr;

    let mut harm_synth = vec![0.0f32; n];
    for k in 1..=harmonic_amps.len().min(8) {
        let amp = harmonic_amps[k - 1];
        let fk = k as f32 * f0;
        for i in 0..n {
            harm_synth[i] += amp * (std::f32::consts::TAU * fk * (i as f32 * dt)).sin();
        }
    }

    let signal_energy: f32 = pcm[..n].iter().map(|&s| s * s).sum::<f32>().max(1e-7);
    let residual_energy: f32 = (0..n).map(|i| (pcm[i] - harm_synth[i]).powi(2)).sum();
    let snr_db = (signal_energy / residual_energy.max(1e-7)).log10() * 10.0;

    let noise_level_db = (-snr_db).clamp(-60.0, -10.0);

    NoiseConfig {
        enabled: noise_level_db > -45.0,
        level_db: noise_level_db,
        alpha: 1.0, // Pink noise baseline
        knee_hz: 1500.0,
        mix: 0.05,
    }
}

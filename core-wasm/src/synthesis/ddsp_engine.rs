//! Motor de Síntese DDSP (Differentiable Digital Signal Processing) e TreeNN em Rust
//!
//! Implementação com zero alocações no loop interno, garantindo fidelidade matemática
//! estrita com as leis físicas e os invariantes do currículo SynthNN (E00-E18).

use std::f32::consts::{PI, TAU};
use super::ddsp_model::*;

/// Avalia a curva de envelope ADSR C1 contínua com interpolação cúbica nos nós
#[inline(always)]
pub fn evaluate_adsr(
    t: f32,
    duration: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    curve: EnvCurveKind,
) -> f32 {
    if t < 0.0 || t >= duration {
        return 0.0;
    }

    let a = attack.max(0.001);
    let d = decay.max(0.001);
    let s = sustain.clamp(0.0, 1.0);
    let r = release.max(0.001);

    let t_release_start = (duration - r).max(a + d);

    if t < a {
        // Ataque: [0, a) -> sobe de 0.0 a 1.0
        let u = (t / a).clamp(0.0, 1.0);
        match curve {
            EnvCurveKind::Linear => u,
            EnvCurveKind::Exponential => u * u,
            EnvCurveKind::CubicSmooth => 3.0 * u * u - 2.0 * u * u * u,
        }
    } else if t < a + d {
        // Decay: [a, a + d) -> desce de 1.0 a sustain
        let u = ((t - a) / d).clamp(0.0, 1.0);
        let factor = match curve {
            EnvCurveKind::Linear => u,
            EnvCurveKind::Exponential => u * u,
            EnvCurveKind::CubicSmooth => 3.0 * u * u - 2.0 * u * u * u,
        };
        1.0 - factor * (1.0 - s)
    } else if t < t_release_start {
        // Sustain
        s
    } else {
        // Release: [t_release_start, duration) -> desce de sustain a 0.0
        let rel_duration = duration - t_release_start;
        let u = if rel_duration > 1e-6 {
            ((t - t_release_start) / rel_duration).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let factor = match curve {
            EnvCurveKind::Linear => u,
            EnvCurveKind::Exponential => u * u,
            EnvCurveKind::CubicSmooth => 3.0 * u * u - 2.0 * u * u * u,
        };
        s * (1.0 - factor).max(0.0)
    }
}

/// Avalia a base do envelope espectral sob condições de gauge (Estágio E07)
/// Phi_0(u) = u^2
/// Phi_1(u) = u^3
/// Phi_2(u) = u^2 / (1 + 0.5 * u^2)
/// Phi_3(u) = u^3 / (1 + 0.5 * u^2)
/// onde u = log2(f / 440 Hz). Satisfaz Phi_j(0) = 0 e Phi'_j(0) = 0.
#[inline(always)]
pub fn evaluate_gauge_spectral_basis(freq_hz: f32, weights: &[f32; 4]) -> f32 {
    let u = (freq_hz.max(10.0) / 440.0).log2();
    let u2 = u * u;
    let u3 = u2 * u;
    let denom = 1.0 + 0.5 * u2;

    let phi0 = u2;
    let phi1 = u3;
    let phi2 = u2 / denom;
    let phi3 = u3 / denom;

    let db = weights[0] * phi0 + weights[1] * phi1 + weights[2] * phi2 + weights[3] * phi3;
    // Converte ganho em dB para escala linear com saturação de segurança (-40 dB a +20 dB)
    10.0f32.powf(db.clamp(-40.0, 20.0) / 20.0)
}

/// Avalia formantes acústicos/vocais
#[inline(always)]
pub fn evaluate_formants(freq_hz: f32, formants: &[FormantFilter]) -> f32 {
    if formants.is_empty() {
        return 1.0;
    }
    let f_safe = freq_hz.max(10.0);
    let mut total_linear = 1.0f32;
    for f in formants {
        if f.center_hz > 20.0 {
            let u = (f_safe / f.center_hz).log2();
            let bw_oct = (f.bandwidth_hz / f.center_hz).max(0.05);
            let peak_db = f.gain_db * (-0.5 * (u / bw_oct).powi(2)).exp();
            total_linear *= 10.0f32.powf(peak_db.clamp(-24.0, 24.0) / 20.0);
        }
    }
    total_linear.clamp(0.05, 10.0)
}

/// Avalia forma de onda para LFO
#[inline(always)]
pub fn evaluate_lfo_waveform(phase: f32, shape: LfoWaveform) -> f32 {
    let norm_phase = (phase / TAU).rem_euclid(1.0);
    match shape {
        LfoWaveform::Sine => phase.sin(),
        LfoWaveform::Triangle => {
            if norm_phase < 0.5 {
                4.0 * norm_phase - 1.0
            } else {
                3.0 - 4.0 * norm_phase
            }
        }
        LfoWaveform::Saw => 2.0 * norm_phase - 1.0,
        LfoWaveform::Square => {
            if norm_phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
    }
}

/// Filtro SVF (State Variable Filter) de topologia zero-delay feedback
#[derive(Clone, Debug)]
pub struct StateVariableFilter {
    ic1eq: f32,
    ic2eq: f32,
}

impl StateVariableFilter {
    pub fn new() -> Self {
        Self {
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    #[inline(always)]
    pub fn process(&mut self, sample: f32, cutoff: f32, q: f32, sr: f32, filter_type: DdspFilterType) -> f32 {
        let fc = cutoff.clamp(20.0, sr * 0.48);
        let q_val = q.clamp(0.5, 20.0);
        let g = (PI * fc / sr).tan();
        let k = 1.0 / q_val;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = sample - self.ic2eq;
        let v1 = a1 * self.ic1eq + a2 * v3;
        let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;
        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        match filter_type {
            DdspFilterType::Lowpass => v2,
            DdspFilterType::Highpass => sample - k * v1 - v2,
            DdspFilterType::Bandpass => v1,
            DdspFilterType::Notch => sample - k * v1,
        }
    }
}

/// Buffer de Delay com interpolação linear
#[derive(Clone, Debug)]
pub struct DelayEffect {
    buffer: Vec<f32>,
    write_idx: usize,
}

impl DelayEffect {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples.max(1024)],
            write_idx: 0,
        }
    }

    #[inline(always)]
    pub fn process(&mut self, input: f32, delay_samples: f32, feedback: f32, mix: f32) -> f32 {
        let buf_len = self.buffer.len();
        let delay_clamped = delay_samples.clamp(1.0, (buf_len - 2) as f32);
        let read_pos = (self.write_idx as f32 - delay_clamped + buf_len as f32) % (buf_len as f32);
        let r0 = read_pos.floor() as usize % buf_len;
        let r1 = (r0 + 1) % buf_len;
        let frac = read_pos - read_pos.floor();

        let delayed_sample = (1.0 - frac) * self.buffer[r0] + frac * self.buffer[r1];
        self.buffer[self.write_idx] = input + delayed_sample * feedback.clamp(-0.95, 0.95);
        self.write_idx = (self.write_idx + 1) % buf_len;

        (1.0 - mix) * input + mix * delayed_sample
    }
}

/// Reverb estilo Schroeder simplificado (4 filtros comb em paralelo + 2 allpass)
#[derive(Clone, Debug)]
pub struct SchroederReverb {
    comb_delays: [Vec<f32>; 4],
    comb_ptrs: [usize; 4],
    allpass_delays: [Vec<f32>; 2],
    allpass_ptrs: [usize; 2],
}

impl SchroederReverb {
    pub fn new(sr: f32) -> Self {
        let scale = sr / 44100.0;
        let comb_lens = [
            (1116.0 * scale) as usize,
            (1188.0 * scale) as usize,
            (1277.0 * scale) as usize,
            (1356.0 * scale) as usize,
        ];
        let ap_lens = [(225.0 * scale) as usize, (556.0 * scale) as usize];

        Self {
            comb_delays: [
                vec![0.0; comb_lens[0]],
                vec![0.0; comb_lens[1]],
                vec![0.0; comb_lens[2]],
                vec![0.0; comb_lens[3]],
            ],
            comb_ptrs: [0, 0, 0, 0],
            allpass_delays: [vec![0.0; ap_lens[0]], vec![0.0; ap_lens[1]]],
            allpass_ptrs: [0, 0],
        }
    }

    #[inline(always)]
    pub fn process(&mut self, input: f32, decay_s: f32, mix: f32) -> f32 {
        let feedback = (1.0 - 1.0 / (decay_s.max(0.1) * 20.0)).clamp(0.3, 0.88);
        let mut comb_sum = 0.0f32;

        for i in 0..4 {
            let buf_len = self.comb_delays[i].len();
            let ptr = self.comb_ptrs[i];
            let out = self.comb_delays[i][ptr];
            self.comb_delays[i][ptr] = input + out * feedback;
            self.comb_ptrs[i] = (ptr + 1) % buf_len;
            comb_sum += out * 0.25;
        }

        let mut ap_out = comb_sum;
        for i in 0..2 {
            let buf_len = self.allpass_delays[i].len();
            let ptr = self.allpass_ptrs[i];
            let buf_val = self.allpass_delays[i][ptr];
            let g = 0.5f32;
            let current = -g * ap_out + buf_val;
            self.allpass_delays[i][ptr] = ap_out + g * current;
            self.allpass_ptrs[i] = (ptr + 1) % buf_len;
            ap_out = current;
        }

        (1.0 - mix) * input + mix * ap_out
    }
}

/// Gerador determinístico de ruído fractal (Estágio E13)
pub struct FractalNoiseGenerator {
    state: u32,
    b0: f32,
    b1: f32,
    b2: f32,
}

impl FractalNoiseGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x12345678 } else { seed },
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
        }
    }

    #[inline(always)]
    fn white(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        let bits = (self.state >> 9) | 0x3f800000;
        let f = f32::from_bits(bits) - 1.0;
        2.0 * f - 1.0
    }

    #[inline(always)]
    pub fn next_sample(&mut self, alpha: f32) -> f32 {
        let w = self.white();
        if alpha < 0.2 {
            // Quase ruído branco
            w
        } else if alpha <= 1.2 {
            // Ruído Rosa (Paul Kellet's filter)
            self.b0 = 0.99886 * self.b0 + w * 0.0555179;
            self.b1 = 0.99332 * self.b1 + w * 0.0750759;
            self.b2 = 0.96900 * self.b2 + w * 0.1538520;
            (self.b0 + self.b1 + self.b2 + w * 0.5362) * 0.4
        } else {
            // Ruído Marrom / Browniano (integração passa-baixas com vazamento)
            self.b0 = 0.97 * self.b0 + w * 0.03;
            self.b0 * 3.5
        }
    }
}

/// Sintetizador DDSP Principal
pub struct DdspSynthesizer {
    config: DdspConfig,
    svf: StateVariableFilter,
    delay: DelayEffect,
    reverb: SchroederReverb,
    noise_gen: FractalNoiseGenerator,
}

impl DdspSynthesizer {
    pub fn new(config: DdspConfig) -> Self {
        let max_delay_samples = (config.sample_rate * 2.0) as usize;
        let sr = config.sample_rate;
        Self {
            svf: StateVariableFilter::new(),
            delay: DelayEffect::new(max_delay_samples),
            reverb: SchroederReverb::new(sr),
            noise_gen: FractalNoiseGenerator::new(42),
            config,
        }
    }

    /// Renderiza as amostras em formato PCM Float32 (normalizado em [-1.0, 1.0])
    pub fn render_pcm(&mut self) -> Vec<f32> {
        let sr = self.config.sample_rate;
        let duration = self.config.duration_s;
        let n_samples = (duration * sr).round() as usize;
        let dt = 1.0 / sr;

        let num_harmonics = self.config.harmonics.amplitudes.len().max(1);
        let mut harmonic_phases = vec![0.0f32; num_harmonics];
        // Inicializa fases com as fases configuradas
        for (i, p) in self.config.harmonics.phases.iter().enumerate() {
            if i < num_harmonics {
                harmonic_phases[i] = *p;
            }
        }

        let mut lfo_phase = self.config.lfo.phase_rad;
        let mut fm_mod_phase = self.config.fm.phase_rad;
        let mut am_mod_phase = self.config.am.phase_rad;

        // Fases e estados para nós do grafo TreeNN (se ativos)
        let num_tree_nodes = self.config.tree.nodes.len();
        let mut tree_phases = vec![0.0f32; num_tree_nodes];
        let mut tree_signals = vec![0.0f32; num_tree_nodes];

        let mut out = Vec::with_capacity(n_samples);

        for step in 0..n_samples {
            let t = step as f32 * dt;

            // 1. Envelope ADSR C1
            let adsr_amp = evaluate_adsr(
                t,
                duration,
                self.config.adsr.attack_s,
                self.config.adsr.decay_s,
                self.config.adsr.sustain,
                self.config.adsr.release_s,
                self.config.adsr.curve,
            );

            // 2. LFO Vibrato de Pitch em Cents
            let mut pitch_factor = 1.0f32;
            if self.config.lfo.enabled {
                lfo_phase += TAU * self.config.lfo.rate_hz * dt;
                let lfo_val = evaluate_lfo_waveform(lfo_phase, self.config.lfo.waveform);
                let cents = lfo_val * self.config.lfo.depth_cents;
                pitch_factor = 2.0f32.powf(cents / 1200.0);
            }

            // 3. Modulação FM angular (Jacobi-Anger)
            let mut fm_shift = 0.0f32;
            if self.config.fm.enabled {
                let fm_freq = self.config.f0 * self.config.fm.ratio;
                fm_mod_phase += TAU * fm_freq * dt;
                fm_shift = self.config.fm.index * fm_freq * fm_mod_phase.cos();
            }

            // 4. Modulação AM (Tremolo)
            let mut am_gain = 1.0f32;
            if self.config.am.enabled {
                am_mod_phase += TAU * self.config.am.rate_hz * dt;
                am_gain = 1.0 + self.config.am.depth * (am_mod_phase + self.config.am.phase_rad).sin();
            }

            // 5. Avaliação do Grafo TreeNN (se habilitado)
            let mut tree_carrier_shift = 0.0f32;
            if self.config.tree.enabled && num_tree_nodes > 0 {
                // Sintetiza nós folha / filhos primeiro
                for i in 0..num_tree_nodes {
                    let node = &self.config.tree.nodes[i];
                    tree_phases[i] += TAU * node.freq_hz * dt;
                    tree_signals[i] = node.amplitude * (tree_phases[i] + node.phase_rad).sin();
                }

                // Propaga modulação ao longo das arestas (child modula parent)
                for edge in &self.config.tree.edges {
                    if let (Some(p_idx), Some(c_idx)) = (
                        self.config.tree.nodes.iter().position(|n| n.id == edge.parent_id),
                        self.config.tree.nodes.iter().position(|n| n.id == edge.child_id),
                    ) {
                        let child_val = tree_signals[c_idx];
                        if p_idx == 0 {
                            tree_carrier_shift += edge.beta * child_val * self.config.f0;
                        }
                    }
                }
            }

            let effective_f0 = (self.config.f0 * pitch_factor + tree_carrier_shift).max(10.0);
            let inharm_b = self.config.harmonics.inharmonicity_b.max(0.0);

            // 6. Banco de Osciladores Harmônicos com Inarmonicidade e Envelope Espectral sob Gauge
            let mut harmonic_sum = 0.0f32;
            for k in 1..=num_harmonics {
                let k_f = k as f32;
                // Lei de dispersão acústica de cordas rígidas (E08): fk = k * f0 * sqrt(1 + B * k^2)
                let dispersion = (1.0 + inharm_b * k_f * k_f).sqrt();
                let f_k = (k_f * effective_f0 * dispersion + (if k == 1 { fm_shift } else { 0.0 })).max(10.0);

                harmonic_phases[k - 1] += TAU * f_k * dt;

                // Base analítica de roll-off + amplitude fornecida
                let base_amp = self.config.harmonics.amplitudes[k - 1];
                let gauge_gain = evaluate_gauge_spectral_basis(f_k, &self.config.spectral_envelope.gauge_weights);
                let formant_gain = evaluate_formants(f_k, &self.config.spectral_envelope.formants);

                let h_amp = base_amp * gauge_gain * formant_gain;
                harmonic_sum += h_amp * harmonic_phases[k - 1].sin();
            }

            let mut sample = adsr_amp * am_gain * harmonic_sum * self.config.amplitude;

            // 7. Ruído Estocástico Fractal (E13)
            if self.config.noise.enabled {
                let noise_sample = self.noise_gen.next_sample(self.config.noise.alpha);
                let noise_linear = 10.0f32.powf(self.config.noise.level_db / 20.0);
                sample += noise_sample * noise_linear * self.config.noise.mix * adsr_amp;
            }

            // 8. Efeitos Não-Locais (E14)
            // Filtro Passa-Baixas SVF
            if self.config.effects.filter_enabled {
                sample = self.svf.process(
                    sample,
                    self.config.effects.filter_cutoff_hz,
                    self.config.effects.filter_resonance_q,
                    sr,
                    self.config.effects.filter_type,
                );
            }

            // Delay com Feedback
            if self.config.effects.delay_enabled {
                let delay_samples = (self.config.effects.delay_time_ms * 1e-3 * sr).max(1.0);
                sample = self.delay.process(
                    sample,
                    delay_samples,
                    self.config.effects.delay_feedback,
                    self.config.effects.delay_mix,
                );
            }

            // Reverb
            if self.config.effects.reverb_enabled {
                sample = self.reverb.process(
                    sample,
                    self.config.effects.reverb_decay_s,
                    self.config.effects.reverb_mix,
                );
            }

            out.push(sample);
        }

        // Normalização suave contra clipping
        let peak = out.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if peak > 0.98 {
            let norm = 0.98 / peak;
            for s in &mut out {
                *s *= norm;
            }
        }

        out
    }

    /// Renderiza as amostras codificadas diretamente em arquivo WAV 16-bit PCM RIFF canônico
    pub fn render_wav(&mut self) -> Vec<u8> {
        let pcm = self.render_pcm();
        let sr = self.config.sample_rate as u32;
        encode_pcm_to_wav_bytes(&pcm, sr)
    }
}

/// Codifica vetor de amostras float [-1.0, 1.0] em bytes WAV 16-bit PCM estritamente canônicos
pub fn encode_pcm_to_wav_bytes(pcm: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = pcm.len() as u32;
    let byte_rate = sample_rate * 2;
    let block_align = 2u16;
    let data_chunk_size = num_samples * 2;
    let file_size = 36 + data_chunk_size;

    let mut wav = Vec::with_capacity((44 + data_chunk_size) as usize);

    // RIFF Header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt subchunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 para PCM)
    wav.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (1 para PCM linear)
    wav.extend_from_slice(&1u16.to_le_bytes());  // NumChannels (1 = mono)
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample (16 bits)

    // data subchunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_chunk_size.to_le_bytes());

    for &s in pcm {
        let clamped = s.clamp(-1.0, 1.0);
        let sample_i16 = (clamped * 32767.0).round() as i16;
        wav.extend_from_slice(&sample_i16.to_le_bytes());
    }

    wav
}

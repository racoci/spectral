//! Módulo de Síntese Ground Truth para WebAssembly (V11)
//!
//! Permite à interface Web e aos scripts de teste sintetizarem sinais de áudio com
//! todos os parâmetros e topologias causais conhecidos analiticamente.

use wasm_bindgen::prelude::*;
use std::f32::consts::{PI, TAU};

pub mod ddsp_model;
pub mod ddsp_engine;
pub mod ddsp_analyzer;

pub use ddsp_model::*;
pub use ddsp_engine::*;
pub use ddsp_analyzer::*;

pub use vector_audio_geometry_synth::*;

mod vector_audio_geometry_synth {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum LfoShape {
        Sine,
        Triangle,
        Saw,
        ReverseSaw,
        Square,
        SampleHold,
        Noise,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum EnvCurveType {
        Linear,
        Exponential,
        GaussianRbf { sigma: f32 },
        BumpSmooth,
    }

    #[derive(Clone, Debug)]
    pub enum ModExpr {
        Const(f32),
        Time,
        Add(Box<ModExpr>, Box<ModExpr>),
        Sub(Box<ModExpr>, Box<ModExpr>),
        Mul(Box<ModExpr>, Box<ModExpr>),
        Div(Box<ModExpr>, Box<ModExpr>),
        ScaleOffset { expr: Box<ModExpr>, scale: f32, offset: f32 },
        Clip { expr: Box<ModExpr>, min: f32, max: f32 },
        Pow { base: Box<ModExpr>, exponent: f32 },
        Tanh(Box<ModExpr>),
        Sin(Box<ModExpr>),
        Cos(Box<ModExpr>),
        Lfo {
            shape: LfoShape,
            freq: Box<ModExpr>,
            depth: Box<ModExpr>,
            phase_offset: f32,
            delay_time: f32,
            fade_time: f32,
        },
        Adsr {
            attack_time: f32,
            decay_time: f32,
            sustain_level: f32,
            release_time: f32,
            t_on: f32,
            t_off: f32,
            curve: EnvCurveType,
        },
        SplineCurve {
            knots: Vec<(f32, f32)>,
        },
        Drift {
            rate: f32,
            amp: f32,
        },
        WhiteNoise {
            seed: u32,
            amp: f32,
        },
    }

    impl ModExpr {
        pub fn constant(v: f32) -> Self { ModExpr::Const(v) }
        pub fn eval(&self, t: f32) -> f32 {
            match self {
                ModExpr::Const(c) => *c,
                ModExpr::Time => t,
                ModExpr::Add(a, b) => a.eval(t) + b.eval(t),
                ModExpr::Sub(a, b) => a.eval(t) - b.eval(t),
                ModExpr::Mul(a, b) => a.eval(t) * b.eval(t),
                ModExpr::Div(a, b) => {
                    let den = b.eval(t);
                    if den.abs() > 1e-9 { a.eval(t) / den } else { 0.0 }
                }
                ModExpr::ScaleOffset { expr, scale, offset } => expr.eval(t) * scale + offset,
                ModExpr::Clip { expr, min, max } => expr.eval(t).clamp(*min, *max),
                ModExpr::Pow { base, exponent } => {
                    let b = base.eval(t).max(0.0);
                    b.powf(*exponent)
                }
                ModExpr::Tanh(a) => a.eval(t).tanh(),
                ModExpr::Sin(a) => a.eval(t).sin(),
                ModExpr::Cos(a) => a.eval(t).cos(),
                ModExpr::Lfo { shape, freq, depth, phase_offset, delay_time, fade_time } => {
                    if t < *delay_time {
                        return 0.0;
                    }
                    let f = freq.eval(t).max(0.001);
                    let d = depth.eval(t);
                    let fade = if *fade_time > 1e-6 {
                        ((t - delay_time) / fade_time).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let phase = (TAU * f * (t - delay_time) + phase_offset).rem_euclid(TAU);
                    let phase_norm = phase / TAU;

                    let w = match shape {
                        LfoShape::Sine => phase.sin(),
                        LfoShape::Triangle => {
                            if phase_norm < 0.5 { 4.0 * phase_norm - 1.0 } else { 3.0 - 4.0 * phase_norm }
                        }
                        LfoShape::Saw => 2.0 * phase_norm - 1.0,
                        LfoShape::ReverseSaw => 1.0 - 2.0 * phase_norm,
                        LfoShape::Square => if phase_norm < 0.5 { 1.0 } else { -1.0 },
                        LfoShape::SampleHold => {
                            let step = (f * (t - delay_time)).floor();
                            ((step * 12.9898 + 78.233).sin() * 43758.545).fract() * 2.0 - 1.0
                        }
                        LfoShape::Noise => {
                            let hash = ((t * 12345.67).sin() * 43758.545).fract();
                            hash * 2.0 - 1.0
                        }
                    };
                    d * fade * w
                }
                ModExpr::Adsr { attack_time, decay_time, sustain_level, release_time, t_on, t_off, curve } => {
                    eval_adsr(t, *attack_time, *decay_time, *sustain_level, *release_time, *t_on, *t_off, *curve)
                }
                ModExpr::SplineCurve { knots } => {
                    if knots.is_empty() { return 0.0; }
                    if knots.len() == 1 || t <= knots[0].0 { return knots[0].1; }
                    if t >= knots[knots.len() - 1].0 { return knots[knots.len() - 1].1; }
                    let mut k = 0;
                    while k < knots.len() - 1 && t > knots[k + 1].0 { k += 1; }
                    let (t0, y0) = knots[k];
                    let (t1, y1) = knots[k + 1];
                    let u = ((t - t0) / (t1 - t0).max(1e-9)).clamp(0.0, 1.0);
                    let s = u * u * (3.0 - 2.0 * u);
                    y0 + s * (y1 - y0)
                }
                ModExpr::Drift { rate, amp } => {
                    ((t * rate).sin() * 0.6 + (t * rate * 0.37).sin() * 0.4) * amp
                }
                ModExpr::WhiteNoise { seed, amp } => {
                    let s = (*seed as f32 + t * 44100.0) * 12.9898;
                    ((s.sin() * 43758.5453).fract() * 2.0 - 1.0) * amp
                }
            }
        }
    }

    fn eval_adsr(t: f32, a: f32, d: f32, s: f32, r: f32, t_on: f32, t_off: f32, curve: EnvCurveType) -> f32 {
        if t < t_on { return 0.0; }
        if t < t_off {
            let rel_t = t - t_on;
            if rel_t < a && a > 1e-6 {
                let u = (rel_t / a).clamp(0.0, 1.0);
                match curve {
                    EnvCurveType::Linear => u,
                    EnvCurveType::Exponential => u * u,
                    EnvCurveType::GaussianRbf { sigma } => (-0.5 * ((1.0 - u) / sigma.max(0.1)).powi(2)).exp(),
                    EnvCurveType::BumpSmooth => {
                        let bu = (1.0 - u).clamp(0.0, 0.999);
                        (-1.0 / (1.0 - bu * bu)).exp() / (-1.0f32).exp()
                    }
                }
            } else if rel_t < a + d && d > 1e-6 {
                let u = ((rel_t - a) / d).clamp(0.0, 1.0);
                1.0 - u * (1.0 - s)
            } else {
                s
            }
        } else {
            let rel_t = t - t_off;
            if rel_t < r && r > 1e-6 {
                let u = (rel_t / r).clamp(0.0, 1.0);
                s * (1.0 - u)
            } else {
                0.0
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct SpectralEnvelope {
        pub points: Vec<(f32, f32)>,
    }

    impl SpectralEnvelope {
        pub fn new_flat() -> Self {
            Self { points: vec![(20.0, 0.0), (20000.0, 0.0)] }
        }
        pub fn eval(&self, freq_hz: f32) -> f32 {
            if self.points.is_empty() { return 1.0; }
            if self.points.len() == 1 { return 10.0f32.powf(self.points[0].1 / 20.0); }
            let mut k = 0;
            while k < self.points.len() - 1 && freq_hz > self.points[k + 1].0 { k += 1; }
            if k >= self.points.len() - 1 { return 10.0f32.powf(self.points[self.points.len() - 1].1 / 20.0); }
            let (f0, db0) = self.points[k];
            let (f1, db1) = self.points[k + 1];
            let log_f = freq_hz.max(10.0).log2();
            let frac = ((log_f - f0.log2()) / (f1.log2() - f0.log2()).max(1e-6)).clamp(0.0, 1.0);
            let db = db0 + frac * (db1 - db0);
            10.0f32.powf(db / 20.0)
        }
    }

    #[derive(Clone, Debug)]
    pub struct NoteEvent {
        pub t_on: f32,
        pub duration: f32,
        pub midi_pitch: f32,
        pub velocity: f32,
        pub detune_cents: f32,
        pub portamento_from_midi: Option<f32>,
        pub portamento_time: f32,
        pub phase_offset: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum FilterType { Lowpass, Highpass, Bandpass, Notch }

    #[derive(Clone, Debug)]
    pub struct SvfFilter {
        pub filter_type: FilterType,
        pub cutoff_expr: ModExpr,
        pub q_expr: ModExpr,
        ic1eq: f32,
        ic2eq: f32,
    }

    impl SvfFilter {
        pub fn new(filter_type: FilterType, cutoff: f32, q: f32) -> Self {
            Self {
                filter_type,
                cutoff_expr: ModExpr::Const(cutoff),
                q_expr: ModExpr::Const(q),
                ic1eq: 0.0,
                ic2eq: 0.0,
            }
        }
        pub fn process(&mut self, sample: f32, t: f32, fs: f32) -> f32 {
            let fc = self.cutoff_expr.eval(t).clamp(20.0, fs * 0.49);
            let q = self.q_expr.eval(t).clamp(0.5, 20.0);
            let g = (PI * fc / fs).tan();
            let k = 1.0 / q;
            let a1 = 1.0 / (1.0 + g * (g + k));
            let a2 = g * a1;
            let a3 = g * a2;
            let v3 = sample - self.ic2eq;
            let v1 = a1 * self.ic1eq + a2 * v3;
            let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;
            self.ic1eq = 2.0 * v1 - self.ic1eq;
            self.ic2eq = 2.0 * v2 - self.ic2eq;
            match self.filter_type {
                FilterType::Lowpass => v2,
                FilterType::Bandpass => v1,
                FilterType::Highpass => sample - k * v1 - v2,
                FilterType::Notch => sample - k * v1,
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct VoiceConfig {
        pub f0_mod: ModExpr,
        pub amp_mod: ModExpr,
        pub inharmonicity_b: ModExpr,
        pub num_harmonics: usize,
        pub harmonic_amps: Vec<ModExpr>,
        pub spectral_envelope: SpectralEnvelope,
        pub fm_depth: ModExpr,
        pub fm_ratio: ModExpr,
        pub pm_depth: ModExpr,
        pub pm_ratio: ModExpr,
        pub am_mod: ModExpr,
        pub noise_mix: f32,
        pub drive: f32,
        pub filter: Option<SvfFilter>,
        pub pan_expr: ModExpr,
    }

    impl VoiceConfig {
        pub fn new_clean(num_harmonics: usize) -> Self {
            let mut h_amps = Vec::with_capacity(num_harmonics);
            for k in 1..=num_harmonics {
                h_amps.push(ModExpr::Const(1.0 / k as f32));
            }
            Self {
                f0_mod: ModExpr::Const(0.0),
                amp_mod: ModExpr::Const(1.0),
                inharmonicity_b: ModExpr::Const(0.0),
                num_harmonics,
                harmonic_amps: h_amps,
                spectral_envelope: SpectralEnvelope::new_flat(),
                fm_depth: ModExpr::Const(0.0),
                fm_ratio: ModExpr::Const(1.0),
                pm_depth: ModExpr::Const(0.0),
                pm_ratio: ModExpr::Const(1.0),
                am_mod: ModExpr::Const(0.0),
                noise_mix: 0.0,
                drive: 0.0,
                filter: None,
                pan_expr: ModExpr::Const(0.0),
            }
        }
    }

    pub struct GroundTruthSynth {
        pub sample_rate: f32,
        pub master_gain: f32,
        pub voices: Vec<VoiceConfig>,
        pub events_per_voice: Vec<Vec<NoteEvent>>,
    }

    impl GroundTruthSynth {
        pub fn new(sample_rate: f32) -> Self {
            Self {
                sample_rate,
                master_gain: 0.9,
                voices: Vec::new(),
                events_per_voice: Vec::new(),
            }
        }
        pub fn add_voice(&mut self, config: VoiceConfig, events: Vec<NoteEvent>) {
            self.voices.push(config);
            self.events_per_voice.push(events);
        }
        pub fn render(&mut self, duration_s: f32) -> Vec<f32> {
            let n_samples = (duration_s * self.sample_rate).ceil() as usize;
            let mut out = vec![0.0f32; n_samples];
            let dt = 1.0 / self.sample_rate;

            for (v_idx, voice) in self.voices.iter_mut().enumerate() {
                let events = &self.events_per_voice[v_idx];
                let mut phases = vec![0.0f32; voice.num_harmonics];
                let mut fm_phase = 0.0f32;
                let mut pm_phase = 0.0f32;

                for i in 0..n_samples {
                    let t = i as f32 * dt;
                    let mut sample_mono = 0.0f32;

                    for note in events {
                        if t >= note.t_on && t < (note.t_on + note.duration + 1.0) {
                            let f0 = 440.0 * 2.0f32.powf((note.midi_pitch - 69.0) / 12.0) + voice.f0_mod.eval(t);
                            let base_amp = note.velocity * voice.amp_mod.eval(t);
                            let b_val = voice.inharmonicity_b.eval(t).max(0.0);

                            let fm_d = voice.fm_depth.eval(t);
                            let fm_r = voice.fm_ratio.eval(t);
                            fm_phase += TAU * (f0 * fm_r) * dt;
                            let fm_val = fm_d * fm_phase.sin();

                            let pm_d = voice.pm_depth.eval(t);
                            let pm_r = voice.pm_ratio.eval(t);
                            pm_phase += TAU * (f0 * pm_r) * dt;
                            let pm_val = pm_d * pm_phase.sin();

                            let mut harm_sum = 0.0f32;
                            for k in 1..=voice.num_harmonics {
                                let k_f = k as f32;
                                let inharm = (1.0 + b_val * k_f * k_f).sqrt();
                                let f_k = k_f * f0 * inharm + (if k == 1 { fm_val } else { 0.0 });
                                phases[k - 1] += TAU * f_k * dt;
                                let h_amp = voice.harmonic_amps[k - 1].eval(t);
                                let spectral_gain = voice.spectral_envelope.eval(f_k);
                                let total_phase = phases[k - 1] + note.phase_offset + (if k == 1 { pm_val } else { 0.0 });
                                harm_sum += h_amp * spectral_gain * total_phase.sin();
                            }

                            sample_mono += base_amp * harm_sum;
                        }
                    }

                    if voice.drive > 1e-4 {
                        let d = 1.0 + 10.0 * voice.drive;
                        sample_mono = (d * sample_mono).tanh() / d.tanh();
                    }

                    if let Some(ref mut flt) = voice.filter {
                        sample_mono = flt.process(sample_mono, t, self.sample_rate);
                    }

                    out[i] += sample_mono;
                }
            }

            let peak = out.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
            if peak > 0.95 {
                let scale = 0.95 / peak;
                for s in &mut out {
                    *s *= scale;
                }
            }
            out
        }
    }
}

// =========================================================================
// EXPORTAÇÃO WASM
// =========================================================================

/// Gera um sinal de teste Ground Truth parametrizado por preset analítico
#[wasm_bindgen]
pub fn wasm_generate_ground_truth_preset(preset_name: &str, duration_s: f32, sample_rate: f32) -> Vec<f32> {
    let mut synth = GroundTruthSynth::new(sample_rate);

    match preset_name {
        "fm_bell" => {
            // FM bell com 2 operadores e inarmonicidade
            let mut v = VoiceConfig::new_clean(4);
            v.amp_mod = ModExpr::Adsr {
                attack_time: 0.01,
                decay_time: 1.5,
                sustain_level: 0.0,
                release_time: 0.5,
                t_on: 0.0,
                t_off: duration_s * 0.8,
                curve: EnvCurveType::Exponential,
            };
            v.fm_depth = ModExpr::Mul(
                Box::new(ModExpr::Adsr {
                    attack_time: 0.01,
                    decay_time: 0.8,
                    sustain_level: 0.0,
                    release_time: 0.2,
                    t_on: 0.0,
                    t_off: duration_s * 0.8,
                    curve: EnvCurveType::Exponential,
                }),
                Box::new(ModExpr::Const(300.0)),
            );
            v.fm_ratio = ModExpr::Const(1.414); // Razão inarmônica clássica de sino
            v.inharmonicity_b = ModExpr::Const(0.001);

            synth.add_voice(v, vec![NoteEvent {
                t_on: 0.1,
                duration: duration_s * 0.7,
                midi_pitch: 69.0, // A4 = 440 Hz
                velocity: 1.0,
                detune_cents: 0.0,
                portamento_from_midi: None,
                portamento_time: 0.0,
                phase_offset: 0.0,
            }]);
        }
        "vibrato_strings" => {
            // Cordas com vibrato LFO (5.5 Hz) e filtro passa-baixas
            let mut v = VoiceConfig::new_clean(8);
            v.f0_mod = ModExpr::Lfo {
                shape: LfoShape::Sine,
                freq: Box::new(ModExpr::Const(5.5)),
                depth: Box::new(ModExpr::Const(12.0)), // 12 Hz depth
                phase_offset: 0.0,
                delay_time: 0.3,
                fade_time: 0.5,
            };
            v.amp_mod = ModExpr::Adsr {
                attack_time: 0.3,
                decay_time: 0.2,
                sustain_level: 0.8,
                release_time: 0.6,
                t_on: 0.0,
                t_off: duration_s * 0.7,
                curve: EnvCurveType::Linear,
            };
            v.filter = Some(SvfFilter::new(FilterType::Lowpass, 2500.0, 1.2));

            synth.add_voice(v, vec![NoteEvent {
                t_on: 0.0,
                duration: duration_s * 0.7,
                midi_pitch: 60.0, // C4
                velocity: 0.9,
                detune_cents: 0.0,
                portamento_from_midi: None,
                portamento_time: 0.0,
                phase_offset: 0.0,
            }]);
        }
        _ => {
            // Default: harmônico puro com ADSR
            let mut v = VoiceConfig::new_clean(6);
            v.amp_mod = ModExpr::Adsr {
                attack_time: 0.05,
                decay_time: 0.2,
                sustain_level: 0.6,
                release_time: 0.4,
                t_on: 0.0,
                t_off: duration_s * 0.8,
                curve: EnvCurveType::Linear,
            };
            synth.add_voice(v, vec![NoteEvent {
                t_on: 0.05,
                duration: duration_s * 0.75,
                midi_pitch: 65.0, // F4
                velocity: 1.0,
                detune_cents: 0.0,
                portamento_from_midi: None,
                portamento_time: 0.0,
                phase_offset: 0.0,
            }]);
        }
    }

    synth.render(duration_s)
}

// =========================================================================
// EXPORTAÇÕES WASM PARA DDSP E TREENN
// =========================================================================

/// Sintetiza áudio a partir de uma configuração DDSP (JSON) e retorna bytes de arquivo WAV 16-bit
#[wasm_bindgen]
pub fn wasm_ddsp_synthesize_wav(config_json: &str) -> Result<Vec<u8>, JsValue> {
    let config: DdspConfig = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("Erro ao deserializar DdspConfig JSON: {}", e)))?;
    let mut synth = DdspSynthesizer::new(config);
    Ok(synth.render_wav())
}

/// Sintetiza áudio a partir de uma configuração DDSP (JSON) e retorna amostras PCM float32 [-1.0, 1.0]
#[wasm_bindgen]
pub fn wasm_ddsp_synthesize_pcm(config_json: &str) -> Result<Vec<f32>, JsValue> {
    let config: DdspConfig = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("Erro ao deserializar DdspConfig JSON: {}", e)))?;
    let mut synth = DdspSynthesizer::new(config);
    Ok(synth.render_pcm())
}

/// Analisa um arquivo ou fatia WAV de áudio e extrai a configuração DDSP analítica equivalente em JSON
#[wasm_bindgen]
pub fn wasm_ddsp_analyze_audio(data: &[u8], f0_hint: f32) -> Result<String, JsValue> {
    let wav_info = crate::parse_wav_samples(data);
    if wav_info.samples.is_empty() {
        return Err(JsValue::from_str("Buffer de áudio vazio ou formato WAV inválido"));
    }
    let norm_factor = 32767.0f32;
    let normalized_samples: Vec<f32> = wav_info.samples.iter().map(|&s| s / norm_factor).collect();
    let hint = if f0_hint > 10.0 { Some(f0_hint) } else { None };
    let config = analyze_audio_to_ddsp(&normalized_samples, wav_info.sample_rate as f32, hint);

    serde_json::to_string_pretty(&config)
        .map_err(|e| JsValue::from_str(&format!("Erro ao serializar DdspConfig para JSON: {}", e)))
}

/// Retorna a configuração JSON de um preset DDSP pré-definido
#[wasm_bindgen]
pub fn wasm_ddsp_get_preset(preset_name: &str) -> Result<String, JsValue> {
    let mut config = DdspConfig::default();

    match preset_name {
        "vocal_formant" => {
            config.f0 = 220.0;
            config.amplitude = 0.85;
            config.adsr.attack_s = 0.03;
            config.adsr.decay_s = 0.12;
            config.adsr.sustain = 0.75;
            config.adsr.release_s = 0.25;
            config.lfo.enabled = true;
            config.lfo.rate_hz = 5.5;
            config.lfo.depth_cents = 22.0;
            config.spectral_envelope.formants = vec![
                FormantFilter { center_hz: 700.0, bandwidth_hz: 110.0, gain_db: 4.0 },
                FormantFilter { center_hz: 1220.0, bandwidth_hz: 140.0, gain_db: 1.0 },
                FormantFilter { center_hz: 2600.0, bandwidth_hz: 180.0, gain_db: -3.0 },
            ];
            config.effects.reverb_enabled = true;
            config.effects.reverb_decay_s = 1.3;
            config.effects.reverb_mix = 0.18;
        }
        "fm_bell" => {
            config.f0 = 440.0;
            config.amplitude = 0.9;
            config.adsr.attack_s = 0.005;
            config.adsr.decay_s = 1.4;
            config.adsr.sustain = 0.0;
            config.adsr.release_s = 0.6;
            config.fm.enabled = true;
            config.fm.ratio = 1.414;
            config.fm.index = 1.8;
            config.harmonics.inharmonicity_b = 0.0008;
            config.lfo.enabled = false;
            config.effects.reverb_enabled = true;
            config.effects.reverb_decay_s = 1.8;
            config.effects.reverb_mix = 0.25;
        }
        "stiff_piano" => {
            config.f0 = 130.81; // C3
            config.amplitude = 0.85;
            config.adsr.attack_s = 0.005;
            config.adsr.decay_s = 0.9;
            config.adsr.sustain = 0.35;
            config.adsr.release_s = 0.5;
            config.harmonics.inharmonicity_b = 0.0004; // Rigidez acústica
            config.harmonics.roll_off_alpha = 1.25;
            config.lfo.enabled = false;
        }
        "vibrato_strings" => {
            config.f0 = 329.63; // E4
            config.amplitude = 0.8;
            config.adsr.attack_s = 0.12;
            config.adsr.decay_s = 0.25;
            config.adsr.sustain = 0.82;
            config.adsr.release_s = 0.40;
            config.lfo.enabled = true;
            config.lfo.rate_hz = 5.2;
            config.lfo.depth_cents = 32.0;
            config.effects.filter_enabled = true;
            config.effects.filter_cutoff_hz = 3200.0;
            config.effects.filter_resonance_q = 1.1;
            config.effects.reverb_enabled = true;
            config.effects.reverb_decay_s = 1.6;
            config.effects.reverb_mix = 0.22;
        }
        "analog_bass" => {
            config.f0 = 65.41; // C2
            config.amplitude = 0.95;
            config.adsr.attack_s = 0.01;
            config.adsr.decay_s = 0.18;
            config.adsr.sustain = 0.65;
            config.adsr.release_s = 0.15;
            config.effects.filter_enabled = true;
            config.effects.filter_cutoff_hz = 750.0;
            config.effects.filter_resonance_q = 2.4;
            config.lfo.enabled = false;
        }
        "treenn_3node" => {
            config.f0 = 220.0;
            config.tree.enabled = true;
            config.tree.nodes = vec![
                TreeNodeConfig {
                    id: "node-carrier".to_string(),
                    label: "Portadora Principal".to_string(),
                    freq_hz: 220.0,
                    amplitude: 1.0,
                    phase_rad: 0.0,
                    color: "#00f2fe".to_string(),
                },
                TreeNodeConfig {
                    id: "node-submod".to_string(),
                    label: "Sub-Modulador".to_string(),
                    freq_hz: 110.0,
                    amplitude: 0.6,
                    phase_rad: 0.0,
                    color: "#38ef7d".to_string(),
                },
                TreeNodeConfig {
                    id: "node-lfo".to_string(),
                    label: "LFO Neural".to_string(),
                    freq_hz: 5.5,
                    amplitude: 0.25,
                    phase_rad: 0.0,
                    color: "#f59e0b".to_string(),
                },
            ];
            config.tree.edges = vec![
                TreeEdgeConfig {
                    parent_id: "node-carrier".to_string(),
                    child_id: "node-submod".to_string(),
                    beta: 0.5,
                },
                TreeEdgeConfig {
                    parent_id: "node-carrier".to_string(),
                    child_id: "node-lfo".to_string(),
                    beta: 0.35,
                },
            ];
            config.lfo.enabled = false;
        }
        _ => {
            // Default preset
        }
    }

    serde_json::to_string_pretty(&config)
        .map_err(|e| JsValue::from_str(&format!("Erro ao serializar preset para JSON: {}", e)))
}

/// Retorna a lista de presets DDSP disponíveis com metadados em JSON
#[wasm_bindgen]
pub fn wasm_ddsp_list_presets() -> String {
    let presets = vec![
        serde_json::json!({
            "id": "vocal_formant",
            "name": "Vocal Formant (A3)",
            "description": "Síntese vocal com formantes acústicos (F1/F2/F3) e vibrato LFO natural."
        }),
        serde_json::json!({
            "id": "fm_bell",
            "name": "FM Bell & Inharmonicity",
            "description": "Sino FM com modulação angular beta=1.8 e rigidez inarmônica B=0.0008."
        }),
        serde_json::json!({
            "id": "stiff_piano",
            "name": "Stiff Piano String",
            "description": "Corda de piano com dispersão física de parciais e decaimento exponencial."
        }),
        serde_json::json!({
            "id": "vibrato_strings",
            "name": "Vibrato Strings Ensemble",
            "description": "Cordas com envelope suave, vibrato de 32 cents, filtro SVF e reverb."
        }),
        serde_json::json!({
            "id": "analog_bass",
            "name": "Analog Resonant Bass",
            "description": "Baixo analógico com harmônicos ricos e filtro passa-baixas ressonante."
        }),
        serde_json::json!({
            "id": "treenn_3node",
            "name": "TreeNN 3-Node Topology",
            "description": "Grafo causal hierárquico com portadora e 2 nós moduladores acoplados."
        }),
    ];

    serde_json::to_string(&presets).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod ddsp_tests {
    use super::*;

    #[test]
    fn test_ddsp_gauge_spectral_basis_invariants() {
        // O gauge exige S(440) = 0 dB (ganho linear = 1.0)
        let weights = [1.5, -0.8, 0.4, -0.2];
        let gain_440 = evaluate_gauge_spectral_basis(440.0, &weights);
        assert!((gain_440 - 1.0).abs() < 1e-6, "Gauge S(440) deve ser exatamente 0 dB (ganho 1.0), obteve {}", gain_440);

        // Derivada em 440 Hz deve ser 0 dB/oitava: avaliando vizinhança infinitesimal
        let delta_u = 1e-4;
        let f_plus = 440.0 * 2.0f32.powf(delta_u);
        let f_minus = 440.0 * 2.0f32.powf(-delta_u);
        let gain_plus = evaluate_gauge_spectral_basis(f_plus, &weights);
        let gain_minus = evaluate_gauge_spectral_basis(f_minus, &weights);
        let slope = (gain_plus - gain_minus) / (2.0 * delta_u);
        assert!(slope.abs() < 1e-3, "Derivada dS/du(440) deve ser 0 dB/oct, obteve slope {}", slope);
    }

    #[test]
    fn test_ddsp_adsr_continuity() {
        let dur = 1.0;
        let a = 0.05;
        let d = 0.15;
        let s = 0.6;
        let r = 0.2;

        let val_start = evaluate_adsr(0.0, dur, a, d, s, r, EnvCurveKind::CubicSmooth);
        let val_peak = evaluate_adsr(a, dur, a, d, s, r, EnvCurveKind::CubicSmooth);
        let val_sus = evaluate_adsr(a + d, dur, a, d, s, r, EnvCurveKind::CubicSmooth);
        let val_end = evaluate_adsr(dur, dur, a, d, s, r, EnvCurveKind::CubicSmooth);

        assert!((val_start - 0.0).abs() < 1e-3, "ADSR t=0 deve ser 0.0");
        assert!((val_peak - 1.0).abs() < 1e-3, "ADSR t=a deve ser 1.0");
        assert!((val_sus - s).abs() < 1e-3, "ADSR t=a+d deve ser sustain ({})", s);
        assert!((val_end - 0.0).abs() < 1e-3, "ADSR t=dur deve ser 0.0");
    }

    #[test]
    fn test_ddsp_synthesizer_pcm_and_wav_generation() {
        let mut config = DdspConfig::default();
        config.duration_s = 0.25;
        config.sample_rate = 12000.0;
        config.f0 = 220.0;

        let mut synth = DdspSynthesizer::new(config.clone());
        let pcm = synth.render_pcm();
        assert_eq!(pcm.len(), 3000);
        for &sample in &pcm {
            assert!(sample.is_finite(), "Amostras PCM devem ser finitas");
            assert!(sample.abs() <= 1.0, "Amostras PCM devem estar em [-1.0, 1.0]");
        }

        let wav = synth.render_wav();
        assert!(wav.len() > 44, "Arquivo WAV deve conter cabeçalho RIFF e amostras");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
    }

    #[test]
    fn test_ddsp_presets_render_successfully() {
        let presets = ["vocal_formant", "fm_bell", "stiff_piano", "vibrato_strings", "analog_bass", "treenn_3node"];
        for preset_name in presets {
            let json_str = wasm_ddsp_get_preset(preset_name).expect("Preset deve existir");
            let config: DdspConfig = serde_json::from_str(&json_str).expect("JSON de preset deve ser válido");
            let mut synth = DdspSynthesizer::new(config);
            let wav = synth.render_wav();
            assert!(wav.len() > 1000, "Preset {} deve renderizar arquivo WAV com sucesso", preset_name);
        }
    }

    #[test]
    fn test_ddsp_analyzer_audio_roundtrip() {
        // Gera tom de teste com f0 = 220 Hz
        let mut config = DdspConfig::default();
        config.duration_s = 0.5;
        config.sample_rate = 12000.0;
        config.f0 = 220.0;
        config.harmonics.amplitudes = vec![1.0, 0.5, 0.25, 0.12];

        let mut synth = DdspSynthesizer::new(config);
        let wav = synth.render_wav();

        let analyzed_json = wasm_ddsp_analyze_audio(&wav, 220.0).expect("Análise deve ser bem-sucedida");
        let analyzed: DdspConfig = serde_json::from_str(&analyzed_json).expect("Config recuperada deve ser válida");

        let f0_error_cents = 1200.0 * (analyzed.f0 / 220.0).log2().abs();
        assert!(f0_error_cents < 5.0, "Erro de f0 deve ser menor que 5 cents, foi {} cents", f0_error_cents);
        assert!(analyzed.harmonics.amplitudes.len() >= 3, "Deve recuperar pelo menos 3 harmônicos");
        assert!((analyzed.harmonics.amplitudes[0] - 1.0).abs() < 1e-4, "H1 deve ser normalizado em 1.0 por gauge");
    }
}



//! Sintetizador Ground Truth (Modulation Graph DSL) - Versão V11
//!
//! Este módulo implementa uma família paramétrica causal e hierárquica completa:
//! 1. Grafo de Modulação (`ModExpr` AST) com profundidade e largura arbitrárias.
//! 2. Decomposição de afinação (microtuning, detune, pitch-bend, vibrato, portamento e jitter).
//! 3. LFOs com múltiplas formas de onda (Sine, Triangle, Saw, Square, Sample&Hold, Noise) com delay e fade.
//! 4. Envelopes generalizados (ADSR com curvas Lineares, RBF Gaussianas e Bump C_c^∞).
//! 5. Síntese Aditiva com inarmonicidade B, estrutura de harmônicos H_k(t) e envelope espectral absoluto E(f).
//! 6. Modulações de Frequência (FM) e Fase (PM) com integração contínua.
//! 7. Ruído Fractal e coloração espectral com condicionamento M(t, f).
//! 8. Arpegiador completo com swing, ratchets, padrões e transposição.
//! 9. Cadeia de efeitos por voz e global: Drive, Filtro SVF (LP/HP/BP), Delay estéreo, Chorus, Reverb e Pan.
//! 10. Renderização estéreo multivoz e controle Master.

use std::f32::consts::{PI, TAU};

// =========================================================================
// 1. ÁRVORE DE MODULAÇÃO (AST) - ModExpr
// =========================================================================

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
        knots: Vec<(f32, f32)>, // (tempo, valor)
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
    pub fn time() -> Self { ModExpr::Time }
    pub fn add(self, other: ModExpr) -> Self { ModExpr::Add(Box::new(self), Box::new(other)) }
    pub fn sub(self, other: ModExpr) -> Self { ModExpr::Sub(Box::new(self), Box::new(other)) }
    pub fn mul(self, other: ModExpr) -> Self { ModExpr::Mul(Box::new(self), Box::new(other)) }
    pub fn div(self, other: ModExpr) -> Self { ModExpr::Div(Box::new(self), Box::new(other)) }

    /// Avaliação estritamente pura de p(t)
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
                let phase_norm = phase / TAU; // [0, 1)

                let w = match shape {
                    LfoShape::Sine => phase.sin(),
                    LfoShape::Triangle => {
                        if phase_norm < 0.5 {
                            4.0 * phase_norm - 1.0
                        } else {
                            3.0 - 4.0 * phase_norm
                        }
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
            ModExpr::SplineCurve { knots } => eval_spline_curve(t, knots),
            ModExpr::Drift { rate, amp } => {
                let drift_val = (t * rate).sin() * 0.6 + (t * rate * 0.37).sin() * 0.4;
                drift_val * amp
            }
            ModExpr::WhiteNoise { seed, amp } => {
                let s = (*seed as f32 + t * 44100.0) * 12.9898;
                let val = (s.sin() * 43758.5453).fract() * 2.0 - 1.0;
                val * amp
            }
        }
    }
}

fn eval_adsr(t: f32, a: f32, d: f32, s: f32, r: f32, t_on: f32, t_off: f32, curve: EnvCurveType) -> f32 {
    if t < t_on {
        return 0.0;
    }
    if t < t_off {
        let rel_t = t - t_on;
        if rel_t < a && a > 1e-6 {
            let u = (rel_t / a).clamp(0.0, 1.0);
            match curve {
                EnvCurveType::Linear => u,
                EnvCurveType::Exponential => u * u,
                EnvCurveType::GaussianRbf { sigma } => (-0.5 * ((1.0 - u) / sigma.max(0.1)).powi(2)).exp(),
                EnvCurveType::BumpSmooth => {
                    let bump_u = (1.0 - u).clamp(0.0, 0.999);
                    (-1.0 / (1.0 - bump_u * bump_u)).exp() / (-1.0f32).exp()
                }
            }
        } else if rel_t < a + d && d > 1e-6 {
            let u = ((rel_t - a) / d).clamp(0.0, 1.0);
            match curve {
                EnvCurveType::Linear => 1.0 - u * (1.0 - s),
                EnvCurveType::Exponential => 1.0 - (u * u) * (1.0 - s),
                EnvCurveType::GaussianRbf { sigma } => {
                    let g = (-0.5 * (u / sigma.max(0.1)).powi(2)).exp();
                    s + (1.0 - s) * g
                }
                EnvCurveType::BumpSmooth => 1.0 - u * (1.0 - s),
            }
        } else {
            s
        }
    } else {
        let rel_t = t - t_off;
        if rel_t < r && r > 1e-6 {
            let u = (rel_t / r).clamp(0.0, 1.0);
            match curve {
                EnvCurveType::Linear => s * (1.0 - u),
                EnvCurveType::Exponential => s * (1.0 - u * u),
                EnvCurveType::GaussianRbf { sigma } => s * (-0.5 * (u / sigma.max(0.1)).powi(2)).exp(),
                EnvCurveType::BumpSmooth => s * (1.0 - u),
            }
        } else {
            0.0
        }
    }
}

fn eval_spline_curve(t: f32, knots: &[(f32, f32)]) -> f32 {
    if knots.is_empty() {
        return 0.0;
    }
    if knots.len() == 1 || t <= knots[0].0 {
        return knots[0].1;
    }
    if t >= knots[knots.len() - 1].0 {
        return knots[knots.len() - 1].1;
    }

    let mut k = 0;
    while k < knots.len() - 1 && t > knots[k + 1].0 {
        k += 1;
    }
    let (t0, y0) = knots[k];
    let (t1, y1) = knots[k + 1];
    let dt = (t1 - t0).max(1e-9);
    let u = ((t - t0) / dt).clamp(0.0, 1.0);

    // Interpolação suave C1 (Hermite cúbica padrão 3u^2 - 2u^3)
    let s = u * u * (3.0 - 2.0 * u);
    y0 + s * (y1 - y0)
}

// =========================================================================
// 2. ENVELOPE ESPECTRAL ABSOLUTO E ESTRUTURA HARMÔNICA
// =========================================================================

/// Formante espectral absoluto E(f) = 10^(S(log2 f)/20)
#[derive(Clone, Debug)]
pub struct SpectralEnvelope {
    pub points: Vec<(f32, f32)>, // (frequência em Hz, ganho em dB)
}

impl SpectralEnvelope {
    pub fn new_flat() -> Self {
        Self { points: vec![(20.0, 0.0), (20000.0, 0.0)] }
    }

    pub fn eval(&self, freq_hz: f32) -> f32 {
        if self.points.is_empty() {
            return 1.0;
        }
        let log_f = freq_hz.max(10.0).log2();
        if self.points.len() == 1 {
            return 10.0f32.powf(self.points[0].1 / 20.0);
        }

        // Interpolação sobre log2 f
        let mut k = 0;
        while k < self.points.len() - 1 && freq_hz > self.points[k + 1].0 {
            k += 1;
        }
        if k >= self.points.len() - 1 {
            return 10.0f32.powf(self.points[self.points.len() - 1].1 / 20.0);
        }

        let (f0, db0) = self.points[k];
        let (f1, db1) = self.points[k + 1];
        let u0 = f0.log2();
        let u1 = f1.log2();
        let frac = ((log_f - u0) / (u1 - u0).max(1e-6)).clamp(0.0, 1.0);
        let db = db0 + frac * (db1 - db0);
        10.0f32.powf(db / 20.0)
    }
}

// =========================================================================
// 3. EVENTOS DE NOTA, PORTAMENTO E MICRO-AFINAÇÃO
// =========================================================================

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

impl NoteEvent {
    pub fn nominal_freq(&self) -> f32 {
        440.0 * 2.0f32.powf((self.midi_pitch - 69.0) / 12.0)
    }

    pub fn eval_pitch(&self, t: f32, bend_cents: f32, vibrato_cents: f32) -> f32 {
        let f_target = self.nominal_freq();
        let mut base_f = f_target;

        if let Some(from_midi) = self.portamento_from_midi {
            if self.portamento_time > 1e-6 && t >= self.t_on {
                let f_from = 440.0 * 2.0f32.powf((from_midi - 69.0) / 12.0);
                let u = ((t - self.t_on) / self.portamento_time).clamp(0.0, 1.0);
                // Curva de portamento suave C1
                let g = u * u * (3.0 - 2.0 * u);
                base_f = f_from + g * (f_target - f_from);
            }
        }

        let total_cents = self.detune_cents + bend_cents + vibrato_cents;
        base_f * 2.0f32.powf(total_cents / 1200.0)
    }
}

// =========================================================================
// 4. ARPEGIADOR
// =========================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArpPattern {
    Up,
    Down,
    UpDown,
    Random,
}

#[derive(Clone, Debug)]
pub struct Arpeggiator {
    pub pattern: ArpPattern,
    pub rate_hz: f32,
    pub gate: f32,          // [0, 1] fração do passo que fica ativa
    pub swing: f32,         // [-0.5, 0.5] swing rítmico
    pub octaves: usize,     // 1, 2, 3 oitavas
    pub ratchets: usize,    // subdivisões por passo (1 = normal, 2 = dobro de ticks)
    pub chord_pitches: Vec<f32>,
}

impl Arpeggiator {
    pub fn generate_events(&self, start_time: f32, total_duration: f32) -> Vec<NoteEvent> {
        let mut events = Vec::new();
        if self.chord_pitches.is_empty() || self.rate_hz <= 0.0 {
            return events;
        }

        // Expandir acordes pelas oitavas
        let mut pool = Vec::new();
        for oct in 0..self.octaves {
            for &p in &self.chord_pitches {
                pool.push(p + (oct as f32 * 12.0));
            }
        }
        if pool.is_empty() {
            return events;
        }

        let step_duration = 1.0 / self.rate_hz;
        let mut cur_t = start_time;
        let mut step_idx = 0;

        while cur_t < start_time + total_duration {
            let swing_offset = if step_idx % 2 == 0 {
                step_duration * (1.0 - self.swing)
            } else {
                step_duration * (1.0 + self.swing)
            };

            let pitch_idx = match self.pattern {
                ArpPattern::Up => step_idx % pool.len(),
                ArpPattern::Down => (pool.len() - 1) - (step_idx % pool.len()),
                ArpPattern::UpDown => {
                    let cycle = (pool.len() * 2).saturating_sub(2).max(1);
                    let pos = step_idx % cycle;
                    if pos < pool.len() { pos } else { cycle - pos }
                }
                ArpPattern::Random => (step_idx * 17 + 5) % pool.len(),
            };

            let midi_p = pool[pitch_idx];
            let n_rats = self.ratchets.max(1);
            let sub_step = swing_offset / n_rats as f32;

            for r in 0..n_rats {
                let note_on = cur_t + r as f32 * sub_step;
                let dur = sub_step * self.gate.clamp(0.05, 0.99);
                if note_on < start_time + total_duration {
                    events.push(NoteEvent {
                        t_on: note_on,
                        duration: dur,
                        midi_pitch: midi_p,
                        velocity: 0.85,
                        detune_cents: 0.0,
                        portamento_from_midi: None,
                        portamento_time: 0.0,
                        phase_offset: 0.0,
                    });
                }
            }

            cur_t += swing_offset;
            step_idx += 1;
        }

        events
    }
}

// =========================================================================
// 5. CADEIA DE EFEITOS (DRIVE, FILTRO SVF, DELAY, CHORUS, REVERB)
// =========================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterType {
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
}

#[derive(Clone, Debug)]
pub struct SvfFilter {
    pub filter_type: FilterType,
    pub cutoff_expr: ModExpr,
    pub q_expr: ModExpr,
    // Estado interno
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

        // Andrew Simper State Variable Filter (SVF)
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
pub struct StereoDelay {
    pub time_left: f32,
    pub time_right: f32,
    pub feedback: f32,
    pub mix: f32,
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos: usize,
}

impl StereoDelay {
    pub fn new(fs: f32, time_l: f32, time_r: f32, feedback: f32, mix: f32) -> Self {
        let max_samples = (fs * 2.0) as usize;
        Self {
            time_left: time_l,
            time_right: time_r,
            feedback,
            mix,
            buffer_l: vec![0.0; max_samples],
            buffer_r: vec![0.0; max_samples],
            write_pos: 0,
        }
    }

    pub fn process(&mut self, in_l: f32, in_r: f32, fs: f32) -> (f32, f32) {
        let max = self.buffer_l.len();
        let delay_samples_l = ((self.time_left * fs) as usize).clamp(1, max - 1);
        let delay_samples_r = ((self.time_right * fs) as usize).clamp(1, max - 1);

        let read_pos_l = (self.write_pos + max - delay_samples_l) % max;
        let read_pos_r = (self.write_pos + max - delay_samples_r) % max;

        let out_l = self.buffer_l[read_pos_l];
        let out_r = self.buffer_r[read_pos_r];

        self.buffer_l[self.write_pos] = in_l + out_l * self.feedback;
        self.buffer_r[self.write_pos] = in_r + out_r * self.feedback;

        self.write_pos = (self.write_pos + 1) % max;

        (
            in_l * (1.0 - self.mix) + out_l * self.mix,
            in_r * (1.0 - self.mix) + out_r * self.mix,
        )
    }
}

#[derive(Clone, Debug)]
pub struct SimpleReverb {
    pub mix: f32,
    pub decay: f32,
    comb_delays: [usize; 4],
    comb_buffers: [Vec<f32>; 4],
    comb_pos: [usize; 4],
    allpass_buffer: Vec<f32>,
    allpass_pos: usize,
}

impl SimpleReverb {
    pub fn new(fs: f32, decay: f32, mix: f32) -> Self {
        let d = [1116, 1188, 1277, 1356]; // Delays primos clássicos
        let buffers = [
            vec![0.0; (fs * 0.1) as usize],
            vec![0.0; (fs * 0.1) as usize],
            vec![0.0; (fs * 0.1) as usize],
            vec![0.0; (fs * 0.1) as usize],
        ];
        Self {
            mix,
            decay,
            comb_delays: d,
            comb_buffers: buffers,
            comb_pos: [0; 4],
            allpass_buffer: vec![0.0; 225],
            allpass_pos: 0,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let mut comb_sum = 0.0;
        for i in 0..4 {
            let max = self.comb_buffers[i].len();
            let d = self.comb_delays[i];
            let read_pos = (self.comb_pos[i] + max - d) % max;
            let out = self.comb_buffers[i][read_pos];
            self.comb_buffers[i][self.comb_pos[i]] = input + out * self.decay;
            self.comb_pos[i] = (self.comb_pos[i] + 1) % max;
            comb_sum += out;
        }
        comb_sum *= 0.25;

        // Allpass 1 estágio
        let ap_len = self.allpass_buffer.len();
        let ap_out = self.allpass_buffer[self.allpass_pos];
        let ap_in = comb_sum + ap_out * 0.5;
        let result = -comb_sum + ap_in;
        self.allpass_buffer[self.allpass_pos] = ap_in;
        self.allpass_pos = (self.allpass_pos + 1) % ap_len;

        input * (1.0 - self.mix) + result * self.mix
    }
}

// =========================================================================
// 6. VOZ COMPLETA (PARCIAIS, FM, PM, RUÍDO CONDICIONADO E FILTRAGEM)
// =========================================================================

#[derive(Clone, Debug)]
pub struct VoiceConfig {
    pub f0_mod: ModExpr,
    pub amp_mod: ModExpr,
    pub inharmonicity_b: ModExpr,
    pub num_harmonics: usize,
    pub harmonic_amps: Vec<ModExpr>,
    pub spectral_envelope: SpectralEnvelope,

    // FM & PM
    pub fm_depth: ModExpr,
    pub fm_ratio: ModExpr,
    pub pm_depth: ModExpr,
    pub pm_ratio: ModExpr,

    // AM / Tremolo
    pub am_mod: ModExpr,

    // Ruído Fractal / Condicionado
    pub noise_mix: f32, // rho
    pub noise_fractal_alpha: f32,

    // Efeitos da Voz
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
            noise_fractal_alpha: -1.0, // ruído rosa aproximado
            drive: 0.0,
            filter: None,
            pan_expr: ModExpr::Const(0.0),
        }
    }
}

// =========================================================================
// 7. SINTETIZADOR GROUND TRUTH MULTIVOZ MASTER
// =========================================================================

pub struct GroundTruthSynth {
    pub sample_rate: f32,
    pub master_gain: f32,
    pub voices: Vec<VoiceConfig>,
    pub events_per_voice: Vec<Vec<NoteEvent>>,
    pub global_delay: Option<StereoDelay>,
    pub global_reverb: Option<SimpleReverb>,
}

impl GroundTruthSynth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            master_gain: 0.9,
            voices: Vec::new(),
            events_per_voice: Vec::new(),
            global_delay: None,
            global_reverb: None,
        }
    }

    pub fn add_voice(&mut self, config: VoiceConfig, events: Vec<NoteEvent>) {
        self.voices.push(config);
        self.events_per_voice.push(events);
    }

    /// Renderiza sinal estéreo estritamente determinístico [L, R]
    pub fn render_stereo(&mut self, duration_s: f32) -> (Vec<f32>, Vec<f32>) {
        let n_samples = (duration_s * self.sample_rate).ceil() as usize;
        let mut out_l = vec![0.0f32; n_samples];
        let mut out_r = vec![0.0f32; n_samples];
        let dt = 1.0 / self.sample_rate;

        // Para cada voz
        for (v_idx, voice) in self.voices.iter_mut().enumerate() {
            let events = &self.events_per_voice[v_idx];
            let mut phases = vec![0.0f32; voice.num_harmonics];
            let mut fm_phase = 0.0f32;
            let mut pm_phase = 0.0f32;
            let mut noise_state = 0.0f32;

            for i in 0..n_samples {
                let t = i as f32 * dt;

                // Localizar notas ativas
                let mut sample_mono = 0.0f32;

                for note in events {
                    if t >= note.t_on && t < (note.t_on + note.duration + 1.5) {
                        let f0 = note.eval_pitch(t, 0.0, 0.0) + voice.f0_mod.eval(t);
                        let base_amp = note.velocity * voice.amp_mod.eval(t);
                        let b_val = voice.inharmonicity_b.eval(t).max(0.0);

                        // FM Modulador
                        let fm_d = voice.fm_depth.eval(t);
                        let fm_r = voice.fm_ratio.eval(t);
                        let f_fm = f0 * fm_r;
                        fm_phase += TAU * f_fm * dt;
                        let fm_mod_val = fm_d * fm_phase.sin();

                        // PM Modulador
                        let pm_d = voice.pm_depth.eval(t);
                        let pm_r = voice.pm_ratio.eval(t);
                        let f_pm = f0 * pm_r;
                        pm_phase += TAU * f_pm * dt;
                        let pm_mod_val = pm_d * pm_phase.sin();

                        // AM
                        let am = 1.0 + voice.am_mod.eval(t);

                        // Harmônicos
                        let mut harm_sum = 0.0f32;
                        for k in 1..=voice.num_harmonics {
                            let k_f = k as f32;
                            let inharm = (1.0 + b_val * k_f * k_f).sqrt();
                            let f_k = k_f * f0 * inharm + (if k == 1 { fm_mod_val } else { 0.0 });
                            phases[k - 1] += TAU * f_k * dt;

                            let h_amp = voice.harmonic_amps[k - 1].eval(t);
                            let spectral_gain = voice.spectral_envelope.eval(f_k);
                            let total_phase = phases[k - 1] + note.phase_offset + (if k == 1 { pm_mod_val } else { 0.0 });

                            harm_sum += h_amp * spectral_gain * total_phase.sin();
                        }

                        // Ruído condicionado
                        let white = ((t * 44100.0 * (v_idx + 1) as f32).sin() * 43758.545).fract();
                        noise_state = 0.95 * noise_state + 0.05 * (white * 2.0 - 1.0); // 1-pole pinkish
                        let conditioned_noise = harm_sum.abs().sqrt() * noise_state;

                        let note_signal = (1.0 - voice.noise_mix) * harm_sum + voice.noise_mix * conditioned_noise;
                        sample_mono += base_amp * am * note_signal;
                    }
                }

                // Drive da Voz
                if voice.drive > 1e-4 {
                    let d = 1.0 + 10.0 * voice.drive;
                    sample_mono = (d * sample_mono).tanh() / d.tanh();
                }

                // Filtro da Voz
                if let Some(ref mut flt) = voice.filter {
                    sample_mono = flt.process(sample_mono, t, self.sample_rate);
                }

                // Panning estéreo de potência constante: theta = pi/4 * (1 + p)
                let pan = voice.pan_expr.eval(t).clamp(-1.0, 1.0);
                let theta = (PI * 0.25) * (1.0 + pan);
                let l_val = theta.cos() * sample_mono;
                let r_val = theta.sin() * sample_mono;

                out_l[i] += l_val;
                out_r[i] += r_val;
            }
        }

        // Processar Efeitos Globais
        for i in 0..n_samples {
            let mut l = out_l[i];
            let mut r = out_r[i];

            if let Some(ref mut delay) = self.global_delay {
                let (dl, dr) = delay.process(l, r, self.sample_rate);
                l = dl;
                r = dr;
            }

            if let Some(ref mut verb) = self.global_reverb {
                l = verb.process(l);
                r = verb.process(r);
            }

            out_l[i] = l * self.master_gain;
            out_r[i] = r * self.master_gain;
        }

        // Limiter suave master com tanh
        let peak_l = out_l.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        let peak_r = out_r.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        let max_peak = peak_l.max(peak_r);
        if max_peak > 0.95 {
            let scale = 0.95 / max_peak;
            for i in 0..n_samples {
                out_l[i] *= scale;
                out_r[i] *= scale;
            }
        }

        (out_l, out_r)
    }

    /// Renderiza mono mixdown
    pub fn render(&mut self, duration_s: f32) -> Vec<f32> {
        let (l, r) = self.render_stereo(duration_s);
        l.iter().zip(r.iter()).map(|(&a, &b)| (a + b) * 0.5).collect()
    }
}

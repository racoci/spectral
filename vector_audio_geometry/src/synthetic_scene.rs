//! Gerador Genérico de Cenas Sintéticas para Experimentos de Rastreamento de Cristas (Ridge Tracking).
//!
//! Fornece:
//! 1. Cristas com trajetórias arbitrárias e cruzamentos explícitos (X-crossing, dupla hélice, quase-contato/grazing).
//! 2. Modulação em Amplitude (AM - tremolo) e Frequência (FM - vibrato).
//! 3. Geradores de múltiplos tipos de ruído:
//!    - Ruído Branco Gaussiano
//!    - Ruído Rosa (1/f com filtro de 3 pólos de Kellet)
//!    - Ruído Browniano / Vermelho (1/f^2 por integrador com perda)
//!    - Ruído Fractal (Movimento Browniano Fracionário fBm com expoente de Hurst H)
//!    - Ruído Impulsivo / Estocástico (Processo de Poisson para estalos e cliques)
//! 4. Dados Ground Truth analíticos exatos (f(t), A(t), phi(t) e pontos de interseção) para benchmarking.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f32::consts::PI;

/// Tipos de ruído estocástico suportados
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoiseType {
    /// Sem ruído adicional (sinal limpo analítico)
    None,
    /// Ruído Branco Gaussiano N(0, sigma^2)
    White,
    /// Ruído Rosa com densidade espectral 1/f (-3 dB/oitava)
    Pink,
    /// Ruído Browniano / Vermelho com densidade espectral 1/f^2 (-6 dB/oitava)
    Brownian,
    /// Ruído Fractal (fBm) parametrizado pelo expoente de Hurst H in (0, 1)
    Fractal { hurst: f32 },
    /// Ruído impulsivo (estalos/cliques modelados por processo de Poisson)
    Impulsive { rate_hz: f32, amplitude: f32 },
}

/// Parâmetros de Modulação de Amplitude (AM)
#[derive(Clone, Copy, Debug)]
pub struct AmModulation {
    /// Frequência de modulação em Hz (ex: 4 a 8 Hz para tremolo vocal)
    pub rate_hz: f32,
    /// Profundidade de modulação m in [0, 1]
    pub depth: f32,
    /// Fase inicial da modulação em radianos
    pub phase_rad: f32,
}

impl AmModulation {
    #[inline(always)]
    pub fn envelope_at(&self, t: f32) -> f32 {
        1.0 + self.depth * (2.0 * PI * self.rate_hz * t + self.phase_rad).sin()
    }
}

/// Parâmetros de Modulação de Frequência (FM)
#[derive(Clone, Copy, Debug)]
pub struct FmModulation {
    /// Frequência de modulação em Hz (ex: 5 Hz para vibrato de violino/voz)
    pub rate_hz: f32,
    /// Desvio máximo de frequência em Hz (ex: 20 Hz)
    pub deviation_hz: f32,
    /// Fase inicial da modulação em radianos
    pub phase_rad: f32,
}

impl FmModulation {
    #[inline(always)]
    pub fn freq_offset_at(&self, t: f32) -> f32 {
        self.deviation_hz * (2.0 * PI * self.rate_hz * t + self.phase_rad).sin()
    }

    /// Integral da modulação de frequência para avanço de fase analítico
    #[inline(always)]
    pub fn phase_integral_at(&self, t: f32) -> f32 {
        if self.rate_hz.abs() < 1e-6 {
            return 0.0;
        }
        -(self.deviation_hz / self.rate_hz) * (2.0 * PI * self.rate_hz * t + self.phase_rad).cos()
    }
}

/// Geometria base da trajetória da frequência
#[derive(Clone, Debug)]
pub enum TrajectoryShape {
    /// Frequência linear f(t) = f0 + (f1 - f0) * ((t - t0) / (t1 - t0))
    LinearChirp { f_start: f32, f_end: f32 },
    /// Chirp hiperbólico ou exponencial f(t) = f0 * (f1 / f0)^(t / T)
    ExponentialChirp { f_start: f32, f_end: f32 },
    /// Oscilação senoidal em torno de uma portadora f_carrier + delta_f * sin(2*pi*f_mod*t)
    SinusoidalWander { f_center: f32, delta_f: f32, mod_hz: f32, phase_rad: f32 },
    /// Polinômio cúbico generalizado f(t) = c0 + c1*t + c2*t^2 + c3*t^3
    PolynomialCubic { c0: f32, c1: f32, c2: f32, c3: f32 },
}

impl TrajectoryShape {
    pub fn freq_at(&self, t: f32, t_start: f32, t_end: f32) -> f32 {
        let duration = (t_end - t_start).max(1e-6);
        let tau = ((t - t_start) / duration).clamp(0.0, 1.0);

        match self {
            Self::LinearChirp { f_start, f_end } => {
                f_start + (f_end - f_start) * tau
            }
            Self::ExponentialChirp { f_start, f_end } => {
                f_start * (f_end / f_start.max(1.0)).powf(tau)
            }
            Self::SinusoidalWander { f_center, delta_f, mod_hz, phase_rad } => {
                f_center + delta_f * (2.0 * PI * mod_hz * (t - t_start) + phase_rad).sin()
            }
            Self::PolynomialCubic { c0, c1, c2, c3 } => {
                let dt = t - t_start;
                c0 + c1 * dt + c2 * dt * dt + c3 * dt * dt * dt
            }
        }
    }

    /// Integral da frequência base no intervalo [0, t] para cálculo da fase
    pub fn phase_integral_at(&self, t: f32, t_start: f32, t_end: f32) -> f32 {
        let duration = (t_end - t_start).max(1e-6);
        let dt = (t - t_start).max(0.0);
        let tau = (dt / duration).clamp(0.0, 1.0);

        match self {
            Self::LinearChirp { f_start, f_end } => {
                2.0 * PI * (f_start * dt + 0.5 * (f_end - f_start) * (dt * tau))
            }
            Self::ExponentialChirp { f_start, f_end } => {
                let ratio = (f_end / f_start.max(1.0)).ln();
                if ratio.abs() < 1e-6 {
                    2.0 * PI * f_start * dt
                } else {
                    2.0 * PI * f_start * duration * (f_end / f_start).powf(tau) / ratio
                }
            }
            Self::SinusoidalWander { f_center, delta_f, mod_hz, phase_rad } => {
                let base = 2.0 * PI * f_center * dt;
                let mod_int = if mod_hz.abs() > 1e-6 {
                    -(delta_f / mod_hz) * (2.0 * PI * mod_hz * dt + phase_rad).cos()
                } else {
                    0.0
                };
                base + mod_int
            }
            Self::PolynomialCubic { c0, c1, c2, c3 } => {
                2.0 * PI * (c0 * dt + 0.5 * c1 * dt * dt + (1.0 / 3.0) * c2 * dt.powi(3) + 0.25 * c3 * dt.powi(4))
            }
        }
    }
}

/// Representação completa de uma crista sintética analítica
#[derive(Clone, Debug)]
pub struct RidgeTrajectory {
    pub id: usize,
    pub label: String,
    pub t_start: f32,
    pub t_end: f32,
    pub shape: TrajectoryShape,
    pub base_amp: f32,
    pub am: Option<AmModulation>,
    pub fm: Option<FmModulation>,
    pub initial_phase_rad: f32,
    /// Harmônicos adicionais opcionais (ordem, amplitude_relativa)
    pub harmonics: Vec<(usize, f32)>,
}

impl RidgeTrajectory {
    /// Frequência instantânea total no tempo t (incluindo FM)
    #[inline]
    pub fn freq_at(&self, t: f32) -> f32 {
        if t < self.t_start || t > self.t_end {
            return 0.0;
        }
        let base_f = self.shape.freq_at(t, self.t_start, self.t_end);
        let fm_offset = self.fm.map_or(0.0, |fm| fm.freq_offset_at(t));
        (base_f + fm_offset).max(1.0)
    }

    /// Amplitude instantânea total no tempo t (incluindo AM e envelope de borda)
    #[inline]
    pub fn amp_at(&self, t: f32) -> f32 {
        if t < self.t_start || t > self.t_end {
            return 0.0;
        }
        let am_factor = self.am.map_or(1.0, |am| am.envelope_at(t));
        
        // Suavização suave de Tukey/cosseno nas bordas para evitar descontinuidades
        let edge_fade_s = 0.005; // 5 ms fade
        let fade = if t - self.t_start < edge_fade_s {
            0.5 * (1.0 - (PI * (t - self.t_start) / edge_fade_s).cos())
        } else if self.t_end - t < edge_fade_s {
            0.5 * (1.0 - (PI * (self.t_end - t) / edge_fade_s).cos())
        } else {
            1.0
        };

        self.base_amp * am_factor * fade
    }

    /// Fase acumulada analítica exata no tempo t
    pub fn phase_at(&self, t: f32) -> f32 {
        if t < self.t_start {
            return self.initial_phase_rad;
        }
        let base_phi = self.shape.phase_integral_at(t.min(self.t_end), self.t_start, self.t_end);
        let fm_phi = self.fm.map_or(0.0, |fm| 2.0 * PI * fm.phase_integral_at(t));
        self.initial_phase_rad + base_phi + fm_phi
    }
}

/// Ponto de cruzamento exato entre duas cristas (Ground Truth)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossingPoint {
    pub time_s: f32,
    pub freq_hz: f32,
    pub ridge_a_id: usize,
    pub ridge_b_id: usize,
    pub crossing_angle_rad: f32,
}

/// Configuração de Geração de Cena
#[derive(Clone, Debug)]
pub struct SceneConfig {
    pub duration_s: f32,
    pub fs: f32,
    pub noise_type: NoiseType,
    pub snr_db: Option<f32>,
    pub seed: Option<u64>,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            duration_s: 1.0,
            fs: 48000.0,
            noise_type: NoiseType::None,
            snr_db: None,
            seed: Some(42),
        }
    }
}

/// Cena Sintética Completa contendo o áudio no tempo e Ground Truth analítico
pub struct SyntheticScene {
    pub config: SceneConfig,
    pub ridges: Vec<RidgeTrajectory>,
    pub crossing_points: Vec<CrossingPoint>,
    pub samples: Vec<f32>,
}

impl SyntheticScene {
    /// 1. Construtor específico para Cruzamento em X (X-Crossing) clássico
    /// Ridge 1 sobe de f_low para f_high, Ridge 2 desce de f_high para f_low
    pub fn new_x_crossing(
        config: SceneConfig,
        f_low: f32,
        f_high: f32,
        am: Option<AmModulation>,
        fm: Option<FmModulation>,
    ) -> Self {
        let dur = config.duration_s;
        let r1 = RidgeTrajectory {
            id: 0,
            label: "Chirp Ascendente".to_string(),
            t_start: 0.05 * dur,
            t_end: 0.95 * dur,
            shape: TrajectoryShape::LinearChirp { f_start: f_low, f_end: f_high },
            base_amp: 0.45,
            am,
            fm,
            initial_phase_rad: 0.0,
            harmonics: Vec::new(),
        };

        let r2 = RidgeTrajectory {
            id: 1,
            label: "Chirp Descendente".to_string(),
            t_start: 0.05 * dur,
            t_end: 0.95 * dur,
            shape: TrajectoryShape::LinearChirp { f_start: f_high, f_end: f_low },
            base_amp: 0.45,
            am,
            fm,
            initial_phase_rad: PI * 0.5,
            harmonics: Vec::new(),
        };

        let t_cross = 0.5 * dur;
        let f_cross = 0.5 * (f_low + f_high);
        let slope1 = (f_high - f_low) / (0.9 * dur);
        let slope2 = (f_low - f_high) / (0.9 * dur);
        let angle = (slope1 - slope2).abs().atan();

        let crossings = vec![CrossingPoint {
            time_s: t_cross,
            freq_hz: f_cross,
            ridge_a_id: 0,
            ridge_b_id: 1,
            crossing_angle_rad: angle,
        }];

        let mut scene = Self {
            config,
            ridges: vec![r1, r2],
            crossing_points: crossings,
            samples: Vec::new(),
        };
        scene.synthesize_audio();
        scene
    }

    /// 2. Construtor para Cristas em Dupla Hélice (Intertwined / DNA Ridges)
    /// Duas cristas oscilando em antifase, cruzando periodicamente
    pub fn new_double_helix(
        config: SceneConfig,
        f_center: f32,
        delta_f: f32,
        mod_hz: f32,
    ) -> Self {
        let dur = config.duration_s;
        let r1 = RidgeTrajectory {
            id: 0,
            label: "Hélice A".to_string(),
            t_start: 0.0,
            t_end: dur,
            shape: TrajectoryShape::SinusoidalWander {
                f_center,
                delta_f,
                mod_hz,
                phase_rad: 0.0,
            },
            base_amp: 0.4,
            am: Some(AmModulation { rate_hz: mod_hz * 2.0, depth: 0.25, phase_rad: 0.0 }),
            fm: None,
            initial_phase_rad: 0.0,
            harmonics: Vec::new(),
        };

        let r2 = RidgeTrajectory {
            id: 1,
            label: "Hélice B".to_string(),
            t_start: 0.0,
            t_end: dur,
            shape: TrajectoryShape::SinusoidalWander {
                f_center,
                delta_f,
                mod_hz,
                phase_rad: PI,
            },
            base_amp: 0.4,
            am: Some(AmModulation { rate_hz: mod_hz * 2.0, depth: 0.25, phase_rad: PI }),
            fm: None,
            initial_phase_rad: PI * 0.5,
            harmonics: Vec::new(),
        };

        // Cruzamentos ocorrem a cada meio período de modulação: t = k / (2 * mod_hz)
        let mut crossings = Vec::new();
        let half_period = 0.5 / mod_hz;
        let mut t = 0.0;
        while t <= dur {
            crossings.push(CrossingPoint {
                time_s: t,
                freq_hz: f_center,
                ridge_a_id: 0,
                ridge_b_id: 1,
                crossing_angle_rad: (4.0 * PI * delta_f * mod_hz).atan(),
            });
            t += half_period;
        }

        let mut scene = Self {
            config,
            ridges: vec![r1, r2],
            crossing_points: crossings,
            samples: Vec::new(),
        };
        scene.synthesize_audio();
        scene
    }

    /// 3. Construtor para Quase-Cruzamento / Grazing Contact (Avoided Crossing)
    /// Duas cristas que se aproximam até uma distância delta_f_min e voltam a divergir
    pub fn new_grazing_contact(
        config: SceneConfig,
        f_center: f32,
        min_separation_hz: f32,
    ) -> Self {
        let dur = config.duration_s;
        let t_mid = 0.5 * dur;
        // c0 + c2 * (t - t_mid)^2
        let c2 = 4000.0 / (t_mid * t_mid);

        let r1 = RidgeTrajectory {
            id: 0,
            label: "Crista Superior (Parábola)".to_string(),
            t_start: 0.0,
            t_end: dur,
            shape: TrajectoryShape::PolynomialCubic {
                c0: f_center + 0.5 * min_separation_hz,
                c1: 0.0,
                c2,
                c3: 0.0,
            },
            base_amp: 0.45,
            am: None,
            fm: None,
            initial_phase_rad: 0.0,
            harmonics: Vec::new(),
        };

        let r2 = RidgeTrajectory {
            id: 1,
            label: "Crista Inferior (Parábola Invertida)".to_string(),
            t_start: 0.0,
            t_end: dur,
            shape: TrajectoryShape::PolynomialCubic {
                c0: f_center - 0.5 * min_separation_hz,
                c1: 0.0,
                c2: -c2,
                c3: 0.0,
            },
            base_amp: 0.45,
            am: None,
            fm: None,
            initial_phase_rad: PI * 0.25,
            harmonics: Vec::new(),
        };

        let mut scene = Self {
            config,
            ridges: vec![r1, r2],
            crossing_points: Vec::new(), // Sem cruzamento real, apenas aproximação mínima
            samples: Vec::new(),
        };
        scene.synthesize_audio();
        scene
    }

    /// 4. Gerador Aleatório Completo com Múltiplas Cristas e Cruzamentos Arbitrários
    pub fn new_random(
        config: SceneConfig,
        num_ridges: usize,
        _num_expected_crossings: usize,
    ) -> Self {
        let mut rng: StdRng = match config.seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };

        let dur = config.duration_s;
        let mut ridges = Vec::with_capacity(num_ridges);

        for id in 0..num_ridges {
            let t_start = rng.gen_range(0.0..(dur * 0.3));
            let t_end = rng.gen_range((dur * 0.7)..dur);
            let f0 = rng.gen_range(200.0..3500.0);
            let f1 = rng.gen_range(200.0..3500.0);

            let shape = if rng.gen_bool(0.5) {
                TrajectoryShape::LinearChirp { f_start: f0, f_end: f1 }
            } else {
                let delta = rng.gen_range(50.0..400.0);
                let mod_hz = rng.gen_range(2.0..10.0);
                TrajectoryShape::SinusoidalWander {
                    f_center: 0.5 * (f0 + f1),
                    delta_f: delta,
                    mod_hz,
                    phase_rad: rng.gen_range(-PI..PI),
                }
            };

            let am = if rng.gen_bool(0.7) {
                Some(AmModulation {
                    rate_hz: rng.gen_range(3.0..15.0),
                    depth: rng.gen_range(0.1..0.4),
                    phase_rad: rng.gen_range(-PI..PI),
                })
            } else {
                None
            };

            let fm = if rng.gen_bool(0.7) {
                Some(FmModulation {
                    rate_hz: rng.gen_range(4.0..12.0),
                    deviation_hz: rng.gen_range(10.0..60.0),
                    phase_rad: rng.gen_range(-PI..PI),
                })
            } else {
                None
            };

            let harmonics = if rng.gen_bool(0.3) {
                vec![(2, rng.gen_range(0.1..0.3)), (3, rng.gen_range(0.05..0.15))]
            } else {
                Vec::new()
            };

            ridges.push(RidgeTrajectory {
                id,
                label: format!("Crista Aleatória {}", id + 1),
                t_start,
                t_end,
                shape,
                base_amp: rng.gen_range(0.2..0.5),
                am,
                fm,
                initial_phase_rad: rng.gen_range(-PI..PI),
                harmonics,
            });
        }

        // Detecta cruzamentos analiticamente por varredura temporal fina
        let mut crossing_points = Vec::new();
        let scan_steps = 1000;
        for i in 0..ridges.len() {
            for j in (i + 1)..ridges.len() {
                let r_a = &ridges[i];
                let r_b = &ridges[j];
                let t_overlap_start = r_a.t_start.max(r_b.t_start);
                let t_overlap_end = r_a.t_end.min(r_b.t_end);

                if t_overlap_end > t_overlap_start {
                    let mut prev_diff = r_a.freq_at(t_overlap_start) - r_b.freq_at(t_overlap_start);
                    for s in 1..=scan_steps {
                        let t = t_overlap_start + (t_overlap_end - t_overlap_start) * (s as f32 / scan_steps as f32);
                        let diff = r_a.freq_at(t) - r_b.freq_at(t);
                        if (prev_diff > 0.0 && diff <= 0.0) || (prev_diff < 0.0 && diff >= 0.0) {
                            let f_cross = 0.5 * (r_a.freq_at(t) + r_b.freq_at(t));
                            crossing_points.push(CrossingPoint {
                                time_s: t,
                                freq_hz: f_cross,
                                ridge_a_id: i,
                                ridge_b_id: j,
                                crossing_angle_rad: 0.1, // aproximação do ângulo
                            });
                        }
                        prev_diff = diff;
                    }
                }
            }
        }

        let mut scene = Self {
            config,
            ridges,
            crossing_points,
            samples: Vec::new(),
        };
        scene.synthesize_audio();
        scene
    }

    /// Sintetiza o sinal temporal combinando todas as cristas e aplicando o ruído configurado
    fn synthesize_audio(&mut self) {
        let total_samples = (self.config.duration_s * self.config.fs).round() as usize;
        let mut clean_signal = vec![0.0f32; total_samples];
        let dt = 1.0 / self.config.fs;

        // 1. Renderiza cada crista e seus harmônicos
        for ridge in &self.ridges {
            let start_sample = (ridge.t_start * self.config.fs).floor() as usize;
            let end_sample = (ridge.t_end * self.config.fs).ceil() as usize;
            let end_sample = end_sample.min(total_samples);

            let mut phase_accum = ridge.initial_phase_rad;
            for n in start_sample..end_sample {
                let t = n as f32 * dt;
                let f_inst = ridge.freq_at(t);
                let amp = ridge.amp_at(t);

                // Avanço de fase instantâneo
                phase_accum += 2.0 * PI * f_inst * dt;

                let mut s = amp * phase_accum.cos();

                // Adiciona harmônicos
                for &(h_order, h_amp) in &ridge.harmonics {
                    s += (amp * h_amp) * (phase_accum * h_order as f32).cos();
                }

                clean_signal[n] += s;
            }
        }

        // 2. Calcula potência do sinal limpo
        let mut sig_power = 0.0f32;
        for &s in &clean_signal {
            sig_power += s * s;
        }
        sig_power /= total_samples as f32;

        // 3. Gera e adiciona o ruído estocástico
        let mut noise = self.generate_noise_buffer(total_samples);

        // Se SNR for especificado em dB, escala o ruído para corresponder exatamente
        if let Some(snr) = self.config.snr_db {
            let mut noise_power = 0.0f32;
            for &n_val in &noise {
                noise_power += n_val * n_val;
            }
            noise_power /= total_samples as f32;

            if noise_power > 1e-12 && sig_power > 1e-12 {
                let target_noise_power = sig_power / (10.0f32.powf(snr / 10.0));
                let scale = (target_noise_power / noise_power).sqrt();
                for n_val in &mut noise {
                    *n_val *= scale;
                }
            }
        }

        // 4. Mistura Sinal + Ruído
        let mut mixed = vec![0.0f32; total_samples];
        for i in 0..total_samples {
            mixed[i] = clean_signal[i] + noise[i];
        }

        // Normalização de pico para faixa [-0.98, +0.98]
        let peak = mixed.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
        if peak > 0.98 {
            let scale = 0.98 / peak;
            for s in &mut mixed {
                *s *= scale;
            }
        }

        self.samples = mixed;
    }

    /// Gerador dos diversos tipos de ruído configurados
    fn generate_noise_buffer(&self, count: usize) -> Vec<f32> {
        let mut rng: StdRng = match self.config.seed {
            Some(s) => StdRng::seed_from_u64(s.wrapping_add(1337)),
            None => StdRng::from_entropy(),
        };

        let mut out = vec![0.0f32; count];

        match self.config.noise_type {
            NoiseType::None => {}

            NoiseType::White => {
                // Box-Muller Gaussian Noise
                for i in (0..count).step_by(2) {
                    let u1: f32 = rng.gen_range(1e-7..1.0);
                    let u2: f32 = rng.gen_range(0.0..1.0);
                    let r = (-2.0 * u1.ln()).sqrt();
                    let theta = 2.0 * PI * u2;
                    out[i] = r * theta.cos() * 0.15;
                    if i + 1 < count {
                        out[i + 1] = r * theta.sin() * 0.15;
                    }
                }
            }

            NoiseType::Pink => {
                // Filtro IIR de 3 pólos de Paul Kellet para Ruído Rosa (-3 dB/oitava com < 0.5 dB de erro)
                let mut b0 = 0.0f32;
                let mut b1 = 0.0f32;
                let mut b2 = 0.0f32;
                let mut b3 = 0.0f32;
                let mut b4 = 0.0f32;
                let mut b5 = 0.0f32;
                let mut b6 = 0.0f32;

                for s in &mut out {
                    let white = rng.gen_range(-1.0f32..1.0f32);
                    b0 = 0.99886 * b0 + white * 0.0555179;
                    b1 = 0.99332 * b1 + white * 0.0750759;
                    b2 = 0.96900 * b2 + white * 0.1538520;
                    b3 = 0.86650 * b3 + white * 0.3104856;
                    b4 = 0.55000 * b4 + white * 0.5329522;
                    b5 = -0.7616 * b5 - white * 0.0168980;
                    *s = (b0 + b1 + b2 + b3 + b4 + b5 + b6 + white * 0.5362) * 0.04;
                    b6 = white * 0.115926;
                }
            }

            NoiseType::Brownian => {
                // Integrador com fuga (leaky integrator) para ruído 1/f^2 (-6 dB/oitava)
                let mut accum = 0.0f32;
                for s in &mut out {
                    let white = rng.gen_range(-1.0f32..1.0f32);
                    accum = 0.995 * accum + white * 0.08;
                    *s = accum * 0.2;
                }
            }

            NoiseType::Fractal { hurst } => {
                // Ruído Fractal / Movimento Browniano Fracionário (fBm)
                // Implementado por soma de octavas de ruído com peso proporcional a 2^(-H * octave)
                let h_clamped = hurst.clamp(0.05, 0.95);
                let octaves = 8;
                let mut white_layers = Vec::new();
                for _ in 0..octaves {
                    let layer: Vec<f32> = (0..count).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect();
                    white_layers.push(layer);
                }

                for n in 0..count {
                    let mut sum = 0.0f32;
                    for (oct, layer) in white_layers.iter().enumerate() {
                        let step = 1 << oct;
                        let idx = (n / step) * step;
                        let weight = 2.0f32.powf(-h_clamped * (oct as f32));
                        sum += layer[idx.min(count - 1)] * weight;
                    }
                    out[n] = sum * 0.15;
                }
            }

            NoiseType::Impulsive { rate_hz, amplitude } => {
                // Processo estocástico de Poisson para estalos e cliques transientes
                let prob = rate_hz / self.config.fs;
                for s in &mut out {
                    if rng.gen_bool(prob.min(1.0) as f64) {
                        let sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                        *s = sign * amplitude * rng.gen_range(0.5..1.0);
                    }
                }
            }
        }

        out
    }

    /// Retorna as trajetórias de referência (Ground Truth) em formato de pontos [t, freq_hz]
    /// para uma lista arbitrária de tempos de consulta
    pub fn ground_truth_at(&self, query_times: &[f32]) -> Vec<Vec<(f32, f32)>> {
        let mut tracks = Vec::with_capacity(self.ridges.len());
        for ridge in &self.ridges {
            let mut track_points = Vec::new();
            for &t in query_times {
                if t >= ridge.t_start && t <= ridge.t_end {
                    let f = ridge.freq_at(t);
                    track_points.push((t, f));
                }
            }
            tracks.push(track_points);
        }
        tracks
    }
}

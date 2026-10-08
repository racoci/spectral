//! Modelos de Dados e Configurações para DDSP e TreeNN
//!
//! Baseado nas especificações formais do currículo SynthNN (E00-E18).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LfoWaveform {
    Sine,
    Triangle,
    Saw,
    Square,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnvCurveKind {
    Linear,
    Exponential,
    CubicSmooth,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DdspFilterType {
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
}

/// Configuração do envelope ADSR C1 contínuo (Estágio E05)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdsrEnvelopeConfig {
    pub attack_s: f32,
    pub decay_s: f32,
    pub sustain: f32,
    pub release_s: f32,
    pub curve: EnvCurveKind,
}

impl Default for AdsrEnvelopeConfig {
    fn default() -> Self {
        Self {
            attack_s: 0.02,
            decay_s: 0.15,
            sustain: 0.70,
            release_s: 0.25,
            curve: EnvCurveKind::CubicSmooth,
        }
    }
}

/// Configuração do banco harmônico e inarmonicidade (Estágios E06, E08)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HarmonicsConfig {
    /// Amplitudes dos harmônicos H1..HK. H1 é normalizado em 1.0 por gauge.
    pub amplitudes: Vec<f32>,
    /// Fases iniciais de cada harmônico em radianos
    pub phases: Vec<f32>,
    /// Coeficiente de inarmonicidade B >= 0 (rigidez de cordas acústicas)
    pub inharmonicity_b: f32,
    /// Inclinação espectral alpha (roll-off power law: k^(-alpha))
    pub roll_off_alpha: f32,
}

impl Default for HarmonicsConfig {
    fn default() -> Self {
        let mut amps = vec![1.0; 12];
        for k in 1..=12 {
            amps[k - 1] = 1.0 / (k as f32).powf(1.1);
        }
        Self {
            amplitudes: amps,
            phases: vec![0.0; 12],
            inharmonicity_b: 0.0,
            roll_off_alpha: 1.1,
        }
    }
}

/// Formante espectral parametrizado
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FormantFilter {
    pub center_hz: f32,
    pub bandwidth_hz: f32,
    pub gain_db: f32,
}

/// Envelope espectral contínuo com base sob gauge (Estágio E07)
/// S(u) = sum_j w_j * Phi_j(u), com u = log2(f / 440 Hz)
/// Satisfaz estritamente S(0) = 0 dB e S'(0) = 0 dB/oitava.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpectralEnvelopeConfig {
    /// Pesos w0, w1, w2, w3 da base polinomial/racional sob gauge
    pub gauge_weights: [f32; 4],
    /// Filtros de formantes vocais/acústicos opcionais
    pub formants: Vec<FormantFilter>,
}

impl Default for SpectralEnvelopeConfig {
    fn default() -> Self {
        Self {
            gauge_weights: [0.0, 0.0, 0.0, 0.0],
            formants: vec![
                FormantFilter { center_hz: 700.0, bandwidth_hz: 120.0, gain_db: 3.0 },
                FormantFilter { center_hz: 1220.0, bandwidth_hz: 150.0, gain_db: 0.0 },
                FormantFilter { center_hz: 2600.0, bandwidth_hz: 200.0, gain_db: -4.0 },
            ],
        }
    }
}

/// Modulador periódico LFO / Vibrato (Estágio E09)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LfoConfig {
    pub enabled: bool,
    pub rate_hz: f32,
    pub depth_cents: f32,
    pub phase_rad: f32,
    pub waveform: LfoWaveform,
}

impl Default for LfoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rate_hz: 5.5,
            depth_cents: 25.0,
            phase_rad: 0.0,
            waveform: LfoWaveform::Sine,
        }
    }
}

/// Modulação de Frequência (FM) (Estágio E10)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FmConfig {
    pub enabled: bool,
    pub ratio: f32,
    pub index: f32,
    pub phase_rad: f32,
}

impl Default for FmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ratio: 1.414,
            index: 0.6,
            phase_rad: 0.0,
        }
    }
}

/// Modulação de Amplitude (AM / Tremolo) (Estágio E12)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AmConfig {
    pub enabled: bool,
    pub rate_hz: f32,
    pub depth: f32,
    pub phase_rad: f32,
}

impl Default for AmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rate_hz: 4.5,
            depth: 0.35,
            phase_rad: 0.0,
        }
    }
}

/// Ruído Estocástico Fractal (Estágio E13)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NoiseConfig {
    pub enabled: bool,
    pub level_db: f32,
    pub alpha: f32,
    pub knee_hz: f32,
    pub mix: f32,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level_db: -36.0,
            alpha: 1.0, // Ruído rosa (1/f)
            knee_hz: 1500.0,
            mix: 0.05,
        }
    }
}

/// Efeitos acústicos não-locais (Estágio E14)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EffectsConfig {
    pub filter_enabled: bool,
    pub filter_type: DdspFilterType,
    pub filter_cutoff_hz: f32,
    pub filter_resonance_q: f32,

    pub delay_enabled: bool,
    pub delay_time_ms: f32,
    pub delay_feedback: f32,
    pub delay_mix: f32,

    pub reverb_enabled: bool,
    pub reverb_decay_s: f32,
    pub reverb_mix: f32,
}

impl Default for EffectsConfig {
    fn default() -> Self {
        Self {
            filter_enabled: false,
            filter_type: DdspFilterType::Lowpass,
            filter_cutoff_hz: 3500.0,
            filter_resonance_q: 1.0,

            delay_enabled: false,
            delay_time_ms: 150.0,
            delay_feedback: 0.35,
            delay_mix: 0.20,

            reverb_enabled: false,
            reverb_decay_s: 1.2,
            reverb_mix: 0.15,
        }
    }
}

/// Nó oscilatório do grafo TreeNN (Estágios E16, E17, E18)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TreeNodeConfig {
    pub id: String,
    pub label: String,
    pub freq_hz: f32,
    pub amplitude: f32,
    pub phase_rad: f32,
    pub color: String,
}

/// Aresta direcionada de modulação na TreeNN: nó child modula nó parent com acoplamento beta
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TreeEdgeConfig {
    pub parent_id: String,
    pub child_id: String,
    pub beta: f32,
}

/// Grafo de modulação TreeNN completo
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TreeConfig {
    pub enabled: bool,
    pub nodes: Vec<TreeNodeConfig>,
    pub edges: Vec<TreeEdgeConfig>,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            nodes: vec![
                TreeNodeConfig {
                    id: "node-root".to_string(),
                    label: "Raiz Portadora (F0)".to_string(),
                    freq_hz: 220.0,
                    amplitude: 1.0,
                    phase_rad: 0.0,
                    color: "#00f2fe".to_string(),
                },
                TreeNodeConfig {
                    id: "node-mod1".to_string(),
                    label: "Modulador 1 (Sub)".to_string(),
                    freq_hz: 110.0,
                    amplitude: 0.5,
                    phase_rad: 0.0,
                    color: "#38ef7d".to_string(),
                },
                TreeNodeConfig {
                    id: "node-vib".to_string(),
                    label: "Vibrato LFO".to_string(),
                    freq_hz: 5.5,
                    amplitude: 0.2,
                    phase_rad: 0.0,
                    color: "#f59e0b".to_string(),
                },
            ],
            edges: vec![
                TreeEdgeConfig {
                    parent_id: "node-root".to_string(),
                    child_id: "node-mod1".to_string(),
                    beta: 0.6,
                },
                TreeEdgeConfig {
                    parent_id: "node-root".to_string(),
                    child_id: "node-vib".to_string(),
                    beta: 0.35,
                },
            ],
        }
    }
}

/// Configuração Completa do Sintetizador DDSP
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DdspConfig {
    pub f0: f32,
    pub duration_s: f32,
    pub sample_rate: f32,
    pub amplitude: f32,
    pub adsr: AdsrEnvelopeConfig,
    pub harmonics: HarmonicsConfig,
    pub spectral_envelope: SpectralEnvelopeConfig,
    pub lfo: LfoConfig,
    pub fm: FmConfig,
    pub am: AmConfig,
    pub noise: NoiseConfig,
    pub effects: EffectsConfig,
    pub tree: TreeConfig,
}

impl Default for DdspConfig {
    fn default() -> Self {
        Self {
            f0: 220.0,
            duration_s: 2.0,
            sample_rate: 44100.0,
            amplitude: 0.8,
            adsr: AdsrEnvelopeConfig::default(),
            harmonics: HarmonicsConfig::default(),
            spectral_envelope: SpectralEnvelopeConfig::default(),
            lfo: LfoConfig::default(),
            fm: FmConfig::default(),
            am: AmConfig::default(),
            noise: NoiseConfig::default(),
            effects: EffectsConfig::default(),
            tree: TreeConfig::default(),
        }
    }
}

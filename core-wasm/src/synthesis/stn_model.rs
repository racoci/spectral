//! Definições de Dados e Configuração do Sintetizador STN (Sines + Transients + Noise)

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StnSinePartial {
    pub id: usize,
    pub enabled: bool,
    pub f_hz: f32,
    pub db: f32,
    pub phase_rad: f32,
    pub start_s: f32,
    pub end_s: f32,
    #[serde(default = "default_interp")]
    pub interp: String,
    #[serde(default = "default_fade")]
    pub fade_in_ms: f32,
    #[serde(default = "default_fade")]
    pub fade_out_ms: f32,
    #[serde(default)]
    pub vibrato_hz: f32,
    #[serde(default = "default_vib_rate")]
    pub vib_rate_hz: f32,
    #[serde(default)]
    pub inharmonicity: f32,
    #[serde(default)]
    pub bandwidth_hz: f32,
}

fn default_interp() -> String { "Cúbica".to_string() }
fn default_fade() -> f32 { 5.0 }
fn default_vib_rate() -> f32 { 5.0 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StnTransientEvent {
    pub id: usize,
    pub enabled: bool,
    pub start_s: f32,
    pub dur_ms: f32,
    pub db: f32,
    #[serde(default = "default_trans_type")]
    pub type_repr: String,
    #[serde(default = "default_window")]
    pub window: String,
    #[serde(default = "default_decay")]
    pub decay: String,
}

fn default_trans_type() -> String { "pcm".to_string() }
fn default_window() -> String { "Hann".to_string() }
fn default_decay() -> String { "Exponencial".to_string() }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StnNoiseConfig {
    #[serde(default = "default_noise_model")]
    pub model: String,
    #[serde(default = "default_lpc_order")]
    pub order: usize,
    #[serde(default = "default_noise_env_gain")]
    pub env_gain_db: f32,
    #[serde(default = "default_excitation")]
    pub excitation: String,
    #[serde(default = "default_seed")]
    pub seed: u32,
    #[serde(default = "default_bands")]
    pub bands_db: Vec<f32>,
    #[serde(default = "default_noise_corr")]
    pub correlation: f32,
    #[serde(default = "default_update_ms")]
    pub update_ms: f32,
}

fn default_noise_model() -> String { "lpc".to_string() }
fn default_lpc_order() -> usize { 24 }
fn default_noise_env_gain() -> f32 { -12.0 }
fn default_excitation() -> String { "Ruído branco".to_string() }
fn default_seed() -> u32 { 12345 }
fn default_bands() -> Vec<f32> { vec![-4.0, -2.0, 0.0, -3.0, -8.0, -12.0, -18.0, -24.0] }
fn default_noise_corr() -> f32 { 0.15 }
fn default_update_ms() -> f32 { 10.0 }

impl Default for StnNoiseConfig {
    fn default() -> Self {
        Self {
            model: default_noise_model(),
            order: default_lpc_order(),
            env_gain_db: default_noise_env_gain(),
            excitation: default_excitation(),
            seed: default_seed(),
            bands_db: default_bands(),
            correlation: default_noise_corr(),
            update_ms: default_update_ms(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StnConfig {
    #[serde(default = "default_sr")]
    pub sample_rate: f32,
    #[serde(default = "default_dur")]
    pub duration_s: f32,

    #[serde(default = "default_true")]
    pub enable_s: bool,
    #[serde(default)]
    pub s_gain_db: f32,

    #[serde(default = "default_true")]
    pub enable_t: bool,
    #[serde(default)]
    pub t_gain_db: f32,

    #[serde(default = "default_true")]
    pub enable_n: bool,
    #[serde(default = "default_n_gain")]
    pub n_gain_db: f32,

    #[serde(default)]
    pub master_gain_db: f32,
    #[serde(default = "default_true")]
    pub normalize: bool,
    #[serde(default = "default_peak_limit")]
    pub peak_limit_db: f32,

    #[serde(default)]
    pub itd_ms: f32,
    #[serde(default)]
    pub ild_db: f32,
    #[serde(default)]
    pub pan_s: f32,
    #[serde(default)]
    pub pan_t: f32,
    #[serde(default)]
    pub pan_n: f32,

    #[serde(default)]
    pub reverb_enabled: bool,
    #[serde(default = "default_rt60")]
    pub reverb_rt60: f32,
    #[serde(default = "default_wet")]
    pub reverb_wet: f32,

    #[serde(default)]
    pub sines: Vec<StnSinePartial>,
    #[serde(default)]
    pub events: Vec<StnTransientEvent>,
    #[serde(default)]
    pub noise: StnNoiseConfig,
}

fn default_sr() -> f32 { 48000.0 }
fn default_dur() -> f32 { 2.0 }
fn default_true() -> bool { true }
fn default_n_gain() -> f32 { -12.0 }
fn default_peak_limit() -> f32 { -1.0 }
fn default_rt60() -> f32 { 1.2 }
fn default_wet() -> f32 { 0.25 }

impl Default for StnConfig {
    fn default() -> Self {
        Self {
            sample_rate: default_sr(),
            duration_s: default_dur(),
            enable_s: true,
            s_gain_db: 0.0,
            enable_t: true,
            t_gain_db: 0.0,
            enable_n: true,
            n_gain_db: -12.0,
            master_gain_db: 0.0,
            normalize: true,
            peak_limit_db: -1.0,
            itd_ms: 0.0,
            ild_db: 0.0,
            pan_s: 0.0,
            pan_t: 0.0,
            pan_n: 0.0,
            reverb_enabled: false,
            reverb_rt60: 1.2,
            reverb_wet: 0.25,
            sines: vec![
                StnSinePartial { id: 1, enabled: true, f_hz: 110.0, db: -6.2, phase_rad: 0.0, start_s: 0.0, end_s: 2.0, interp: "Cúbica".to_string(), fade_in_ms: 5.0, fade_out_ms: 5.0, vibrato_hz: 0.0, vib_rate_hz: 5.0, inharmonicity: 0.0, bandwidth_hz: 0.0 },
                StnSinePartial { id: 2, enabled: true, f_hz: 220.0, db: -9.1, phase_rad: 0.3, start_s: 0.0, end_s: 2.0, interp: "Cúbica".to_string(), fade_in_ms: 5.0, fade_out_ms: 5.0, vibrato_hz: 0.0, vib_rate_hz: 5.0, inharmonicity: 0.0, bandwidth_hz: 0.0 },
                StnSinePartial { id: 3, enabled: true, f_hz: 330.0, db: -12.5, phase_rad: 1.1, start_s: 0.12, end_s: 1.85, interp: "Cúbica".to_string(), fade_in_ms: 5.0, fade_out_ms: 5.0, vibrato_hz: 0.0, vib_rate_hz: 5.0, inharmonicity: 0.0, bandwidth_hz: 0.0 },
                StnSinePartial { id: 4, enabled: true, f_hz: 440.0, db: -15.0, phase_rad: -0.8, start_s: 0.0, end_s: 1.7, interp: "Cúbica".to_string(), fade_in_ms: 5.0, fade_out_ms: 5.0, vibrato_hz: 0.0, vib_rate_hz: 5.0, inharmonicity: 0.0, bandwidth_hz: 0.0 },
            ],
            events: vec![
                StnTransientEvent { id: 1, enabled: true, start_s: 0.12, dur_ms: 12.5, db: -3.0, type_repr: "pcm".to_string(), window: "Hann".to_string(), decay: "Exponencial".to_string() },
                StnTransientEvent { id: 2, enabled: true, start_s: 0.85, dur_ms: 8.0, db: -6.0, type_repr: "dct".to_string(), window: "Hann".to_string(), decay: "Exponencial".to_string() },
            ],
            noise: StnNoiseConfig::default(),
        }
    }
}

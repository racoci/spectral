use crate::geometry::Spline2;

#[derive(Clone, Debug)]
pub struct Partial {
    pub relative_log_freq: Vec<f32>,
    pub amplitude: Vec<f32>,
    pub phase_offset: f32,
}

#[derive(Clone, Debug)]
pub struct SoundObject {
    pub fundamental: Spline2,
    pub global_amplitude: Vec<f32>,
    pub partials: Vec<Partial>,
    pub sample_rate: f32,
    pub duration_s: f32,
}

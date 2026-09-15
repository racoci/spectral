use crate::geometry::spline::{Spline2, ScalarSpline};

/// Defines a localized audio object containing an implicit partial overtone
/// relative to the fundamental pitch trajectory.
#[derive(Clone, Debug)]
pub struct Partial {
    /// Relative log-frequency delta over time: \delta u_k(t)
    pub relative_log_freq: ScalarSpline,
    
    /// Global amplitude envelope of the partial: a_k(t)
    pub amplitude: ScalarSpline,
    
    /// The initial phase offset of the integrated frequency: \phi_{k,0}
    pub phase_offset: f32,
}

/// Defines a surface texturing field for stochastic residual energy (noise, breath, friction).
#[derive(Clone, Debug)]
pub struct Texture {
    pub time_scale: f32,
    pub freq_scale_oct: f32,
    pub seed: u64,
}

/// Defines non-C2 discrete acoustic breaks or transient strikes (e.g. piano attack, snare hit).
#[derive(Clone, Debug)]
pub struct Event {
    pub time_s: f32,
    pub description: String,
}

/// The V7 master vector format describing a fully parametric geometric audio source.
#[derive(Clone, Debug)]
pub struct SoundObject {
    /// Fundamental continuous log-frequency trajectory: u_0(t) in C^2 space.
    pub fundamental: Spline2,
    
    /// Macro amplitude envelope for the whole object (e.g., ADSR string).
    pub global_amplitude: ScalarSpline,
    
    /// The timbre model containing relative trajectory curves representing instrument overtones/formants.
    pub partials: Vec<Partial>,
    
    /// Surface texturing providing non-deterministic energy distributions (noise/residual bounds).
    pub texture: Option<Texture>,
    
    /// Array of singular events that temporally break global curve continuity.
    pub events: Vec<Event>,
}

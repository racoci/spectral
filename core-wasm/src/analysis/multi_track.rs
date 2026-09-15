//! Multi-track geometric fitting on the reassigned log-frequency plane.
//!
//! The algorithm has two stages:
//! 1. greedy dynamic-programming initialization with soft energy suppression;
//! 2. joint coordinate-descent refinement of all tracks using a single objective
//!    with data fidelity, velocity, acceleration, and pairwise repulsion terms.
//!
//! The final representation is a set of C2 cubic splines in time/log-frequency.

use crate::geometry::spline::Spline2;
use crate::geometry::bezier::Point2;
use crate::analysis::spline_fit::{fit_c2, sample_uniform};
use crate::{C32, Grad};

#[derive(Clone, Copy, Debug)]
pub struct TrackingConfig {
    pub fmin_hz: f32,
    pub bins_per_octave: usize,
    pub sigma_oct: f32,
    pub max_jump_oct: f32,
    pub lambda_velocity: f32,
    pub lambda_acceleration: f32,
    pub lambda_repulsion: f32,
    pub repulsion_sigma_oct: f32,
    pub local_search_oct: f32,
    pub local_search_step_oct: f32,
    pub refinement_passes: usize,
    pub control_points: usize,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            fmin_hz: 20.0,
            bins_per_octave: 12,
            sigma_oct: 1.0 / 18.0,
            max_jump_oct: 0.30,
            lambda_velocity: 0.50,
            lambda_acceleration: 3.0,
            lambda_repulsion: 0.25,
            repulsion_sigma_oct: 1.0 / 12.0,
            local_search_oct: 0.10,
            local_search_step_oct: 1.0 / 48.0,
            refinement_passes: 8,
            control_points: 24,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TrackPath {
    pub u: Vec<f32>,
    pub spline: Spline2,
}

#[derive(Clone, Debug)]
pub struct TrackingResult {
    pub tracks: Vec<TrackPath>,
    pub energy: Vec<f32>,
    pub channels: usize,
    pub frames: usize,
}

/// Scatter reassigned coefficient energy onto a fixed log-frequency grid.
/// `energy[frame * bins + bin]` is linear power.
pub fn reassigned_energy_grid(
    grads: &[Vec<Grad>],
    fmin_hz: f32,
    bins_per_octave: usize,
) -> (Vec<f32>, usize, usize) {
    assert!(!grads.is_empty());
    let channels = grads.len();
    let frames = grads[0].len();
    let mut energy = vec![0.0f32; channels * frames];

    for row in grads {
        assert_eq!(row.len(), frames);
    }

    for row in grads {
        for (m, g) in row.iter().enumerate() {
            if !g.freq_hz.is_finite() || g.freq_hz <= 0.0 {
                continue;
            }
            let q = bins_per_octave as f32 * (g.freq_hz / fmin_hz).log2();
            if !q.is_finite() {
                continue;
            }
            let q = q.clamp(0.0, (channels - 1) as f32);
            let q0 = q.floor() as usize;
            let frac = q - q0 as f32;
            energy[m * channels + q0] += g.coeff.abs2() * (1.0 - frac);
            if q0 + 1 < channels {
                energy[m * channels + q0 + 1] += g.coeff.abs2() * frac;
            }
        }
    }

    (energy, channels, frames)
}

#[inline]
fn e_at(energy: &[f32], channels: usize, frame: usize, bin: usize) -> f32 {
    energy[frame * channels + bin]
}

/// Greedy dynamic-programming initialization.
/// A found path is exponentially suppressed in a narrow log-frequency tube.
pub fn initialize_paths(
    energy_in: &[f32],
    channels: usize,
    frames: usize,
    tracks: usize,
    cfg: &TrackingConfig,
) -> Vec<Vec<f32>> {
    let mut energy = energy_in.to_vec();
    let mut out = Vec::with_capacity(tracks);
    let jump_bins = (cfg.max_jump_oct * cfg.bins_per_octave as f32).ceil() as isize;

    for _ in 0..tracks {
        let mut dp = vec![f32::NEG_INFINITY; channels * frames];
        let mut prev = vec![0u16; channels * frames];

        for k in 0..channels {
            let e = e_at(&energy, channels, 0, k);
            dp[k] = (1.0 + e).ln();
        }

        for m in 1..frames {
            for k in 0..channels {
                let e = (1.0 + e_at(&energy, channels, m, k)).ln();
                let lo = (k as isize - jump_bins).max(0) as usize;
                let hi = (k as isize + jump_bins).min(channels as isize - 1) as usize;
                let mut best = f32::NEG_INFINITY;
                let mut best_p = k;

                for p in lo..=hi {
                    let du = (k as f32 - p as f32) / cfg.bins_per_octave as f32;
                    let score = dp[(m - 1) * channels + p]
                        + e
                        - cfg.lambda_velocity * du * du;
                    if score > best {
                        best = score;
                        best_p = p;
                    }
                }

                dp[m * channels + k] = best;
                prev[m * channels + k] = best_p as u16;
            }
        }

        let mut k = (0..channels)
            .max_by(|&a, &b| {
                dp[(frames - 1) * channels + a]
                    .partial_cmp(&dp[(frames - 1) * channels + b])
                    .unwrap()
            })
            .unwrap_or(0);

        let mut path = vec![0usize; frames];
        path[frames - 1] = k;
        for m in (1..frames).rev() {
            k = prev[m * channels + k] as usize;
            path[m - 1] = k;
        }

        let u: Vec<f32> = path
            .iter()
            .map(|&k| k as f32 / cfg.bins_per_octave as f32)
            .collect();

        // Suppress a soft tube around the extracted path so subsequent tracks
        // prefer distinct energy while still being allowed to cross it.
        for m in 0..frames {
            let center = u[m];
            for k in 0..channels {
                let uk = k as f32 / cfg.bins_per_octave as f32;
                let d = (uk - center) / cfg.sigma_oct;
                let factor = (-0.5 * d * d).exp();
                let idx = m * channels + k;
                energy[idx] *= (1.0 - factor).max(0.0);
            }
        }

        out.push(u);
    }

    out
}

#[inline]
fn interp_energy(energy: &[f32], channels: usize, frame: usize, u: f32, q_min: f32, q_max: f32, bins_per_octave: usize) -> f32 {
    let q = (u - q_min).max(0.0).min(q_max - q_min);
    let x = q * bins_per_octave as f32;
    let lo = x.floor() as usize;
    let hi = (lo + 1).min(channels - 1);
    let a = x - lo as f32;
    e_at(energy, channels, frame, lo) * (1.0 - a) + e_at(energy, channels, frame, hi) * a
}

fn local_objective(
    paths: &[Vec<f32>],
    j: usize,
    m: usize,
    candidate: f32,
    energy: &[f32],
    channels: usize,
    frames: usize,
    cfg: &TrackingConfig,
) -> f64 {
    let mut e = 0.0f64;
    let qmax = (channels - 1) as f32 / cfg.bins_per_octave as f32;
    e -= (1.0 + interp_energy(
        energy,
        channels,
        m,
        candidate,
        0.0,
        qmax,
        cfg.bins_per_octave,
    ) as f64).ln();

    if m > 0 {
        let dv = candidate - paths[j][m - 1];
        e += cfg.lambda_velocity as f64 * (dv as f64).powi(2);
    }
    if m + 1 < frames {
        let dv = paths[j][m + 1] - candidate;
        e += cfg.lambda_velocity as f64 * (dv as f64).powi(2);
    }

    // The sample participates in up to three discrete second differences:
    // (m-2,m-1,m), (m-1,m,m+1), (m,m+1,m+2).
    for start in m.saturating_sub(2)..=m.min(frames.saturating_sub(3)) {
        let a = if start + 2 == m { candidate } else { paths[j][start] };
        let b = if start + 1 == m { candidate } else { paths[j][start + 1] };
        let c = if start == m { candidate } else { paths[j][start + 2] };
        let d2 = c - 2.0 * b + a;
        e += cfg.lambda_acceleration as f64 * (d2 as f64).powi(2);
    }

    // Pairwise repulsion only depends on the changed time sample.
    for q in 0..paths.len() {
        if q == j { continue; }
        let d = candidate - paths[q][m];
        let s = cfg.repulsion_sigma_oct;
        e += cfg.lambda_repulsion as f64
            * (-0.5 * (d / s).powi(2)).exp() as f64;
    }

    e
}

/// Joint coordinate-descent refinement of all paths under a shared objective.
pub fn refine_paths(
    mut paths: Vec<Vec<f32>>,
    energy: &[f32],
    channels: usize,
    frames: usize,
    cfg: &TrackingConfig,
) -> Vec<Vec<f32>> {
    let qmax = (channels - 1) as f32 / cfg.bins_per_octave as f32;
    let mut step = cfg.local_search_step_oct;

    for _pass in 0..cfg.refinement_passes {
        for j in 0..paths.len() {
            for m in 0..frames {
                let current = paths[j][m];
                let mut best_u = current;
                let mut best_local = local_objective(
                    &paths, j, m, current,
                    energy, channels, frames, cfg,
                );
                let radius = (cfg.local_search_oct / step).round() as i32;

                for r in -radius..=radius {
                    let candidate =
                        (current + r as f32 * step).clamp(0.0, qmax);
                    let cand_local = local_objective(
                        &paths, j, m, candidate,
                        energy, channels, frames, cfg,
                    );
                    if cand_local < best_local {
                        best_local = cand_local;
                        best_u = candidate;
                    }
                }

                paths[j][m] = best_u;
            }
        }
        step *= 0.5;
    }

    paths
}

pub fn paths_to_splines(
    paths: &[Vec<f32>],
    frames: usize,
    duration_s: f32,
    control_points: usize,
) -> Vec<TrackPath> {
    paths.iter().map(|u| {
        let t: Vec<f32> = (0..frames)
            .map(|m| m as f32 * duration_s / (frames.saturating_sub(1).max(1) as f32))
            .collect();
        let pts = sample_uniform(&t, u, control_points.max(2));
        let spline = fit_c2(&pts);
        TrackPath { u: u.clone(), spline }
    }).collect()
}

pub fn track_multiple(
    grads: &[Vec<Grad>],
    tracks: usize,
    duration_s: f32,
    cfg: &TrackingConfig,
) -> TrackingResult {
    let (energy, channels, frames) =
        reassigned_energy_grid(grads, cfg.fmin_hz, cfg.bins_per_octave);
    let init = initialize_paths(&energy, channels, frames, tracks, cfg);
    let refined = refine_paths(init, &energy, channels, frames, cfg);
    let tracks = paths_to_splines(&refined, frames, duration_s, cfg.control_points);
    TrackingResult { tracks, energy, channels, frames }
}

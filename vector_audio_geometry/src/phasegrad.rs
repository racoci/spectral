//! Log-spaced Gaussian filter bank with full phase-gradient reassignment.
//!
//! For channel c with center frequency fc and Gaussian
//!     g(t) = exp(-ln(2) * (t/a)^2),
//! we evaluate the complex coefficient
//!     C(tau,fc) = integral x(t) g(t-tau) exp(-i 2 pi fc t) dt.
//!
//! The two phase gradients are obtained from auxiliary windows:
//!     g'(u) = -(2 ln 2 / a^2) u g(u)
//!     t g(t-tau)
//!
//! Since d/dtau C = - C[g'] and
//! d/df C = -i 2 pi C[t g],
//!
//!     f_hat = fc + Im((-C[g']) / C) / (2 pi)
//!     tau_hat = Re(C[t g] / C).
//!
//! These are the continuous-time formulas translated directly to the
//! sampled implementation. The result contains both time reassignment and
//! frequency reassignment; it is therefore suitable for a general filter-bank
//! routing stage rather than a ridge-only display.

use rayon::prelude::*;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Default)]
pub struct C32 {
    pub re: f32,
    pub im: f32,
}

impl C32 {
    #[inline(always)] pub fn new(re: f32, im: f32) -> Self { Self { re, im } }
    #[inline(always)] pub fn conj(self) -> Self { Self::new(self.re, -self.im) }
    #[inline(always)] pub fn abs2(self) -> f32 { self.re * self.re + self.im * self.im }
    #[inline(always)] pub fn mul(self, b: Self) -> Self {
        Self::new(self.re * b.re - self.im * b.im,
                  self.re * b.im + self.im * b.re)
    }
    #[inline(always)] pub fn add_assign(&mut self, b: Self) {
        self.re += b.re; self.im += b.im;
    }
    #[inline(always)] pub fn scale(self, s: f32) -> Self { Self::new(self.re * s, self.im * s) }
}

#[derive(Clone, Copy, Debug)]
pub struct Channel {
    pub center_hz: f32,
    /// Gaussian scale a in seconds: g(u)=exp(-ln2*(u/a)^2).
    pub scale_s: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Grad {
    pub coeff: C32,
    pub freq_hz: f32,
    pub time_s: f32,
    pub confidence: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ReassignedBin {
    pub source_channel: usize,
    pub source_frame: usize,
    pub destination_channel: isize,
    pub destination_frame: isize,
    pub destination_freq_hz: f32,
    pub destination_time_s: f32,
    pub confidence: f32,
}

pub fn log_channels(fmin: f32, fmax: f32, bins_per_octave: usize, cycles: f32) -> Vec<Channel> {
    assert!(fmin > 0.0 && fmax > fmin && bins_per_octave > 0 && cycles > 0.0);
    let mut out = Vec::new();
    let step = 1.0 / bins_per_octave as f32;
    let mut j = 0usize;
    loop {
        let fc = fmin * 2.0_f32.powf(j as f32 * step);
        if fc > fmax * (1.0 + 1e-6) { break; }
        out.push(Channel { center_hz: fc, scale_s: cycles / fc });
        j += 1;
    }
    out
}

#[inline(always)]
fn sinc_pi(x: f32) -> f32 {
    if x.abs() < 1e-5 { 1.0 } else { (PI * x).sin() / (PI * x) }
}

/// Analyze all channels at the requested hop positions.
///
/// `input` is real mono PCM in [-1,1] or arbitrary floating-point samples.
/// Boundary samples outside the signal are treated as zero.
pub fn analyze(input: &[f32], fs_hz: f32, hop: usize, channels: &[Channel]) -> Vec<Vec<Grad>> {
    assert!(fs_hz > 0.0 && hop > 0);
    let frames = (input.len() + hop - 1) / hop;

    channels.par_iter().map(|ch| {
        let mut out = vec![Grad::default(); frames];
        let a = ch.scale_s;
        let half = ((4.0 * a * fs_hz).ceil() as isize).max(1);
        let inv_a2 = 1.0 / (a * a);
        let ln2 = std::f32::consts::LN_2;
        let phase_inc = -2.0 * PI * ch.center_hz / fs_hz;

        for m in 0..frames {
            let center = (m * hop) as isize;
            let mut c = C32::default();
            let mut cg = C32::default();
            let mut ct = C32::default();

            for k in -half..=half {
                let ni = center + k;
                if ni < 0 || ni >= input.len() as isize { continue; }
                let n = ni as usize;
                let u = k as f32 / fs_hz;
                let q = u / a;
                let g = (-ln2 * q * q).exp();
                let gp = -(2.0 * ln2 * inv_a2) * u * g;
                let phase = phase_inc * ni as f32;
                let (s, co) = phase.sin_cos();
                let z = C32::new(input[n] * co * g, input[n] * s * g);
                let zg = C32::new(input[n] * co * gp, input[n] * s * gp);
                let t = ni as f32 / fs_hz;
                let zt = C32::new(z.re * t, z.im * t);
                c.add_assign(z);
                cg.add_assign(zg);
                ct.add_assign(zt);
            }

            let eps = 1e-14_f32;
            let denom = c.abs2().max(eps);
            let ratio_g = c.conj().mul(cg).scale(1.0 / denom);
            let ratio_t = c.conj().mul(ct).scale(1.0 / denom);
            let fhat = ch.center_hz + (-ratio_g.im) / (2.0 * PI);
            let that = ratio_t.re;
            let conf = (c.abs2() / (1.0 + c.abs2())).sqrt();
            out[m] = Grad { coeff: c, freq_hz: fhat, time_s: that, confidence: conf };
        }
        out
    }).collect()
}

pub fn reassign_bins(
    grads: &[Vec<Grad>],
    fs_hz: f32,
    hop: usize,
    fmin: f32,
    bins_per_octave: usize,
) -> Vec<ReassignedBin> {
    let channels = grads.len();
    assert!(channels > 0);
    let frames = grads[0].len();
    grads.iter().enumerate().flat_map(|(k, row)| {
        row.iter().enumerate().filter_map(move |(m, g)| {
            if !g.freq_hz.is_finite() || !g.time_s.is_finite() || g.freq_hz <= 0.0 { return None; }
            let qf = bins_per_octave as f32 * (g.freq_hz / fmin).log2();
            let qt = g.time_s * fs_hz / hop as f32;
            let dc = qf.round() as isize;
            let dt = qt.round() as isize;
            if dc < 0 || dc >= channels as isize || dt < 0 || dt >= frames as isize { return None; }
            Some(ReassignedBin {
                source_channel: k,
                source_frame: m,
                destination_channel: dc,
                destination_frame: dt,
                destination_freq_hz: g.freq_hz,
                destination_time_s: g.time_s,
                confidence: g.confidence,
            })
        })
    }).collect()
}

/// Utility for validating the local-frequency formula independently.
pub fn phase_increment_hz(a: C32, b: C32, dt: f32, fc: f32) -> f32 {
    let p = a.mul(b.conj());
    fc + p.im.atan2(p.re) / (2.0 * PI * dt)
}

// Prevent accidental dead-code warnings in downstream examples for the tiny
// helper kept here for test/reference code.
#[allow(dead_code)]
fn _sinc_guard(x: f32) -> f32 { sinc_pi(x) }

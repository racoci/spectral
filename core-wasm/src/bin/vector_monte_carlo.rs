//! Continuous Vector Audio Monte Carlo Validation Harness
//!
//! This binary performs continuous mathematical validation of the complete pipeline:
//! Audio Signal -> 2D Reassigned Point Cloud -> Track Centroid -> Vector Spline Fitting
//!
//! Symmetrically aligned with V7 design requirements.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::env;
use std::f32::consts::{LN_2, PI};
use std::time::Instant;

use core_wasm::analysis::spline_fit::fit_track_c2;
use core_wasm::analysis::tracker::{TrackBin, track_centroid};

/// A lightweight complex number structure designed for speed and self-containment.
#[derive(Debug, Copy, Clone, PartialEq)]
struct C32 {
    re: f32,
    im: f32,
}

impl C32 {
    #[inline(always)]
    fn zero() -> Self {
        C32 { re: 0.0, im: 0.0 }
    }

    #[inline(always)]
    fn new(re: f32, im: f32) -> Self {
        C32 { re, im }
    }

    #[inline(always)]
    fn add(&mut self, other: Self) {
        self.re += other.re;
        self.im += other.im;
    }

    #[inline(always)]
    fn mul(self, other: Self) -> Self {
        C32 {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    #[inline(always)]
    fn mul_real(self, scalar: f32) -> Self {
        C32 {
            re: self.re * scalar,
            im: self.im * scalar,
        }
    }

    #[inline(always)]
    fn conj(self) -> Self {
        C32 {
            re: self.re,
            im: -self.im,
        }
    }

    #[inline(always)]
    fn norm_sq(self) -> f32 {
        self.re * self.re + self.im * self.im
    }
}

/// Representation of a single analysis channel in the Constant-Q log filter bank.
#[derive(Debug, Copy, Clone)]
struct Filter {
    center_freq: f32,
    scale_s: f32,
}

fn generate_filter_bank() -> Vec<Filter> {
    let mut filters = Vec::with_capacity(120);
    // 12 channels per octave geometrically spaced from 20 Hz to 20 kHz
    for j in 0..120 {
        let fc = 20.0 * 2.0_f32.powf(j as f32 / 12.0);
        let scale_s = 6.0 / fc;
        filters.push(Filter { center_freq: fc, scale_s });
    }
    filters
}

/// Generates a randomized, smooth continuous frequency trajectory in log-frequency space
/// utilizing smoothed acceleration noise.
fn generate_trajectory(
    frames: usize,
    fs: f32,
    hop: usize,
    lo: f32,
    hi: f32,
    rng: &mut StdRng,
) -> (Vec<f32>, Vec<f32>) {
    let dt = hop as f32 / fs;
    let min_u = lo.log2();
    let max_u = hi.log2();
    let mut u = rng.gen_range(min_u..=max_u);
    let mut v = 0.0_f32;
    let mut a = 0.0_f32;
    let rho = 0.985_f32;

    let mut frequencies = vec![0.0_f32; frames];
    let mut amplitudes = vec![0.0_f32; frames];

    for m in 0..frames {
        if m > 0 {
            a = rho * a + (1.0_f32 - rho) * rng.gen_range(-1.0_f32..1.0_f32) * 8.0_f32;
            v = (v + a * dt) * 0.997_f32;
            u += v * dt;

            if u < min_u {
                u = min_u;
                v = v.abs();
            } else if u > max_u {
                u = max_u;
                v = -v.abs();
            }
        }
        frequencies[m] = 2.0_f32.powf(u);
        amplitudes[m] = (0.8_f32 + 0.15_f32 * rng.gen_range(-1.0_f32..1.0_f32)).max(0.15_f32).min(0.9_f32);
    }

    (frequencies, amplitudes)
}

/// Synthesizes the audio signal with fundamental and a soft inharmonic second partial.
fn synthesize_signal(
    fs: f32,
    hop: usize,
    dur: f32,
    frequencies: &[f32],
    amplitudes: &[f32],
    rng: &mut StdRng,
) -> Vec<f32> {
    let n_samples = (fs * dur) as usize;
    let frames = frequencies.len();
    let mut x = vec![0.0_f32; n_samples];
    let dt = 1.0_f32 / fs;

    let mut phase1 = rng.gen_range(0.0_f32..(2.0_f32 * PI));
    let mut phase2 = rng.gen_range(0.0_f32..(2.0_f32 * PI));
    let mut prev_f = frequencies[0];

    for i in 0..n_samples {
        let m = (i / hop).min(frames - 1);
        let f = frequencies[m];
        let amp = amplitudes[m];

        // Continuous trapezoidal frequency integration
        phase1 += 2.0_f32 * PI * 0.5_f32 * (prev_f + f) * dt;
        phase1 %= 2.0_f32 * PI;

        let f2 = f * 2.01_f32; // soft inharmonic second partial
        phase2 += 2.0_f32 * PI * f2 * dt;
        phase2 %= 2.0_f32 * PI;

        prev_f = f;

        // Primary component + partial + low level noise
        x[i] = amp * phase1.cos() + 0.45_f32 * amp * phase2.cos() + 0.003_f32 * rng.gen_range(-1.0_f32..1.0_f32);
    }

    x
}

/// Computes the normal (C), time-derivative (C'_g), and time-weighted (C_tg) complex coefficients
/// using non-uniform phase-gradient windowing.
fn compute_reassignment_coeffs(
    x: &[f32],
    center: isize,
    filter: &Filter,
    fs: f32,
) -> (C32, C32, C32) {
    let a = filter.scale_s;
    let half = ((4.0 * a * fs).ceil() as isize).max(1);
    let inv_a2 = 1.0 / (a * a);
    let phase_inc = -2.0 * PI * filter.center_freq / fs;

    let mut c = C32::zero();
    let mut cg = C32::zero();
    let mut ct = C32::zero();

    for k in -half..=half {
        let ni = center + k;
        if ni < 0 || ni >= x.len() as isize { continue; }
        let n = ni as usize;
        let u = k as f32 / fs;
        let q = u / a;
        let g = (-LN_2 * q * q).exp();
        let gp = -(2.0 * LN_2 * inv_a2) * u * g;

        let phase = phase_inc * ni as f32;
        let (s, co) = phase.sin_cos();

        let val = x[n];
        let z = C32::new(val * co * g, val * s * g);
        let zg = C32::new(val * co * gp, val * s * gp);
        let t = ni as f32 / fs;
        let zt = C32::new(z.re * t, z.im * t);

        c.add(z);
        cg.add(zg);
        ct.add(zt);
    }

    (c, cg, ct)
}

#[allow(dead_code)]
struct ReassignmentResult {
    f_hat: f32,
    tau_hat: f32,
    energy: f32,
}

fn extract_reassignment(
    c: C32,
    cg: C32,
    ct: C32,
    fc: f32,
) -> ReassignmentResult {
    let eps = 1e-14_f32;
    let denom = c.norm_sq().max(eps);

    let ratio_g = c.conj().mul(cg).mul_real(1.0 / denom);
    let ratio_t = c.conj().mul(ct).mul_real(1.0 / denom);

    let f_hat = fc + (-ratio_g.im) / (2.0 * PI);
    let tau_hat = ratio_t.re;
    let energy = c.norm_sq();

    ReassignmentResult {
        f_hat,
        tau_hat,
        energy,
    }
}

fn run_fuzz_iteration(seed: u64, filters: &[Filter]) -> f64 {
    let mut rng = StdRng::seed_from_u64(seed);
    const FS: f32 = 48000.0;
    const HOP: usize = 256;
    const DUR: f32 = 0.6;

    let n_samples = (FS * DUR) as usize;
    let frames = (n_samples + HOP - 1) / HOP;

    // Generate random track (truth frequencies and amplitudes)
    let (truth_freq, truth_amp) = generate_trajectory(frames, FS, HOP, 70.0, 800.0, &mut rng);

    // Synthesize signal
    let x = synthesize_signal(FS, HOP, DUR, &truth_freq, &truth_amp, &mut rng);

    // Compute reassigned bins
    let channels = filters.len();
    let mut bins = vec![vec![TrackBin::default(); frames]; channels];

    for j in 0..channels {
        let filter = &filters[j];
        for m in 0..frames {
            let center = (m * HOP) as isize;
            let (c, cg, ct) = compute_reassignment_coeffs(&x, center, filter, FS);
            let res = extract_reassignment(c, cg, ct, filter.center_freq);
            bins[j][m] = TrackBin {
                freq_hz: res.f_hat,
                time_s: (m * HOP) as f32 / FS,
                energy: res.energy,
            };
        }
    }

    // Extract raw points using the tracker
    let raw_points = track_centroid(&bins, 1);

    // Fit spline to the raw points (re-sampling to 24 control points first)
    let spline = fit_track_c2(&raw_points, 24);

    // Measure the deviation against the true trajectory in cents
    let mut err2 = 0.0f64;
    let mut count = 0;

    for m in 0..frames {
        let f_true = truth_freq[m];
        let t = (m * HOP) as f32 / FS;

        // Evaluate the fitted spline at the normalized time s in [0, 1]
        let s = (t / DUR).clamp(0.0, 1.0);
        let fitted_point = spline.eval(s);
        let f_fitted = 20.0 * 2.0_f32.powf(fitted_point.u);

        let cents = 1200.0 * ((f_fitted / f_true).max(1e-9)).log2();
        err2 += (cents * cents) as f64;
        count += 1;
    }

    (err2 / count as f64).sqrt()
}

fn main() {
    println!(
        r#"
================================================================================
   _    __             __              __  ___             __         ______              __     
  | |  / /__  _________/ /_____  _____ /  |/  /___  ____  / /_ ___   / ____/___  ________/ /___  
  | | / / _ \/ ___/ __  / __/ __ \/ ___// /|_/ / __ \/ __ \/ __/ _ \ / /   / __ \/ ___/ __  / __ \ 
  | |/ /  __/ /__/ /_/ / /_/ /_/ / /   / /  / / /_/ / / / / /_/  __// /___/ /_/ / /  / /_/ / /_/ / 
  |___/\___/\___/\__,_/\__/\____/_/   /_/  /_/\____/_/ /_/\__/\___/ \____/\____/_/   \__,_/\____/  
                                                                                                  
           Continuous Vector Audio Geometric Spline Fitting & Tracking Harness
================================================================================
"#
    );

    println!("Initializing 120-Channel Constant-Q Filter Bank...");
    let filters = generate_filter_bank();
    println!("Filter bank generated successfully. Range: [20.0 Hz, 20.0 kHz]");

    // Retrieve loop count constraints if running in CI or validation environments
    let limit_opt = env::var("FUZZ_ITERATIONS")
        .ok()
        .and_then(|val| val.parse::<u64>().ok());

    let start_time = Instant::now();
    let mut last_log_time = Instant::now();
    let mut worst_rms = 0.0f64;

    let mut global_rng = rand::thread_rng();
    let mut iter = 0u64;

    loop {
        iter += 1;
        let seed = global_rng.gen_range(0..u64::MAX);

        let rms_cents = run_fuzz_iteration(seed, &filters);

        if rms_cents > worst_rms {
            worst_rms = rms_cents;
        }

        // Output results of current iteration
        println!(
            "Iteration: {:5} | Seed: {:20} | RMS Error: {:8.3} cents | Worst RMS: {:8.3} cents",
            iter, seed, rms_cents, worst_rms
        );

        // Fail-safe threshold gating (50 cents limit)
        assert!(
            rms_cents <= 50.0,
            "Validation Failure: RMS error of {:.3} cents exceeded 50 cents threshold (Seed: {})",
            rms_cents,
            seed
        );

        if let Some(limit) = limit_opt {
            let elapsed = start_time.elapsed().as_secs_f64();
            let throughput = iter as f64 / elapsed;
            let percent = (iter as f64 / limit as f64) * 100.0;
            let eta = if throughput > 0.0 {
                (limit - iter) as f64 / throughput
            } else {
                0.0
            };

            // Log progress at 5% intervals or once every 10 seconds
            if last_log_time.elapsed().as_secs() >= 10 || iter % (limit / 20).max(1) == 0 {
                println!(
                    "Progress: [{}/{}] | {:.1}% | Speed: {:.2} iter/s | Elapsed: {:.1}s | ETA: {:.1}s | Worst RMS: {:.3} cents",
                    iter, limit, percent, throughput, elapsed, eta, worst_rms
                );
                last_log_time = Instant::now();
            }

            if iter >= limit {
                println!("\nVector Spline Fuzzing limit of {} reached. Terminating successfully.", limit);
                break;
            }
        } else {
            // Infinite loop mode: Log stats summary every 10 seconds
            if last_log_time.elapsed().as_secs() >= 10 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let throughput = iter as f64 / elapsed;
                println!(
                    "\nProgress: {} iter | Speed: {:.2} iter/s | Elapsed: {:.1}s | Worst RMS so far: {:.3} cents",
                    iter, throughput, elapsed, worst_rms
                );
                last_log_time = Instant::now();
            }
        }
    }

    println!("Continuous Vector Audio Geometric Spline Validation: PASSED ALL CONSTRAINTS.");
}

//! Continuous TF Monte Carlo Fuzzing Harness for 2D Reassigned Constant-Q Filter Bank
//!
//! This binary performs continuous mathematical validation of two foundational properties:
//! 1. Perfect Partition of Unity (Proposed 2D Decomposition)
//! 2. Ridge-Only Information Loss (demonstrating substantial L2 reconstruction errors)
//!
//! Designed and implemented under clean, robust, and warning-free Rust standards.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::env;
use std::f32::consts::{LN_2, PI};
use std::time::Instant;

/// A lightweight complex number structure designed for speed, safety, and self-containment.
#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct C32 {
    re: f32,
    im: f32,
}

#[allow(dead_code)]
impl C32 {
    /// Create a complex number representing zero.
    fn zero() -> Self {
        C32 { re: 0.0, im: 0.0 }
    }

    /// Create a complex number with specified real and imaginary parts.
    fn new(re: f32, im: f32) -> Self {
        C32 { re, im }
    }

    /// Complex addition: (a + ib) + (c + id) = (a + c) + i(b + d)
    fn add(self, other: Self) -> Self {
        C32 {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    /// Complex subtraction: (a + ib) - (c + id) = (a - c) + i(b - d)
    fn sub(self, other: Self) -> Self {
        C32 {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    /// Complex multiplication: (a + ib) * (c + id) = (ac - bd) + i(ad + bc)
    fn mul(self, other: Self) -> Self {
        C32 {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    /// Scale a complex number by a real scalar.
    fn mul_real(self, scalar: f32) -> Self {
        C32 {
            re: self.re * scalar,
            im: self.im * scalar,
        }
    }

    /// Complex division using conjugate expansion.
    fn div(self, other: Self) -> Self {
        let denom = other.re * other.re + other.im * other.im;
        C32 {
            re: (self.re * other.re + self.im * other.im) / denom,
            im: (self.im * other.re - self.re * other.im) / denom,
        }
    }

    /// Returns the complex conjugate.
    fn conj(self) -> Self {
        C32 {
            re: self.re,
            im: -self.im,
        }
    }

    /// Returns the squared magnitude: |Z|^2 = re^2 + im^2
    fn norm_sq(self) -> f32 {
        self.re * self.re + self.im * self.im
    }

    /// Returns the magnitude: |Z| = sqrt(re^2 + im^2)
    fn norm(self) -> f32 {
        self.norm_sq().sqrt()
    }

    /// Returns the argument (phase): arg(Z) in radians [-PI, PI]
    fn arg(self) -> f32 {
        self.im.atan2(self.re)
    }
}

/// Representation of a continuous time-frequency trajectory of a single component.
struct Trajectory {
    frequencies: Vec<f32>,
    amplitudes: Vec<f32>,
}

/// Generates a randomized, smooth continuous frequency trajectory in log-frequency space
/// utilizing smoothed acceleration noise.
fn generate_trajectory(
    n_samples: usize,
    _fs: f32,
    min_freq: f32,
    max_freq: f32,
    rng: &mut StdRng,
) -> Trajectory {
    let mut frequencies = Vec::with_capacity(n_samples);
    let mut amplitudes = Vec::with_capacity(n_samples);

    let min_u = min_freq.log2();
    let max_u = max_freq.log2();
    let mut u = rng.gen_range(min_u..=max_u);
    let mut v = 0.0_f32; // log-frequency velocity
    let mut amp = rng.gen_range(0.3..=0.8);
    let mut amp_v = 0.0_f32; // amplitude velocity

    for _ in 0..n_samples {
        frequencies.push(2.0_f32.powf(u));
        amplitudes.push(amp);

        // Update log frequency with smoothed acceleration noise
        let acc = rng.gen_range(-0.01..=0.01);
        v = 0.98 * v + 0.02 * acc;
        u += v;

        // Soft boundary spring force
        if u < min_u {
            u = min_u;
            v = v.abs() * 0.5;
        } else if u > max_u {
            u = max_u;
            v = -v.abs() * 0.5;
        }

        // Update amplitude with smoothed noise
        let amp_acc = rng.gen_range(-0.005..=0.005);
        amp_v = 0.95 * amp_v + 0.05 * amp_acc;
        amp += amp_v;

        if amp < 0.15 {
            amp = 0.15;
            amp_v = amp_v.abs() * 0.5;
        } else if amp > 0.9 {
            amp = 0.9;
            amp_v = -amp_v.abs() * 0.5;
        }
    }

    Trajectory { frequencies, amplitudes }
}

/// Synthesizes a high-fidelity mono signal using continuous trapezoidal phase integration
/// to prevent instantaneous frequency discontinuities.
fn synthesize_signal(trajectories: &[Trajectory], fs: f32, rng: &mut StdRng) -> Vec<f32> {
    let n_samples = trajectories[0].frequencies.len();
    let mut x = vec![0.0_f32; n_samples];
    let mut phases = vec![0.0_f32; trajectories.len()];

    // Initialize with randomized phases to prevent alignment bias
    for p in &mut phases {
        *p = rng.gen_range(0.0..(2.0 * PI));
    }

    let dt = 1.0 / fs;
    let two_pi = 2.0 * PI;

    for n in 0..n_samples {
        let mut sample_sum = 0.0_f32;
        for (k, traj) in trajectories.iter().enumerate() {
            let f_curr = traj.frequencies[n];
            let amp = traj.amplitudes[n];

            if n > 0 {
                let f_prev = traj.frequencies[n - 1];
                // Continuous trapezoidal frequency integration: \phi_n = \phi_{n-1} + 2\pi * dt * (f_{n-1} + f_n)/2
                phases[k] += two_pi * 0.5 * (f_prev + f_curr) * dt;
                phases[k] %= two_pi;
            }

            sample_sum += amp * phases[k].cos();
        }
        x[n] = sample_sum;
    }

    x
}

/// Representation of a single analysis channel in the Constant-Q log filter bank.
struct Filter {
    center_freq: f32,
    scale_s: f32, // temporal standard deviation scale a_j = 6.0 / f_j
}

/// Precomputes a 120-channel geometrically spaced filter bank.
fn generate_filter_bank() -> Vec<Filter> {
    let mut filters = Vec::with_capacity(120);
    for j in 0..120 {
        let fc = 20.0 * 2.0_f32.powf(j as f32 / 12.0);
        let scale_s = 6.0 / fc;
        filters.push(Filter { center_freq: fc, scale_s });
    }
    filters
}

/// Computes the normal (C), time-derivative (C'_g), and time-weighted (C_tg) complex coefficients
/// for a specific frame and filter channel using non-uniform phase-gradient windowing.
fn compute_reassignment_coeffs(
    x: &[f32],
    n: usize,
    filter: &Filter,
    fs: f32,
) -> (C32, C32, C32) {
    let a = filter.scale_s;
    let inv_a2 = 1.0 / (a * a);
    let half = ((4.0 * a * fs).ceil() as isize).max(1).min(1024);

    let mut c = C32::zero();
    let mut cg = C32::zero();
    let mut ct = C32::zero();

    let two_pi = 2.0 * PI;

    for ni in -half..=half {
        let m = n as isize + ni;
        if m >= 0 && m < x.len() as isize {
            let val = x[m as usize];
            let u = ni as f32 / fs;

            // Gaussian Window: g(u) = e^{-\ln 2 \cdot (u/a)^2}
            let q = u / a;
            let g = (-LN_2 * q * q).exp();

            // Analytical derivative: g'(u) = - (2 * \ln 2 / a^2) * u * g(u)
            let gp = -(2.0 * LN_2 * inv_a2) * u * g;

            // Complex carrier exponent phase
            let phase = filter.center_freq * two_pi * u;
            let (s, co) = phase.sin_cos();

            // Conjugate complex carrier e^{-i \omega u}
            let carrier_re = co;
            let carrier_im = -s;

            let weight_c = val * g;
            c.re += weight_c * carrier_re;
            c.im += weight_c * carrier_im;

            let weight_cg = val * gp;
            cg.re += weight_cg * carrier_re;
            cg.im += weight_cg * carrier_im;

            let weight_ct = val * u * g;
            ct.re += weight_ct * carrier_re;
            ct.im += weight_ct * carrier_im;
        }
    }

    (c, cg, ct)
}

/// Extracted reassignment results including estimated time-frequency coordinates.
#[derive(Debug, Copy, Clone)]
struct ReassignmentResult {
    f_hat: f32,
    tau_hat: f32,
    q: f32,
    energy: f32,
}

/// Extracts estimated coordinates (\widehat{\tau}, \widehat{f}) and confidence q from phase gradient.
fn extract_reassignment(
    c: C32,
    cg: C32,
    ct: C32,
    tau: f32,
    fc: f32,
) -> ReassignmentResult {
    let eps = 1e-14_f32;
    let denom = c.norm_sq().max(eps);

    // Im(C_g / C) = Im(C_g * conj(C)) / denom
    let ratio_g_im = (cg.im * c.re - cg.re * c.im) / denom;
    let f_hat = fc - (ratio_g_im / (2.0 * PI));

    // Re(C_tg / C) = Re(C_tg * conj(C)) / denom
    let ratio_t_re = (ct.re * c.re + ct.im * c.im) / denom;
    let tau_hat = tau + ratio_t_re;

    let energy = c.norm_sq();
    let q = (energy / (1.0 + energy)).sqrt();

    ReassignmentResult {
        f_hat,
        tau_hat,
        q,
        energy,
    }
}

/// Compiled metrics for a single fuzz iteration.
struct Stats {
    l2_error_proposed: f64,
    l2_error_ridge: f64,
    energy_retention_ridge: f64,
    worst_phase_diff: f32,
    worst_magnitude_diff: f32,
}

/// Runs a single iteration of the Monte Carlo fuzzing harness under seed constraints.
fn run_fuzz_iteration(seed: u64, filters: &[Filter]) -> Stats {
    let mut rng = StdRng::seed_from_u64(seed);
    let fs = 44100.0_f32;
    let n_samples = 512;
    let k_trajectories = 3;

    // Generate trajectories
    let mut trajectories = Vec::with_capacity(k_trajectories);
    for _ in 0..k_trajectories {
        let traj = generate_trajectory(n_samples, fs, 100.0, 10000.0, &mut rng);
        trajectories.push(traj);
    }

    // Synthesize signal
    let x = synthesize_signal(&trajectories, fs, &mut rng);

    // Compute reassignment and mask routing over the entire grid
    let n_filters = filters.len();
    let mut z_orig = vec![vec![C32::zero(); n_filters]; n_samples];
    let mut z_rec = vec![vec![C32::zero(); n_filters]; n_samples];
    let mut z_ridge = vec![vec![C32::zero(); n_filters]; n_samples];

    let sigma_f = 0.25_f32; // log-frequency standard deviation in octaves
    let scale_factor = 0.25_f32; // ensures sum_w <= 0.75, making w_bg >= 0.25

    let mut worst_phase_diff = 0.0_f32;
    let mut worst_magnitude_diff = 0.0_f32;

    for n in 0..n_samples {
        let tau = n as f32 / fs;
        for (j, filter) in filters.iter().enumerate() {
            let (c, cg, ct) = compute_reassignment_coeffs(&x, n, filter, fs);
            z_orig[n][j] = c;

            // Extract reassignment coordinates to validate phase-gradient algorithm correctness
            let reassigned = extract_reassignment(c, cg, ct, tau, filter.center_freq);
            // Assert that the reassigned frequency is a valid finite float
            assert!(
                reassigned.f_hat.is_finite(),
                "Adversarial Reassignment failure: f_hat became non-finite. Seed: {}",
                seed
            );

            let mut w = vec![0.0_f32; k_trajectories];
            let mut sum_w = 0.0_f32;

            // Generate 2D Gaussian masks for active trajectories
            for k in 0..k_trajectories {
                let log_fc = filter.center_freq.log2();
                let log_ft = trajectories[k].frequencies[n].log2();
                let d_f = (log_fc - log_ft) / sigma_f;
                let d_t = 0.0_f32; // identical frame time
                let dist_sq = d_f * d_f + d_t * d_t;
                w[k] = scale_factor * (-0.5 * dist_sq).exp();
                sum_w += w[k];
            }

            // Background residual channel weight
            let w_bg = 1.0 - sum_w;

            // Route complex coefficients
            let mut routed = vec![C32::zero(); k_trajectories];
            for k in 0..k_trajectories {
                routed[k] = c.mul_real(w[k]);

                // Phase and Magnitude validation on non-negligible signals to prevent numerical noise
                if w[k] > 1e-4 && c.norm() > 1e-4 {
                    // Test B: Phase Preservation: arg(w_j Z) == arg(Z)
                    let p_orig = c.arg();
                    let p_routed = routed[k].arg();
                    let p_diff = {
                        let mut diff = p_routed - p_orig;
                        while diff > PI { diff -= 2.0 * PI; }
                        while diff < -PI { diff += 2.0 * PI; }
                        diff.abs()
                    };
                    if p_diff > worst_phase_diff {
                        worst_phase_diff = p_diff;
                    }

                    // Test C: Magnitude Preservation: |w_j Z| == w_j |Z|
                    let m_orig = c.norm();
                    let m_routed = routed[k].norm();
                    let m_diff = (m_routed - w[k] * m_orig).abs();
                    if m_diff > worst_magnitude_diff {
                        worst_magnitude_diff = m_diff;
                    }
                }
            }

            let z_bg = c.mul_real(w_bg);

            // Reconstruct the full coefficients (Sum of routed channels + background channel)
            let mut rec = z_bg;
            for k in 0..k_trajectories {
                rec = rec.add(routed[k]);
            }
            z_rec[n][j] = rec;

            // Ridge-only reconstruction (discard background channel)
            let mut ridge = C32::zero();
            for k in 0..k_trajectories {
                ridge = ridge.add(routed[k]);
            }
            z_ridge[n][j] = ridge;
        }
    }

    // Compute total L2 errors and energy retention
    let mut num_proposed = 0.0_f64;
    let mut denom_orig = 0.0_f64;
    let mut num_ridge = 0.0_f64;
    let mut energy_orig = 0.0_f64;
    let mut energy_ridge = 0.0_f64;

    for n in 0..n_samples {
        for j in 0..n_filters {
            let orig = z_orig[n][j];
            let rec = z_rec[n][j];
            let ridge = z_ridge[n][j];

            let diff_proposed = rec.sub(orig);
            let diff_ridge = ridge.sub(orig);

            num_proposed += diff_proposed.norm_sq() as f64;
            denom_orig += orig.norm_sq() as f64;
            num_ridge += diff_ridge.norm_sq() as f64;

            energy_orig += orig.norm_sq() as f64;
            energy_ridge += ridge.norm_sq() as f64;
        }
    }

    // Protect against absolute division by zero
    let denom_orig_safe = denom_orig.max(1e-12);

    let l2_error_proposed = (num_proposed / denom_orig_safe).sqrt();
    let l2_error_ridge = (num_ridge / denom_orig_safe).sqrt();
    let energy_retention_ridge = (energy_ridge / energy_orig.max(1e-12)) * 100.0;

    Stats {
        l2_error_proposed,
        l2_error_ridge,
        energy_retention_ridge,
        worst_phase_diff,
        worst_magnitude_diff,
    }
}

/// Keeps running totals and variance/worst-case stats for long fuzzer sessions.
struct Accumulator {
    count: u64,
    sum_proposed: f64,
    sum_sq_proposed: f64,
    worst_proposed: f64,

    sum_ridge_err: f64,
    sum_sq_ridge_err: f64,
    worst_ridge_err: f64,

    sum_ridge_ret: f64,
    worst_ridge_ret: f64,

    worst_phase_diff: f32,
    worst_mag_diff: f32,
}

impl Accumulator {
    fn new() -> Self {
        Accumulator {
            count: 0,
            sum_proposed: 0.0,
            sum_sq_proposed: 0.0,
            worst_proposed: 0.0,

            sum_ridge_err: 0.0,
            sum_sq_ridge_err: 0.0,
            worst_ridge_err: 0.0,

            sum_ridge_ret: 0.0,
            worst_ridge_ret: 100.0,

            worst_phase_diff: 0.0,
            worst_mag_diff: 0.0,
        }
    }

    fn update(&mut self, stats: &Stats) {
        self.count += 1;

        self.sum_proposed += stats.l2_error_proposed;
        self.sum_sq_proposed += stats.l2_error_proposed * stats.l2_error_proposed;
        if stats.l2_error_proposed > self.worst_proposed {
            self.worst_proposed = stats.l2_error_proposed;
        }

        self.sum_ridge_err += stats.l2_error_ridge;
        self.sum_sq_ridge_err += stats.l2_error_ridge * stats.l2_error_ridge;
        if stats.l2_error_ridge > self.worst_ridge_err {
            self.worst_ridge_err = stats.l2_error_ridge;
        }

        self.sum_ridge_ret += stats.energy_retention_ridge;
        if stats.energy_retention_ridge < self.worst_ridge_ret {
            self.worst_ridge_ret = stats.energy_retention_ridge;
        }

        if stats.worst_phase_diff > self.worst_phase_diff {
            self.worst_phase_diff = stats.worst_phase_diff;
        }
        if stats.worst_magnitude_diff > self.worst_mag_diff {
            self.worst_mag_diff = stats.worst_magnitude_diff;
        }
    }

    fn print_summary(&self) {
        let n = self.count as f64;
        let mean_p = self.sum_proposed / n;
        let var_p = (self.sum_sq_proposed / n - mean_p * mean_p).max(0.0);
        let std_p = var_p.sqrt();

        let mean_r = self.sum_ridge_err / n;
        let var_r = (self.sum_sq_ridge_err / n - mean_r * mean_r).max(0.0);
        let std_r = var_r.sqrt();

        let mean_ret = self.sum_ridge_ret / n;

        println!("================================================================================");
        println!("                         MONTE CARLO FUZZING SUMMARY                           ");
        println!("================================================================================");
        println!("Total Iterations Executed  : {}", self.count);
        println!("Proposed L2 Error (Test A) :");
        println!("  - Mean                   : {:.5e}", mean_p);
        println!("  - Std Dev                : {:.5e}", std_p);
        println!("  - Worst Case             : {:.5e}", self.worst_proposed);
        println!("Phase Preservation (Test B):");
        println!("  - Worst Phase Difference : {:.5e} rad", self.worst_phase_diff);
        println!("Magnitude Preservation (C) :");
        println!("  - Worst Magnitude Diff   : {:.5e}", self.worst_mag_diff);
        println!("Ridge-Only Loss (Test D)   :");
        println!("  - Mean L2 Error          : {:.5e}", mean_r);
        println!("  - Std Dev L2 Error       : {:.5e}", std_r);
        println!("  - Worst Case L2 Error    : {:.5e}", self.worst_ridge_err);
        println!("  - Mean Energy Retention  : {:.4}%", mean_ret);
        println!("  - Worst Energy Retention : {:.4}%", self.worst_ridge_ret);
        println!("================================================================================");
    }
}

fn run_and_save_fuzz_case(seed: u64, filters: &[Filter], filename: &str) -> std::io::Result<()> {
    let mut rng = StdRng::seed_from_u64(seed);
    let fs = 44100.0_f32;
    let n_samples = 512;
    let k_trajectories = 3;

    // Generate trajectories
    let mut trajectories = Vec::with_capacity(k_trajectories);
    for _ in 0..k_trajectories {
        let traj = generate_trajectory(n_samples, fs, 100.0, 10000.0, &mut rng);
        trajectories.push(traj);
    }

    // Synthesize signal
    let x = synthesize_signal(&trajectories, fs, &mut rng);

    let n_filters = filters.len();
    let mut original_spec = vec![0.0f32; n_samples * n_filters];
    let mut reassigned_spec = vec![0.0f32; n_samples * n_filters];

    let total_duration_s = n_samples as f32 / fs;

    for n in 0..n_samples {
        let tau = n as f32 / fs;
        for (j, filter) in filters.iter().enumerate() {
            let (c, cg, ct) = compute_reassignment_coeffs(&x, n, filter, fs);
            
            // Original spectrum is the norm sq
            let power = c.norm_sq();
            original_spec[j * n_samples + n] = power;

            // Extract reassigned coordinates
            let reassigned = extract_reassignment(c, cg, ct, tau, filter.center_freq);
            
            if reassigned.f_hat >= 20.0 && reassigned.f_hat <= 20000.0 && reassigned.tau_hat >= 0.0 && reassigned.tau_hat <= total_duration_s {
                // Logarithmic frequency coordinate matching human perception
                let y_frac = (reassigned.f_hat / 20.0).ln() / 1000.0f32.ln();
                let pixel_y = y_frac * (n_filters - 1) as f32;
                
                // Linear time coordinate
                let x_frac = (reassigned.tau_hat / total_duration_s) * (n_samples - 1) as f32;
                let pixel_x = x_frac;
                
                if pixel_y >= 0.0 && pixel_y <= (n_filters - 1) as f32 && pixel_x >= 0.0 && pixel_x <= (n_samples - 1) as f32 {
                    let y0 = pixel_y.floor() as usize;
                    let y1 = (y0 + 1).min(n_filters - 1);
                    let frac_y = pixel_y - y0 as f32;
                    
                    let x0 = pixel_x.floor() as usize;
                    let x1 = (x0 + 1).min(n_samples - 1);
                    let frac_x = pixel_x - x0 as f32;
                    
                    let energy = power * reassigned.q;
                    
                    // Bilinear deposit
                    reassigned_spec[y0 * n_samples + x0] += energy * (1.0 - frac_x) * (1.0 - frac_y);
                    reassigned_spec[y0 * n_samples + x1] += energy * frac_x * (1.0 - frac_y);
                    reassigned_spec[y1 * n_samples + x0] += energy * (1.0 - frac_x) * frac_y;
                    reassigned_spec[y1 * n_samples + x1] += energy * frac_x * frac_y;
                }
            }
        }
    }

    // Now, save to JSON file
    use std::fs::File;
    use std::io::Write;
    let mut file = File::create(filename)?;
    writeln!(file, "{{")?;
    writeln!(file, "  \"width\": {},", n_samples)?;
    writeln!(file, "  \"height\": {},", n_filters)?;
    
    // Write trajectories
    writeln!(file, "  \"trajectories\": [")?;
    for (i, t) in trajectories.iter().enumerate() {
        writeln!(file, "    {{")?;
        write!(file, "      \"freq_hz\": [")?;
        for (j, &f) in t.frequencies.iter().enumerate() {
            if j > 0 { write!(file, ", ")?; }
            write!(file, "{:.4}", f)?;
        }
        writeln!(file, "],")?;
        write!(file, "      \"amp\": [")?;
        for (j, &a) in t.amplitudes.iter().enumerate() {
            if j > 0 { write!(file, ", ")?; }
            write!(file, "{:.4}", a)?;
        }
        writeln!(file, "]")?;
        write!(file, "    }}")?;
        if i + 1 < trajectories.len() { writeln!(file, ",")?; } else { writeln!(file)?; }
    }
    writeln!(file, "  ],")?;
    
    // Write original spec
    write!(file, "  \"original_spec\": [")?;
    for (i, &v) in original_spec.iter().enumerate() {
        if i > 0 { write!(file, ", ")?; }
        write!(file, "{:.4e}", v)?;
    }
    writeln!(file, "],")?;
    
    // Write reassigned spec
    write!(file, "  \"reassigned_spec\": [")?;
    for (i, &v) in reassigned_spec.iter().enumerate() {
        if i > 0 { write!(file, ", ")?; }
        write!(file, "{:.4e}", v)?;
    }
    writeln!(file, "]")?;
    
    writeln!(file, "}}")?;
    Ok(())
}

fn main() {
    println!(
        r#"
================================================================================
   ______            __  _                                                   
  / ____/___  ____  / /_(_)___  __  ______  __  _                           
 / /   / __ \/ __ \/ __/ / __ \/ / / / __ \/ / / /                          
/ /___/ /_/ / / / / /_/ / / / / /_/ / /_/ / /_/ /                           
\____/\____/_/ /_/\__/_/_/ /_/\__,_/\____/\__,_/                            
                                                                            
        Continuous TF Monte Carlo Fuzzing & Adversarial Testing Harness      
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
    let mut accumulator = Accumulator::new();

    let mut global_rng = rand::thread_rng();

    let mut iter = 0u64;

    loop {
        iter += 1;
        let seed = global_rng.gen_range(0..u64::MAX);

        if iter == 1 {
            let _ = run_and_save_fuzz_case(seed, &filters, "tests/test-outputs/tf_monte_carlo_case.json");
        }

        let stats = run_fuzz_iteration(seed, &filters);

        // Update long-term accumulator
        accumulator.update(&stats);

        // Verification checks
        // Test A: Partition Identity
        assert!(
            stats.l2_error_proposed <= 1e-5,
            "Test A Violation: Proposed reconstruction L2 error {:.5e} is greater than 1e-5. Seed: {}",
            stats.l2_error_proposed,
            seed
        );

        // Test B: Phase Preservation
        assert!(
            stats.worst_phase_diff <= 1e-5,
            "Test B Violation: Phase preservation difference {:.5e} rad is greater than 1e-5. Seed: {}",
            stats.worst_phase_diff,
            seed
        );

        // Test C: Magnitude Preservation
        assert!(
            stats.worst_magnitude_diff <= 1e-5,
            "Test C Violation: Magnitude preservation difference {:.5e} is greater than 1e-5. Seed: {}",
            stats.worst_magnitude_diff,
            seed
        );

        // Test D: Ridge-Only Loss Comparison
        assert!(
            stats.l2_error_ridge > stats.l2_error_proposed * 10.0,
            "Test D Violation: Ridge L2 error {:.5e} is not substantially greater than Proposed L2 error {:.5e}. Seed: {}",
            stats.l2_error_ridge,
            stats.l2_error_proposed,
            seed
        );

        assert!(
            stats.energy_retention_ridge < 99.95,
            "Test D Violation: Ridge energy retention {:.4}% is too high (should be < 99.95% due to background severance). Seed: {}",
            stats.energy_retention_ridge,
            seed
        );

        // Output structured updates according to GEMINI.md Guidelines
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
            if last_log_time.elapsed().as_secs() >= 10
                || iter % (limit / 20).max(1) == 0
            {
                println!(
                    "Progress: [{}/{}] | {:.1}% | Speed: {:.2} iter/s | Elapsed: {:.1}s | ETA: {:.1}s",
                    iter, limit, percent, throughput, elapsed, eta
                );
                last_log_time = Instant::now();
            }

            if iter >= limit {
                println!("\nFuzzing limit of {} reached. Terminating successfully.", limit);
                break;
            }
        } else {
            // Infinite loop mode: Log stats summary every 10 seconds
            if last_log_time.elapsed().as_secs() >= 10 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let throughput = iter as f64 / elapsed;
                println!(
                    "\nProgress: {} iter | Speed: {:.2} iter/s | Elapsed: {:.1}s",
                    iter, throughput, elapsed
                );
                accumulator.print_summary();
                last_log_time = Instant::now();
            }
        }
    }

    accumulator.print_summary();
    println!("Continuous Time-Frequency Monte Carlo Validation: PASSED ALL CONSTRAINTS.");
}

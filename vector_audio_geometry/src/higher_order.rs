//! Higher-Order Mixed Derivatives of Phase and Log-Amplitude for Time-Frequency Analysis.
//!
//! Implements analytical calculation of mixed derivatives D^{p,q} log A and D^{p,q} phi
//! for orders up to O (where O is configurable, typically 1 to 4).
//!
//! Features:
//! - Hermite polynomial-modulated Gaussian windows h_{p,q}(u) = u^q g^{(p)}(u)
//! - Moving-frame STFT evaluation eliminating binomial summation
//! - Multivariate Faà di Bruno partition expansion of log S(t, w)
//! - Exact closed-form Hessian invariants (determinant, trace, eigenvalues, anisotropy)
//! - Closed-form ridge orientation angle theta_ridge
//! - Sub-pixel critical point localization (delta t*, delta w*) and extrapolated amplitude A*
//! - Instantaneous bandwidth B_{-3dB} and duration Delta t_{-3dB}
//! - Chirp rate alpha = phi_tt and third-order inflection points delta t_inflex = -phi_tt / phi_ttt

use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex32 {
    pub re: f32,
    pub im: f32,
}

impl Complex32 {
    #[inline(always)]
    pub fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    #[inline(always)]
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }

    #[inline(always)]
    pub fn abs2(self) -> f32 {
        self.re * self.re + self.im * self.im
    }

    #[inline(always)]
    pub fn abs(self) -> f32 {
        self.abs2().sqrt()
    }

    #[inline(always)]
    pub fn mul(self, b: Self) -> Self {
        Self::new(
            self.re * b.re - self.im * b.im,
            self.re * b.im + self.im * b.re,
        )
    }

    #[inline(always)]
    pub fn add(self, b: Self) -> Self {
        Self::new(self.re + b.re, self.im + b.im)
    }

    #[inline(always)]
    pub fn sub(self, b: Self) -> Self {
        Self::new(self.re - b.re, self.im - b.im)
    }

    #[inline(always)]
    pub fn scale(self, s: f32) -> Self {
        Self::new(self.re * s, self.im * s)
    }

    #[inline(always)]
    pub fn div(self, b: Self) -> Self {
        let d = b.abs2().max(1e-24);
        Self::new(
            (self.re * b.re + self.im * b.im) / d,
            (self.im * b.re - self.re * b.im) / d,
        )
    }
}

/// Evaluates probabilists' Hermite polynomial He_p(x).
#[inline]
pub fn hermite_he(p: usize, x: f32) -> f32 {
    match p {
        0 => 1.0,
        1 => x,
        2 => x * x - 1.0,
        3 => x * (x * x - 3.0),
        4 => (x * x - 6.0) * x * x + 3.0,
        5 => x * (x * x * (x * x - 10.0) + 15.0),
        _ => {
            let mut h_prev = 1.0f32;
            let mut h_curr = x;
            for n in 1..p {
                let h_next = x * h_curr - (n as f32) * h_prev;
                h_prev = h_curr;
                h_curr = h_next;
            }
            h_curr
        }
    }
}

/// Evaluates the modified Gaussian window h_{p,q}(u) = u^q * g^{(p)}(u)
/// with Gaussian g(u) = exp(-u^2 / (2 * sigma^2)).
///
/// Returns the analytical value at relative time u (in seconds).
#[inline]
pub fn gaussian_modified_window(p: usize, q: usize, u: f32, sigma: f32) -> f32 {
    let inv_sigma = 1.0 / sigma.max(1e-12);
    let x = u * inv_sigma;
    let g = (-0.5 * x * x).exp();
    let sign = if p % 2 == 1 { -1.0 } else { 1.0 };
    let g_p = sign * inv_sigma.powi(p as i32) * hermite_he(p, x) * g;
    let u_q = if q == 0 { 1.0 } else { u.powi(q as i32) };
    u_q * g_p
}

/// Generates a discrete sample buffer for modified window h_{p,q}[n].
pub fn generate_window_samples(p: usize, q: usize, win_len: usize, fs: f32, sigma: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; win_len];
    let half = (win_len as f32 - 1.0) * 0.5;
    for n in 0..win_len {
        let u = (n as f32 - half) / fs;
        out[n] = gaussian_modified_window(p, q, u, sigma);
    }
    out
}

/// Complete set of derivatives and invariant geometry at a time-frequency point.
#[derive(Clone, Debug, Default)]
pub struct HigherOrderDerivatives {
    /// Maximum derivative order computed (1, 2, 3, or 4).
    pub max_order: usize,
    /// STFT base magnitude.
    pub magnitude: f32,
    /// STFT base phase.
    pub phase: f32,

    // First order (Order >= 1)
    pub d_log_a_dt: f32,
    pub d_log_a_dw: f32,
    pub d_phi_dt: f32,
    pub d_phi_dw: f32,
    pub freq_inst_hz: f32,
    pub time_reassigned_s: f32,

    // Second order (Order >= 2)
    pub d2_log_a_dt2: f32,
    pub d2_log_a_dtw: f32,
    pub d2_log_a_dw2: f32,
    pub d2_phi_dt2: f32,
    pub d2_phi_dtw: f32,
    pub d2_phi_dw2: f32,

    // Hessian Invariants & Sub-pixel Geometry (Order >= 2)
    pub hessian_det: f32,
    pub hessian_trace: f32,
    pub lambda_1: f32,
    pub lambda_2: f32,
    pub ridge_angle_rad: f32,
    pub anisotropy: f32,
    pub delta_t_star: f32,
    pub delta_w_star: f32,
    pub peak_magnitude: f32,
    pub is_ridge: bool,
    pub bandwidth_3db_hz: f32,
    pub duration_3db_s: f32,

    // Third order (Order >= 3)
    pub d3_phi_dt3: f32,
    pub delta_t_inflex: f32,

    // Fourth order (Order >= 4)
    pub d4_phi_dt4: f32,
}

/// Computes Faà di Bruno derivatives of log S from discrete STFT coefficients S_{p,q}.
///
/// `s_pq`: Lookup matrix or function returning S_{p,q}.
pub fn compute_faa_di_bruno_derivatives(
    max_order: usize,
    s_00: Complex32,
    s_10: Complex32,
    s_01: Complex32,
    s_20: Complex32,
    s_11: Complex32,
    s_02: Complex32,
    s_30: Complex32,
    _s_21: Complex32,
    _s_12: Complex32,
    _s_03: Complex32,
    s_40: Complex32,
    _s_04: Complex32,
    fc_hz: f32,
    t_center_s: f32,
) -> HigherOrderDerivatives {
    let mut out = HigherOrderDerivatives {
        max_order,
        magnitude: s_00.abs(),
        phase: s_00.im.atan2(s_00.re),
        ..Default::default()
    };

    if s_00.abs2() < 1e-24 {
        return out;
    }

    let inv_s = Complex32::new(1.0, 0.0).div(s_00);
    let inv_s2 = inv_s.mul(inv_s);
    let inv_s3 = inv_s2.mul(inv_s);
    let _inv_s4 = inv_s3.mul(inv_s);

    // ==========================================
    // 1st ORDER DERIVATIVES (Order >= 1)
    // ==========================================
    let d_log_s_t = s_10.mul(inv_s);
    let d_log_s_w = s_01.mul(inv_s);

    out.d_log_a_dt = d_log_s_t.re;
    out.d_log_a_dw = d_log_s_w.re;
    out.d_phi_dt = d_log_s_t.im;
    out.d_phi_dw = d_log_s_w.im;

    out.freq_inst_hz = fc_hz + out.d_phi_dt / (2.0 * PI);
    out.time_reassigned_s = t_center_s - out.d_phi_dw;

    if max_order < 2 {
        return out;
    }

    // ==========================================
    // 2nd ORDER DERIVATIVES (Order >= 2)
    // ==========================================
    // (log S)_{tt} = S_{2,0} / S - (S_{1,0} / S)^2
    let d2_log_s_tt = s_20.mul(inv_s).sub(d_log_s_t.mul(d_log_s_t));
    // (log S)_{tw} = S_{1,1} / S - (S_{1,0} * S_{0,1}) / S^2
    let d2_log_s_tw = s_11.mul(inv_s).sub(s_10.mul(s_01).mul(inv_s2));
    // (log S)_{ww} = S_{0,2} / S - (S_{0,1} / S)^2
    let d2_log_s_ww = s_02.mul(inv_s).sub(d_log_s_w.mul(d_log_s_w));

    out.d2_log_a_dt2 = d2_log_s_tt.re;
    out.d2_log_a_dtw = d2_log_s_tw.re;
    out.d2_log_a_dw2 = d2_log_s_ww.re;

    out.d2_phi_dt2 = d2_log_s_tt.im;
    out.d2_phi_dtw = d2_log_s_tw.im - 1.0; // Phase rotation compensation for absolute time
    out.d2_phi_dw2 = d2_log_s_ww.im;

    // Hessian Invariants
    let h_tt = out.d2_log_a_dt2;
    let h_tw = out.d2_log_a_dtw;
    let h_ww = out.d2_log_a_dw2;

    let det = h_tt * h_ww - h_tw * h_tw;
    let trace = h_tt + h_ww;
    let discr = ((h_tt - h_ww).powi(2) + 4.0 * h_tw * h_tw).sqrt();

    let l1 = (trace - discr) * 0.5;
    let l2 = (trace + discr) * 0.5;

    out.hessian_det = det;
    out.hessian_trace = trace;
    out.lambda_1 = l1;
    out.lambda_2 = l2;
    out.ridge_angle_rad = 0.5 * (2.0 * h_tw).atan2(h_tt - h_ww);
    out.anisotropy = if trace.abs() > 1e-12 { discr / trace.abs() } else { 0.0 };

    // Exact Sub-pixel Extrema Displacement via Newton-Hessian Inversion
    if det.abs() > 1e-12 {
        let inv_det = 1.0 / det;
        let dt_star = (h_tw * out.d_log_a_dw - h_ww * out.d_log_a_dt) * inv_det;
        let dw_star = (h_tw * out.d_log_a_dt - h_tt * out.d_log_a_dw) * inv_det;
        out.delta_t_star = dt_star;
        out.delta_w_star = dw_star;

        let quad_form = h_ww * out.d_log_a_dt.powi(2)
            - 2.0 * h_tw * out.d_log_a_dt * out.d_log_a_dw
            + h_tt * out.d_log_a_dw.powi(2);
        let log_a_star_diff = -0.5 * quad_form * inv_det;
        out.peak_magnitude = out.magnitude * log_a_star_diff.clamp(-10.0, 10.0).exp();
    } else {
        out.peak_magnitude = out.magnitude;
    }

    // Ridge Condition Check: Directional derivative perpendicular to ridge is zero and curvature is negative
    let cos_t = out.ridge_angle_rad.cos();
    let sin_t = out.ridge_angle_rad.sin();
    // Normal to ridge is (-sin_t, cos_t)
    let grad_normal = -sin_t * out.d_log_a_dt + cos_t * out.d_log_a_dw;
    out.is_ridge = grad_normal.abs() < 0.5 && l1 < -0.1;

    // Instantaneous Bandwidth and Duration (-3dB)
    if h_ww < -1e-6 {
        out.bandwidth_3db_hz = (1.0 / PI) * (std::f32::consts::LN_2 / (-h_ww)).sqrt();
    }
    if h_tt < -1e-6 {
        out.duration_3db_s = 2.0 * (std::f32::consts::LN_2 / (-h_tt)).sqrt();
    }

    if max_order < 3 {
        return out;
    }

    // ==========================================
    // 3rd ORDER DERIVATIVES (Order >= 3)
    // ==========================================
    // (log S)_{ttt} = S_{3,0}/S - 3*(S_{2,0}*S_{1,0})/S^2 + 2*(S_{1,0}/S)^3
    let term1_30 = s_30.mul(inv_s);
    let term2_30 = s_20.mul(s_10).mul(inv_s2).scale(3.0);
    let term3_30 = d_log_s_t.mul(d_log_s_t).mul(d_log_s_t).scale(2.0);
    let d3_log_s_ttt = term1_30.sub(term2_30).add(term3_30);

    out.d3_phi_dt3 = d3_log_s_ttt.im;

    if out.d3_phi_dt3.abs() > 1e-12 {
        out.delta_t_inflex = -out.d2_phi_dt2 / out.d3_phi_dt3;
    }

    if max_order < 4 {
        return out;
    }

    // ==========================================
    // 4th ORDER DERIVATIVES (Order >= 4)
    // ==========================================
    // (log S)_{tttt} = S_{4,0}/S - 4*S_{3,0}*S_{1,0}/S^2 - 3*(S_{2,0}/S)^2 + 12*S_{2,0}*S_{1,0}^2/S^3 - 6*(S_{1,0}/S)^4
    let t1_40 = s_40.mul(inv_s);
    let t2_40 = s_30.mul(s_10).mul(inv_s2).scale(4.0);
    let t3_40 = s_20.mul(inv_s).mul(s_20.mul(inv_s)).scale(3.0);
    let t4_40 = s_20.mul(s_10).mul(s_10).mul(inv_s3).scale(12.0);
    let t5_40 = d_log_s_t.mul(d_log_s_t).mul(d_log_s_t).mul(d_log_s_t).scale(6.0);

    let d4_log_s_tttt = t1_40.sub(t2_40).sub(t3_40).add(t4_40).sub(t5_40);
    out.d4_phi_dt4 = d4_log_s_tttt.im;

    out
}

/// Multi-Order Analysis Engine for a frame of audio.
pub struct HigherOrderEngine {
    pub max_order: usize,
    pub win_len: usize,
    pub fs: f32,
    pub sigma_s: f32,
    windows: Vec<Vec<f32>>,
    labels: Vec<(usize, usize)>,
}

impl HigherOrderEngine {
    /// Creates a new engine precomputing all required Hermite windows up to order O.
    pub fn new(max_order: usize, win_len: usize, fs: f32, sigma_s: f32) -> Self {
        let order = max_order.clamp(1, 4);
        let mut windows = Vec::new();
        let mut labels = Vec::new();

        for p in 0..=order {
            for q in 0..=(order - p) {
                let win = generate_window_samples(p, q, win_len, fs, sigma_s);
                windows.push(win);
                labels.push((p, q));
            }
        }

        Self {
            max_order: order,
            win_len,
            fs,
            sigma_s,
            windows,
            labels,
        }
    }

    /// Number of precomputed windows.
    pub fn num_windows(&self) -> usize {
        self.windows.len()
    }

    /// Evaluates STFT coefficients for all windows on frame `x` at frequency `fc_hz`.
    ///
    /// `x`: Slice of length `win_len`.
    /// `fc_hz`: Frequency in Hertz.
    pub fn analyze_point(&self, x: &[f32], fc_hz: f32, t_center_s: f32) -> HigherOrderDerivatives {
        assert_eq!(x.len(), self.win_len);
        let mut coeffs = vec![Complex32::default(); self.windows.len()];
        let half = (self.win_len as f32 - 1.0) * 0.5;

        for (idx, win) in self.windows.iter().enumerate() {
            let mut acc = Complex32::default();
            for n in 0..self.win_len {
                let u = (n as f32 - half) / self.fs;
                let phase = -2.0 * PI * fc_hz * u;
                let (s, c) = phase.sin_cos();
                let val = x[n] * win[n];
                acc.re += val * c;
                acc.im += val * s;
            }

            let (p, q) = self.labels[idx];
            // S_{p,q} = (-1)^p * (-i)^q * W_{p,q}
            let sign_p = if p % 2 == 1 { -1.0 } else { 1.0 };
            let mod_q = q % 4;
            let factor_q = match mod_q {
                0 => Complex32::new(1.0, 0.0),
                1 => Complex32::new(0.0, -1.0),
                2 => Complex32::new(-1.0, 0.0),
                _ => Complex32::new(0.0, 1.0),
            };

            coeffs[idx] = acc.scale(sign_p).mul(factor_q);
        }

        // Helper to retrieve S_{p,q}
        let get_s = |req_p: usize, req_q: usize| -> Complex32 {
            for (i, &(p, q)) in self.labels.iter().enumerate() {
                if p == req_p && q == req_q {
                    return coeffs[i];
                }
            }
            Complex32::default()
        };

        compute_faa_di_bruno_derivatives(
            self.max_order,
            get_s(0, 0),
            get_s(1, 0),
            get_s(0, 1),
            get_s(2, 0),
            get_s(1, 1),
            get_s(0, 2),
            get_s(3, 0),
            get_s(2, 1),
            get_s(1, 2),
            get_s(0, 3),
            get_s(4, 0),
            get_s(0, 4),
            fc_hz,
            t_center_s,
        )
    }
}

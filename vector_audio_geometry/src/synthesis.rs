use crate::SoundObject;
use std::f32::consts::TAU;

pub fn eval_u(curve: &crate::Spline2, t01: f32) -> f32 {
    curve.eval(t01).u
}

// =========================================================================
// 1. SÍNTESE BÁSICA POR SPLINE CÚBICA (V7)
// =========================================================================
pub fn synthesize(obj: &SoundObject) -> Vec<f32> {
    let n = (obj.duration_s * obj.sample_rate) as usize;
    let mut x = vec![0.0f32; n];
    let mut phases = vec![0.0f32; obj.partials.len() + 1];
    let dt = 1.0 / obj.sample_rate;

    for i in 0..n {
        let t = i as f32 * dt;
        let s = (t / obj.duration_s).clamp(0.0, 1.0);
        let u0 = eval_u(&obj.fundamental, s);
        let f0 = 20.0 * 2.0f32.powf(u0);
        let ga = obj.global_amplitude[((obj.global_amplitude.len() - 1) as f32 * s) as usize];
        phases[0] += TAU * f0 * dt;
        x[i] += ga * phases[0].cos();
        for (j, p) in obj.partials.iter().enumerate() {
            let idx = ((p.relative_log_freq.len() - 1) as f32 * s) as usize;
            let du = p.relative_log_freq[idx];
            let f = f0 * 2.0f32.powf(du);
            let a = ga * p.amplitude[idx];
            phases[j + 1] += TAU * f * dt;
            x[i] += a * (phases[j + 1] + p.phase_offset).cos();
        }
    }
    let peak = x.iter().map(|v| v.abs()).fold(0.0, f32::max);
    if peak > 0.95 {
        let q = 0.95 / peak;
        for v in &mut x {
            *v *= q;
        }
    }
    x
}

// =========================================================================
// 2. FUNÇÕES DE BUMP C_c^∞ E TRANSIÇÕES INFINITAMENTE DIFERENCIÁVEIS
// =========================================================================

/// Avalia a função de bump canônica C_c^∞:
/// Ψ(u) = exp(-1 / (1 - u²)) se |u| < 1, ou 0 se |u| >= 1.
#[inline(always)]
pub fn eval_bump(u: f32) -> f32 {
    let u2 = u * u;
    if u2 >= 0.999f32 {
        return 0.0f32;
    }
    (-1.0f32 / (1.0f32 - u2)).exp()
}

/// Transição C_c^∞ suave baseada no integral da função de bump no intervalo [0, 1].
/// Possui todas as derivadas de ordem n >= 1 nulas em u=0 e u=1.
pub fn smooth_bump_transition(u: f32) -> f32 {
    let u_clamped = u.clamp(0.0, 1.0);
    if u_clamped <= 0.0f32 {
        return 0.0f32;
    }
    if u_clamped >= 1.0f32 {
        return 1.0f32;
    }

    // Quadratura por 16 passos da função exp(-1 / (4x(1-x)))
    let n_steps = 16;
    let du = u_clamped / n_steps as f32;
    let mut num = 0.0f32;
    for s in 0..n_steps {
        let x = (s as f32 + 0.5f32) * du;
        let denom = 4.0f32 * x * (1.0f32 - x);
        if denom > 1e-6f32 {
            num += (-1.0f32 / denom).exp() * du;
        }
    }

    let mut den = 0.0f32;
    let dx = 1.0f32 / n_steps as f32;
    for s in 0..n_steps {
        let x = (s as f32 + 0.5f32) * dx;
        let denom = 4.0f32 * x * (1.0f32 - x);
        if denom > 1e-6f32 {
            den += (-1.0f32 / denom).exp() * dx;
        }
    }

    if den > 1e-12f32 {
        (num / den).clamp(0.0f32, 1.0f32)
    } else {
        u_clamped
    }
}

/// Reconstrói uma trajetória contínua colando polinômios de Taylor nos nós
/// através da transição C_c^∞ estrita (eliminando descontinuidades de qualquer ordem).
pub fn synthesize_smooth_bump_trajectory(
    t: f32,
    centers: &[f32],
    values: &[f32],
    derivs: &[f32],
) -> f32 {
    if centers.is_empty() {
        return 0.0f32;
    }
    if t <= centers[0] {
        return values[0] + derivs[0] * (t - centers[0]);
    }
    let last = centers.len() - 1;
    if t >= centers[last] {
        return values[last] + derivs[last] * (t - centers[last]);
    }

    // Localizar segmento [k, k+1]
    let mut k = 0;
    while k < last && t > centers[k + 1] {
        k += 1;
    }

    let t0 = centers[k];
    let t1 = centers[k + 1];
    let dt_span = t1 - t0;
    if dt_span <= 1e-9f32 {
        return values[k];
    }

    let u = (t - t0) / dt_span;
    let w = smooth_bump_transition(u);

    // Polinômios de Taylor nos dois extremos
    let p0 = values[k] + derivs[k] * (t - t0);
    let p1 = values[k + 1] + derivs[k + 1] * (t - t1);

    (1.0f32 - w) * p0 + w * p1
}

// =========================================================================
// 3. RBFS GAUSSIANAS E ESTADOS COERENTES DE HERMITE-BIRKHOFF
// =========================================================================

/// Avalia o núcleo Gaussiano padrão: K(dt) = exp(-dt² / (2 σ²)).
#[inline(always)]
pub fn eval_gaussian_rbf(dt: f32, sigma: f32) -> f32 {
    let u = dt / sigma;
    (-0.5f32 * u * u).exp()
}

/// Avalia a base Hermite-Gaussiana de ordem p (p = 0..4).
pub fn eval_hermite_gaussian(dt: f32, sigma: f32, order: usize) -> f32 {
    let u = dt / sigma;
    let g = (-0.5f32 * u * u).exp();
    let hp = match order {
        0 => 1.0f32,
        1 => 2.0f32 * u,
        2 => 4.0f32 * u * u - 2.0f32,
        3 => 8.0f32 * u.powi(3) - 12.0f32 * u,
        4 => 16.0f32 * u.powi(4) - 48.0f32 * u * u + 12.0f32,
        _ => 1.0f32,
    };
    hp * g
}

/// Síntese contínua de um tom modulado via RBFs com aceleração quadrática de chirp (phi_tt).
pub fn synthesize_chirp_rbf_point(
    t: f32,
    centers: &[f32],
    amps: &[f32],
    f_insts: &[f32],
    chirps: &[f32],
    phases: &[f32],
    sigma: f32,
) -> f32 {
    let mut num = 0.0f32;
    let mut den = 0.0f32;

    for k in 0..centers.len() {
        let dt = t - centers[k];
        if dt.abs() > 4.0f32 * sigma {
            continue;
        }
        let w = eval_gaussian_rbf(dt, sigma);
        let phase = phases[k] + TAU * f_insts[k] * dt + 0.5f32 * chirps[k] * dt * dt;
        num += w * amps[k] * phase.cos();
        den += w;
    }

    if den > 1e-12f32 {
        num / den
    } else {
        0.0f32
    }
}

// =========================================================================
// 4. SPLINES DE CONVOLUÇÃO (CONVOLUTION SPLINES ANTI-ALIASED)
// =========================================================================

/// Avalia B-spline linear (triangular) B_1(u) com suporte [-1, 1].
#[inline(always)]
fn eval_b1(u: f32) -> f32 {
    let abs_u = u.abs();
    if abs_u < 1.0f32 {
        1.0f32 - abs_u
    } else {
        0.0f32
    }
}

/// Avalia a Spline de Convolução contínua convoluindo B_1 com um núcleo Gaussiano
/// por quadratura numérica simétrica em 7 pontos: β_σ(t) = (B_1 * G_σ)(t).
pub fn eval_convolution_spline(u: f32, sigma: f32) -> f32 {
    let mut acc = 0.0f32;
    let mut norm = 0.0f32;
    let n_quad = 7;
    let range = 3.0f32 * sigma;
    let step = (2.0f32 * range) / (n_quad - 1) as f32;

    for i in 0..n_quad {
        let tau = -range + i as f32 * step;
        let g = (-0.5f32 * (tau / sigma).powi(2)).exp();
        let b = eval_b1(u - tau);
        acc += b * g;
        norm += g;
    }
    if norm > 1e-9f32 {
        acc / norm
    } else {
        0.0f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_function_smooth_boundaries() {
        let center_val = eval_bump(0.0);
        assert!((center_val - (-1.0f32).exp()).abs() < 1e-5);
        assert_eq!(eval_bump(1.0), 0.0);
        assert_eq!(eval_bump(-1.0), 0.0);
        assert_eq!(eval_bump(1.5), 0.0);
        let edge_val = eval_bump(0.99);
        assert!(edge_val > 0.0 && edge_val < 1e-15);
    }

    #[test]
    fn test_smooth_bump_transition_endpoints() {
        assert_eq!(smooth_bump_transition(0.0), 0.0);
        assert_eq!(smooth_bump_transition(1.0), 1.0);
        let mid = smooth_bump_transition(0.5);
        assert!((mid - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_hermite_gaussian_rbf_reconstruction() {
        let sigma = 0.01;
        assert!((eval_hermite_gaussian(0.0, sigma, 0) - 1.0).abs() < 1e-6);
        assert_eq!(eval_hermite_gaussian(0.0, sigma, 1), 0.0);
        assert!((eval_hermite_gaussian(0.0, sigma, 2) - (-2.0)).abs() < 1e-6);
    }

    #[test]
    fn test_convolution_spline_anti_aliased_smoothness() {
        let sigma = 0.1;
        let v0 = eval_convolution_spline(0.0, sigma);
        let v_half = eval_convolution_spline(0.5, sigma);
        let v1 = eval_convolution_spline(1.0, sigma);

        assert!(v0 > v_half);
        assert!(v_half > v1);
        assert!(v1 >= 0.0);
    }
}

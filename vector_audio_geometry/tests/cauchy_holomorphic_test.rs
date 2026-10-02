use vector_audio_geometry::cauchy_jet::*;
use std::f64::consts::PI;

#[test]
fn test_cauchy_riemann_pde() {
    let q = 2.0;
    let field = CauchyHolomorphicField::new(q);

    // Sinal de teste harmônico: 440 Hz + 880 Hz
    let mut spectrum = Vec::new();
    let fs = 48000.0;
    let n_fft = 4096;
    for k in 1..(n_fft / 2) {
        let f_k = k as f64 * fs / n_fft as f64;
        let amp440 = (-0.5 * ((f_k - 440.0) / 10.0).powi(2)).exp();
        let amp880 = 0.5 * (-0.5 * ((f_k - 880.0) / 10.0).powi(2)).exp();
        spectrum.push((f_k, Complex64::new(amp440 + amp880, 0.0)));
    }

    let t = 0.05; // 50 ms
    let p = 1.0 / 440.0; // Período de 440 Hz
    let dt = 1e-6; // Resolução temporal
    let dp = 1e-7; // Resolução no período

    let (err1, err2) = field.check_cauchy_riemann(&spectrum, t, p, dt, dp);
    println!("Cauchy-Riemann Residuals: err1={:.6e}, err2={:.6e}", err1, err2);

    assert!(
        err1 < 1e-4,
        "Equação de Cauchy-Riemann (dU/dt = (2pi/q) dV/dp) violada: err1 = {:.6e}",
        err1
    );
    assert!(
        err2 < 1e-4,
        "Equação de Cauchy-Riemann (dV/dt = -(2pi/q) dU/dp) violada: err2 = {:.6e}",
        err2
    );
}

#[test]
fn test_cauchy_ladder_derivatives() {
    let q = 2.5;
    let field = CauchyHolomorphicField::new(q);

    let mut spectrum = Vec::new();
    let fs = 44100.0;
    let n_fft = 2048;
    for k in 1..(n_fft / 2) {
        let f_k = k as f64 * fs / n_fft as f64;
        let amp = (-0.5 * ((f_k - 523.25) / 15.0).powi(2)).exp(); // Dó C5
        spectrum.push((f_k, Complex64::new(amp, 0.2 * amp)));
    }

    let t = 0.03;
    let p = 1.0 / 523.25;
    let dt = 1e-6;

    for n in 0..3 {
        let (err_u, err_v) = field.check_ladder_derivative(&spectrum, t, p, n, dt);
        println!("Ordem n={}: err_u={:.6e}, err_v={:.6e}", n, err_u, err_v);
        assert!(
            err_u < 1e-4 && err_v < 1e-4,
            "Falha na escada F_{} = (2*pi*i)^-1 * d_z F_{}: err_u={}, err_v={}",
            n + 1, n, err_u, err_v
        );
    }
}

#[test]
fn test_cauchy_vertical_horizontal_pde() {
    let q = 3.0;
    let field = CauchyHolomorphicField::new(q);

    let mut spectrum = Vec::new();
    let fs = 48000.0;
    let n_fft = 4096;
    for k in 1..(n_fft / 2) {
        let f_k = k as f64 * fs / n_fft as f64;
        let amp = (-0.5 * ((f_k - 330.0) / 12.0).powi(2)).exp(); // Mi E4
        spectrum.push((f_k, Complex64::new(amp, 0.0)));
    }

    let t = 0.04;
    let p = 1.0 / 330.0;
    let dt = 1e-6;
    let dp = 1e-7;

    for n in 0..2 {
        let (err_re, err_im) = field.check_pde(&spectrum, t, p, n, dt, dp);
        println!("PDE Ordem n={}: err_re={:.6e}, err_im={:.6e}", n, err_re, err_im);
        assert!(
            err_re < 1e-4 && err_im < 1e-4,
            "Falha na EDP d_p F_{} = i*(q / 2*pi)*d_t F_{}: err_re={}, err_im={}",
            n, n, err_re, err_im
        );
    }
}

#[test]
fn test_cauchy_reassignment_conformal_quotient() {
    let q = 2.0;
    let field = CauchyHolomorphicField::new(q);

    let mut spectrum = Vec::new();
    let fs = 48000.0;
    let n_fft = 4096;
    let f_tonal = 440.0;
    for k in 1..(n_fft / 2) {
        let f_k = k as f64 * fs / n_fft as f64;
        let amp = (-0.5 * ((f_k - f_tonal) / 5.0).powi(2)).exp();
        spectrum.push((f_k, Complex64::new(amp, 0.0)));
    }

    let t = 0.05;
    // Ponto ligeiramente deslocado da frequência central (430 Hz vs 440 Hz)
    let p_displaced = 1.0 / 430.0;
    let (f_hat, t_hat, r) = field.reassignment(&spectrum, t, p_displaced);

    println!("Frequência Nominal: {:.2} Hz", 1.0 / p_displaced);
    println!("Frequência Reatribuída f_hat: {:.2} Hz (Convergindo para {:.2} Hz)", f_hat, f_tonal);
    println!("Tempo Reatribuído t_hat: {:.5} s (Shift = {:.3} ms)", t_hat, (t_hat - t) * 1000.0);
    println!("Quociente Complexo R = W_1 / W_0: re={:.5}, im={:.5}", r.re, r.im);

    // Como a frequência nominal analisada é 430 Hz e o tom real está em 440 Hz,
    // o quociente analítico R reatribui a frequência para cima (~448 Hz)
    assert!(
        f_hat > 430.0 && f_hat < 455.0,
        "A reatribuição f_hat ({:.2}) deve convergir para 440 Hz a partir de 430 Hz",
        f_hat
    );
}

#[test]
fn test_gaussian_bargmann_holomorphy() {
    // Sinal gaussiano x(tau) = exp(-pi * a * tau^2), a = 1.5
    let a = 1.5;
    let x = |tau: f64| (-PI * a * tau * tau).exp();

    let t = 0.25;
    let f = 2.0;
    let dt = 1e-5;
    let df = 1e-5;
    let tau_range = 4.0;
    let steps = 4000;

    let residual = BargmannGaussianSTFT::check_bargmann_holomorphy(x, t, f, dt, df, tau_range, steps);
    println!("Gaussian Bargmann Holomorphy Residual: {:.6e}", residual);

    assert!(
        residual < 1e-4,
        "A Transformada de Bargmann com gauge B_x(z) = exp(pi*t^2 - pi*z^2/2)*V_g deve satisfazer d_t B + i*d_f B = 0: residual = {:.6e}",
        residual
    );
}

#[test]
fn test_canonical_pure_cauchy_riemann() {
    // q_novo = 0.5 (correspondente a q_antigo = pi)
    let q_canon = 0.5;
    let canon_field = CauchyCanonicalField::new(q_canon);

    let mut spectrum = Vec::new();
    let fs = 48000.0;
    let n_fft = 2048;
    for k in 1..(n_fft / 2) {
        let f_k = k as f64 * fs / n_fft as f64;
        let amp = (-0.5 * ((f_k - 440.0) / 10.0).powi(2)).exp();
        spectrum.push((f_k, Complex64::new(amp, 0.0)));
    }

    let t = 0.04;
    // eta = q / f_c
    let fc = 440.0;
    let eta = q_canon / fc;
    let dt = 1e-6;
    let deta = 1e-6;

    let (err1, err2) = canon_field.check_pure_cauchy_riemann(&spectrum, t, eta, dt, deta);
    println!("Pure Cauchy-Riemann (Canonical): err1={:.6e}, err2={:.6e}", err1, err2);

    assert!(
        err1 < 1e-4 && err2 < 1e-4,
        "Equações puras de Cauchy-Riemann dU/deta = -dV/dt e dV/deta = dU/dt falharam: err1={}, err2={}",
        err1, err2
    );
}

#[test]
fn test_canonical_stationary_frequency() {
    let q_canon = 0.75;
    let canon_field = CauchyCanonicalField::new(q_canon);

    let fc_target = 500.0;
    let eta = q_canon / fc_target; // eta = q / f_c
    let t = 0.0; // Centro temporal t = 0

    let f_star = canon_field.stationary_frequency(t, eta);
    println!("Frequência Estacionária f_* no centro (t=0): re={:.2}, im={:.2}", f_star.re, f_star.im);

    assert!(
        (f_star.re - fc_target).abs() < 1e-6,
        "O ponto estacionário f_* deve ser identicamente f_c = q / eta no centro temporal: f_* = {:.4}, target = {:.4}",
        f_star.re, fc_target
    );
    assert!(
        f_star.im.abs() < 1e-6,
        "A parte imaginária do ponto estacionário no centro deve ser zero: im = {:.6e}",
        f_star.im
    );
}

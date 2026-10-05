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

#[test]
fn test_mel_scale_holomorphic_embedding() {
    let q = 1.5;
    let f0 = 440.0;
    let mel_field = ShiftedScaleHolomorphicField::new(PerceptualScaleType::Mel, q, f0);

    // 1. Verificação da condição de máximo estrito: eta'(y) * f'(y) < 0
    let y_test = mel_field.y_from_f(1000.0);
    let eta_prime = mel_field.d_eta_dy(y_test);
    let dy = 1e-6;
    let f_prime = (mel_field.f_from_y(y_test + dy) - mel_field.f_from_y(y_test - dy)) / (2.0 * dy);
    println!("Mel Scale: y={:.2}, eta'={:.6e}, f'={:.6e}, prod={:.6e}", y_test, eta_prime, f_prime, eta_prime * f_prime);
    assert!(eta_prime * f_prime < 0.0, "Condição de máximo estrito violada na escala Mel: eta'*f' >= 0");

    // 2. Verificação da EDP de colapso vertical: dE/dy = i * eta'(y) * dE/dt
    let mut spectrum = Vec::new();
    let fs = 48000.0;
    let n_fft = 2048;
    for k in 1..(n_fft / 2) {
        let f_k = k as f64 * fs / n_fft as f64;
        let amp = (-0.5 * ((f_k - 1000.0) / 20.0).powi(2)).exp();
        spectrum.push((f_k, Complex64::new(amp, 0.0)));
    }

    let t = 0.02;
    let dt = 1e-6;
    let (err_re, err_im) = mel_field.check_vertical_collapse_pde(&spectrum, t, y_test, dt, dy);
    println!("Mel Vertical Collapse PDE Residuals: err_re={:.6e}, err_im={:.6e}", err_re, err_im);
    assert!(err_re < 1e-4 && err_im < 1e-4, "EDP de colapso vertical falhou para a escala Mel");
}

#[test]
fn test_bark_scale_holomorphic_embedding() {
    let q = 2.0;
    let f0 = 1000.0;
    let bark_field = ShiftedScaleHolomorphicField::new(PerceptualScaleType::Bark, q, f0);

    // 1. Verificação da condição de máximo estrito: eta'(y) * f'(y) < 0
    let y_test = bark_field.y_from_f(1500.0);
    let eta_prime = bark_field.d_eta_dy(y_test);
    let dy = 1e-6;
    let f_prime = (bark_field.f_from_y(y_test + dy) - bark_field.f_from_y(y_test - dy)) / (2.0 * dy);
    println!("Bark Scale: y={:.2}, eta'={:.6e}, f'={:.6e}, prod={:.6e}", y_test, eta_prime, f_prime, eta_prime * f_prime);
    assert!(eta_prime * f_prime < 0.0, "Condição de máximo estrito violada na escala Bark: eta'*f' >= 0");

    // 2. Verificação da linearidade de eta_B(y) na fórmula de Traunmüller
    let y_a = 5.0;
    let y_b = 15.0;
    let eta_a_prime = bark_field.d_eta_dy(y_a);
    let eta_b_prime = bark_field.d_eta_dy(y_b);
    println!("Bark linearity: eta'(5.0)={:.8e}, eta'(15.0)={:.8e}", eta_a_prime, eta_b_prime);
    assert!(
        (eta_a_prime - eta_b_prime).abs() / eta_a_prime.abs() < 1e-4,
        "eta(y) deve ser linear para a escala Bark"
    );

    // 3. Verificação da EDP de colapso vertical: dE/dy = i * eta'(y) * dE/dt
    let mut spectrum = Vec::new();
    let fs = 48000.0;
    let n_fft = 2048;
    for k in 1..(n_fft / 2) {
        let f_k = k as f64 * fs / n_fft as f64;
        let amp = (-0.5 * ((f_k - 1500.0) / 25.0).powi(2)).exp();
        spectrum.push((f_k, Complex64::new(amp, 0.0)));
    }

    let t = 0.03;
    let dt = 1e-6;
    let (err_re, err_im) = bark_field.check_vertical_collapse_pde(&spectrum, t, y_test, dt, dy);
    println!("Bark Vertical Collapse PDE Residuals: err_re={:.6e}, err_im={:.6e}", err_re, err_im);
    assert!(err_re < 1e-4 && err_im < 1e-4, "EDP de colapso vertical falhou para a escala Bark");
}

#[test]
fn test_unit_l2_energy_normalization() {
    let fs = 48000.0;
    let n_fft = 4096;
    let df = fs / n_fft as f64;

    for &scale in &[PerceptualScaleType::Cqt, PerceptualScaleType::Mel, PerceptualScaleType::Bark] {
        let field = ShiftedScaleHolomorphicField::new(scale, 1.5, 440.0);
        let y_test = field.y_from_f(1000.0);

        let window = field.normalized_window(y_test, fs, n_fft);
        let energy: f64 = window.iter().map(|&w| w * w * df).sum();
        println!("Scale {:?}: L^2 Integral Energy = {:.6}", scale, energy);

        assert!(
            (energy - 1.0).abs() < 0.02,
            "Energia L^2 deve ser unitária (=1.0) para {:?}: obtido {:.6}",
            scale, energy
        );
    }
}

#[test]
fn test_cauchy_l2_scale_factor_reconciliation() {
    // Prova numérica: para Cauchy (lambda=0), N_2(y) é estritamente proporcional a p^(2*pi*q + 1/2)
    let q = 1.0; // beta = 2*pi
    let field = ShiftedScaleHolomorphicField::new(PerceptualScaleType::Cqt, q, 440.0);
    let fs = 48000.0;
    let n_fft = 4096;

    let f1 = 500.0;
    let f2 = 2000.0;
    let y1 = field.y_from_f(f1);
    let y2 = field.y_from_f(f2);

    let n2_1 = field.l2_normalization_factor(y1, fs, n_fft);
    let n2_2 = field.l2_normalization_factor(y2, fs, n_fft);

    let ratio_n2 = n2_2 / n2_1;

    // Teoria: N_2 propto p^(2*pi*q + 1/2) = f^-(2*pi*q + 1/2)
    let p_power = 2.0 * PI * q + 0.5;
    let expected_ratio = (f1 / f2).powf(p_power);

    println!("Cauchy L^2 Ratio (2000 Hz / 500 Hz): {:.6e}, Teórico: {:.6e}", ratio_n2, expected_ratio);
    let rel_diff = (ratio_n2 - expected_ratio).abs() / expected_ratio;
    assert!(
        rel_diff < 0.02,
        "N_2(y) deve ser proporcional a p^(2*pi*q + 1/2) com alta precisão: diff={:.4}%",
        rel_diff * 100.0
    );
}

#[test]
fn test_tight_frame_reconstruction_ls() {
    // Validação de reconstrução exata por mínimos quadrados de frame tight
    let fs = 48000.0;
    let n_fft = 2048;
    let half_n = n_fft / 2;
    let mel_field = ShiftedScaleHolomorphicField::new(PerceptualScaleType::Mel, 1.5, 440.0);

    // Espectro de teste sintético suave entre 200 Hz e 6000 Hz
    let mut x_k = vec![Complex64::new(0.0, 0.0); half_n];
    for k in 1..half_n {
        let f = k as f64 * fs / n_fft as f64;
        if f >= 200.0 && f <= 6000.0 {
            let re = (f * 0.01).sin();
            let im = (f * 0.02).cos();
            x_k[k] = Complex64::new(re, im);
        }
    }

    // Cria banco de 48 filtros de Mel sobrepostos cobrindo a faixa ativa
    let m_channels = 48;
    let f_min = 50.0;
    let f_max = 7000.0;
    let y_min = mel_field.y_from_f(f_min);
    let y_max = mel_field.y_from_f(f_max);

    let mut windows = Vec::with_capacity(m_channels);
    let mut channels = Vec::with_capacity(m_channels);

    for j in 0..m_channels {
        let y = y_min + (j as f64 / (m_channels - 1) as f64) * (y_max - y_min);
        let win = mel_field.normalized_window(y, fs, n_fft);

        // Canal Y_j[k] = X[k] * G_tilde_j[k]
        let mut y_j = vec![Complex64::new(0.0, 0.0); half_n];
        for k in 0..half_n {
            y_j[k] = x_k[k] * win[k];
        }

        windows.push(win);
        channels.push(y_j);
    }
    
    let k_min = (f_min * n_fft as f64 / fs).round() as usize;
    let k_max = (f_max * n_fft as f64 / fs).round() as usize;
    
    // Otimiza pesos via NNLS projetado
    let weights = ShiftedScaleHolomorphicField::optimize_frame_density_nnls(&windows, k_min, k_max, 500, 1e-3);

    // Reconstrói X_hat via mínimos quadrados com pesos NNLS
    let x_hat = ShiftedScaleHolomorphicField::reconstruct_spectrum_ls(&channels, &windows, &weights, half_n);

    // Avalia o erro de reconstrução no intervalo de interesse [300 Hz .. 5000 Hz]
    let mut num_err = 0.0;
    let mut den_sig = 0.0;

    for k in 1..half_n {
        let f = k as f64 * fs / n_fft as f64;
        if f >= 300.0 && f <= 5000.0 {
            let err = (x_k[k] - x_hat[k]).abs();
            num_err += err * err;
            den_sig += x_k[k].norm_sq();
        }
    }

    let snr_db = 10.0 * (den_sig / (num_err + 1e-15)).log10();
    println!("Mel Frame Reconstruction SNR no centro de banda: {:.2} dB (num_err={:.6e})", snr_db, num_err);

    assert!(
        snr_db > 80.0,
        "A reconstrução por mínimos quadrados do frame deve atingir SNR > 80 dB: obtido {:.2} dB",
        snr_db
    );
}

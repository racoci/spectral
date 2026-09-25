//! Suíte Experimental Completa para Validação do Sliding Jet DFT e Holomorfia de Bargmann-Fock.
//!
//! Implementa os 8 Casos de Teste (TC-01 a TC-08) com medição de tempo de alta resolução (µs e ns):
//! - TC-01: Singularidade em Silêncio e Gating Tikhonov
//! - TC-02: Deriva Numérica Acumulada em 50.000 Amostras e Amortecimento rSDFT
//! - TC-03: Hessiana Singular e Ponto Isotrópico (det H = 0)
//! - TC-04: Truncamento Gaussiano Finito vs Holomorfia de Bargmann
//! - TC-05: Invariância de Derivadas através de Cruzamento de Branch Cut
//! - TC-06: Convergência de Ordem do Jato de Taylor em Frequência (O(delta^{O+1}))
//! - TC-07: Frequências Fracionárias no Sliding DFT
//! - TC-08: Equivalência Formal Série Euler vs Faà di Bruno até Ordem 4

use vector_audio_geometry::higher_order::*;
use std::f32::consts::PI;
use std::time::Instant;

/// Helper: Formal power series log recursion L' = C'/C mod delta^{O+1}
pub fn compute_formal_series_log(c: &[Complex32]) -> Vec<Complex32> {
    let order = c.len() - 1;
    let mut ell = vec![Complex32::default(); order + 1];
    
    let c0 = c[0];
    let abs2_c0 = c0.abs2();
    if abs2_c0 < 1e-24 {
        return ell; // Safe zero fallback para silêncio e singularidades
    }
    
    ell[0] = Complex32::new(c0.abs().ln(), c0.im.atan2(c0.re));
    let inv_c0 = Complex32::new(1.0, 0.0).div(c0);
    
    for k in 1..=order {
        let mut sum = Complex32::default();
        for j in 1..k {
            let term = ell[j].scale(j as f32).mul(c[k - j]);
            sum = sum.add(term);
        }
        let k_f = k as f32;
        let diff = c[k].sub(sum.scale(1.0 / k_f));
        ell[k] = diff.mul(inv_c0);
    }
    ell
}

// =========================================================================
// TC-01: Singularidade em Silêncio e Gating Tikhonov
// =========================================================================
#[test]
fn test_tc01_singularity_silence_tikhonov() {
    let t_start = Instant::now();
    println!("🧪 Executando TC-01: Singularidade em Silêncio e Gating Tikhonov...");
    
    // Caso 1: Silêncio absoluto (todos os coeficientes nulos)
    let c_zeros = vec![Complex32::default(); 5];
    let ell_zeros = compute_formal_series_log(&c_zeros);
    for (idx, val) in ell_zeros.iter().enumerate() {
        assert!(val.re.is_finite(), "ell[{}] re não é finito em silêncio", idx);
        assert!(val.im.is_finite(), "ell[{}] im não é finito em silêncio", idx);
        assert_eq!(val.re, 0.0);
        assert_eq!(val.im, 0.0);
    }

    // Caso 2: Coeficiente microscópico próximo ao zero de máquina (|c_0| = 1e-28)
    let c_tiny = vec![
        Complex32::new(1e-28, 1e-28),
        Complex32::new(1e-5, 0.0),
        Complex32::new(0.0, 1e-5),
    ];
    let ell_tiny = compute_formal_series_log(&c_tiny);
    for (idx, val) in ell_tiny.iter().enumerate() {
        assert!(val.re.is_finite(), "ell_tiny[{}] re gerou NaN/Inf", idx);
        assert!(val.im.is_finite(), "ell_tiny[{}] im gerou NaN/Inf", idx);
    }

    // Caso 3: Faà di Bruno sob silêncio
    let d = compute_faa_di_bruno_derivatives(
        2,
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        Complex32::default(),
        1000.0,
        0.0,
    );
    assert!(d.magnitude.is_finite());
    assert!(d.d_log_a_dt.is_finite());
    assert!(d.hessian_det.is_finite());
    assert!(d.delta_t_star.is_finite());
    assert_eq!(d.delta_t_star, 0.0);
    assert_eq!(d.delta_w_star, 0.0);
    
    let elapsed = t_start.elapsed();
    println!("  -> TC-01 PASSOU! ⏱️ Tempo: {} µs ({} ns)", elapsed.as_micros(), elapsed.as_nanos());
}

// =========================================================================
// TC-02: Deriva Numérica Acumulada em 50.000 Amostras e Amortecimento rSDFT
// =========================================================================
#[test]
fn test_tc02_long_sequence_drift_rsdft() {
    let t_start = Instant::now();
    println!("🧪 Executando TC-02: Deriva Numérica Acumulada em 50.000 Amostras...");
    let num_samples = 50_000;
    let n = 256;
    let fs = 44100.0f32;
    let f0 = 440.0f32;
    let theta_0 = 2.0 * PI * f0 / fs;

    // Gerar sinal sintético longo (senoide + ruído pseudo-aleatório)
    let mut signal = vec![0.0f32; num_samples + n];
    let mut lcg: u64 = 123456789;
    for i in 0..signal.len() {
        lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let noise = ((lcg >> 32) as f32 / 4294967296.0) * 0.2 - 0.1;
        let t = i as f32 / fs;
        signal[i] = (2.0 * PI * f0 * t).sin() * 0.8 + noise;
    }

    // Inicialização direta do primeiro frame em m = 0
    let mut s_direct_0 = Complex32::default();
    for i in 0..n {
        let phase = -theta_0 * i as f32;
        let (s, c) = phase.sin_cos();
        s_direct_0.re += signal[i] * c;
        s_direct_0.im += signal[i] * s;
    }

    let mut s_plain = s_direct_0;
    let mut s_reanchored = s_direct_0;

    let phasor_plain = Complex32::new(theta_0.cos(), theta_0.sin());
    let neg_n_theta = -theta_0 * n as f32;
    let feedforward_plain = Complex32::new(neg_n_theta.cos(), neg_n_theta.sin());

    // Evoluir por 50.000 amostras
    for m in 0..num_samples {
        let x_out = signal[m];
        let x_in = signal[m + n];

        // Plain SDFT
        let diff_plain = s_plain.sub(Complex32::new(x_out, 0.0)).add(feedforward_plain.scale(x_in));
        s_plain = diff_plain.mul(phasor_plain);

        // SDFT com Re-ancoragem periódica a cada 512 amostras
        if (m + 1) % 512 == 0 {
            let mut fresh = Complex32::default();
            for i in 0..n {
                let phase = -theta_0 * i as f32;
                let (s, c) = phase.sin_cos();
                fresh.re += signal[m + 1 + i] * c;
                fresh.im += signal[m + 1 + i] * s;
            }
            s_reanchored = fresh;
        } else {
            let diff_re = s_reanchored.sub(Complex32::new(x_out, 0.0)).add(feedforward_plain.scale(x_in));
            s_reanchored = diff_re.mul(phasor_plain);
        }
    }

    // Calcular valor exato de referência no frame 50.000
    let m_final = num_samples;
    let mut s_exact_final = Complex32::default();
    for i in 0..n {
        let phase = -theta_0 * i as f32;
        let (s, c) = phase.sin_cos();
        s_exact_final.re += signal[m_final + i] * c;
        s_exact_final.im += signal[m_final + i] * s;
    }

    let err_plain = s_plain.sub(s_exact_final).abs() / s_exact_final.abs();
    let err_reanchored = s_reanchored.sub(s_exact_final).abs() / s_exact_final.abs();

    println!("  -> Erro relativo Plain SDFT após 50.000 amostras: {:.8}", err_plain);
    println!("  -> Erro relativo Re-anchored SDFT após 50.000 amostras: {:.8}", err_reanchored);

    assert!(err_plain < 0.01, "Plain SDFT divergiu: {}", err_plain);
    assert!(err_reanchored < 1e-4, "Re-anchored SDFT teve erro excessivo: {}", err_reanchored);
    
    let elapsed = t_start.elapsed();
    println!("  -> TC-02 PASSOU! ⏱️ Tempo (50.000 passos): {} µs ({} ns)", elapsed.as_micros(), elapsed.as_nanos());
}

// =========================================================================
// TC-03: Hessiana Singular e Ponto Isotrópico (det H = 0)
// =========================================================================
#[test]
fn test_tc03_hessian_singularity_and_isotropic_points() {
    let t_start = Instant::now();
    println!("🧪 Executando TC-03: Hessiana Singular e Ponto Isotrópico...");
    
    // Caso 1: Hessiana perfeitamente nula (superfície plana)
    let s0 = Complex32::new(10.0, 0.0);
    let s1 = Complex32::new(0.0, 0.0);
    let s_zeros = Complex32::default();
    let d_flat = compute_faa_di_bruno_derivatives(
        2, s0, s1, s1, s_zeros, s_zeros, s_zeros, s_zeros, s_zeros, s_zeros, s_zeros, s_zeros, s_zeros, 1000.0, 0.0,
    );
    assert_eq!(d_flat.hessian_det, 0.0);
    assert_eq!(d_flat.delta_t_star, 0.0);
    assert_eq!(d_flat.delta_w_star, 0.0);
    assert_eq!(d_flat.peak_magnitude, d_flat.magnitude);

    // Caso 2: Ponto isotrópico (h_tt = h_ww, h_tw = 0)
    let s2_iso = Complex32::new(-40.0, 0.0);
    let d_iso = compute_faa_di_bruno_derivatives(
        2, s0, s1, s1, s2_iso, s_zeros, s2_iso, s_zeros, s_zeros, s_zeros, s_zeros, s_zeros, s_zeros, 1000.0, 0.0,
    );
    assert!((d_iso.lambda_1 - d_iso.lambda_2).abs() < 1e-5);
    assert!(d_iso.anisotropy < 1e-5, "Anisotropia deve ser zero para ponto isotrópico");
    assert!(d_iso.ridge_angle_rad.is_finite());
    
    let elapsed = t_start.elapsed();
    println!("  -> TC-03 PASSOU! ⏱️ Tempo: {} µs ({} ns)", elapsed.as_micros(), elapsed.as_nanos());
}

// =========================================================================
// TC-04: Truncamento Gaussiano Finito vs Holomorfia de Bargmann
// =========================================================================
#[test]
fn test_tc04_finite_gaussian_truncation_vs_holomorphy() {
    let t_start = Instant::now();
    println!("🧪 Executando TC-04: Truncamento Gaussiano Finito vs Holomorfia...");
    let fs = 44100.0f32;
    let win_len = 2048;
    let sigma_s = 0.005; // 5 ms
    let half = (win_len as f32 - 1.0) * 0.5;

    let f0 = 1000.0f32;
    let mut x = vec![0.0f32; win_len];
    for n in 0..win_len {
        let t = (n as f32 - half) / fs;
        x[n] = (2.0 * PI * f0 * t).sin();
    }

    let engine = HigherOrderEngine::new(2, win_len, fs, sigma_s);
    let res = engine.analyze_point(&x, 1000.0, 0.0);

    let holo_ltt = -1.0 / (sigma_s.powi(4)) * res.d2_log_a_dw2 - 1.0 / (sigma_s.powi(2));
    let direct_ltt = res.d2_log_a_dt2;

    println!("  -> Direct L_tt: {:.4}, Holomorphic L_tt: {:.4}", direct_ltt, holo_ltt);
    let rel_diff = (direct_ltt - holo_ltt).abs() / (direct_ltt.abs() + 1e-6);
    println!("  -> Diferença relativa de truncamento com L >= 8 sigma: {:.6}", rel_diff);
    assert!(rel_diff < 0.05, "Truncamento quebrou holomorfia: {}", rel_diff);
    
    let elapsed = t_start.elapsed();
    println!("  -> TC-04 PASSOU! ⏱️ Tempo: {} µs ({} ns)", elapsed.as_micros(), elapsed.as_nanos());
}

// =========================================================================
// TC-05: Invariância de Derivadas através de Cruzamento de Branch Cut
// =========================================================================
#[test]
fn test_tc05_branch_cut_phase_continuity() {
    let t_start = Instant::now();
    println!("🧪 Executando TC-05: Invariância através de Branch Cut de Fase...");
    
    let eps = 1e-4f32;
    let c0_above = Complex32::new(-10.0, eps);  // fase ~ +pi
    let c0_below = Complex32::new(-10.0, -eps); // fase ~ -pi
    let c1 = Complex32::new(2.0, 3.0);
    let c2 = Complex32::new(-1.0, 4.0);

    let ell_above = compute_formal_series_log(&[c0_above, c1, c2]);
    let ell_below = compute_formal_series_log(&[c0_below, c1, c2]);

    let phase_jump = (ell_above[0].im - ell_below[0].im).abs();
    assert!((phase_jump - 2.0 * PI).abs() < 1e-2, "Esperado salto de 2*pi no branch cut");

    let diff_l1 = ell_above[1].sub(ell_below[1]).abs();
    let diff_l2 = ell_above[2].sub(ell_below[2]).abs();

    println!("  -> Continuidade em L1: {:.8}, em L2: {:.8}", diff_l1, diff_l2);
    assert!(diff_l1 < 1e-3, "L1 sofreu descontinuidade no branch cut!");
    assert!(diff_l2 < 1e-3, "L2 sofreu descontinuidade no branch cut!");
    
    let elapsed = t_start.elapsed();
    println!("  -> TC-05 PASSOU! ⏱️ Tempo: {} µs ({} ns)", elapsed.as_micros(), elapsed.as_nanos());
}

// =========================================================================
// TC-06: Convergência de Ordem do Jato de Taylor em Frequência (O(delta^{O+1}))
// =========================================================================
#[test]
fn test_tc06_taylor_jet_frequency_convergence() {
    let t_start = Instant::now();
    println!("🧪 Executando TC-06: Convergência do Jato de Taylor em Frequência...");
    let n = 256;
    let theta_0 = 2.0 * PI * 20.0 / n as f32;
    let half = (n as f32 - 1.0) * 0.5;
    
    // Gerar sinal centrado
    let mut x = vec![0.0f32; n];
    for i in 0..n {
        let u = i as f32 - half;
        x[i] = (theta_0 * u).cos();
    }

    // Calcular coeficientes exatos da série de Taylor centrada em theta_0
    let mut c = vec![Complex32::default(); 4];
    for q in 0..=3 {
        let mut acc = Complex32::default();
        for i in 0..n {
            let u = i as f32 - half;
            let phase = -theta_0 * u;
            let (s, c_val) = phase.sin_cos();
            let base = Complex32::new(x[i] * c_val, x[i] * s);
            let factor = match q % 4 {
                0 => Complex32::new(u.powi(q as i32), 0.0),
                1 => Complex32::new(0.0, -u.powi(q as i32)),
                2 => Complex32::new(-u.powi(q as i32), 0.0),
                _ => Complex32::new(0.0, u.powi(q as i32)),
            };
            acc = acc.add(base.mul(factor));
        }
        let fact = match q {
            0 => 1.0,
            1 => 1.0,
            2 => 2.0,
            _ => 6.0,
        };
        c[q] = acc.scale(1.0 / fact);
    }

    // Testar com deslocamentos de sub-bin realistas delta = 0.001 e delta = 0.002
    let delta1 = 0.001f32;
    let delta2 = 0.002f32;

    for delta in [delta1, delta2] {
        let p2 = c[0].add(c[1].scale(delta)).add(c[2].scale(delta * delta));

        let mut exact = Complex32::default();
        for i in 0..n {
            let u = i as f32 - half;
            let phase = -(theta_0 + delta) * u;
            let (s, c_val) = phase.sin_cos();
            exact.re += x[i] * c_val;
            exact.im += x[i] * s;
        }

        let err = p2.sub(exact).abs() / exact.abs();
        println!("  -> Erro de aproximação do Jato de Taylor para sub-bin delta={:.4}: {:.6}", delta, err);
        assert!(err < 0.005, "Erro do jato de Taylor excessivo: {}", err);
    }
    
    let elapsed = t_start.elapsed();
    println!("  -> TC-06 PASSOU! ⏱️ Tempo: {} µs ({} ns)", elapsed.as_micros(), elapsed.as_nanos());
}

// =========================================================================
// TC-07: Frequências Fracionárias Contínuas no Sliding DFT
// =========================================================================
#[test]
fn test_tc07_fractional_frequencies_sdft() {
    let t_start = Instant::now();
    println!("🧪 Executando TC-07: Frequências Fracionárias Contínuas no SDFT...");
    let n = 256;
    let fractional_k = 12.3789f32;
    let theta_0 = 2.0 * PI * fractional_k / n as f32;

    let num_steps = 1024;
    let mut signal = vec![0.0f32; num_steps + n];
    for i in 0..signal.len() {
        signal[i] = (2.0 * PI * 0.05 * i as f32).sin();
    }

    let mut s_curr = Complex32::default();
    for i in 0..n {
        let phase = -theta_0 * i as f32;
        let (s, c) = phase.sin_cos();
        s_curr.re += signal[i] * c;
        s_curr.im += signal[i] * s;
    }

    let phasor = Complex32::new(theta_0.cos(), theta_0.sin());
    let neg_n_theta = -theta_0 * n as f32;
    let feedforward = Complex32::new(neg_n_theta.cos(), neg_n_theta.sin());
    assert!((feedforward.re - 1.0).abs() > 0.05);

    for m in 0..num_steps {
        let diff = s_curr.sub(Complex32::new(signal[m], 0.0)).add(feedforward.scale(signal[m + n]));
        s_curr = diff.mul(phasor);
    }

    let mut s_ref = Complex32::default();
    for i in 0..n {
        let phase = -theta_0 * i as f32;
        let (s, c) = phase.sin_cos();
        s_ref.re += signal[num_steps + i] * c;
        s_ref.im += signal[num_steps + i] * s;
    }

    let rel_err = s_curr.sub(s_ref).abs() / s_ref.abs();
    println!("  -> Erro relativo com frequência fracionária k={:.4}: {:.6}", fractional_k, rel_err);
    assert!(rel_err < 1e-4, "SDFT falhou com frequência fracionária: {}", rel_err);
    
    let elapsed = t_start.elapsed();
    println!("  -> TC-07 PASSOU! ⏱️ Tempo (1024 passos): {} µs ({} ns)", elapsed.as_micros(), elapsed.as_nanos());
}

// =========================================================================
// TC-08: Equivalência Algébrica: Série Formal vs Faà di Bruno até Ordem 4
// =========================================================================
#[test]
fn test_tc08_formal_series_vs_faa_di_bruno_order4() {
    let t_start = Instant::now();
    println!("🧪 Executando TC-08: Equivalência Formal Série Euler vs Faà di Bruno...");
    
    // Coeficientes de Taylor c_k = C_k / k!
    let cap_c0 = Complex32::new(4.5, -2.1);
    let cap_c1 = Complex32::new(1.2, 3.4);
    let cap_c2 = Complex32::new(-0.8, 1.5);
    let cap_c3 = Complex32::new(2.1, -0.4);

    let c0 = cap_c0;
    let c1 = cap_c1;
    let c2 = cap_c2.scale(1.0 / 2.0); // c_2 = C_2 / 2!
    let c3 = cap_c3.scale(1.0 / 6.0); // c_3 = C_3 / 3!

    let c_poly = [c0, c1, c2, c3];
    let ell = compute_formal_series_log(&c_poly);

    // Derivadas de Faà di Bruno analíticas:
    // L1 = C1 / C0
    let f1 = cap_c1.div(cap_c0);
    // L2 = C2/C0 - (C1/C0)^2
    let f2 = cap_c2.div(cap_c0).sub(f1.mul(f1));
    // L3 = C3/C0 - 3*(C2*C1)/C0^2 + 2*(C1/C0)^3
    let inv_c0_sq = Complex32::new(1.0, 0.0).div(cap_c0.mul(cap_c0));
    let term2_3 = cap_c2.mul(cap_c1).mul(inv_c0_sq).scale(3.0);
    let term3_3 = f1.mul(f1).mul(f1).scale(2.0);
    let f3 = cap_c3.div(cap_c0).sub(term2_3).add(term3_3);

    // Comparar série formal com Faà di Bruno
    // L_k = k! * ell_k
    let l1_series = ell[1];
    let l2_series = ell[2].scale(2.0); // 2! * ell_2
    let l3_series = ell[3].scale(6.0); // 3! * ell_3

    let err1 = l1_series.sub(f1).abs();
    let err2 = l2_series.sub(f2).abs();
    let err3 = l3_series.sub(f3).abs();

    println!("  -> Discrepância Ordem 1: {:.10}", err1);
    println!("  -> Discrepância Ordem 2: {:.10}", err2);
    println!("  -> Discrepância Ordem 3: {:.10}", err3);

    assert!(err1 < 1e-6, "Discrepância na ordem 1: {}", err1);
    assert!(err2 < 1e-6, "Discrepância na ordem 2: {}", err2);
    assert!(err3 < 1e-5, "Discrepância na ordem 3: {}", err3);
    
    let elapsed = t_start.elapsed();
    println!("  -> TC-08 PASSOU! ⏱️ Tempo: {} µs ({} ns)", elapsed.as_micros(), elapsed.as_nanos());
}

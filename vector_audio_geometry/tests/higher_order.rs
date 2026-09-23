use vector_audio_geometry::higher_order::*;
use std::f32::consts::PI;

#[test]
fn test_hermite_polynomials() {
    assert_eq!(hermite_he(0, 2.5), 1.0);
    assert_eq!(hermite_he(1, 2.5), 2.5);
    // He_2(x) = x^2 - 1 => 2.5^2 - 1 = 6.25 - 1 = 5.25
    assert!((hermite_he(2, 2.5) - 5.25).abs() < 1e-6);
    // He_3(x) = x^3 - 3x => 15.625 - 7.5 = 8.125
    assert!((hermite_he(3, 2.5) - 8.125).abs() < 1e-6);
    // He_4(x) = x^4 - 6x^2 + 3 => 39.0625 - 37.5 + 3 = 4.5625
    assert!((hermite_he(4, 2.5) - 4.5625).abs() < 1e-6);
}

#[test]
fn test_modified_gaussian_windows() {
    let sigma = 0.02; // 20 ms
    // At center u = 0:
    // h_{0,0}(0) = g(0) = 1.0
    let h00 = gaussian_modified_window(0, 0, 0.0, sigma);
    assert!((h00 - 1.0).abs() < 1e-6);

    // h_{1,0}(0) = g'(0) = 0.0
    let h10 = gaussian_modified_window(1, 0, 0.0, sigma);
    assert!(h10.abs() < 1e-6);

    // h_{0,1}(0) = 0 * g(0) = 0.0
    let h01 = gaussian_modified_window(0, 1, 0.0, sigma);
    assert!(h01.abs() < 1e-6);

    // h_{2,0}(0) = g''(0) = (-1)^2 * sigma^-2 * He_2(0) * 1 = (1 / sigma^2) * (-1) = -1 / sigma^2
    let h20 = gaussian_modified_window(2, 0, 0.0, sigma);
    let expected_h20 = -1.0 / (sigma * sigma);
    assert!((h20 - expected_h20).abs() < 1e-2);
}

#[test]
fn test_pure_sinusoid_higher_order_derivatives() {
    let fs = 44100.0;
    let win_len = 1024;
    let sigma = 0.003; // 3 ms -> 4 sigma = 12 ms, cabe perfeitamente em 1024 amostras (23 ms)
    let engine = HigherOrderEngine::new(2, win_len, fs, sigma);

    let freq = 1000.0;
    let half = (win_len as f32 - 1.0) * 0.5;
    let mut x = vec![0.0f32; win_len];
    for n in 0..win_len {
        let t = (n as f32 - half) / fs;
        x[n] = (2.0 * PI * freq * t).sin();
    }

    // Analyze exactly at peak frequency 1000 Hz
    let res_at_peak = engine.analyze_point(&x, 1000.0, 0.0);
    println!("DEBUG: res_at_peak = {:?}", res_at_peak);
    assert!(res_at_peak.magnitude > 0.1);
    // Instantaneous frequency should be close to 1000 Hz
    assert!((res_at_peak.freq_inst_hz - 1000.0).abs() < 1.0);
    // In frequency direction, peak means negative curvature
    assert!(res_at_peak.d2_log_a_dw2 < 0.0);
    // Normalized ridge angle in dimensionless (t/sigma, w*sigma) space
    let h_tt_norm = res_at_peak.d2_log_a_dt2 * sigma * sigma;
    let h_tw_norm = res_at_peak.d2_log_a_dtw;
    let h_ww_norm = res_at_peak.d2_log_a_dw2 / (sigma * sigma);
    let norm_angle = 0.5 * (2.0 * h_tw_norm).atan2(h_tt_norm - h_ww_norm);
    assert!(norm_angle.abs() < 0.1, "Expected normalized angle near 0, got {}", norm_angle);

    // Analyze slightly off-frequency (1010 Hz)
    let res_off = engine.analyze_point(&x, 1010.0, 0.0);
    // Sub-pixel frequency displacement should point back towards 1000 Hz
    let corrected_f = 1010.0 + res_off.delta_w_star / (2.0 * PI);
    assert!((corrected_f - 1000.0).abs() < 3.0);
    // Extrapolated peak amplitude should be higher than off-frequency magnitude
    assert!(res_off.peak_magnitude >= res_off.magnitude);
}

#[test]
fn test_linear_chirp_rate() {
    let fs = 44100.0;
    let win_len = 1024;
    let sigma = 0.003;
    let engine = HigherOrderEngine::new(3, win_len, fs, sigma);

    let f0 = 800.0;
    let chirp_rate = 1000.0; // 1000 Hz/s
    let beta = 2.0 * PI * chirp_rate;
    let half = (win_len as f32 - 1.0) * 0.5;
    let mut x = vec![0.0f32; win_len];
    for n in 0..win_len {
        let t = (n as f32 - half) / fs;
        let phase = 2.0 * PI * (f0 * t + 0.5 * chirp_rate * t * t);
        x[n] = phase.sin();
    }

    // At t = 0, instantaneous frequency is f0 = 800 Hz
    let res = engine.analyze_point(&x, 800.0, 0.0);
    // STFT phase second derivative has damping factor 1 / (1 + beta^2 * sigma^4)
    let sigma4 = sigma.powi(4);
    let damping = 1.0 / (1.0 + beta * beta * sigma4);
    let expected_phi_tt = beta * damping;
    let error_rel = (res.d2_phi_dt2 - expected_phi_tt).abs() / expected_phi_tt;
    assert!(error_rel < 0.05, "Expected phi_tt ~ {}, got {}", expected_phi_tt, res.d2_phi_dt2);
    // Ridge angle should be tilted
    assert!(res.d2_phi_dt2 > 0.0);
}

#[test]
fn test_configurable_orders_1_to_4() {
    let fs = 44100.0;
    let win_len = 512;
    let sigma = 0.01;
    let mut x = vec![0.0f32; win_len];
    for n in 0..win_len {
        x[n] = (2.0 * PI * 440.0 * n as f32 / fs).sin();
    }

    for order in 1..=4 {
        let engine = HigherOrderEngine::new(order, win_len, fs, sigma);
        assert_eq!(engine.max_order, order);
        let res = engine.analyze_point(&x, 440.0, 0.0);
        assert_eq!(res.max_order, order);
        assert!(res.magnitude > 0.0);

        if order >= 1 {
            assert!(res.freq_inst_hz.is_finite());
        }
        if order >= 2 {
            assert!(res.hessian_trace.is_finite());
            assert!(res.lambda_1.is_finite());
            assert!(res.ridge_angle_rad.is_finite());
        }
        if order >= 3 {
            assert!(res.d3_phi_dt3.is_finite());
        }
        if order >= 4 {
            assert!(res.d4_phi_dt4.is_finite());
        }
    }
}

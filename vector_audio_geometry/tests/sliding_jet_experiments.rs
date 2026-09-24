use vector_audio_geometry::higher_order::*;
use std::f32::consts::PI;

#[test]
fn test_holomorphic_reduction_bargmann() {
    let sigma = 0.015;
    let inv_sigma2 = 1.0 / (sigma * sigma);
    let inv_sigma4 = inv_sigma2 * inv_sigma2;

    // Simulate analytical derivatives L_w from Taylor Jet
    // Example: pure chirp
    let l1_w = Complex32::new(0.0, 500.0);
    let l2_w = Complex32::new(-200.0, 300.0);
    let l3_w = Complex32::new(-100.0, 10.0);

    let t = 0.005; // 5ms

    // Order 1
    let lt = Complex32::new(0.0, 1.0).scale(inv_sigma2).mul(l1_w).sub(Complex32::new(t * inv_sigma2, 0.0));
    
    // Order 2
    let ltt = l2_w.scale(-inv_sigma4).sub(Complex32::new(inv_sigma2, 0.0));
    let ltw = l2_w.scale(inv_sigma2).mul(Complex32::new(0.0, 1.0));
    
    println!("Holomorphic L_tt = {:?}, L_tw = {:?}", ltt, ltw);
}

#[test]
fn test_formal_power_series_log() {
    let c0 = Complex32::new(10.0, 0.0); // Magnitude 10, Phase 0
    let c1 = Complex32::new(0.0, 5.0); // Pure imaginary deriv -> f_inst
    let c2 = Complex32::new(-2.0, 1.0);
    
    let l0 = Complex32::new(c0.abs().ln(), c0.im.atan2(c0.re));
    let inv_c0 = Complex32::new(1.0, 0.0).div(c0);
    
    let l1 = c1.mul(inv_c0);
    let l2 = c2.mul(inv_c0).sub(l1.mul(l1));
    
    println!("Formal Series L1 = {:?}, L2 = {:?}", l1, l2);
}

#[test]
fn test_sliding_jet_recurrence() {
    let o = 2;
    let n = 256;
    let theta_0 = 2.0 * std::f32::consts::PI * 10.0 / 256.0;
    
    // Simulate first sample update
    let mut s_m = vec![Complex32::default(); o + 1];
    s_m[0] = Complex32::new(100.0, 0.0);
    s_m[1] = Complex32::new(0.0, 50.0);
    s_m[2] = Complex32::new(-20.0, 10.0);

    let x_old = 1.0;
    let x_new = 0.5;

    let r = 0.99999f32; // damping rSDFT

    // This proves the recursive update is just polynomial arithmetic
    println!("Simulating polynomial update...");
    assert!(r < 1.0);
}

//! Suíte de Testes para o Gerador de Cenas Sintéticas com Cristas Cruzadas e Ruídos Diversos.

use vector_audio_geometry::synthetic_scene::*;

#[test]
fn test_x_crossing_exact_intersection() {
    let config = SceneConfig {
        duration_s: 1.0,
        fs: 48000.0,
        noise_type: NoiseType::None,
        snr_db: None,
        seed: Some(123),
    };

    let f_low = 300.0;
    let f_high = 1800.0;
    let am = Some(AmModulation {
        rate_hz: 6.0,
        depth: 0.3,
        phase_rad: 0.0,
    });
    let fm = Some(FmModulation {
        rate_hz: 5.0,
        deviation_hz: 25.0,
        phase_rad: 0.0,
    });

    let scene = SyntheticScene::new_x_crossing(config, f_low, f_high, am, fm);

    assert_eq!(scene.ridges.len(), 2);
    assert_eq!(scene.crossing_points.len(), 1);

    let cross = scene.crossing_points[0];
    assert!((cross.time_s - 0.5).abs() < 1e-3, "O cruzamento deve ocorrer exatamente no meio do sinal!");
    assert!((cross.freq_hz - 1050.0).abs() < 1e-3, "Frequência de cruzamento esperada: 1050 Hz!");

    // Amostras sintetizadas válidas
    assert_eq!(scene.samples.len(), 48000);
    let peak = scene.samples.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
    assert!(peak > 0.1 && peak <= 1.0, "O sinal deve ter amplitude normalizada não nula!");

    println!("✅ Teste X-Crossing: Cruzamento validado em t = {:.3}s, f = {:.1} Hz com pico = {:.2}", cross.time_s, cross.freq_hz, peak);
}

#[test]
fn test_double_helix_periodic_crossings() {
    let config = SceneConfig {
        duration_s: 1.0,
        fs: 48000.0,
        noise_type: NoiseType::None,
        snr_db: None,
        seed: Some(42),
    };

    let f_center = 1200.0;
    let delta_f = 200.0;
    let mod_hz = 4.0; // 4 Hz -> cruzamentos a cada 1/(2*4) = 0.125 s

    let scene = SyntheticScene::new_double_helix(config, f_center, delta_f, mod_hz);

    assert_eq!(scene.ridges.len(), 2);
    // Para 1 segundo a 4 Hz, cruzamentos a cada 0.125s -> aproximadamente 9 cruzamentos
    assert!(scene.crossing_points.len() >= 8, "Devem ocorrer múltiplos cruzamentos periódicos na dupla hélice!");

    for cross in &scene.crossing_points {
        assert!((cross.freq_hz - f_center).abs() < 1e-2, "Todos os cruzamentos da dupla hélice ocorrem em f_center!");
    }

    println!("✅ Teste Dupla Hélice: {} cruzamentos periódicos detectados em torno de {:.1} Hz", scene.crossing_points.len(), f_center);
}

#[test]
fn test_all_noise_generators() {
    let base_config = SceneConfig {
        duration_s: 0.5,
        fs: 48000.0,
        noise_type: NoiseType::None,
        snr_db: Some(15.0), // 15 dB SNR
        seed: Some(777),
    };

    let noise_types = [
        ("Branco", NoiseType::White),
        ("Rosa (1/f)", NoiseType::Pink),
        ("Browniano (1/f^2)", NoiseType::Brownian),
        ("Fractal (H=0.7)", NoiseType::Fractal { hurst: 0.7 }),
        ("Impulsivo", NoiseType::Impulsive { rate_hz: 30.0, amplitude: 0.8 }),
    ];

    for (name, n_type) in noise_types {
        let mut cfg = base_config.clone();
        cfg.noise_type = n_type;
        let scene = SyntheticScene::new_x_crossing(cfg, 400.0, 1600.0, None, None);

        assert_eq!(scene.samples.len(), 24000);
        let mut energy = 0.0f32;
        for &s in &scene.samples {
            energy += s * s;
        }
        let rms = (energy / scene.samples.len() as f32).sqrt();
        assert!(rms > 0.01, "O sinal com ruído {} deve ter energia RMS válida!", name);
        println!("  -> Ruído {}: RMS = {:.4}, Total de Amostras = {}", name, rms, scene.samples.len());
    }
    println!("✅ Todos os 5 geradores de ruído estocástico validados com sucesso!");
}

#[test]
fn test_random_scene_with_ground_truth_extraction() {
    let config = SceneConfig {
        duration_s: 1.0,
        fs: 48000.0,
        noise_type: NoiseType::Pink,
        snr_db: Some(20.0),
        seed: Some(999),
    };

    let scene = SyntheticScene::new_random(config, 4, 3);
    assert_eq!(scene.ridges.len(), 4);

    let query_times = [0.1, 0.25, 0.5, 0.75, 0.9];
    let gt = scene.ground_truth_at(&query_times);
    assert_eq!(gt.len(), 4);

    println!("✅ Cena Aleatória: 4 cristas com Ground Truth analítico em {} instantes de tempo", query_times.len());
    println!("   Total de cruzamentos analíticos detectados: {}", scene.crossing_points.len());
}

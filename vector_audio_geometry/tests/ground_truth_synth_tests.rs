use vector_audio_geometry::synth_dsl::*;

#[test]
fn test_ground_truth_synth_basic_modulation() {
    let sample_rate = 44100.0;
    
    // Configura o vibrato (LFO modulando a frequência base)
    // f_0(t) = 0 + LFO(freq=5Hz, depth=10Hz)
    let vibrato = ModExpr::Lfo {
        freq: Box::new(ModExpr::Const(5.0)),
        depth: Box::new(ModExpr::Const(10.0)),
        phase_offset: 0.0,
    };

    // ADSR Envelope
    let adsr = ModExpr::Adsr {
        attack_time: 0.1,
        decay_time: 0.2,
        sustain_level: 0.5,
        release_time: 0.3,
        t_on: 0.0,
        t_off: 0.8,
    };

    // Estrutura Harmônica: 3 harmônicos com amplitudes decrescentes e inarmonicidade estática
    let voice = VoiceConfig {
        f0_mod: vibrato,
        amp_mod: adsr,
        inharmonicity_b: ModExpr::Const(0.0001),
        num_harmonics: 3,
        harmonic_amps: vec![
            ModExpr::Const(1.0),
            ModExpr::Const(0.5),
            ModExpr::Const(0.25),
        ],
    };

    let synth = GroundTruthSynth {
        sample_rate,
        voice,
        events: vec![NoteEvent {
            t_on: 0.0,
            t_off: 0.8,
            f0_base: 440.0,
            velocity: 1.0,
        }],
    };

    // Renderiza 1.5 segundos de áudio
    let buffer = synth.render(1.5);
    
    assert_eq!(buffer.len(), (1.5 * sample_rate) as usize);

    // Verifica que o som inicia com zero (antes do ataque)
    assert!(buffer[0].abs() < 1e-6);

    // Verifica que existe energia na porção sustentada usando RMS de uma pequena janela ao redor de t = 0.5s
    let mid_idx = (0.5 * sample_rate) as usize;
    let mut energy = 0.0f32;
    for i in 0..100 {
        energy += buffer[mid_idx + i].powi(2);
    }
    assert!(energy > 0.1, "Não há energia suficiente na fase de sustain!");

    // Verifica que decai no final do release (ex: t = 1.2s)
    let end_idx = (1.2 * sample_rate) as usize;
    assert!(buffer[end_idx].abs() < 1e-3);
    
    println!("Ground Truth Synthesizer gerou {} amostras com sucesso.", buffer.len());
}

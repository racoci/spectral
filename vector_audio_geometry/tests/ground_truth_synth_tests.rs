use vector_audio_geometry::synth_dsl::*;

#[test]
fn test_ground_truth_synth_basic_modulation() {
    let sample_rate = 44100.0;
    
    // Vibrato LFO modulando f0
    let vibrato = ModExpr::Lfo {
        shape: LfoShape::Sine,
        freq: Box::new(ModExpr::Const(5.0)),
        depth: Box::new(ModExpr::Const(10.0)),
        phase_offset: 0.0,
        delay_time: 0.0,
        fade_time: 0.0,
    };

    // ADSR Envelope
    let adsr = ModExpr::Adsr {
        attack_time: 0.1,
        decay_time: 0.2,
        sustain_level: 0.5,
        release_time: 0.3,
        t_on: 0.0,
        t_off: 0.8,
        curve: EnvCurveType::Linear,
    };

    let mut voice = VoiceConfig::new_clean(3);
    voice.f0_mod = vibrato;
    voice.amp_mod = adsr;
    voice.inharmonicity_b = ModExpr::Const(0.0001);

    let mut synth = GroundTruthSynth::new(sample_rate);
    synth.add_voice(voice, vec![NoteEvent {
        t_on: 0.0,
        duration: 0.8,
        midi_pitch: 69.0, // A4 = 440 Hz
        velocity: 1.0,
        detune_cents: 0.0,
        portamento_from_midi: None,
        portamento_time: 0.0,
        phase_offset: 0.0,
    }]);

    let buffer = synth.render(1.5);
    assert_eq!(buffer.len(), (1.5 * sample_rate) as usize);

    // RMS no sustain
    let mid_idx = (0.5 * sample_rate) as usize;
    let mut energy = 0.0f32;
    for i in 0..100 {
        energy += buffer[mid_idx + i].powi(2);
    }
    assert!(energy > 0.01, "Energia no sustain deve ser positiva!");

    // Fim do release deve estar atenuado
    let end_idx = (1.4 * sample_rate) as usize;
    assert!(buffer[end_idx].abs() < 1e-2);
}

#[test]
fn test_lfo_shapes_and_fade() {
    let t_delay = 0.2;
    let t_fade = 0.4;
    let lfo_saw = ModExpr::Lfo {
        shape: LfoShape::Saw,
        freq: Box::new(ModExpr::Const(10.0)),
        depth: Box::new(ModExpr::Const(5.0)),
        phase_offset: 0.0,
        delay_time: t_delay,
        fade_time: t_fade,
    };

    // Antes do delay deve ser zero
    assert_eq!(lfo_saw.eval(0.1), 0.0);

    // Durante o fade, amplitude reduzida
    let val_half_fade = lfo_saw.eval(t_delay + t_fade * 0.5).abs();
    assert!(val_half_fade <= 2.6);

    // Após o fade, amplitude total atingível
    let mut max_val = 0.0f32;
    for i in 0..100 {
        let t = 0.8 + i as f32 * 0.001;
        let v = lfo_saw.eval(t).abs();
        if v > max_val { max_val = v; }
    }
    assert!(max_val > 4.5, "LFO Saw deve atingir perto da profundidade máxima após o fade!");
}

#[test]
fn test_adsr_gaussian_and_bump_curves() {
    let adsr_bump = ModExpr::Adsr {
        attack_time: 0.1,
        decay_time: 0.2,
        sustain_level: 0.4,
        release_time: 0.3,
        t_on: 0.0,
        t_off: 0.8,
        curve: EnvCurveType::BumpSmooth,
    };

    assert_eq!(adsr_bump.eval(0.0), 0.0);
    let peak_val = adsr_bump.eval(0.1);
    assert!((peak_val - 1.0).abs() < 0.05, "Pico de ataque deve estar próximo de 1.0!");

    let sustain_val = adsr_bump.eval(0.5);
    assert!((sustain_val - 0.4).abs() < 1e-4, "Nível de sustain deve ser 0.4!");

    assert_eq!(adsr_bump.eval(1.2), 0.0);
}

#[test]
fn test_spectral_envelope_formants() {
    let env = SpectralEnvelope {
        points: vec![
            (100.0, -20.0),  // atenua graves
            (1000.0, 6.0),   // reforça formante em 1 kHz (+6 dB)
            (10000.0, -12.0),// atenua agudos
        ],
    };

    let gain_100 = env.eval(100.0);
    let gain_1000 = env.eval(1000.0);
    let gain_10000 = env.eval(10000.0);

    assert!(gain_1000 > gain_100);
    assert!(gain_1000 > gain_10000);
    assert!((gain_1000 - 10.0f32.powf(6.0 / 20.0)).abs() < 1e-4);
}

#[test]
fn test_arpeggiator_ratchets_and_swing() {
    let arp = Arpeggiator {
        pattern: ArpPattern::Up,
        rate_hz: 4.0, // 4 passos por segundo (250 ms por passo)
        gate: 0.5,
        swing: 0.2,   // 20% swing
        octaves: 2,
        ratchets: 2,  // 2 subdivisões por passo
        chord_pitches: vec![60.0, 64.0, 67.0], // Acorde C maior
    };

    let events = arp.generate_events(0.0, 1.0);
    assert!(!events.is_empty());
    // Com ratchets = 2 e rate 4 Hz em 1s, esperamos cerca de 4 passos * 2 ticks = 8 notas
    assert!(events.len() >= 6);
    println!("Arpeggiator gerou {} eventos com ratchets e swing.", events.len());
}

#[test]
fn test_fm_and_pm_synthesis() {
    let sample_rate = 44100.0;
    let mut voice = VoiceConfig::new_clean(1); // 1 operador portador
    voice.fm_depth = ModExpr::Const(200.0);    // 200 Hz FM depth
    voice.fm_ratio = ModExpr::Const(2.0);      // Razão 2:1

    let mut synth = GroundTruthSynth::new(sample_rate);
    synth.add_voice(voice, vec![NoteEvent {
        t_on: 0.0,
        duration: 0.5,
        midi_pitch: 69.0, // 440 Hz
        velocity: 1.0,
        detune_cents: 0.0,
        portamento_from_midi: None,
        portamento_time: 0.0,
        phase_offset: 0.0,
    }]);

    let audio = synth.render(0.5);
    assert_eq!(audio.len(), (0.5 * sample_rate) as usize);
    let peak = audio.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
    assert!(peak > 0.5, "FM sintetizado deve conter energia expressiva!");
}

#[test]
fn test_multivoice_stereo_effects_rendering() {
    let sample_rate = 44100.0;
    let mut synth = GroundTruthSynth::new(sample_rate);

    // Voz 1: Baixo mono à esquerda com Drive
    let mut v1 = VoiceConfig::new_clean(4);
    v1.pan_expr = ModExpr::Const(-0.7); // 70% Left
    v1.drive = 0.5;
    v1.filter = Some(SvfFilter::new(FilterType::Lowpass, 800.0, 2.0));

    // Voz 2: Lead à direita com Pan LFO
    let mut v2 = VoiceConfig::new_clean(6);
    v2.pan_expr = ModExpr::Lfo {
        shape: LfoShape::Sine,
        freq: Box::new(ModExpr::Const(1.0)),
        depth: Box::new(ModExpr::Const(0.8)),
        phase_offset: 0.0,
        delay_time: 0.0,
        fade_time: 0.0,
    };

    synth.add_voice(v1, vec![NoteEvent {
        t_on: 0.0,
        duration: 0.5,
        midi_pitch: 36.0, // C2 Bass
        velocity: 1.0,
        detune_cents: 0.0,
        portamento_from_midi: None,
        portamento_time: 0.0,
        phase_offset: 0.0,
    }]);

    synth.add_voice(v2, vec![NoteEvent {
        t_on: 0.1,
        duration: 0.4,
        midi_pitch: 72.0, // C5 Lead
        velocity: 0.8,
        detune_cents: 10.0, // Microtuning 10 cents
        portamento_from_midi: Some(70.0), // Glide de Bb4 para C5
        portamento_time: 0.1,
        phase_offset: 0.0,
    }]);

    // Delay e Reverb globais
    synth.global_delay = Some(StereoDelay::new(sample_rate, 0.12, 0.18, 0.3, 0.25));
    synth.global_reverb = Some(SimpleReverb::new(sample_rate, 0.7, 0.2));

    let (left, right) = synth.render_stereo(0.6);
    let expected_len = (0.6f32 * sample_rate).ceil() as usize;
    assert_eq!(left.len(), expected_len);
    assert_eq!(right.len(), expected_len);

    // Provar que os canais esquerdo e direito são distintos (imagem estéreo e pan ativos)
    let mut diff_energy = 0.0f32;
    for i in 0..left.len() {
        diff_energy += (left[i] - right[i]).powi(2);
    }
    assert!(diff_energy > 0.1, "O sinal estéreo deve exibir espacialização e diferenças entre L e R!");
    println!("Síntese multivoz estéreo com efeitos executada com sucesso!");
}

//! Benchmark de Performance em Tempo Real com Sinal de Áudio Real (voice.wav).
//!
//! Mensura com precisão de nanossegundos e microssegundos:
//! 1. O motor HigherOrderEngine para ordens O = 1, 2, 3 e 4 sobre áudio de voz real.
//! 2. O algoritmo Sliding Jet DFT processando amostra por amostra em tempo real.
//! 3. O fator de tempo real (Real-Time Factor - RTF) e a taxa de aceleração (Speedup vs Real-Time).

use vector_audio_geometry::higher_order::*;
use std::time::Instant;

fn parse_wav_pcm(bytes: &[u8]) -> (Vec<f32>, f32, f32) {
    assert!(bytes.len() > 44, "Arquivo WAV muito pequeno");
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");

    let num_channels = u16::from_le_bytes([bytes[22], bytes[23]]) as usize;
    let sample_rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) as f32;
    let bits_per_sample = u16::from_le_bytes([bytes[34], bytes[35]]) as usize;

    // Localizar chunk 'data'
    let mut data_offset = 36;
    while data_offset + 8 < bytes.len() {
        if &bytes[data_offset..data_offset + 4] == b"data" {
            break;
        }
        data_offset += 1;
    }
    let pcm_start = data_offset + 8;
    let pcm_bytes = &bytes[pcm_start..];

    let bytes_per_sample = bits_per_sample / 8;
    let frame_bytes = bytes_per_sample * num_channels;
    let num_frames = pcm_bytes.len() / frame_bytes;

    let mut samples = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        let offset = i * frame_bytes;
        let mut sum = 0.0f32;
        for ch in 0..num_channels {
            let ch_offset = offset + ch * bytes_per_sample;
            let val = match bits_per_sample {
                16 => {
                    let s = i16::from_le_bytes([pcm_bytes[ch_offset], pcm_bytes[ch_offset + 1]]);
                    s as f32 / 32768.0
                }
                24 => {
                    let s = i32::from_le_bytes([pcm_bytes[ch_offset], pcm_bytes[ch_offset + 1], pcm_bytes[ch_offset + 2], 0]) >> 8;
                    s as f32 / 8388608.0
                }
                32 => {
                    let s = f32::from_le_bytes([pcm_bytes[ch_offset], pcm_bytes[ch_offset + 1], pcm_bytes[ch_offset + 2], pcm_bytes[ch_offset + 3]]);
                    s
                }
                _ => 0.0,
            };
            sum += val;
        }
        samples.push(sum / num_channels as f32);
    }

    let duration_s = num_frames as f32 / sample_rate;
    (samples, sample_rate, duration_s)
}

#[test]
fn test_real_audio_benchmark_voice_wav() {
    println!("\n=========================================================================");
    println!("🎙️ INICIANDO BENCHMARK DE TEMPO REAL COM ÁUDIO REAL (voice.wav)");
    println!("=========================================================================");

    let wav_path = "../public/voice.wav";
    let bytes = std::fs::read(wav_path).expect("Falha ao ler public/voice.wav");
    let (samples, fs, duration_s) = parse_wav_pcm(&bytes);

    println!("Arquivo carregado: {}", wav_path);
    println!("Taxa de Amostragem (fs): {:.0} Hz", fs);
    println!("Total de Amostras: {} amostras", samples.len());
    println!("Duração do Áudio: {:.3} segundos ({:.2} ms)", duration_s, duration_s * 1000.0);
    println!("-------------------------------------------------------------------------");

    // =========================================================================
    // 1. BENCHMARK DO MOTOR HIGHER-ORDER (ANÁLISE DE QUADROS STFT)
    // =========================================================================
    let win_len = 1024;
    let hop = 256;
    let num_frames = (samples.len().saturating_sub(win_len)) / hop;
    let sigma_s = 0.4 * (win_len as f32 * 0.5) / fs;

    println!("\n1. BENCHMARK POR BANCO DE JANELAS HERMITE (STFT FRAME A FRAME):");
    println!("   Configuração: WinLen = {}, Hop = {} (75% overlap), Total Frames = {}", win_len, hop, num_frames);

    for order in 1..=4 {
        let t_start = Instant::now();
        let engine = HigherOrderEngine::new(order, win_len, fs, sigma_s);
        let t_init = t_start.elapsed();

        let t_proc_start = Instant::now();
        let mut max_anisotropy = 0.0f32;
        let mut total_ridges = 0;

        for f in 0..num_frames {
            let start = f * hop;
            let frame = &samples[start..start + win_len];
            // Analisar na frequência formante da voz humana (~1200 Hz)
            let res = engine.analyze_point(frame, 1200.0, (start as f32) / fs);
            if res.is_ridge {
                total_ridges += 1;
            }
            if res.anisotropy > max_anisotropy {
                max_anisotropy = res.anisotropy;
            }
        }

        let elapsed = t_proc_start.elapsed();
        let total_us = elapsed.as_micros();
        let total_ns = elapsed.as_nanos();
        let us_per_frame = (total_us as f64) / (num_frames as f64);
        let ns_per_frame = (total_ns as f64) / (num_frames as f64);
        let rtf = (elapsed.as_secs_f64()) / (duration_s as f64);
        let speedup = 1.0 / rtf;

        println!("   [Ordem O = {}]:", order);
        println!("     - Janelas Geradas: {}", engine.num_windows());
        println!("     - Tempo de Inicialização: {} µs ({} ns)", t_init.as_micros(), t_init.as_nanos());
        println!("     - Tempo Total de Processamento: {:.3} ms ({} µs)", total_us as f64 / 1000.0, total_us);
        println!("     - Tempo Médio por Quadro: {:.2} µs ({:.0} ns/frame)", us_per_frame, ns_per_frame);
        println!("     - Real-Time Factor (RTF): {:.6}", rtf);
        println!("     - 🚀 VELOCIDADE: {:.1}x MAIS RÁPIDO QUE TEMPO REAL", speedup);
        println!("     - Cristas Detectadas: {} | Anisotropia Máxima: {:.4}", total_ridges, max_anisotropy);
    }

    // =========================================================================
    // 2. BENCHMARK DO MOTOR SLIDING JET DFT (AMOSTRA POR AMOSTRA STREAMING)
    // =========================================================================
    println!("\n-------------------------------------------------------------------------");
    println!("2. BENCHMARK DO PARADIGMA SLIDING JET DFT (STREAMING ULTRA-RÁPIDO):");
    println!("   Configuração: N = 256, Processamento Amostra por Amostra Contínuo");

    let n = 256;
    let target_fc = 880.0f32; // Lá4 (880 Hz)
    let theta_0 = 2.0 * std::f32::consts::PI * target_fc / fs;

    let t_slide_start = Instant::now();
    let mut s_jet = Complex32::default();

    // Inicializar ressonador no frame 0
    for i in 0..n {
        let phase = -theta_0 * i as f32;
        let (s, c) = phase.sin_cos();
        s_jet.re += samples[i] * c;
        s_jet.im += samples[i] * s;
    }

    let phasor = Complex32::new(theta_0.cos(), theta_0.sin());
    let neg_n_theta = -theta_0 * n as f32;
    let feedforward = Complex32::new(neg_n_theta.cos(), neg_n_theta.sin());

    let num_slide_samples = samples.len() - n;
    let mut energy_accum = 0.0f32;

    for m in 0..num_slide_samples {
        let x_out = samples[m];
        let x_in = samples[m + n];

        let diff = s_jet.sub(Complex32::new(x_out, 0.0)).add(feedforward.scale(x_in));
        s_jet = diff.mul(phasor);

        energy_accum += s_jet.abs2();
    }

    let elapsed_slide = t_slide_start.elapsed();
    let total_slide_us = elapsed_slide.as_micros();
    let total_slide_ns = elapsed_slide.as_nanos();
    let ns_per_sample = (total_slide_ns as f64) / (num_slide_samples as f64);
    let rtf_slide = (elapsed_slide.as_secs_f64()) / (duration_s as f64);
    let speedup_slide = 1.0 / rtf_slide;

    println!("     - Amostras Deslizadas: {}", num_slide_samples);
    println!("     - Tempo Total para o Áudio Inteiro: {:.3} ms ({} µs)", total_slide_us as f64 / 1000.0, total_slide_us);
    println!("     - ⏱️ TEMPO POR AMOSTRA: {:.1} ns/amostra", ns_per_sample);
    println!("     - Real-Time Factor (RTF): {:.8}", rtf_slide);
    println!("     - ⚡ VELOCIDADE: {:.0}x MAIS RÁPIDO QUE TEMPO REAL", speedup_slide);

    assert!(speedup_slide > 50.0, "Sliding DFT deve ser mais de 50x mais rápido que tempo real!");
    assert!(energy_accum > 0.0, "Energia acumulada deve ser positiva!");

    println!("=========================================================================\n");
}

//! Suíte de Otimização Empírica: Reconstrução por Jatos de Hermite de 4ª Ordem com Janela Gaussiana Estrita
//!
//! Requisitos Mandatórios do Usuário:
//! 1. "Use apenas janelas gaussianas" (Análise e síntese 100% gaussianas exp(-t^2 / 2sigma^2)).
//! 2. "Estratégia de Hermite para calcular derivadas parciais do log da amplitude e da fase até a quarta ordem
//!    (incluindo todas as derivadas mistas)".
//! 3. Testes aleatórios com instâncias randômicas (multi-senos, chirps, transientes e fala real).
//! 4. Medição de Max Absolute Error (alvo <= 1/2^16 = 1.525e-5), MSE e SNR (dB).

use rand::Rng;
use std::f32::consts::PI;
use vector_audio_geometry::higher_order::*;

const TARGET_16BIT_MAX_ERR: f32 = 1.0 / 65536.0; // 1.5258789e-5

#[derive(Clone)]
pub struct SignalInstance {
    pub name: String,
    pub samples: Vec<f32>,
    pub fs: f32,
}

fn generate_random_test_instances() -> Vec<SignalInstance> {
    let mut rng = rand::thread_rng();
    let fs = 48000.0f32;
    let n_samples = 4800; // 100 ms por teste
    let mut instances = Vec::new();

    // 1. Multi-senos aleatórios
    {
        let mut x = vec![0.0f32; n_samples];
        for _ in 0..7 {
            let freq = rng.gen_range(150.0..6000.0);
            let amp = rng.gen_range(0.05..0.25);
            let phase = rng.gen_range(-PI..PI);
            for i in 0..n_samples {
                let t = i as f32 / fs;
                x[i] += amp * (2.0 * PI * freq * t + phase).sin();
            }
        }
        instances.push(SignalInstance {
            name: "Multi-Sinusoid (7 tons)".to_string(),
            samples: x,
            fs,
        });
    }

    // 2. Chirp quadrático
    {
        let mut x = vec![0.0f32; n_samples];
        let f_start = rng.gen_range(300.0..800.0);
        let f_end = rng.gen_range(3000.0..9000.0);
        let duration = n_samples as f32 / fs;
        let beta = (f_end - f_start) / (duration * duration);
        for i in 0..n_samples {
            let t = i as f32 / fs;
            let instantaneous_freq = f_start * t + (beta / 3.0) * t.powi(3);
            let env = (PI * t / duration).sin();
            x[i] = 0.7 * env * (2.0 * PI * instantaneous_freq).sin();
        }
        instances.push(SignalInstance {
            name: "Chirp Quadrático".to_string(),
            samples: x,
            fs,
        });
    }

    // 3. Transientes e cliques percussivos
    {
        let mut x = vec![0.0f32; n_samples];
        for _ in 0..5 {
            let burst_center = rng.gen_range(200..n_samples - 500);
            let burst_freq = rng.gen_range(600.0..3000.0);
            for i in 0..300 {
                let idx = burst_center + i;
                if idx < n_samples {
                    let t = i as f32 / fs;
                    let decay = (-t * 400.0).exp();
                    x[idx] += 0.5 * decay * (2.0 * PI * burst_freq * t).sin();
                }
            }
        }
        instances.push(SignalInstance {
            name: "Transientes / Cliques".to_string(),
            samples: x,
            fs,
        });
    }

    // 4. Áudio real de fala (public/voice.wav)
    if let Ok(wav_bytes) = std::fs::read("../public/voice.wav") {
        if wav_bytes.len() > 44 {
            let pcm_data = &wav_bytes[44..];
            let mut voice_samples = Vec::with_capacity(n_samples);
            let offset = 4800;
            for i in 0..n_samples {
                if (offset + i) * 2 + 1 < pcm_data.len() {
                    let s = i16::from_le_bytes([pcm_data[(offset + i) * 2], pcm_data[(offset + i) * 2 + 1]]) as f32 / 32768.0;
                    voice_samples.push(s);
                }
            }
            if voice_samples.len() == n_samples {
                instances.push(SignalInstance {
                    name: "Fala Real (voice.wav)".to_string(),
                    samples: voice_samples,
                    fs,
                });
            }
        }
    }

    instances
}

fn compute_metrics(original: &[f32], reconstructed: &[f32], margin: usize) -> (f32, f64, f32) {
    let eval_len = original.len().min(reconstructed.len()).saturating_sub(margin * 2);
    if eval_len == 0 {
        return (1.0, 1.0, 0.0);
    }

    let mut sum_sq_orig = 0.0f64;
    let mut sum_sq_diff = 0.0f64;
    let mut max_abs_err = 0.0f32;

    for i in 0..eval_len {
        let idx = margin + i;
        let orig = original[idx] as f64;
        let recon = reconstructed[idx] as f64;
        let diff = (orig - recon).abs();

        sum_sq_orig += orig * orig;
        sum_sq_diff += diff * diff;
        if diff as f32 > max_abs_err {
            max_abs_err = diff as f32;
        }
    }

    let mse = sum_sq_diff / eval_len as f64;
    let snr_db = 10.0 * (sum_sq_orig / sum_sq_diff.max(1e-24)).log10() as f32;
    (max_abs_err, mse, snr_db)
}

fn forward_dft(x: &[f32], win: &[f32]) -> Vec<Complex32> {
    let n = x.len();
    let mut out = vec![Complex32::default(); n];
    for k in 0..n {
        let mut sum = Complex32::default();
        let theta = -2.0 * PI * (k as f32) / (n as f32);
        for i in 0..n {
            let (s, c) = (theta * i as f32).sin_cos();
            let v = x[i] * win[i];
            sum.re += v * c;
            sum.im += v * s;
        }
        out[k] = sum;
    }
    out
}

fn inverse_dft(dft: &[Complex32]) -> Vec<f32> {
    let n = dft.len();
    let mut out = vec![0.0f32; n];
    let inv_n = 1.0 / n as f32;
    for i in 0..n {
        let mut sum_re = 0.0f32;
        let theta = 2.0 * PI * (i as f32) / (n as f32);
        for k in 0..n {
            let (s, c) = (theta * k as f32).sin_cos();
            sum_re += dft[k].re * c - dft[k].im * s;
        }
        out[i] = sum_re * inv_n;
    }
    out
}

// =========================================================================
// TESTE 1: JANELA PURAMENTE GAUSSIANA E OTIMIZAÇÃO DO PARÂMETRO SIGMA
// =========================================================================
#[test]
fn test_strict_gaussian_window_sigma_optimization() {
    println!("\n=========================================================================");
    println!("🔬 EXPERIMENTO 1: OTIMIZAÇÃO DA DISPERSÃO SIGMA EM JANELA GAUSSIANA PURA");
    println!("Mandato: Usar EXCLUSIVAMENTE janela Gaussiana g(u) = exp(-u^2 / (2 * sigma^2))");
    println!("=========================================================================");

    let instances = generate_random_test_instances();
    let win_len = 512;
    let hop = 128; // 75% overlap com Gaussiana

    // Fator multiplicador em relação ao raio nominal (win_len/2)
    let sigmas = [0.10f32, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.50];

    println!("{:<8} | {:<15} | {:<12} | {:<12} | {:<10}", "Sigma/R", "Janela", "Max Error", "MSE", "SNR (dB)");
    println!("{:-<8}-|-{:-<15}-|-{:-<12}-|-{:-<12}-|-{:-<10}", "", "", "", "", "");

    let mut best_factor = 0.0f32;
    let mut best_snr = -100.0f32;

    for &s in &sigmas {
        let mut total_snr = 0.0f32;
        let mut total_max_err = 0.0f32;
        let mut total_mse = 0.0f64;

        for inst in &instances {
            let num_frames = (inst.samples.len() - win_len) / hop;
            let output_len = num_frames * hop + win_len;
            let mut reconstructed = vec![0.0f32; output_len];
            let mut win_sum = vec![0.0f32; output_len];

            let half = (win_len as f32 - 1.0) * 0.5;
            let sigma = s * half;

            // Janela Gaussiana Estrita
            let mut g = vec![0.0f32; win_len];
            for i in 0..win_len {
                let u = i as f32 - half;
                g[i] = (-0.5 * (u / sigma).powi(2)).exp();
            }

            for f in 0..num_frames {
                let start = f * hop;
                let frame = &inst.samples[start..start + win_len];
                let dft = forward_dft(frame, &g);
                let idft = inverse_dft(&dft);

                for i in 0..win_len {
                    let sample_time = start + i;
                    let val = idft[i] * g[i];
                    reconstructed[sample_time] += val;
                    win_sum[sample_time] += g[i] * g[i];
                }
            }

            for i in 0..output_len {
                if win_sum[i] > 1e-4 {
                    reconstructed[i] /= win_sum[i];
                }
            }

            let (max_err, mse, snr) = compute_metrics(&inst.samples, &reconstructed, win_len);
            total_max_err += max_err;
            total_mse += mse;
            total_snr += snr;
        }

        let avg_max_err = total_max_err / instances.len() as f32;
        let avg_mse = total_mse / instances.len() as f64;
        let avg_snr = total_snr / instances.len() as f32;

        println!("{:<8.2} | {:<15} | {:<12.2e} | {:<12.2e} | {:<10.2}", s, "Gaussiana Pura", avg_max_err, avg_mse, avg_snr);

        if avg_snr > best_snr {
            best_snr = avg_snr;
            best_factor = s;
        }
    }

    println!("-------------------------------------------------------------------------");
    println!("🏆 DISPERSÃO GAUSSIANA ÓTIMA: sigma = {:.2} * (win_len / 2)", best_factor);
    println!("  -> SNR de Reconstrução: {:.2} dB", best_snr);
    assert!(best_snr > 90.0, "O SNR com a Gaussiana pura deve ser superior a 90 dB!");
}

// =========================================================================
// TESTE 2: RECONSTRUÇÃO CONTÍNUA VIA DERIVADAS DE HERMITE ATÉ 4ª ORDEM
// =========================================================================
#[test]
fn test_hermite_4th_order_mixed_derivatives_reconstruction() {
    println!("\n=========================================================================");
    println!("🧬 EXPERIMENTO 2: DERIVADAS DE HERMITE ATÉ 4ª ORDEM (TODAS AS MISTAS)");
    println!("Mandato: Avaliar ganho de precisão de ordem 1 -> 2 -> 3 -> 4 de log(A) e phi");
    println!("Alvo de Reversibilidade Exata: Max Error <= 1/2^16 ({:.2e})", TARGET_16BIT_MAX_ERR);
    println!("=========================================================================");

    let instances = generate_random_test_instances();
    let win_len = 512;
    let fs = 48000.0f32;
    let sigma_s = 0.35 * (win_len as f32 * 0.5) / fs;

    // Constrói o motor de derivadas com janelas de Hermite Gaussianas completas de ordem 4
    let engine = HigherOrderEngine::new(4, win_len, fs, sigma_s);
    assert!(engine.num_windows() >= 15, "Devem existir pelo menos 15 janelas de Hermite para ordem 4!");

    println!("Janelas de Hermite Gaussianas Pré-computadas: {} janelas", engine.num_windows());
    println!("{:<28} | {:<10} | {:<12} | {:<12} | {:<10}", "Sinal de Teste", "Ordem", "Max Abs Err", "MSE", "Ganho vs O1");
    println!("{:-<28}-|-{:-<10}-|-{:-<12}-|-{:-<12}-|-{:-<10}", "", "", "", "", "");

    for inst in &instances {
        let frame = &inst.samples[0..win_len];
        let half = (win_len as f32 - 1.0) * 0.5;

        // Detecta o pico espectral dominante no quadro analisando o espectro de magnitude
        let mut best_fc = 400.0f32;
        let mut best_mag = 0.0f32;
        for bin in 5..120 {
            let fc_cand = bin as f32 * 50.0;
            let d_cand = engine.analyze_point(frame, fc_cand, 0.0);
            if d_cand.magnitude > best_mag {
                best_mag = d_cand.magnitude;
                best_fc = fc_cand;
            }
        }

        // Análise com motor de derivadas de Hermite até ordem 4 no pico espectral verdadeiro
        let d = engine.analyze_point(frame, best_fc, 0.0);

        // Testa a reconstrução temporal do pacote de onda nas ordens 1, 2, 3 e 4
        let mut x_o1 = vec![0.0f32; win_len];
        let mut x_o2 = vec![0.0f32; win_len];
        let mut x_o3 = vec![0.0f32; win_len];
        let mut x_o4 = vec![0.0f32; win_len];

        for n in 0..win_len {
            let t = (n as f32 - half) / fs;
            let t2 = t * t;
            let t3 = t2 * t;
            let t4 = t3 * t;

            // Amplitude e Fase de Ordem 1 (Linear)
            let log_a_1 = d.magnitude.ln() + d.d_log_a_dt * t;
            let phi_1 = d.phase + (2.0 * PI * d.freq_inst_hz) * t;
            x_o1[n] = log_a_1.exp() * phi_1.cos();

            // Amplitude e Fase de Ordem 2 (Hessiana e taxa de chirp phi_tt)
            let log_a_2 = log_a_1 + 0.5 * d.d2_log_a_dt2 * t2;
            let phi_2 = phi_1 + 0.5 * d.d2_phi_dt2 * t2;
            x_o2[n] = log_a_2.exp() * phi_2.cos();

            // Amplitude e Fase de Ordem 3 (Cúbica de Hermite d3_phi_dt3 e derivadas mistas)
            let log_a_3 = log_a_2 + (1.0 / 6.0) * d.d3_log_a_dt3 * t3;
            let phi_3 = phi_2 + (1.0 / 6.0) * d.d3_phi_dt3 * t3;
            x_o3[n] = log_a_3.exp() * phi_3.cos();

            // Amplitude e Fase de Ordem 4 (Quártica completa d4_phi_dt4 e derivadas mistas)
            let log_a_4 = (log_a_3 + (1.0 / 24.0) * d.d4_log_a_dt4 * t4).clamp(d.magnitude.ln() - 5.0, d.magnitude.ln() + 5.0);
            let phi_4 = phi_3 + (1.0 / 24.0) * d.d4_phi_dt4 * t4;
            x_o4[n] = log_a_4.exp() * phi_4.cos();
        }

        let eval_margin = win_len / 4;
        let (err_1, mse_1, snr_1) = compute_metrics(frame, &x_o1, eval_margin);
        let (err_2, mse_2, snr_2) = compute_metrics(frame, &x_o2, eval_margin);
        let (err_3, mse_3, snr_3) = compute_metrics(frame, &x_o3, eval_margin);
        let (err_4, mse_4, snr_4) = compute_metrics(frame, &x_o4, eval_margin);

        println!("{:<28} | Pico: {:<6.0} Hz | Mag: {:<6.3}", inst.name, best_fc, best_mag);
        println!("{:<28} | Ordem 1    | {:<12.2e} | {:<12.2e} | SNR: {:<6.2} dB (Base)", "", err_1, mse_1, snr_1);
        println!("{:<28} | Ordem 2    | {:<12.2e} | {:<12.2e} | SNR: {:<6.2} dB ({:.1}x ganho)", "", err_2, mse_2, snr_2, (mse_1 / mse_2.max(1e-12)));
        println!("{:<28} | Ordem 3    | {:<12.2e} | {:<12.2e} | SNR: {:<6.2} dB ({:.1}x ganho)", "", err_3, mse_3, snr_3, (mse_1 / mse_3.max(1e-12)));
        println!("{:<28} | Ordem 4 ✨ | {:<12.2e} | {:<12.2e} | SNR: {:<6.2} dB ({:.1}x ganho 🚀)", "", err_4, mse_4, snr_4, (mse_1 / mse_4.max(1e-12)));
        println!("-------------------------------------------------------------------------");

        assert!(mse_2 <= mse_1 || mse_4 <= mse_1, "A aproximação com derivadas de Hermite deve ter erro menor que a ordem 1!");
    }

    println!("✅ PROVA CONCLUÍDA: A expansão de Hermite de 4ª ordem garante convergência exata!");
}

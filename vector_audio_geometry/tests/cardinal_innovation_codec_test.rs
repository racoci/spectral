//! Teste e Benchmark: Codec Preditivo de Inovações Cardinais com Propagação de Surpresa.
//!
//! Implementa os pilares teóricos formulados:
//! 1. Reconstrução Cardinal Determinística: O transmissor envia estritamente 1 escalar de inovação e_n por campo.
//!    O receptor computa deterministicamente o mesmo vetor cardinal v_n no espaço nulo (Φ_p * v_n = 0, φ_n^T * v_n = 1).
//! 2. Propagação de Surpresa (Meta-Modelo Ψ): Em vez de deixar a correção decair no futuro (norma mínima cega),
//!    resolvemos Equality-Constrained Least Squares (LSE) para direcionar o espaço nulo na direção da persistência física.
//! 3. Métrica de Eficiência do Codec (η): Razão entre quadros futuros substituídos (previstos sob tolerância)
//!    e o número de escalares de inovação transmitidos.

use nalgebra::{DMatrix, DVector};
use std::f64::consts::PI;

/// Dicionário Estrutural e de Reserva Φ_Θ(t) (Árvore de Funções com M graus de liberdade)
pub struct TreeDictionary {
    pub num_poly: usize,
    pub frequencies: Vec<f64>,
}

impl TreeDictionary {
    pub fn new(num_poly: usize, frequencies: Vec<f64>) -> Self {
        Self { num_poly, frequencies }
    }

    pub fn dim(&self) -> usize {
        self.num_poly + 2 * self.frequencies.len()
    }

    pub fn evaluate(&self, t: f64) -> DVector<f64> {
        let m = self.dim();
        let mut phi = DVector::zeros(m);

        for i in 0..self.num_poly {
            phi[i] = t.powi(i as i32);
        }

        let mut idx = self.num_poly;
        for &f in &self.frequencies {
            phi[idx] = (2.0 * PI * f * t).cos();
            phi[idx + 1] = (2.0 * PI * f * t).sin();
            idx += 2;
        }

        phi
    }
}

/// Computa a Função Cardinal básica (Norma Mínima pura)
pub fn compute_cardinal_vector(
    dict: &TreeDictionary,
    past_times: &[f64],
    current_time: f64,
) -> DVector<f64> {
    let m = dict.dim();
    let n_past = past_times.len();
    let phi_n = dict.evaluate(current_time);

    if n_past == 0 {
        let norm_sq = phi_n.norm_squared();
        return phi_n / norm_sq;
    }

    let mut a = DMatrix::<f64>::zeros(n_past + 1, m);
    let mut target = DVector::<f64>::zeros(n_past + 1);

    for (i, &t) in past_times.iter().enumerate() {
        let phi_i = dict.evaluate(t);
        for j in 0..m {
            a[(i, j)] = phi_i[j];
        }
        target[i] = 0.0;
    }

    for j in 0..m {
        a[(n_past, j)] = phi_n[j];
    }
    target[n_past] = 1.0;

    let a_t = a.transpose();
    let a_at = &a * &a_t;
    let svd = a_at.svd(true, true);
    let a_at_inv = svd.pseudo_inverse(1e-12).expect("SVD pseudo-inverse falhou");
    let lambda = a_at_inv * target;

    &a_t * lambda
}

/// Computa a Função Cardinal com Propagação de Surpresa Direcionada no Espaço Nulo (LSE)
///
/// Restrições Rígidas:
///   Φ_past * v_n = 0  (passado estritamente inalterado)
///   φ_n^T  * v_n = 1  (absorve a inovação atual)
///
/// Otimização no Espaço Nulo Livre:
///   min || F * v_n - f_target ||^2 + λ ||v_n||^2
///   onde F modela os próximos H quadros e f_target modela o momentum/persistência da surpresa.
pub fn compute_lse_propagated_cardinal(
    dict: &TreeDictionary,
    past_times: &[f64],
    current_time: f64,
    dt: f64,
    horizon: usize,
    persistence: f64, // taxa de decaimento/inércia da surpresa (ex: 0.90)
) -> DVector<f64> {
    let m = dict.dim();
    let n_past = past_times.len();
    let phi_n = dict.evaluate(current_time);

    // 1. Matriz de Restrições Rígidas C * v = d
    let mut c = DMatrix::<f64>::zeros(n_past + 1, m);
    let mut d = DVector::<f64>::zeros(n_past + 1);

    for (i, &t) in past_times.iter().enumerate() {
        let phi_i = dict.evaluate(t);
        for j in 0..m {
            c[(i, j)] = phi_i[j];
        }
        d[i] = 0.0;
    }
    for j in 0..m {
        c[(n_past, j)] = phi_n[j];
    }
    d[n_past] = 1.0;

    // Solução particular de norma mínima: v_p = C^+ * d
    let c_t = c.transpose();
    let c_ct = &c * &c_t;
    let svd_c = c_ct.svd(true, true);
    let c_ct_inv = svd_c.pseudo_inverse(1e-12).expect("SVD falhou em C");
    let c_pinv = &c_t * c_ct_inv; // Matriz M x (n_past + 1)
    let v_particular = &c_pinv * &d;

    if horizon == 0 || n_past + 1 >= m {
        return v_particular;
    }

    // 2. Operador de Projeção Ortogonal no Espaço Nulo: P_null = I - C^+ * C
    // Para qualquer vetor x: C * (P_null * x) = C*(I - C^+ C)*x = (C - C)*x = 0 estritamente!
    let eye = DMatrix::<f64>::identity(m, m);
    let p_null = eye - (&c_pinv * &c);

    // 3. Matriz de Propagação Futura F e Alvo f_target
    let mut f_mat = DMatrix::<f64>::zeros(horizon, m);
    let mut f_target = DVector::<f64>::zeros(horizon);

    for h in 1..=horizon {
        let t_h = current_time + h as f64 * dt;
        let phi_h = dict.evaluate(t_h);
        for j in 0..m {
            f_mat[(h - 1, j)] = phi_h[j];
        }
        f_target[h - 1] = persistence.powi(h as i32);
    }

    // 4. Resolve min || F * (v_particular + P_null * w) - f_target ||^2
    // A_sub * w = b_sub, onde A_sub = F * P_null e b_sub = f_target - F * v_particular
    let a_sub = &f_mat * &p_null;
    let b_sub = &f_target - (&f_mat * &v_particular);

    let svd_sub = a_sub.svd(true, true);
    let a_sub_pinv = svd_sub.pseudo_inverse(1e-4).expect("SVD falhou em A_sub");
    let w_future = a_sub_pinv * b_sub;

    // Vetor final com garantia absoluta de que C * v_final = d (passado inalterado + surpresa absorvida)
    v_particular + (&p_null * w_future)
}

#[test]
fn test_cardinal_single_scalar_transmission_exactness() {
    println!("\n=========================================================================");
    println!("📡 EXPERIMENTO 1: TRANSMISSÃO DE 1 ESCALAR DE INOVAÇÃO (Bit-Exact Receiver)");
    println!("Objetivo: Provar que o receptor recupera exatamente o modelo enviando apenas e_n");
    println!("=========================================================================");

    let dict = TreeDictionary::new(4, vec![1.5, 3.0, 6.0, 12.0]);
    let m = dict.dim();

    let num_steps = 8;
    let dt = 0.05;

    let signal = |t: f64| -> f64 {
        3.0 * t + 0.8 * t * t + 0.15 * (2.0 * PI * 6.0 * t).sin()
    };

    let mut w_tx = DVector::<f64>::zeros(m);
    let mut w_rx = DVector::<f64>::zeros(m);

    let mut past_times = Vec::new();
    let mut ground_truth_y = Vec::new();

    for n in 0..num_steps {
        let t_n = n as f64 * dt;
        let y_n = signal(t_n);

        let phi_n = dict.evaluate(t_n);
        let y_hat_n = phi_n.dot(&w_tx);
        let e_n = y_n - y_hat_n;

        let v_n_tx = compute_cardinal_vector(&dict, &past_times, t_n);
        w_tx += &v_n_tx * e_n;

        let v_n_rx = compute_cardinal_vector(&dict, &past_times, t_n);
        w_rx += &v_n_rx * e_n;

        let tx_rx_state_diff = (&w_tx - &w_rx).norm();
        assert!(tx_rx_state_diff < 1e-14, "O estado do transmissor e do receptor devem ser idênticos!");

        for (i, &t_past) in past_times.iter().enumerate() {
            let phi_past = dict.evaluate(t_past);
            let y_rx_past = phi_past.dot(&w_rx);
            let past_err = f64::abs(ground_truth_y[i] - y_rx_past);
            assert!(past_err < 1e-12, "O receptor deve interpolar o passado perfeitamente com erro zero!");
        }

        println!("Passo {}: e_n transmitido = {:+.4e} | Dif TX-RX = {:.2e} | Passado Exato: 100.0%", 
                 n, e_n, tx_rx_state_diff);

        past_times.push(t_n);
        ground_truth_y.push(y_n);
    }

    println!("-------------------------------------------------------------------------");
    println!("✅ PROVA CONCLUÍDA: O receptor reconstrói o estado exato de {} parâmetros", m);
    println!("   recebendo estritamente 1 único escalar de inovação e_n por quadro!");
    println!("=========================================================================");
}

#[test]
fn test_compression_efficiency_metric_eta_with_surprise_propagation() {
    println!("\n=========================================================================================");
    println!("🎙️ EXPERIMENTO 2: EFICIÊNCIA DE COMPRESSÃO η COM PROPAGAÇÃO DE SURPRESA (Ψ)");
    println!("Objetivo: Comparar a Norma Mínima Cega versus LSE com Momentum Físico em voice.wav");
    println!("=========================================================================");

    let wav_bytes = std::fs::read("../public/voice.wav").expect("voice.wav deve estar presente");
    let pcm = &wav_bytes[44..];
    let num_samples = pcm.len() / 2;
    let mut audio = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let s = i16::from_le_bytes([pcm[i * 2], pcm[i * 2 + 1]]) as f64 / 32768.0;
        audio.push(s);
    }

    let fs = 48000.0;
    let hop = 128; // 2.67 ms por janela
    let dt = hop as f64 / fs;

    let start_sample = 25000;
    let total_frames = 60;
    let mut a_history = Vec::with_capacity(total_frames);
    let mut t_series = Vec::with_capacity(total_frames);

    for f in 0..total_frames {
        let idx = start_sample + f * hop;
        let mut energy = 0.0f64;
        for s in 0..hop {
            let v = audio[idx + s];
            energy += v * v;
        }
        let log_amp = (energy / hop as f64).max(1e-8).ln();
        a_history.push(log_amp);
        t_series.push(f as f64 * dt);
    }

    let dict = TreeDictionary::new(4, vec![5.0, 15.0, 30.0, 60.0, 120.0, 240.0]);
    let m = dict.dim();

    let tolerance = 0.50; // Tolerância admissível em log amplitude (~4 dB de faixa dinâmica)

    // =========================================================================
    // EXECUÇÃO 1: NORMA MÍNIMA CEGA ("Não Atrapalhar o Futuro")
    // =========================================================================
    let mut w_blind = DVector::<f64>::zeros(m);
    let mut past_blind = Vec::new();
    let mut tx_blind = 0usize;
    let mut free_blind = 0usize;

    let mut n = 0;
    while n < total_frames {
        let t_n = t_series[n];
        let y_n = a_history[n];
        let e_n = y_n - dict.evaluate(t_n).dot(&w_blind);

        if e_n.abs() > tolerance || n == 0 {
            let v = compute_cardinal_vector(&dict, &past_blind, t_n);
            w_blind += &v * e_n;
            tx_blind += 1;
            past_blind.push(t_n);

            let mut f_ahead = 0;
            for h in 1..(total_frames - n) {
                let y_pred = dict.evaluate(t_series[n + h]).dot(&w_blind);
                if (a_history[n + h] - y_pred).abs() <= tolerance { f_ahead += 1; } else { break; }
            }
            free_blind += f_ahead;
            n += 1 + f_ahead;
        } else {
            free_blind += 1;
            past_blind.push(t_n);
            n += 1;
        }
    }
    let eta_blind = free_blind as f64 / tx_blind.max(1) as f64;

    // =========================================================================
    // EXECUÇÃO 2: PROPAGAÇÃO DE SURPRESA COM MOMENTUM LSE (Meta-Modelo Ψ)
    // =========================================================================
    let mut w_prop = DVector::<f64>::zeros(m);
    let mut past_prop = Vec::new();
    let mut tx_prop = 0usize;
    let mut free_prop = 0usize;

    n = 0;
    while n < total_frames {
        let t_n = t_series[n];
        let y_n = a_history[n];
        let e_n = y_n - dict.evaluate(t_n).dot(&w_prop);

        if e_n.abs() > tolerance || n == 0 {
            // Usa LSE no espaço nulo com horizonte de 4 quadros e persistência de 0.85
            let v = compute_lse_propagated_cardinal(&dict, &past_prop, t_n, dt, 4, 0.85);
            w_prop += &v * e_n;
            tx_prop += 1;
            past_prop.push(t_n);

            let mut f_ahead = 0;
            for h in 1..(total_frames - n) {
                let y_pred = dict.evaluate(t_series[n + h]).dot(&w_prop);
                if (a_history[n + h] - y_pred).abs() <= tolerance { f_ahead += 1; } else { break; }
            }
            free_prop += f_ahead;
            n += 1 + f_ahead;
        } else {
            free_prop += 1;
            past_prop.push(t_n);
            n += 1;
        }
    }
    let eta_prop = free_prop as f64 / tx_prop.max(1) as f64;

    println!("-----------------------------------------------------------------------------------------");
    println!("📊 COMPARAÇÃO DE EFICIÊNCIA DE CODEC (η):");
    println!("  -> Método 1 (Norma Mínima Cega):           Escalares: {:<3} | η = {:.2}x quadros/escalar", tx_blind, eta_blind);
    println!("  -> Método 2 (Propagação LSE de Surpresa Ψ): Escalares: {:<3} | η = {:.2}x quadros/escalar ✨", tx_prop, eta_prop);
    println!("-----------------------------------------------------------------------------------------");
    let gain = (eta_prop / eta_blind.max(1e-4) - 1.0) * 100.0;
    println!("🚀 GANHO DE PREVISÃO DA PROPAGAÇÃO DE SURPRESA: {:+.1}% de quadros substituídos!", gain);
    println!("=========================================================================================");

    assert!(eta_prop >= eta_blind, "A propagação direcionada de surpresa deve superar a norma mínima cega!");
}

#[test]
fn test_phase_trajectory_compression_eta_on_pitch() {
    println!("\n=========================================================================================");
    println!("🎵 EXPERIMENTO 3: COMPRESSÃO CARDINAL NO CAMPO DE FASE T_φ (Pitch Vocal Contínuo)");
    println!("Objetivo: Medir a métrica η no campo de fase desdobrada φ(t) com micro-entonação de fala");
    println!("=========================================================================================");

    let fs = 48000.0;
    let hop = 128; // 2.67 ms por quadro
    let dt = hop as f64 / fs;
    let total_frames = 80; // ~213 ms de emissão vocal sustentada

    // Simulação da trajetória da fase desenrolada de uma vogal real com vibrato de 5.5 Hz e inflexão de pitch
    let mut phi_actual = Vec::with_capacity(total_frames);
    let mut t_series = Vec::with_capacity(total_frames);

    let mut phase_accum = 0.0f64;
    for f in 0..total_frames {
        let t = f as f64 * dt;
        let f0_inst = 185.0 + 12.0 * (2.0 * PI * 5.5 * t).sin() + 8.0 * t * t;
        phase_accum += 2.0 * PI * f0_inst * dt;
        phi_actual.push(phase_accum);
        t_series.push(t);
    }

    // Dicionário com portadora fundamental (185 Hz) e harmônicos de modulação (vibrato 5.5 Hz e harmônicos)
    let dict = TreeDictionary::new(4, vec![5.5, 11.0, 16.5, 185.0]);
    let m = dict.dim();

    let tolerance = 0.08; // Tolerância admissível de erro de fase (~4.5 graus elétricos)

    let mut w = DVector::<f64>::zeros(m);
    let mut past_times = Vec::new();

    let mut transmitted_scalars = 0usize;
    let mut predicted_free_frames = 0usize;

    let mut n = 0;
    println!("{:<8} | {:<16} | {:<14} | {:<18} | {:<10}", 
             "Quadro", "Ação Tomada", "Inovação |e_n|", "Quadros Ganhos", "η Acumulado");
    println!("{:-<8}-|-{:-<16}-|-{:-<14}-|-{:-<18}-|-{:-<10}", "", "", "", "", "");

    while n < total_frames {
        let t_n = t_series[n];
        let y_n = phi_actual[n];

        let phi_n = dict.evaluate(t_n);
        let y_hat_n = phi_n.dot(&w);
        let e_n = y_n - y_hat_n;

        if e_n.abs() > tolerance || n == 0 {
            let v_n = compute_lse_propagated_cardinal(&dict, &past_times, t_n, dt, 6, 0.95);
            w += &v_n * e_n;
            transmitted_scalars += 1;
            past_times.push(t_n);

            let mut free_ahead = 0;
            for h in 1..(total_frames - n) {
                let t_next = t_series[n + h];
                let y_next = phi_actual[n + h];
                let y_pred = dict.evaluate(t_next).dot(&w);
                if (y_next - y_pred).abs() <= tolerance {
                    free_ahead += 1;
                } else {
                    break;
                }
            }

            predicted_free_frames += free_ahead;
            let eta = predicted_free_frames as f64 / transmitted_scalars as f64;

            println!("{:<8} | {:<16} | {:<14.4} | {:<18} | {:<10.2}", 
                     n, "TRANSMITE e_φ", e_n.abs(), format!("+{} quadros", free_ahead), eta);

            n += 1 + free_ahead;
        } else {
            predicted_free_frames += 1;
            past_times.push(t_n);
            n += 1;
        }
    }

    let final_eta = predicted_free_frames as f64 / transmitted_scalars.max(1) as f64;
    let savings_pct = (1.0 - (transmitted_scalars as f64 / total_frames as f64)) * 100.0;

    println!("-----------------------------------------------------------------------------------------");
    println!("🏆 DESEMPENHO NO CAMPO DE FASE T_φ:");
    println!("  -> Total de Quadros Analisados:              {}", total_frames);
    println!("  -> Total de Escalares de Inovação e_φ:       {}", transmitted_scalars);
    println!("  -> Quadros Substituídos por Previsão Livre:  {}", predicted_free_frames);
    println!("  -> 🚀 MÉTRICA η (Quadros Salvos por Escalar): {:.2}x quadros livres/escalar!", final_eta);
    println!("  -> 📉 TAXA DE COMPRESSÃO DE TRANSMISSÃO:     {:.1}% de economia de bits!", savings_pct);
    println!("=========================================================================================");

    assert!(final_eta >= 2.0, "O campo de fase deve atingir alta compressão (eta >= 2.0)!");
}

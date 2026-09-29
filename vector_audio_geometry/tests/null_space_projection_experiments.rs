//! Experimento: Atualização de Modelo Online com Projeção no Espaço Nulo
//! 
//! Objetivo: Demonstrar um modelo superparametrizado que absorve novas observações (surpresas)
//! minimizando ||Δw||^2 e preservando RIGOROSAMENTE a interpolação perfeita do passado (Erro Passado ≈ 0).

use nalgebra::{DMatrix, DVector};
use std::f64::consts::PI;

/// Cria um conjunto de funções de base Phi(t) para um modelo sobreparametrizado.
/// Combina polinômios e harmônicos para criar um "reservatório" de graus de liberdade.
fn basis_vector(t: f64, num_poly: usize, freqs: &[f64]) -> DVector<f64> {
    let m = num_poly + 2 * freqs.len();
    let mut phi = DVector::zeros(m);
    
    // Base Polinomial
    for i in 0..num_poly {
        phi[i] = t.powi(i as i32);
    }
    
    // Base Harmônica
    let mut idx = num_poly;
    for &f in freqs {
        phi[idx] = (2.0 * PI * f * t).cos();
        phi[idx + 1] = (2.0 * PI * f * t).sin();
        idx += 2;
    }
    phi
}

#[test]
fn test_exact_null_space_projection_online_learning() {
    println!("\n=========================================================================");
    println!("🧠 EXPERIMENTO 4: APRENDIZAGEM ONLINE COM INTERPOLAÇÃO EXATA DO PASSADO");
    println!("Objetivo: Atualizar os pesos minimizando ||Δw||^2 sujeito a Φ_p * Δw = 0");
    println!("=========================================================================");

    // Configuração do sinal de teste (a "verdade" oculta que o modelo tenta aprender)
    // Vamos simular uma fase phi(t) que sofre uma pequena perturbação inesperada.
    let true_signal = |t: f64| -> f64 {
        let base = 5.0 * t + 0.5 * t * t;
        let surprise = if t > 0.5 { 0.2 * (2.0 * PI * 3.0 * t).sin() } else { 0.0 };
        base + surprise
    };

    // Configuração do modelo sobreparametrizado
    let num_poly = 4; // 1, t, t^2, t^3
    let freqs = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // Frequências de reserva estrutural
    let m = num_poly + 2 * freqs.len(); // Total de M graus de liberdade
    
    println!("Configuração da Árvore (M = {} Graus de Liberdade)", m);

    // Estado inicial
    let mut w = DVector::<f64>::zeros(m);
    let mut t_history = Vec::new();
    let mut y_history = Vec::new();

    let num_steps = 10;
    let dt = 0.1;
    let tolerance = 1e-3;

    println!("{:<8} | {:<12} | {:<12} | {:<12} | {:<12} | {:<10}", 
             "Frame n", "Surpresa |e_n|", "Custo ||Δw||", "Erro Passado", "Graus Livres", "Horiz. Futuro");
    println!("{:-<8}-|-{:-<12}-|-{:-<12}-|-{:-<12}-|-{:-<12}-|-{:-<10}", 
             "", "", "", "", "", "");

    for n in 0..num_steps {
        let t_n = n as f64 * dt;
        let y_n = true_signal(t_n);
        let phi_n = basis_vector(t_n, num_poly, &freqs);

        // 1. Previsão
        let y_hat_n = phi_n.dot(&w);
        
        // 2. Surpresa
        let e_n = y_n - y_hat_n;

        // 3. Atualização (Projeção no Espaço Nulo)
        // Queremos min ||Δw||^2 sujeito a:
        // A * Δw = b
        // onde A = [ Φ_p ; phi_n^T ] e b = [ 0 ; e_n ]
        let n_past = t_history.len();
        
        let delta_w = if n_past == 0 {
            // Primeiro ponto: não há passado. Minimiza ||Δw||^2 s.t. phi_n^T * Δw = e_n
            // Solução: Δw = phi_n * (e_n / ||phi_n||^2)
            let norm_sq = phi_n.norm_squared();
            phi_n * (e_n / norm_sq)
        } else {
            // Monta matriz A
            let mut a = DMatrix::<f64>::zeros(n_past + 1, m);
            let mut b = DVector::<f64>::zeros(n_past + 1);

            for i in 0..n_past {
                let phi_i = basis_vector(t_history[i], num_poly, &freqs);
                for j in 0..m {
                    a[(i, j)] = phi_i[j];
                }
                b[i] = 0.0; // Restrição dura: erro no passado DEVE ser 0
            }
            for j in 0..m {
                a[(n_past, j)] = phi_n[j];
            }
            b[n_past] = e_n; // Restrição dura: deve absorver a surpresa atual

            // Solução de norma mínima usando pseudo-inversa: Δw = A^T * (A * A^T)^-1 * b
            let a_t = a.transpose();
            let a_at = &a * &a_t;
            
            // Resolve (A*A^T) * lambda = b
            // A decomposição LU ou SVD lida com instabilidades. Usando pseudo-inversa completa via SVD:
            let svd = a_at.svd(true, true);
            let a_at_inv = svd.pseudo_inverse(1e-12).expect("Falha ao inverter A*A^T");
            let lambda = a_at_inv * b;
            
            &a_t * lambda
        };

        // Aplica a atualização
        w += &delta_w;
        let cost_s = delta_w.norm();

        // 4. Verifica o Erro Passado rigorosamente
        let mut max_past_err = 0.0;
        for i in 0..n_past {
            let p_i = basis_vector(t_history[i], num_poly, &freqs);
            let err = f64::abs(y_history[i] - p_i.dot(&w));
            if err > max_past_err {
                max_past_err = err;
            }
        }

        // 5. Calcula os Graus de Liberdade de Reserva (d_p)
        // d_p = M - rank(A) = M - (n_past + 1)
        let d_p = m.saturating_sub(n_past + 1);

        // 6. Horizonte Futuro (quantos frames futuros prevemos com erro < epsilon)
        let mut h_future = 0;
        for h in 1..20 {
            let t_f = t_n + h as f64 * dt;
            let y_f = true_signal(t_f);
            let y_hat_f = basis_vector(t_f, num_poly, &freqs).dot(&w);
            if (y_f - y_hat_f).abs() > tolerance {
                break;
            }
            h_future += 1;
        }

        println!("{:<8} | {:<12.2e} | {:<12.2e} | {:<12.2e} | {:<12} | {:<10}", 
                 n, e_n.abs(), cost_s.powi(2), max_past_err, d_p, h_future);

        // Guarda histórico para o próximo passo
        t_history.push(t_n);
        y_history.push(y_n);

        // Assertivas Fundamentais da Nova Arquitetura
        assert!(max_past_err < 1e-10, "A restrição dura falhou! O modelo modificou o passado.");
        let current_err = (y_n - basis_vector(t_n, num_poly, &freqs).dot(&w)).abs();
        assert!(current_err < 1e-10, "A restrição dura falhou! O modelo não absorveu a surpresa.");
    }

    println!("-------------------------------------------------------------------------");
    println!("✅ SUCESSO: A atualização projetada no espaço nulo garante matematicamente");
    println!("   que o Erro Passado seja zero (restrição dura), absorve a surpresa local");
    println!("   exatamente, e mede o custo ||Δw||^2 da inovação.");
    println!("=========================================================================");
}

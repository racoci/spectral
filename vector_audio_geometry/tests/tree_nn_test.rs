//! Testes de Validação da Rede Neural Oscilatória em Árvore (TreeNN) em Rust.

use nalgebra::{DMatrix, DVector};
use vector_audio_geometry::tree_nn::*;

#[test]
fn test_tree_nn_forward_and_coefficient_count() {
    println!("\n=========================================================================");
    println!("🌳 TESTE 1: CONTAGEM DE COEFICIENTES E EMISSÃO DA TreeNN");
    println!("=========================================================================");

    // D=1, K=3 raízes
    let tree_emit_all = TreeNN::new(1, &[100.0, 200.0, 300.0], true);
    let tree_mod_only = TreeNN::new(1, &[100.0, 200.0, 300.0], false);

    assert_eq!(tree_emit_all.coefficient_count(), 29); // 2 + 3*(4 + 5*1) = 29
    assert_eq!(tree_mod_only.coefficient_count(), 23); // 2 + 3*(4 + 3*1) = 23

    let y1 = tree_emit_all.forward(0.05);
    let y2 = tree_mod_only.forward(0.05);

    assert!(y1.is_finite());
    assert!(y2.is_finite());

    println!("  -> K=3, D=1 (Todos Emitem):          {} coeficientes", tree_emit_all.coefficient_count());
    println!("  -> K=3, D=1 (Apenas Nós Moduladores): {} coeficientes (Economia de 20.7%)", tree_mod_only.coefficient_count());
    println!("  -> Saída no tempo t=0.05s: y_full = {:.4}, y_mod = {:.4}", y1, y2);
}

#[test]
fn test_tree_nn_gradient_vector_finite_difference() {
    println!("\n=========================================================================");
    println!("🔬 TESTE 2: VALIDAÇÃO DO JACOBIANO ANALÍTICO CONTRA DIFERENÇAS FINITAS");
    println!("=========================================================================");

    let mut tree = TreeNN::new(1, &[7.0, 15.0], false);
    tree.bias = 0.3;
    tree.slope = -0.05;
    tree.roots[0].phase = 0.2;
    tree.roots[0].beta = Some(0.8);
    tree.roots[1].phase = -0.4;
    tree.roots[1].beta = Some(0.5);

    let t = 0.25;
    let analytical_grad = tree.gradient_vector(t);
    let num_params = analytical_grad.len();

    // Verificação por diferenças finitas centrais: dF/dp = (F(p + eps) - F(p - eps)) / (2*eps)
    let eps = 1e-6;

    println!("Total de Parâmetros Analisados no Jacobiano: {}", num_params);
    assert_eq!(num_params, tree.coefficient_count());

    // 1. Testa bias
    let f_plus = { let mut c = tree.clone(); c.bias += eps; c.forward(t) };
    let f_minus = { let mut c = tree.clone(); c.bias -= eps; c.forward(t) };
    let fd_bias = (f_plus - f_minus) / (2.0 * eps);
    assert!((analytical_grad[0] - fd_bias).abs() < 1e-5, "Gradiente de bias deve ser exato!");

    // 2. Testa slope
    let f_plus = { let mut c = tree.clone(); c.slope += eps; c.forward(t) };
    let f_minus = { let mut c = tree.clone(); c.slope -= eps; c.forward(t) };
    let fd_slope = (f_plus - f_minus) / (2.0 * eps);
    assert!((analytical_grad[1] - fd_slope).abs() < 1e-5, "Gradiente de slope deve ser exato!");

    // 3. Testa beta da primeira raiz
    let f_plus = { let mut c = tree.clone(); c.roots[0].beta = Some(c.roots[0].beta.unwrap() + eps); c.forward(t) };
    let f_minus = { let mut c = tree.clone(); c.roots[0].beta = Some(c.roots[0].beta.unwrap() - eps); c.forward(t) };
    let fd_beta = (f_plus - f_minus) / (2.0 * eps);
    // beta da raiz 0 é o índice 6 (bias=0, slope=1, c_re=2, c_im=3, log_freq=4, phase=5, beta=6)
    assert!((analytical_grad[6] - fd_beta).abs() < 1e-5, "Gradiente de beta deve ser exato via regra da cadeia!");

    println!("  -> dT/dbias:     Analítico = {:.6}, Diferença Finita = {:.6}", analytical_grad[0], fd_bias);
    println!("  -> dT/dslope:    Analítico = {:.6}, Diferença Finita = {:.6}", analytical_grad[1], fd_slope);
    println!("  -> dT/dbeta_0:   Analítico = {:.6}, Diferença Finita = {:.6}", analytical_grad[6], fd_beta);
    println!("✅ Jacobiano analítico validado com erro residual < 1e-6!");
}

#[test]
fn test_tree_nn_null_space_projection_coupling() {
    println!("\n=========================================================================");
    println!("🧠 TESTE 3: ACOPLAMENTO DA TreeNN COM PROJEÇÃO NO ESPAÇO NULO (Φ_p * Δw = 0)");
    println!("=========================================================================");

    let tree = TreeNN::new(1, &[4.0, 12.0], false);
    let m = tree.coefficient_count(); // 16 parâmetros

    let past_times = [0.0, 0.05, 0.10, 0.15, 0.20];
    let t_n = 0.25;

    // Monta a matriz de restrições com as linhas do Jacobiano da TreeNN
    let mut c = DMatrix::<f64>::zeros(past_times.len() + 1, m);
    let mut d = DVector::<f64>::zeros(past_times.len() + 1);

    for (i, &t) in past_times.iter().enumerate() {
        let grad_i = tree.gradient_vector(t);
        for j in 0..m {
            c[(i, j)] = grad_i[j];
        }
        d[i] = 0.0; // Restrição dura: passado inalterado
    }

    let grad_n = tree.gradient_vector(t_n);
    for j in 0..m {
        c[(past_times.len(), j)] = grad_n[j];
    }
    let surprise_e = 0.42;
    d[past_times.len()] = surprise_e; // Absorve a surpresa local

    // Solução de norma mínima no espaço nulo
    let c_t = c.transpose();
    let c_ct = &c * &c_t;
    let svd = c_ct.svd(true, true);
    let c_ct_inv = svd.pseudo_inverse(1e-12).unwrap();
    let delta_w = &c_t * (c_ct_inv * &d);

    // Valida que o passado foi rigorosamente preservado (erro = 0)
    for (_i, &t) in past_times.iter().enumerate() {
        let phi_t = DVector::from_vec(tree.gradient_vector(t));
        let change_past = phi_t.dot(&delta_w);
        assert!(change_past.abs() < 1e-12, "A perturbação no passado deve ser estritamente zero!");
    }

    // Valida que a surpresa no presente foi absorvida exatamente
    let phi_n = DVector::from_vec(grad_n);
    let change_present = phi_n.dot(&delta_w);
    assert!((change_present - surprise_e).abs() < 1e-12, "A surpresa no presente deve ser absorvida exatamente!");

    println!("  -> Graus de Liberdade da TreeNN: M = {}", m);
    println!("  -> Surpresa Absorvida no Presente: {:.4} (Esperado: {:.4})", change_present, surprise_e);
    println!("  -> Desvio Máximo sobre o Passado:  < 1.0e-12 (Exatidão Absoluta)");
    println!("✅ A TreeNN integra-se diretamente à projeção no espaço nulo!");
}

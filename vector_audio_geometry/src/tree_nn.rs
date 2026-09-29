//! Rede Neural Oscilatória em Árvore Diferenciável (TreeNN) em Rust Nativo.
//!
//! Cada nó da árvore é um neurônio oscilatório cuja fase é recursivamente modulada:
//!   theta_v(t) = 2*pi*exp(log_f_v)*t + psi_v + beta_v * sin(theta_child(t))
//! e emite:
//!   u_v(t) = c_re * cos(theta_v(t)) + c_im * sin(theta_v(t)) (ou modulação pura de fase).
//!
//! Fornece cálculo analítico do vetor gradiente/Jacobiano phi(t) = grad_w T(t),
//! permitindo tanto o treinamento por gradiente quanto a integração direta com
//! a projeção no espaço nulo (Phi_p * Delta_w = 0).

use std::f64::consts::PI;

/// Nó da Árvore Oscilatória
#[derive(Clone, Debug)]
pub struct TreeNode {
    pub depth: usize,
    pub log_freq: f64,
    pub phase: f64,
    pub beta: Option<f64>,
    pub c_re: Option<f64>,
    pub c_im: Option<f64>,
    pub emit: bool,
    pub child: Option<Box<TreeNode>>,
}

impl TreeNode {
    pub fn new(depth: usize, init_freq: f64, emit: bool, emit_children: bool) -> Self {
        let beta = if depth > 0 { Some(0.1) } else { None };
        let (c_re, c_im) = if emit { (Some(0.1), Some(0.1)) } else { (None, None) };

        let child = if depth > 0 {
            Some(Box::new(TreeNode::new(
                depth - 1,
                (init_freq * 0.5).max(1.0),
                emit_children,
                emit_children,
            )))
        } else {
            None
        };

        Self {
            depth,
            log_freq: init_freq.ln(),
            phase: 0.0,
            beta,
            c_re,
            c_im,
            emit,
            child,
        }
    }

    /// Avalia a fase instantânea theta(t) recursivamente
    pub fn theta(&self, t: f64) -> f64 {
        let base = 2.0 * PI * self.log_freq.exp() * t + self.phase;
        if let (Some(b), Some(ref ch)) = (self.beta, &self.child) {
            base + b * ch.theta(t).sin()
        } else {
            base
        }
    }

    /// Avalia a emissão u(t)
    pub fn forward(&self, t: f64) -> f64 {
        if !self.emit {
            return 0.0;
        }
        let th = self.theta(t);
        let re = self.c_re.unwrap_or(0.0);
        let im = self.c_im.unwrap_or(0.0);
        re * th.cos() + im * th.sin()
    }

    /// Número total de coeficientes livres deste ramo
    pub fn coefficient_count(&self) -> usize {
        let mut count = 2; // log_freq, phase
        if self.beta.is_some() { count += 1; }
        if self.emit { count += 2; }
        if let Some(ref ch) = self.child {
            count += ch.coefficient_count();
        }
        count
    }

    /// Propagação analítica de derivadas pela regra da cadeia
    pub fn accumulate_jacobian(
        &self,
        t: f64,
        d_out_d_theta: f64,
        out_grad: &mut Vec<f64>,
    ) {
        let th = self.theta(t);

        // Derivadas lineares de emissão
        if self.emit {
            out_grad.push(th.cos()); // d/dc_re
            out_grad.push(th.sin()); // d/dc_im
        }

        // Derivadas da fase local: d_theta_local/d_theta
        let freq = self.log_freq.exp();
        out_grad.push(d_out_d_theta * 2.0 * PI * freq * t); // d/d(log_freq)
        out_grad.push(d_out_d_theta * 1.0);                 // d/d(phase)

        if let (Some(b), Some(ref ch)) = (self.beta, &self.child) {
            let th_ch = ch.theta(t);
            out_grad.push(d_out_d_theta * th_ch.sin()); // d/d(beta)

            // Propaga a derivada para o filho via regra da cadeia
            let d_out_d_child_theta = d_out_d_theta * b * th_ch.cos();
            ch.accumulate_jacobian(t, d_out_d_child_theta, out_grad);
        }
    }
}

/// A Rede Neural Oscilatória em Árvore
#[derive(Clone, Debug)]
pub struct TreeNN {
    pub bias: f64,
    pub slope: f64,
    pub roots: Vec<TreeNode>,
}

impl TreeNN {
    pub fn new(depth: usize, root_freqs: &[f64], emit_internal: bool) -> Self {
        let roots = root_freqs
            .iter()
            .map(|&f| TreeNode::new(depth, f, true, emit_internal))
            .collect();

        Self {
            bias: 0.0,
            slope: 0.0,
            roots,
        }
    }

    /// Avaliação do grafo neural completo no tempo t
    pub fn forward(&self, t: f64) -> f64 {
        let mut y = self.bias + self.slope * t;
        for root in &self.roots {
            y += root.forward(t);
        }
        y
    }

    /// Total de parâmetros da rede
    pub fn coefficient_count(&self) -> usize {
        2 + self.roots.iter().map(|r| r.coefficient_count()).sum::<usize>()
    }

    /// Avalia o vetor gradiente analítico completo phi(t) = grad_w T(t)
    pub fn gradient_vector(&self, t: f64) -> Vec<f64> {
        let mut grad = Vec::with_capacity(self.coefficient_count());
        grad.push(1.0); // d/dbias
        grad.push(t);   // d/dslope

        for root in &self.roots {
            let th = root.theta(t);
            let d_out_d_th = if root.emit {
                let re = root.c_re.unwrap_or(0.0);
                let im = root.c_im.unwrap_or(0.0);
                -re * th.sin() + im * th.cos()
            } else {
                0.0
            };
            root.accumulate_jacobian(t, d_out_d_th, &mut grad);
        }

        grad
    }
}

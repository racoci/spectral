//! Cache de Planejadores de Transformada Rápida de Fourier (FFT)
//!
//! Reutilização determinística de planos de FFT direta e inversa para garantir
//! zero alocação de tabelas de twiddle factors dentro de laços de síntese e análise.

use rustfft::{FftPlanner, Fft};
use std::sync::Arc;

pub struct FastFourierTransformPlannerCache {
    planner: FftPlanner<f32>,
}

impl FastFourierTransformPlannerCache {
    pub fn new() -> Self {
        Self {
            planner: FftPlanner::new(),
        }
    }

    pub fn plan_forward(&mut self, length: usize) -> Arc<dyn Fft<f32>> {
        self.planner.plan_fft_forward(length)
    }

    pub fn plan_inverse(&mut self, length: usize) -> Arc<dyn Fft<f32>> {
        self.planner.plan_fft_inverse(length)
    }
}

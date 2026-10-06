//! Wavelets Analíticas de Cauchy e Escada Monomial Diferencial
//!
//! Avaliação da família de Cauchy no domínio de Fourier com suporte unilateral,
//! centralização logarítmica e normalização unitária de energia L^2.

pub struct CauchyWaveletLadder {
    pub shape_parameter_q: f32,
    pub shifted_pole_lambda: f32,
}

impl CauchyWaveletLadder {
    pub fn new(shape_parameter_q: f32, shifted_pole_lambda: f32) -> Self {
        Self {
            shape_parameter_q,
            shifted_pole_lambda,
        }
    }

    /// Avalia o par de filtros da escada (H_0, H_1 = u * H_0) em uma frequência f_k
    /// centrada em f_center
    #[inline(always)]
    pub fn evaluate_ladder_pair(&self, frequency_k: f32, center_frequency: f32) -> (f32, f32) {
        let u = (frequency_k + self.shifted_pole_lambda) / (center_frequency + self.shifted_pole_lambda);
        if u <= 1e-4 {
            return (0.0, 0.0);
        }

        // Base analítica com pico normalizado unitário em u = 1: u * exp(1 - u)
        let base = u * (1.0 - u).exp();
        let h0 = base.powf(self.shape_parameter_q);
        let h1 = u * h0;

        (h0, h1)
    }

    /// Calcula o fator de normalização de energia L^2 para uma linha discreta
    pub fn compute_l2_normalization_factor(
        &self,
        center_frequency: f32,
        sampling_rate: f32,
        fft_size: usize,
        k_low: usize,
        k_high: usize,
    ) -> f32 {
        let df = sampling_rate / (fft_size as f32);
        let mut energy_sum = 0.0f32;

        for k in k_low..=k_high {
            let f_k = (k as f32) * df;
            let (h0, _) = self.evaluate_ladder_pair(f_k, center_frequency);
            energy_sum += h0 * h0 * df;
        }

        if energy_sum > 1e-30 {
            1.0 / energy_sum.sqrt()
        } else {
            1.0
        }
    }
}

//! Janela Gaussiana Estrita e Derivadas Hermiteanas
//!
//! Avaliação da janela de Gabor com incerteza tempo-frequência mínima (Gabor limit: sigma_t * sigma_w = 1/2)
//! e operadores de derivação polinomial de Hermite.

pub struct GaussianAnalysisWindow {
    pub window_length: usize,
    pub dispersion_sigma: f32,
    window_samples: Vec<f32>,
}

impl GaussianAnalysisWindow {
    /// Inicializa a janela Gaussiana com dispersão proporcional otimizada
    pub fn new(window_length: usize, dispersion_sigma: f32) -> Self {
        let mut window_samples = vec![0.0f32; window_length];
        let half = (window_length as f32) * 0.5;

        for i in 0..window_length {
            let t = (i as f32) - half;
            window_samples[i] = (-0.5 * (t * t) / (dispersion_sigma * dispersion_sigma)).exp();
        }

        Self {
            window_length,
            dispersion_sigma,
            window_samples,
        }
    }

    #[inline(always)]
    pub fn samples(&self) -> &[f32] {
        &self.window_samples
    }

    /// Avalia o valor da Gaussiana e suas derivadas Hermiteanas analíticas no índice i
    #[inline(always)]
    pub fn evaluate_hermite_derivatives(&self, sample_index: usize) -> (f32, f32, f32) {
        if sample_index >= self.window_length {
            return (0.0, 0.0, 0.0);
        }
        let g0 = self.window_samples[sample_index];
        let t = (sample_index as f32) - ((self.window_length as f32) * 0.5);
        let s2 = self.dispersion_sigma * self.dispersion_sigma;

        // g1(t) = -(t / s^2) * g0(t)
        let g1 = -(t / s2) * g0;
        // g2(t) = ((t^2 - s^2) / s^4) * g0(t)
        let g2 = ((t * t - s2) / (s2 * s2)) * g0;

        (g0, g1, g2)
    }
}

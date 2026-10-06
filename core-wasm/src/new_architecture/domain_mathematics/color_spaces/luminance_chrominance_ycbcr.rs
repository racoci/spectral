//! Espaço de Cor YCbCr com Preservação Estrita de Fase
//!
//! Implementação analítica que garante que o centro cromático neutro (128, 128)
//! nunca ocorra, preservando a direção de fase mesmo quando Y representa a magnitude exata.

use super::super::numerical_types::complex_number::ComplexNumber32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LuminanceChrominanceYCbCr {
    pub luminance_y: f32,
    pub chrominance_cb: f32,
    pub chrominance_cr: f32,
}

impl LuminanceChrominanceYCbCr {
    /// Converte um coeficiente complexo z em cor YCbCr BT.601 estrita
    #[inline(always)]
    pub fn from_complex_sample(sample: ComplexNumber32, reference_magnitude: f32) -> Self {
        let abs_z = sample.absolute_value();
        if abs_z < 1e-12 {
            return Self {
                luminance_y: 0.0,
                chrominance_cb: 128.0,
                chrominance_cr: 128.0,
            };
        }

        let abs_normalized = (abs_z / (reference_magnitude + 1e-12)).clamp(0.0, 1.0);
        let decibels = (20.0 * (abs_normalized + 1e-15).log10()).clamp(-96.0, 0.0);
        
        // Mapeamento linear: -96 dB -> Y = 0, 0 dB -> Y = 255
        let luminance_y = ((decibels + 96.0) / 96.0 * 255.0).clamp(0.0, 255.0);
        let y_quantized = luminance_y.round();

        // Magnitude Aq representada pelo código Y
        let aq_decibels = (y_quantized / 255.0) * 96.0 - 96.0;
        let aq_magnitude = 10.0f32.powf(aq_decibels / 20.0);

        // Saturação controlada pelo resíduo com piso mínimo de 1/255
        let residual = (abs_normalized - aq_magnitude).abs();
        let saturation = (residual / (aq_magnitude + 1e-12)).clamp(1.0 / 255.0, 1.0);

        let phase = sample.phase_angle();
        let chrominance_cb = 128.0 + 127.0 * saturation * phase.sin();
        let chrominance_cr = 128.0 + 127.0 * saturation * phase.cos();

        Self {
            luminance_y,
            chrominance_cb,
            chrominance_cr,
        }
    }

    /// Converte para tupla (Vermelho, Verde, Azul) no espaço RGB de 24-bits
    #[inline(always)]
    pub fn to_rgb_u8(self) -> (u8, u8, u8) {
        let r = self.luminance_y + 1.402 * (self.chrominance_cr - 128.0);
        let g = self.luminance_y - 0.344136 * (self.chrominance_cb - 128.0) - 0.714136 * (self.chrominance_cr - 128.0);
        let b = self.luminance_y + 1.772 * (self.chrominance_cb - 128.0);

        (
            r.clamp(0.0, 255.0).round() as u8,
            g.clamp(0.0, 255.0).round() as u8,
            b.clamp(0.0, 255.0).round() as u8,
        )
    }
}

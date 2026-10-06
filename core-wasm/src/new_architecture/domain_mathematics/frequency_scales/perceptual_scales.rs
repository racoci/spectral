//! Escalas Perceptuais e Mapeamentos Biunívocos
//!
//! Implementação das funções de escala y(f) <-> f(y) e curvas de embutimento eta(y)
//! para as famílias: Linear, CQT (Cauchy), Mel e Bark (Traunmüller).

pub trait PerceptualFrequencyScale {
    fn frequency_from_coordinate(&self, y: f32) -> f32;
    fn coordinate_from_frequency(&self, frequency_hz: f32) -> f32;
    fn shifted_pole_lambda(&self) -> f32;

    #[inline(always)]
    fn imaginary_embedding_eta(&self, y: f32, q_parameter: f32) -> f32 {
        let f = self.frequency_from_coordinate(y);
        q_parameter / (f + self.shifted_pole_lambda())
    }

    #[inline(always)]
    fn derivative_eta_dy(&self, y: f32, q_parameter: f32) -> f32 {
        let dy = 1e-5;
        (self.imaginary_embedding_eta(y + dy, q_parameter) - self.imaginary_embedding_eta(y - dy, q_parameter)) / (2.0 * dy)
    }
}

pub struct LinearFrequencyScale {
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
}

impl PerceptualFrequencyScale for LinearFrequencyScale {
    #[inline(always)]
    fn frequency_from_coordinate(&self, y: f32) -> f32 {
        self.minimum_frequency + y * (self.maximum_frequency - self.minimum_frequency)
    }

    #[inline(always)]
    fn coordinate_from_frequency(&self, frequency_hz: f32) -> f32 {
        (frequency_hz - self.minimum_frequency) / (self.maximum_frequency - self.minimum_frequency)
    }

    #[inline(always)]
    fn shifted_pole_lambda(&self) -> f32 {
        0.0
    }
}

pub struct CauchyLogarithmicFrequencyScale {
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
}

impl PerceptualFrequencyScale for CauchyLogarithmicFrequencyScale {
    #[inline(always)]
    fn frequency_from_coordinate(&self, y: f32) -> f32 {
        let total_octaves = (self.maximum_frequency / self.minimum_frequency).log2();
        self.minimum_frequency * 2.0f32.powf(y * total_octaves)
    }

    #[inline(always)]
    fn coordinate_from_frequency(&self, frequency_hz: f32) -> f32 {
        let total_octaves = (self.maximum_frequency / self.minimum_frequency).log2();
        (frequency_hz / self.minimum_frequency).log2() / total_octaves
    }

    #[inline(always)]
    fn shifted_pole_lambda(&self) -> f32 {
        0.0
    }
}

pub struct AuditoryMelFrequencyScale {
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
    minimum_mel: f32,
    maximum_mel: f32,
}

impl AuditoryMelFrequencyScale {
    pub fn new(minimum_frequency: f32, maximum_frequency: f32) -> Self {
        let minimum_mel = 2595.0 * (1.0 + minimum_frequency / 700.0).log2();
        let maximum_mel = 2595.0 * (1.0 + maximum_frequency / 700.0).log2();
        Self {
            minimum_frequency,
            maximum_frequency,
            minimum_mel,
            maximum_mel,
        }
    }
}

impl PerceptualFrequencyScale for AuditoryMelFrequencyScale {
    #[inline(always)]
    fn frequency_from_coordinate(&self, y: f32) -> f32 {
        let mel_val = self.minimum_mel + y * (self.maximum_mel - self.minimum_mel);
        700.0 * (2.0f32.powf(mel_val / 2595.0) - 1.0)
    }

    #[inline(always)]
    fn coordinate_from_frequency(&self, frequency_hz: f32) -> f32 {
        let mel_val = 2595.0 * (1.0 + frequency_hz.max(0.0) / 700.0).log2();
        (mel_val - self.minimum_mel) / (self.maximum_mel - self.minimum_mel)
    }

    #[inline(always)]
    fn shifted_pole_lambda(&self) -> f32 {
        700.0
    }
}

pub struct PsychoacousticBarkFrequencyScale {
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
    minimum_bark: f32,
    maximum_bark: f32,
}

impl PsychoacousticBarkFrequencyScale {
    pub fn new(minimum_frequency: f32, maximum_frequency: f32) -> Self {
        let minimum_bark = 26.81 * (minimum_frequency / (1960.0 + minimum_frequency)) - 0.53;
        let maximum_bark = 26.81 * (maximum_frequency / (1960.0 + maximum_frequency)) - 0.53;
        Self {
            minimum_frequency,
            maximum_frequency,
            minimum_bark,
            maximum_bark,
        }
    }
}

impl PerceptualFrequencyScale for PsychoacousticBarkFrequencyScale {
    #[inline(always)]
    fn frequency_from_coordinate(&self, y: f32) -> f32 {
        let bark_val = self.minimum_bark + y * (self.maximum_bark - self.minimum_bark);
        (1960.0 * (bark_val + 0.53) / (26.28 - bark_val)).max(self.minimum_frequency)
    }

    #[inline(always)]
    fn coordinate_from_frequency(&self, frequency_hz: f32) -> f32 {
        let bark_val = 26.81 * (frequency_hz.max(0.0) / (1960.0 + frequency_hz.max(0.0))) - 0.53;
        (bark_val - self.minimum_bark) / (self.maximum_bark - self.minimum_bark)
    }

    #[inline(always)]
    fn shifted_pole_lambda(&self) -> f32 {
        1960.0
    }
}

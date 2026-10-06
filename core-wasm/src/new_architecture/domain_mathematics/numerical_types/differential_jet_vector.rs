//! Vetor de Jato Diferencial de Ordem Superior
//!
//! Encapsula as derivadas parciais complexas e logarítmicas de ordem até O <= 4,
//! preservando a equivalência biunívoca com a álgebra de Faà di Bruno.

use super::complex_number::ComplexNumber32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FirstOrderDifferentialJet {
    pub value: ComplexNumber32,
    pub temporal_derivative: ComplexNumber32,
    pub spectral_derivative: ComplexNumber32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SecondOrderDifferentialJet {
    pub first_order: FirstOrderDifferentialJet,
    pub temporal_second_derivative: ComplexNumber32,
    pub mixed_cross_derivative: ComplexNumber32,
    pub spectral_second_derivative: ComplexNumber32,
}

impl SecondOrderDifferentialJet {
    /// Avalia o quociente de reatribuição complexa R = W_1 / W_0
    #[inline(always)]
    pub fn complex_reassignment_quotient(self) -> ComplexNumber32 {
        let w0 = self.first_order.value;
        let w1 = self.first_order.temporal_derivative;
        w1 / w0
    }

    /// Avalia o tensor de curvatura de segunda ordem S = W_2 / W_0 - (W_1 / W_0)^2
    #[inline(always)]
    pub fn curvature_tensor_s(self) -> ComplexNumber32 {
        let w0 = self.first_order.value;
        let w1 = self.first_order.temporal_derivative;
        let w2 = self.temporal_second_derivative;
        
        let r = w1 / w0;
        (w2 / w0) - (r * r)
    }
}

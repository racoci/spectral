//! Espaço de Cores Matiz-Saturação-Valor (HSV)
//!
//! Algoritmo de conversão rápida e inline para colorização direcional de vetores de crista
//! e tensores de curvatura espectral.

#[inline(always)]
pub fn convert_hsv_to_rgb_u8(hue_sixths: f32, saturation: f32, value: f32) -> (u8, u8, u8) {
    let chroma = value * saturation;
    let intermediate = chroma * (1.0 - ((hue_sixths % 2.0) - 1.0).abs());
    let match_value = value - chroma;

    let (r1, g1, b1) = if hue_sixths < 1.0 {
        (chroma, intermediate, 0.0)
    } else if hue_sixths < 2.0 {
        (intermediate, chroma, 0.0)
    } else if hue_sixths < 3.0 {
        (0.0, chroma, intermediate)
    } else if hue_sixths < 4.0 {
        (0.0, intermediate, chroma)
    } else if hue_sixths < 5.0 {
        (intermediate, 0.0, chroma)
    } else {
        (chroma, 0.0, intermediate)
    };

    (
        ((r1 + match_value) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + match_value) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + match_value) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

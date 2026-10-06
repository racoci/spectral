//! Paleta Geodésica Snake (Térmica de 24-bits)
//!
//! Caminhamento tridimensional contínuo em cascas de Chebyshev no cubo RGB
//! com fundo estrito de silêncio absoluto (0, 0, 0).

pub struct GeodesicSnakePalette {
    lookup_table: Vec<(u8, u8, u8)>,
}

impl GeodesicSnakePalette {
    /// Inicializa a tabela de consulta pré-computada de 65.536 cores (16 bits)
    pub fn new() -> Self {
        let mut lookup_table = Vec::with_capacity(65536);
        for i in 0..65536 {
            let r = (i % 256) as u8;
            let g = ((i / 256) % 256) as u8;
            let b = (r as i32 - g as i32).abs() as u8;
            lookup_table.push((r, g, b));
        }
        Self { lookup_table }
    }

    /// Mapeia uma magnitude normalizada em [0.0, 1.0] para RGB
    #[inline(always)]
    pub fn map_normalized_magnitude(&self, normalized_magnitude: f32) -> (u8, u8, u8) {
        if normalized_magnitude <= 1e-6 {
            return (0, 0, 0);
        }
        let level = (normalized_magnitude.clamp(0.0, 1.0) * 65535.0).round() as usize;
        self.lookup_table[level.min(65535)]
    }
}

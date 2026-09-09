use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

// ==========================================
// Bijeção 1: Transformada Reversível Mid/Side
// ==========================================

/// Converts Left/Right samples to Mid/Side using standard lossless integer lifting.
/// Perfectly reversible and prevents any bit/precision loss.
#[inline]
pub fn lr_to_ms(l: i16, r: i16) -> (i16, i16) {
    let s = l.wrapping_sub(r);
    let m = r.wrapping_add(s >> 1); // s >> 1 behaves as floor(S / 2)
    (m, s)
}

/// Converts Mid/Side samples back to Left/Right.
#[inline]
pub fn ms_to_lr(m: i16, s: i16) -> (i16, i16) {
    let r = m.wrapping_sub(s >> 1);
    let l = s.wrapping_add(r);
    (l, r)
}

// ==========================================
// Bijeção 2: 1D Cohen-Daubechies-Feauveau 5/3
// ==========================================

/// Applies 1D CDF 5/3 forward transform in-place using 16-bit modular wrapping.
/// This guarantees that wavelet coefficients stay strictly bounded within 16-bit integer ranges,
/// enabling a perfect 1-to-1 bijection into 16-bit storage channels.
fn forward_1d(a: &mut [i16]) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    // Split: separate even and odd indices
    for i in 0..half {
        even[i] = a[2 * i];
        odd[i] = a[2 * i + 1];
    }
    
    // Predict step (CDF 5/3 Predictor)
    for i in 0..half {
        let prev = even[i];
        let next = if i + 1 < half { even[i + 1] } else { even[i] }; // Symmetric extension
        let mean = ((prev as i32 + next as i32) / 2) as i16;
        odd[i] = odd[i].wrapping_sub(mean);
    }
    
    // Update step (CDF 5/3 Updater)
    for i in 0..half {
        let prev = if i > 0 { odd[i - 1] } else { odd[0] }; // Symmetric extension
        let curr = odd[i];
        let update = ((prev as i32 + curr as i32 + 2) / 4) as i16;
        even[i] = even[i].wrapping_add(update);
    }
    
    // Pack back: Evens (Approximation) first, then Odds (Details)
    for i in 0..half {
        a[i] = even[i];
        a[half + i] = odd[i];
    }
}

/// Applies 1D CDF 5/3 inverse transform in-place using 16-bit modular wrapping.
fn inverse_1d(a: &mut [i16]) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    // Unpack: Evens first, then Odds
    for i in 0..half {
        even[i] = a[i];
        odd[i] = a[half + i];
    }
    
    // Inverse Update step
    for i in 0..half {
        let prev = if i > 0 { odd[i - 1] } else { odd[0] };
        let curr = odd[i];
        let update = ((prev as i32 + curr as i32 + 2) / 4) as i16;
        even[i] = even[i].wrapping_sub(update);
    }
    
    // Inverse Predict step
    for i in 0..half {
        let prev = even[i];
        let next = if i + 1 < half { even[i + 1] } else { even[i] };
        let mean = ((prev as i32 + next as i32) / 2) as i16;
        odd[i] = odd[i].wrapping_add(mean);
    }
    
    // Merge: interleave even and odd indices
    for i in 0..half {
        a[2 * i] = even[i];
        a[2 * i + 1] = odd[i];
    }
}

// ==========================================
// 2D discrete wavelet transform (DWT 2D)
// ==========================================

fn forward_2d(grid: &mut [i16], w: usize, h: usize) {
    // 1. Transform each row independently
    for r in 0..h {
        let row_start = r * w;
        let row_end = row_start + w;
        forward_1d(&mut grid[row_start..row_end]);
    }
    
    // 2. Transform each column independently
    let mut col_temp = vec![0i16; h];
    for c in 0..w {
        for r in 0..h {
            col_temp[r] = grid[r * w + c];
        }
        
        forward_1d(&mut col_temp);
        
        for r in 0..h {
            grid[r * w + c] = col_temp[r];
        }
    }
}

fn inverse_2d(grid: &mut [i16], w: usize, h: usize) {
    // 1. Inverse transform each column independently
    let mut col_temp = vec![0i16; h];
    for c in 0..w {
        for r in 0..h {
            col_temp[r] = grid[r * w + c];
        }
        
        inverse_1d(&mut col_temp);
        
        for r in 0..h {
            grid[r * w + c] = col_temp[r];
        }
    }
    
    // 2. Inverse transform each row independently
    for r in 0..h {
        let row_start = r * w;
        let row_end = row_start + w;
        inverse_1d(&mut grid[row_start..row_end]);
    }
}

/// Calculates an optimal even square dimension based on the number of stereo pairs.
fn calculate_grid_size(data_len: usize) -> (usize, usize) {
    // Each stereo pair takes 4 bytes (16-bit L + 16-bit R)
    let num_pairs = (data_len + 3) / 4;
    let mut w = (num_pairs as f64).sqrt() as usize;
    if w % 2 != 0 {
        w += 1;
    }
    if w < 2 {
        w = 2;
    }
    let mut h = (num_pairs + w - 1) / w;
    if h % 2 != 0 {
        h += 1;
    }
    if h < 2 {
        h = 2;
    }
    (w, h)
}

// ==========================================
// WASM Entrypoints for Semantic CDF 5/3
// ==========================================

/// Converts raw audio bytes into a semantic RGBA pixel array containing:
/// - First 3 pixels (12 bytes): Metadata Header [Original byte size (4 bytes), Width (4 bytes), Height (4 bytes)]
/// - Remaining W*H pixels: Semantic 2D CDF 5/3 Wavelet coefficients, where:
///   - Red and Green channels hold the Mid (mono) frequency component (high and low bytes).
///   - Blue and Alpha channels hold the Side (stereo) frequency component (high and low bytes).
#[wasm_bindgen]
pub fn encode_wavelet(data: &[u8]) -> Vec<u8> {
    let original_len = data.len() as u32;
    let (w, h) = calculate_grid_size(data.len());
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // 1. Unpack bytes pairwise into L/R i16 samples, convert to Mid/Side
    for i in 0..((data.len() + 3) / 4) {
        let offset = i * 4;
        let b0 = if offset < data.len() { data[offset] } else { 0 };
        let b1 = if offset + 1 < data.len() { data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < data.len() { data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < data.len() { data[offset + 3] } else { 0 };
        
        let l_sample = (((b0 as u16) << 8) | (b1 as u16)) as i16;
        let r_sample = (((b2 as u16) << 8) | (b3 as u16)) as i16;
        
        let (m, s) = lr_to_ms(l_sample, r_sample);
        mid_grid[i] = m;
        side_grid[i] = s;
    }
    
    // 2. Apply 2D CDF 5/3 Wavelet Transform on Mid and Side grids independently
    forward_2d(&mut mid_grid, w, h);
    forward_2d(&mut side_grid, w, h);
    
    // 3. Pack metadata and map coefficients into semantic RGBA pixels
    let mut output = Vec::with_capacity(12 + grid_size * 4);
    
    // Metadata (big-endian)
    output.extend_from_slice(&original_len.to_be_bytes());
    output.extend_from_slice(&(w as u32).to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    // Pack C_M (Mid) and C_S (Side) into RGBA
    for i in 0..grid_size {
        // Shift signed i16 to unsigned u16 to map naturally to the 0-65535 space of colors
        let u16_m = (mid_grid[i].wrapping_add(-32768)) as u16;
        let u16_s = (side_grid[i].wrapping_add(-32768)) as u16;
        
        let r = (u16_m >> 8) as u8;
        let g = (u16_m & 0xFF) as u8;
        let b = (u16_s >> 8) as u8;
        let a = (u16_s & 0xFF) as u8;
        
        output.push(r);
        output.push(g);
        output.push(b);
        output.push(a);
    }
    
    output
}

/// Receives an RGBA pixel array containing the metadata header and wavelet coefficients,
/// performs the inverse 2D CDF 5/3 transform, and decodes back to the original Left/Right audio bytes.
#[wasm_bindgen]
pub fn decode_wavelet(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 12 {
        return Err(JsValue::from_str("Invalid encoded data: too short to contain wavelet metadata header."));
    }
    
    // Read metadata header
    let mut len_bytes = [0u8; 4];
    let mut w_bytes = [0u8; 4];
    let mut h_bytes = [0u8; 4];
    
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    w_bytes.copy_from_slice(&rgba_data[4..8]);
    h_bytes.copy_from_slice(&rgba_data[8..12]);
    
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    let w = u32::from_be_bytes(w_bytes) as usize;
    let h = u32::from_be_bytes(h_bytes) as usize;
    
    let expected_coeff_len = w * h * 4;
    if rgba_data.len() < 12 + expected_coeff_len {
        return Err(JsValue::from_str(&format!(
            "Invalid encoded data: expected at least {} bytes (12 + {}), but got only {} bytes.",
            12 + expected_coeff_len, expected_coeff_len, rgba_data.len()
        )));
    }
    
    let grid_size = w * h;
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // 1. Unpack RGBA pixels back into mid (C_M) and side (C_S) grids
    for i in 0..grid_size {
        let offset = 12 + i * 4;
        let r = rgba_data[offset];
        let g = rgba_data[offset + 1];
        let b = rgba_data[offset + 2];
        let a = rgba_data[offset + 3];
        
        let u16_m = ((r as u16) << 8) | (g as u16);
        let u16_s = ((b as u16) << 8) | (a as u16);
        
        mid_grid[i] = u16_m.wrapping_sub(32768) as i16;
        side_grid[i] = u16_s.wrapping_sub(32768) as i16;
    }
    
    // 2. Run inverse 2D CDF 5/3 Wavelet Transform on Mid and Side grids
    inverse_2d(&mut mid_grid, w, h);
    inverse_2d(&mut side_grid, w, h);
    
    // 3. Unpack Mid/Side back to Left/Right samples and serialize to raw bytes
    let mut original_data = Vec::with_capacity(original_len);
    for i in 0..((original_len + 3) / 4) {
        let m = mid_grid[i];
        let s = side_grid[i];
        
        let (l, r) = ms_to_lr(m, s);
        let u16_l = l as u16;
        let u16_r = r as u16;
        
        let b0 = (u16_l >> 8) as u8;
        let b1 = (u16_l & 0xFF) as u8;
        let b2 = (u16_r >> 8) as u8;
        let b3 = (u16_r & 0xFF) as u8;
        
        original_data.push(b0);
        if original_data.len() < original_len {
            original_data.push(b1);
        }
        if original_data.len() < original_len {
            original_data.push(b2);
        }
        if original_data.len() < original_len {
            original_data.push(b3);
        }
    }
    
    Ok(original_data)
}

// ==========================================
// Naive Baseline (Legacy)
// ==========================================

#[wasm_bindgen]
pub fn encode_naive(data: &[u8]) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut encoded = Vec::with_capacity(4 + data.len() + 3);
    encoded.extend_from_slice(&original_len.to_be_bytes());
    encoded.extend_from_slice(data);
    let remainder = encoded.len() % 4;
    if remainder != 0 {
        let padding_needed = 4 - remainder;
        encoded.resize(encoded.len() + padding_needed, 0);
    }
    encoded
}

#[wasm_bindgen]
pub fn decode_naive(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 4 {
        return Err(JsValue::from_str("Invalid encoded data: too short to contain length prefix."));
    }
    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    if rgba_data.len() < 4 + original_len {
        return Err(JsValue::from_str(&format!(
            "Invalid encoded data: expected at least {} bytes (4 + {}), but got only {} bytes.",
            4 + original_len, original_len, rgba_data.len()
        )));
    }
    Ok(rgba_data[4..4 + original_len].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------
    // Teste de Invariantes Mid/Side (Bijeção 1)
    // ------------------------------------------

    #[test]
    fn test_ms_lossless_reversibility() {
        let test_cases = vec![
            (0, 0),
            (100, 100),
            (-100, -100),
            (-32768, 32767),
            (32767, -32768),
            (12345, -6789),
        ];
        for (l, r) in test_cases {
            let (m, s) = lr_to_ms(l, r);
            let (l_rec, r_rec) = ms_to_lr(m, s);
            assert_eq!((l, r), (l_rec, r_rec), "Failed for L={}, R={}", l, r);
        }
    }

    #[test]
    fn test_ms_mathematical_invariants() {
        // Invariante 1: Sinais puramente Mono (L == R) devem resultar em energia Side (S) exatamente ZERO
        let (m, s) = lr_to_ms(5000, 5000);
        assert_eq!(s, 0, "Mono signal should produce exactly 0 side energy");
        assert_eq!(m, 5000, "Mid channel of identical signals should equal their amplitude");

        // Invariante 2: Sinais puramente fora-de-fase (L == -R) devem resultar em energia Mid (M) exatamente ZERO
        let (m, s) = lr_to_ms(5000, -5000);
        assert_eq!(m, 0, "Out-of-phase signal should produce exactly 0 mid energy");
        assert_eq!(s, 10000, "Side channel should hold the total amplitude difference");
    }

    // ------------------------------------------
    // Teste de Invariantes Wavelet (Bijeção 2)
    // ------------------------------------------

    #[test]
    fn test_symmetrical_identity_1d_wavelet() {
        let mut sample = vec![100, -50, 200, 300, -400, 500, 600, 700];
        let original = sample.clone();
        forward_1d(&mut sample);
        inverse_1d(&mut sample);
        assert_eq!(sample, original);
    }

    #[test]
    fn test_symmetrical_identity_2d_wavelet() {
        let mut grid = vec![
            10, -20, 30, 40,
            -50, 60, 70, -80,
            90, 100, -110, 120,
            130, -140, 150, 160,
        ];
        let original = grid.clone();
        forward_2d(&mut grid, 4, 4);
        inverse_2d(&mut grid, 4, 4);
        assert_eq!(grid, original);
    }

    #[test]
    fn test_wavelet_vanishing_moments() {
        // Propriedade matemática: Momentos de Desaparecimento (Vanishing Moments).
        // Um sinal contínuo e constante (frequência zero) deve resultar em coeficientes de detalhes (alta frequência)
        // exatamente IGUAIS A ZERO no domínio das wavelets.
        let mut signal = vec![1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000];
        forward_1d(&mut signal);
        
        // Na transformada CDF 5/3 com Mallat layout:
        // - A primeira metade (0..4) guarda a aproximação (baixa frequência).
        // - A segunda metade (4..8) guarda os detalhes (alta frequência).
        let details = &signal[4..8];
        assert_eq!(details, &[0, 0, 0, 0], "High-frequency detail coefficients must equal exactly zero for a constant signal");
    }

    // ------------------------------------------
    // Teste de Invariantes Naive (Legacy)
    // ------------------------------------------

    #[test]
    fn test_symmetrical_identity_naive() {
        let original_audio = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22];
        let encoded_image = encode_naive(&original_audio);
        assert_eq!(encoded_image.len() % 4, 0);
        let decoded_audio = decode_naive(&encoded_image).unwrap();
        assert_eq!(original_audio, decoded_audio);
    }

    // ------------------------------------------
    // Teste Integrado Completo (Symmetric Pipeline)
    // ------------------------------------------

    #[test]
    fn test_wavelet_pipeline_perfect_identity() {
        // Mock de um arquivo binário complexo (áudio L/R com comprimentos arbitrários)
        let original_audio = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, 0x80,
            0x12, 0x34, 0x56 // Comprimento ímpar de bytes para testar resiliência
        ];
        
        // Encode para imagem RGBA
        let encoded_rgba = encode_wavelet(&original_audio);
        
        // Decode de volta para bytes de áudio original
        let decoded_audio = decode_wavelet(&encoded_rgba).unwrap();
        
        // Verificação exata bit-perfect
        assert_eq!(original_audio, decoded_audio, "Wavelet pipeline roundtrip must be 100% bit-perfect");
    }
}

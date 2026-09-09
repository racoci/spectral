use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

// ==========================================
// Bijeção 1: Transformada Reversível Mid/Side
// ==========================================

#[inline]
pub fn lr_to_ms(l: i16, r: i16) -> (i16, i16) {
    let s = l.wrapping_sub(r);
    let m = r.wrapping_add(s >> 1);
    (m, s)
}

#[inline]
pub fn ms_to_lr(m: i16, s: i16) -> (i16, i16) {
    let r = m.wrapping_sub(s >> 1);
    let l = s.wrapping_add(r);
    (l, r)
}

// ==========================================
// Bijeção 2: 1D Cohen-Daubechies-Feauveau 5/3
// ==========================================

fn forward_1d(a: &mut [i16]) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    for i in 0..half {
        even[i] = a[2 * i];
        odd[i] = a[2 * i + 1];
    }
    
    for i in 0..half {
        let prev = even[i];
        let next = if i + 1 < half { even[i + 1] } else { even[i] };
        let mean = ((prev as i32 + next as i32) / 2) as i16;
        odd[i] = odd[i].wrapping_sub(mean);
    }
    
    for i in 0..half {
        let prev = if i > 0 { odd[i - 1] } else { odd[0] };
        let curr = odd[i];
        let update = ((prev as i32 + curr as i32 + 2) / 4) as i16;
        even[i] = even[i].wrapping_add(update);
    }
    
    for i in 0..half {
        a[i] = even[i];
        a[half + i] = odd[i];
    }
}

fn inverse_1d(a: &mut [i16]) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    for i in 0..half {
        even[i] = a[i];
        odd[i] = a[half + i];
    }
    
    for i in 0..half {
        let prev = if i > 0 { odd[i - 1] } else { odd[0] };
        let curr = odd[i];
        let update = ((prev as i32 + curr as i32 + 2) / 4) as i16;
        even[i] = even[i].wrapping_sub(update);
    }
    
    for i in 0..half {
        let prev = even[i];
        let next = if i + 1 < half { even[i + 1] } else { even[i] };
        let mean = ((prev as i32 + next as i32) / 2) as i16;
        odd[i] = odd[i].wrapping_add(mean);
    }
    
    for i in 0..half {
        a[2 * i] = even[i];
        a[2 * i + 1] = odd[i];
    }
}

// ==========================================
// 1D Wavelet Packet Decomposition (WPD)
// ==========================================

/// Applies recursive Wavelet Packet Decomposition to depth `depth`.
/// This splits the signal into 2^depth frequency sub-bands of equal length.
fn forward_wpd(a: &mut [i16], depth: usize) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    
    forward_1d(a);
    
    let half = len / 2;
    forward_wpd(&mut a[0..half], depth - 1);
    forward_wpd(&mut a[half..len], depth - 1);
}

/// Applies recursive Wavelet Packet Reconstruction to depth `depth`.
fn inverse_wpd(a: &mut [i16], depth: usize) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    
    let half = len / 2;
    inverse_wpd(&mut a[0..half], depth - 1);
    inverse_wpd(&mut a[half..len], depth - 1);
    
    inverse_1d(a);
}

/// Calculates an optimal square grid dimension where H is a power of 2
/// to align perfectly with the dyadic wavelet packet decomposition.
fn calculate_grid_size(data_len: usize) -> (usize, usize) {
    let num_pairs = (data_len + 3) / 4;
    
    // Choose H as a power of 2 near the square root of num_pairs
    let target_h = (num_pairs as f64).sqrt() as usize;
    let mut h = 2;
    while h * 2 <= target_h {
        h *= 2;
    }
    if h < 4 { h = 4; } // Minimal resolution
    
    // Ensure W is even and large enough to hold all samples
    let mut w = (num_pairs + h - 1) / h;
    if w % 2 != 0 {
        w += 1;
    }
    if w < 2 { w = 2; }
    
    (w, h)
}

// ==========================================
// Bijeção 3: Codificação Semântica ZigZag (Sem Ramificação)
// ==========================================

#[inline]
pub fn zigzag_encode(val: i16) -> u16 {
    ((val << 1) ^ (val >> 15)) as u16
}

#[inline]
pub fn zigzag_decode(val: u16) -> i16 {
    ((val >> 1) as i16) ^ (-((val & 1) as i16))
}

// ==========================================
// WASM Entrypoints for Semantic WPD Spectrogram
// ==========================================

#[wasm_bindgen]
pub fn encode_wavelet(data: &[u8]) -> Vec<u8> {
    let original_len = data.len() as u32;
    let (w, h) = calculate_grid_size(data.len());
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // 1. Unpack bytes pairwise into L/R i16 samples (WAV Little-Endian byte-ordering)
    for i in 0..((data.len() + 3) / 4) {
        let offset = i * 4;
        let b0 = if offset < data.len() { data[offset] } else { 0 };
        let b1 = if offset + 1 < data.len() { data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < data.len() { data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < data.len() { data[offset + 3] } else { 0 };
        
        // Little-Endian: b0 is low-byte, b1 is high-byte
        let l_sample = (((b1 as u16) << 8) | (b0 as u16)) as i16;
        // b2 is low-byte, b3 is high-byte
        let r_sample = (((b3 as u16) << 8) | (b2 as u16)) as i16;
        
        let (m, s) = lr_to_ms(l_sample, r_sample);
        mid_grid[i] = m;
        side_grid[i] = s;
    }
    
    // 2. Apply 1D Wavelet Packet Decomposition to arrange sub-bands vertically (Frequency rows)
    let depth = (h as f64).log2() as usize;
    forward_wpd(&mut mid_grid, depth);
    forward_wpd(&mut side_grid, depth);
    
    // 3. Pack metadata and map coefficients into semantic RGBA pixels
    let mut output = Vec::with_capacity(12 + grid_size * 4);
    
    // Metadata (big-endian)
    output.extend_from_slice(&original_len.to_be_bytes());
    output.extend_from_slice(&(w as u32).to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    // Reorganize wavelet sub-bands into image rows (LL ... HH)
    // The WPD output has H sub-bands of length W packed sequentially.
    // Row r represents the r-th sub-band.
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            let u16_m = zigzag_encode(mid_grid[idx]);
            let u16_s = zigzag_encode(side_grid[idx]);
            
            let r_chan = (u16_m >> 8) as u8;
            let g_chan = (u16_m & 0xFF) as u8;
            let b_chan = (u16_s >> 8) as u8;
            let a_chan = (u16_s & 0xFF) as u8;
            
            output.push(r_chan);
            output.push(g_chan);
            output.push(b_chan);
            output.push(a_chan);
        }
    }
    
    output
}

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
    
    // 1. Unpack RGBA rows back into mid (C_M) and side (C_S) grids
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            let offset = 12 + idx * 4;
            let r_chan = rgba_data[offset];
            let g_chan = rgba_data[offset + 1];
            let b_chan = rgba_data[offset + 2];
            let a_chan = rgba_data[offset + 3];
            
            let u16_m = ((r_chan as u16) << 8) | (g_chan as u16);
            let u16_s = ((b_chan as u16) << 8) | (a_chan as u16);
            
            mid_grid[idx] = zigzag_decode(u16_m);
            side_grid[idx] = zigzag_decode(u16_s);
        }
    }
    
    // 2. Run inverse 1D Wavelet Packet Reconstruction on Mid and Side grids
    let depth = (h as f64).log2() as usize;
    inverse_wpd(&mut mid_grid, depth);
    inverse_wpd(&mut side_grid, depth);
    
    // 3. Unpack Mid/Side back to Left/Right samples and serialize as Little-Endian
    let mut original_data = Vec::with_capacity(original_len);
    for i in 0..((original_len + 3) / 4) {
        let m = mid_grid[i];
        let s = side_grid[i];
        
        let (l, r) = ms_to_lr(m, s);
        let u16_l = l as u16;
        let u16_r = r as u16;
        
        // Little-Endian: low-byte (b0/b2) first, high-byte (b1/b3) after
        let b0 = (u16_l & 0xFF) as u8;
        let b1 = (u16_l >> 8) as u8;
        let b2 = (u16_r & 0xFF) as u8;
        let b3 = (u16_r >> 8) as u8;
        
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

    #[test]
    fn test_zigzag_bijection() {
        let test_cases = vec![0, 1, -1, 32767, -32768, 12345, -12345];
        for val in test_cases {
            let encoded = zigzag_encode(val);
            let decoded = zigzag_decode(encoded);
            assert_eq!(val, decoded, "ZigZag failed for {}", val);
        }
    }

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
        let (m, s) = lr_to_ms(5000, 5000);
        assert_eq!(s, 0);
        assert_eq!(m, 5000);

        let (m, s) = lr_to_ms(5000, -5000);
        assert_eq!(m, 0);
        assert_eq!(s, 10000);
    }

    #[test]
    fn test_wpd_perfect_reversibility() {
        let mut sample = vec![100, -50, 200, 300, -400, 500, 600, 700];
        let original = sample.clone();
        
        // Depth 2 splits 8 samples into 4 sub-bands of size 2
        forward_wpd(&mut sample, 2);
        inverse_wpd(&mut sample, 2);
        assert_eq!(sample, original, "WPD 1D failed to reconstruct perfectly");
    }

    // ------------------------------------------
    // Teste de Correlação Espacial (Anti-Ruído)
    // ------------------------------------------

    #[test]
    fn test_image_semantic_spatial_correlation() {
        // Gera um sinal senoidal estéreo com variação suave (alta correlação natural)
        let size = 1024; // 256 amostras estéreo (1024 bytes)
        let mut signal = Vec::with_capacity(size);
        for i in 0..256 {
            // Seno de 440 Hz amostrado a 44100 Hz
            let val = ( (i as f32 * 2.0 * 3.14159 * 440.0 / 44100.0).sin() * 15000.0 ) as i16;
            let u16_val = val as u16;
            signal.push((u16_val >> 8) as u8);
            signal.push((u16_val & 0xFF) as u8);
            signal.push((u16_val >> 8) as u8);
            signal.push((u16_val & 0xFF) as u8);
        }

        // Codifica para obter a imagem de Wavelets Packet (WPD)
        let encoded_rgba = encode_wavelet(&signal);
        
        // Mede a auto-correlação horizontal e vertical nos canais de brilho (R/G)
        let w = u32::from_be_bytes([encoded_rgba[4], encoded_rgba[5], encoded_rgba[6], encoded_rgba[7]]) as usize;
        let h = u32::from_be_bytes([encoded_rgba[8], encoded_rgba[9], encoded_rgba[10], encoded_rgba[11]]) as usize;
        
        let mut diff_sum_h = 0.0;
        let mut diff_sum_v = 0.0;
        let mut count_h = 0;
        let mut count_v = 0;

        for r in 0..h {
            for c in 0..w {
                let idx = r * w + c;
                let offset = 12 + idx * 4;
                let val = ((encoded_rgba[offset] as u16) << 8) | (encoded_rgba[offset + 1] as u16);
                
                // Vizinho horizontal
                if c + 1 < w {
                    let next_offset = 12 + (idx + 1) * 4;
                    let next_val = ((encoded_rgba[next_offset] as u16) << 8) | (encoded_rgba[next_offset + 1] as u16);
                    diff_sum_h += (val as f32 - next_val as f32).abs();
                    count_h += 1;
                }
                
                // Vizinho vertical
                if r + 1 < h {
                    let next_offset = 12 + (idx + w) * 4;
                    let next_val = ((encoded_rgba[next_offset] as u16) << 8) | (encoded_rgba[next_offset + 1] as u16);
                    diff_sum_v += (val as f32 - next_val as f32).abs();
                    count_v += 1;
                }
            }
        }

        let avg_diff_h = diff_sum_h / (count_h as f32);
        let avg_diff_v = diff_sum_v / (count_v as f32);

        // Se fosse ruído branco puro de alta amplitude (0 a 65535), a diferença média esperada seria ~21845.
        // Em um espectrograma de wavelets packet suave, a diferença de vizinhos deve ser extremamente baixa
        // (indicando alta correlação espacial, com limiar conservador < 8000 para sinais senoidais).
        assert!(avg_diff_h < 8000.0, "Horizontal adjacent difference is too high (looks like noise): {}", avg_diff_h);
        assert!(avg_diff_v < 8000.0, "Vertical adjacent difference is too high (looks like noise): {}", avg_diff_v);
    }

    #[test]
    fn test_symmetrical_identity_naive() {
        let original_audio = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22];
        let encoded_image = encode_naive(&original_audio);
        assert_eq!(encoded_image.len() % 4, 0);
        let decoded_audio = decode_naive(&encoded_image).unwrap();
        assert_eq!(original_audio, decoded_audio);
    }

    #[test]
    fn test_wavelet_pipeline_perfect_identity() {
        let original_audio = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, 0x80,
            0x12, 0x34, 0x56, 0x78
        ];
        
        let encoded_rgba = encode_wavelet(&original_audio);
        let decoded_audio = decode_wavelet(&encoded_rgba).unwrap();
        assert_eq!(original_audio, decoded_audio);
    }
}

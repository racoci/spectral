use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

// ==========================================
// 1D Cohen-Daubechies-Feauveau 5/3 (CDF 5/3)
// ==========================================

fn forward_1d(a: &mut [i32]) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i32; half];
    let mut odd = vec![0i32; half];
    
    // Split: separate even and odd indices
    for i in 0..half {
        even[i] = a[2 * i];
        odd[i] = a[2 * i + 1];
    }
    
    // Predict step (CDF 5/3 Predictor)
    for i in 0..half {
        let prev = even[i];
        let next = if i + 1 < half { even[i + 1] } else { even[i] }; // Symmetric extension
        odd[i] -= (prev + next) / 2;
    }
    
    // Update step (CDF 5/3 Updater)
    for i in 0..half {
        let prev = if i > 0 { odd[i - 1] } else { odd[0] }; // Symmetric extension
        let curr = odd[i];
        even[i] += (prev + curr + 2) / 4;
    }
    
    // Pack back: Evens (Approximation) first, then Odds (Details)
    for i in 0..half {
        a[i] = even[i];
        a[half + i] = odd[i];
    }
}

fn inverse_1d(a: &mut [i32]) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i32; half];
    let mut odd = vec![0i32; half];
    
    // Unpack: Evens first, then Odds
    for i in 0..half {
        even[i] = a[i];
        odd[i] = a[half + i];
    }
    
    // Inverse Update step
    for i in 0..half {
        let prev = if i > 0 { odd[i - 1] } else { odd[0] };
        let curr = odd[i];
        even[i] -= (prev + curr + 2) / 4;
    }
    
    // Inverse Predict step
    for i in 0..half {
        let prev = even[i];
        let next = if i + 1 < half { even[i + 1] } else { even[i] };
        odd[i] += (prev + next) / 2;
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

fn forward_2d(grid: &mut [i32], w: usize, h: usize) {
    // 1. Transform each row independently
    for r in 0..h {
        let row_start = r * w;
        let row_end = row_start + w;
        forward_1d(&mut grid[row_start..row_end]);
    }
    
    // 2. Transform each column independently
    let mut col_temp = vec![0i32; h];
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

fn inverse_2d(grid: &mut [i32], w: usize, h: usize) {
    // 1. Inverse transform each column independently
    let mut col_temp = vec![0i32; h];
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

/// Helper to calculate an optimal even square dimension for the audio samples.
fn calculate_grid_size(data_len: usize) -> (usize, usize) {
    let samples_count = (data_len + 1) / 2;
    let mut w = (samples_count as f64).sqrt() as usize;
    if w % 2 != 0 {
        w += 1;
    }
    if w < 2 {
        w = 2;
    }
    let mut h = (samples_count + w - 1) / w;
    if h % 2 != 0 {
        h += 1;
    }
    if h < 2 {
        h = 2;
    }
    (w, h)
}

// ==========================================
// WASM Entrypoints for CDF 5/3 Wavelet
// ==========================================

/// Converts raw audio bytes into an RGBA pixel array containing:
/// - First 3 pixels (12 bytes): Metadata Header [Original byte size (4 bytes), Width (4 bytes), Height (4 bytes)]
/// - Remaining W*H pixels: 2D CDF 5/3 Wavelet coefficients, where each coefficient is packed as a 4-byte i32 (RGBA).
#[wasm_bindgen]
pub fn encode_wavelet(data: &[u8]) -> Vec<u8> {
    let original_len = data.len() as u32;
    let (w, h) = calculate_grid_size(data.len());
    let grid_size = w * h;
    let mut grid = vec![0i32; grid_size];
    
    // Pack bytes pairwise into i16 values, then upcast to i32 for transform headroom
    for i in 0..((data.len() + 1) / 2) {
        let b0 = data[2 * i];
        let b1 = if 2 * i + 1 < data.len() { data[2 * i + 1] } else { 0 };
        let val_u16 = ((b0 as u16) << 8) | (b1 as u16);
        let val_i16 = val_u16 as i16;
        grid[i] = val_i16 as i32;
    }
    
    // Run 2D CDF 5/3 Integer Wavelet Transform
    forward_2d(&mut grid, w, h);
    
    // Pack metadata header and coefficients into the final byte array
    let mut output = Vec::with_capacity(12 + grid_size * 4);
    
    // Metadata (big-endian)
    output.extend_from_slice(&original_len.to_be_bytes());
    output.extend_from_slice(&(w as u32).to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    // Wavelet coefficients
    for coeff in grid {
        output.extend_from_slice(&coeff.to_be_bytes());
    }
    
    output
}

/// Receives an RGBA pixel array containing the metadata header and wavelet coefficients,
/// performs the inverse 2D CDF 5/3 transform, and decodes back to the original audio file bytes.
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
    
    // Reconstruct the wavelet grid
    let mut grid = vec![0i32; w * h];
    for i in 0..(w * h) {
        let offset = 12 + i * 4;
        let mut coeff_bytes = [0u8; 4];
        coeff_bytes.copy_from_slice(&rgba_data[offset..offset + 4]);
        grid[i] = i32::from_be_bytes(coeff_bytes);
    }
    
    // Run inverse 2D CDF 5/3 Integer Wavelet Transform
    inverse_2d(&mut grid, w, h);
    
    // Unpack wavelet coefficients back to original bytes
    let mut original_data = Vec::with_capacity(original_len);
    for i in 0..((original_len + 1) / 2) {
        let val_i16 = grid[i] as i16;
        let val_u16 = val_i16 as u16;
        let b0 = (val_u16 >> 8) as u8;
        let b1 = (val_u16 & 0xFF) as u8;
        
        original_data.push(b0);
        if original_data.len() < original_len {
            original_data.push(b1);
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
    fn test_symmetrical_identity_naive() {
        let original_audio = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22];
        let encoded_image = encode_naive(&original_audio);
        assert_eq!(encoded_image.len() % 4, 0);
        let decoded_audio = decode_naive(&encoded_image).unwrap();
        assert_eq!(original_audio, decoded_audio);
    }

    #[test]
    fn test_wavelet_pipeline_perfect_identity() {
        // Mock WAV or binary audio data
        let original_audio = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, 0x80
        ];
        
        // Encode to wavelet image RGBA buffer
        let encoded_rgba = encode_wavelet(&original_audio);
        
        // Decode back to raw audio bytes
        let decoded_audio = decode_wavelet(&encoded_rgba).unwrap();
        
        // Verify bit-perfect symmetry
        assert_eq!(original_audio, decoded_audio);
    }
}

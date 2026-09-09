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
// Bijeção 2.1: 1D Cohen-Daubechies-Feauveau 5/3 (CDF 5/3)
// ==========================================

fn forward_53_1d(a: &mut [i16]) {
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

fn inverse_53_1d(a: &mut [i16]) {
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
// Bijeção 2.2: 1D Haar Wavelet (CDF 1/1)
// ==========================================

fn forward_haar_1d(a: &mut [i16]) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    for i in 0..half {
        even[i] = a[2 * i];
        odd[i] = a[2 * i + 1];
    }
    
    // Haar Lifting Scheme:
    // d[n] = odd[n] - even[n]
    // s[n] = even[n] + floor(d[n] / 2)
    for i in 0..half {
        odd[i] = odd[i].wrapping_sub(even[i]);
        even[i] = even[i].wrapping_add(odd[i] >> 1);
    }
    
    for i in 0..half {
        a[i] = even[i];
        a[half + i] = odd[i];
    }
}

fn inverse_haar_1d(a: &mut [i16]) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    for i in 0..half {
        even[i] = a[i];
        odd[i] = a[half + i];
    }
    
    // Inverse Haar Lifting:
    // even[n] = s[n] - floor(d[n] / 2)
    // odd[n] = d[n] + even[n]
    for i in 0..half {
        even[i] = even[i].wrapping_sub(odd[i] >> 1);
        odd[i] = odd[i].wrapping_add(even[i]);
    }
    
    for i in 0..half {
        a[2 * i] = even[i];
        a[2 * i + 1] = odd[i];
    }
}

// ==========================================
// 1D Wavelet Packet Decomposition (WPD)
// ==========================================

/// Applies recursive Wavelet Packet Decomposition.
/// wavelet_type: 0 for CDF 5/3, 1 for Haar (CDF 1/1).
fn forward_wpd(a: &mut [i16], depth: usize, wavelet_type: u32) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    
    if wavelet_type == 0 {
        forward_53_1d(a);
    } else {
        forward_haar_1d(a);
    }
    
    let half = len / 2;
    forward_wpd(&mut a[0..half], depth - 1, wavelet_type);
    forward_wpd(&mut a[half..len], depth - 1, wavelet_type);
}

/// Applies recursive Wavelet Packet Reconstruction.
fn inverse_wpd(a: &mut [i16], depth: usize, wavelet_type: u32) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    
    let half = len / 2;
    inverse_wpd(&mut a[0..half], depth - 1, wavelet_type);
    inverse_wpd(&mut a[half..len], depth - 1, wavelet_type);
    
    if wavelet_type == 0 {
        inverse_53_1d(a);
    } else {
        inverse_haar_1d(a);
    }
}

/// Calculates image width W based on the number of stereo pairs and a custom height H.
fn calculate_grid_width(data_len: usize, h: usize) -> usize {
    let num_pairs = (data_len + 3) / 4;
    let mut w = (num_pairs + h - 1) / h;
    if w % 2 != 0 {
        w += 1;
    }
    if w < 2 {
        w = 2;
    }
    w
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
// WASM Entrypoints for Dynamic, Self-Contained Spectrogram
// ==========================================

/// Converts raw audio bytes into an autossuficiente RGBA pixel array containing:
/// - First 4 pixels (16 bytes): Metadata Header [Original byte size (4 bytes), W (4 bytes), H (4 bytes), Parameters (4 bytes)]
/// - Remaining W*H pixels: 2D Wavelet Packet coefficients packed in RGBA.
///
/// Parameters Byte Mapping (Pixel 3 / Bytes 12-15):
/// - Byte 12: `wavelet_type` (0 for CDF 5/3, 1 for Haar)
/// - Byte 13: `channels` (always 2 for stereo)
/// - Bytes 14-15: `sample_rate` (u16, big-endian)
#[wasm_bindgen]
pub fn encode_wavelet(data: &[u8], h_custom: usize, wavelet_type: u32) -> Vec<u8> {
    let original_len = data.len() as u32;
    
    // Ensure h_custom is a valid power of 2
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 {
        h = 1024; // Default to 21.5 Hz resolution
    }
    
    let w = calculate_grid_width(data.len(), h);
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // 1. Unpack bytes pairwise into L/R i16 samples (WAV Little-Endian) and convert to Mid/Side
    for i in 0..((data.len() + 3) / 4) {
        let offset = i * 4;
        let b0 = if offset < data.len() { data[offset] } else { 0 };
        let b1 = if offset + 1 < data.len() { data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < data.len() { data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < data.len() { data[offset + 3] } else { 0 };
        
        let l_sample = (((b1 as u16) << 8) | (b0 as u16)) as i16;
        let r_sample = (((b3 as u16) << 8) | (b2 as u16)) as i16;
        
        let (m, s) = lr_to_ms(l_sample, r_sample);
        mid_grid[i] = m;
        side_grid[i] = s;
    }
    
    // 2. Apply 1D Wavelet Packet Decomposition (Time is horizontal, Frequency is vertical)
    let depth = (h as f64).log2() as usize;
    forward_wpd(&mut mid_grid, depth, wavelet_type);
    forward_wpd(&mut side_grid, depth, wavelet_type);
    
    // 3. Pack self-contained metadata and coefficients into RGBA pixels
    let mut output = Vec::with_capacity(16 + grid_size * 4);
    
    // Metadata (big-endian)
    output.extend_from_slice(&original_len.to_be_bytes());
    output.extend_from_slice(&(w as u32).to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    // Pixel 3: Parameters [Wavelet Type (1B), Channels (1B), Sample Rate (2B)]
    output.push(wavelet_type as u8);
    output.push(2u8); // Channels: Stereo
    output.extend_from_slice(&44100u16.to_be_bytes()); // Default Sample Rate: 44.1 kHz
    
    // Pack C_M and C_S into RGBA rows
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

/// Receives an RGBA pixel array, reads the self-contained metadata header to identify
/// the dimensions and wavelet algorithm used, performs the exact inverse transform,
/// and decodes back to the original audio file bytes.
#[wasm_bindgen]
pub fn decode_wavelet(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 {
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
    
    // Read parameters from Pixel 3
    let wavelet_type = rgba_data[12] as u32;
    
    let expected_coeff_len = w * h * 4;
    if rgba_data.len() < 16 + expected_coeff_len {
        return Err(JsValue::from_str(&format!(
            "Invalid encoded data: expected at least {} bytes (16 + {}), but got only {} bytes.",
            16 + expected_coeff_len, expected_coeff_len, rgba_data.len()
        )));
    }
    
    let grid_size = w * h;
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // 1. Unpack RGBA pixels back into mid and side grids
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            let offset = 16 + idx * 4;
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
    
    // 2. Run inverse Wavelet Packet Reconstruction using the dynamically detected wavelet algorithm
    let depth = (h as f64).log2() as usize;
    inverse_wpd(&mut mid_grid, depth, wavelet_type);
    inverse_wpd(&mut side_grid, depth, wavelet_type);
    
    // 3. Unpack Mid/Side back to Left/Right samples and serialize as Little-Endian
    let mut original_data = Vec::with_capacity(original_len);
    for i in 0..((original_len + 3) / 4) {
        let m = mid_grid[i];
        let s = side_grid[i];
        
        let (l, r) = ms_to_lr(m, s);
        let u16_l = l as u16;
        let u16_r = r as u16;
        
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
            assert_eq!(val, decoded);
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
            assert_eq!((l, r), (l_rec, r_rec));
        }
    }

    #[test]
    fn test_haar_perfect_reversibility() {
        let mut sample = vec![100, -50, 200, 300, -400, 500, 600, 700];
        let original = sample.clone();
        
        forward_wpd(&mut sample, 2, 1); // 1 = Haar
        inverse_wpd(&mut sample, 2, 1);
        assert_eq!(sample, original, "Haar WPD failed to reconstruct perfectly");
    }

    #[test]
    fn test_wpd_perfect_reversibility() {
        let mut sample = vec![100, -50, 200, 300, -400, 500, 600, 700];
        let original = sample.clone();
        
        forward_wpd(&mut sample, 2, 0); // 0 = CDF 5/3
        inverse_wpd(&mut sample, 2, 0);
        assert_eq!(sample, original, "CDF 5/3 WPD failed to reconstruct perfectly");
    }

    #[test]
    fn test_wavelet_pipeline_perfect_identity() {
        let original_audio = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, 0x80,
            0x12, 0x34, 0x56, 0x78
        ];
        
        // Teste com CDF 5/3 a 256 de altura
        let encoded_rgba_53 = encode_wavelet(&original_audio, 256, 0);
        let decoded_audio_53 = decode_wavelet(&encoded_rgba_53).unwrap();
        assert_eq!(original_audio, decoded_audio_53);

        // Teste com Haar a 256 de altura
        let encoded_rgba_haar = encode_wavelet(&original_audio, 256, 1);
        let decoded_audio_haar = decode_wavelet(&encoded_rgba_haar).unwrap();
        assert_eq!(original_audio, decoded_audio_haar);
    }
}

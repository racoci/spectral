use wasm_bindgen::prelude::*;
use std::sync::OnceLock;

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

// ==========================================
// Cubic-Shell Geodesic Snake LUT Generators
// ==========================================

static COLOR_LUT: OnceLock<[(u8, u8, u8); 65536]> = OnceLock::new();
static DECODE_LUT: OnceLock<[(u64, u16); 65536]> = OnceLock::new();
static REVERSE_RG: OnceLock<[u16; 65536]> = OnceLock::new();

fn generate_color_lut() -> [(u8, u8, u8); 65536] {
    let mut lut = [(0u8, 0u8, 0u8); 65536];
    let mut count = 0;
    
    for l in 0..=255 {
        if l == 0 {
            lut[0] = (0, 0, 0);
            count += 1;
            continue;
        }
        
        if l % 2 == 0 {
            for g in 0..=l {
                if count >= 65536 { break; }
                let r = l;
                let b = l;
                lut[count] = (r as u8, g as u8, b as u8);
                count += 1;
            }
            for r_idx in 1..=l {
                if count >= 65536 { break; }
                let r = l - r_idx;
                let g = l;
                let b = l;
                lut[count] = (r as u8, g as u8, b as u8);
                count += 1;
            }
        } else {
            for r in 0..=l {
                if count >= 65536 { break; }
                let g = l;
                let b = l;
                lut[count] = (r as u8, g as u8, b as u8);
                count += 1;
            }
            for g_idx in 1..=l {
                if count >= 65536 { break; }
                let r = l;
                let g = l - g_idx;
                let b = l;
                lut[count] = (r as u8, g as u8, b as u8);
                count += 1;
            }
        }
    }
    lut
}

fn get_color_lut() -> &'static [(u8, u8, u8); 65536] {
    COLOR_LUT.get_or_init(generate_color_lut)
}

fn get_decode_lut() -> &'static [(u64, u16); 65536] {
    DECODE_LUT.get_or_init(|| {
        let color_lut = get_color_lut();
        let mut decode = [(0u64, 0u16); 65536];
        for i in 0..65536 {
            let (r, g, b) = color_lut[i];
            let r_u64 = r as u64;
            let g_u64 = g as u64;
            let b_u64 = b as u64;
            let key = (r_u64 * r_u64 + g_u64 * g_u64 + b_u64 * b_u64) * 16777216 + r_u64 * 65536 + g_u64 * 256 + b_u64;
            decode[i] = (key, i as u16);
        }
        decode.sort_by_key(|entry| entry.0);
        decode
    })
}

fn get_reverse_rg() -> &'static [u16; 65536] {
    REVERSE_RG.get_or_init(|| {
        let color_lut = get_color_lut();
        let mut reverse = [0u16; 65536];
        for i in 0..65536 {
            let (r, g, _) = color_lut[i];
            let idx = (r as usize) * 256 + (g as usize);
            reverse[idx] = i as u16;
        }
        reverse
    })
}

#[wasm_bindgen]
pub fn decode_color_to_coefficient(r: u8, g: u8, b: u8) -> u16 {
    let r_u64 = r as u64;
    let g_u64 = g as u64;
    let b_u64 = b as u64;
    let key = (r_u64 * r_u64 + g_u64 * g_u64 + b_u64 * b_u64) * 16777216 + r_u64 * 65536 + g_u64 * 256 + b_u64;
    
    let decode_lut = get_decode_lut();
    match decode_lut.binary_search_by_key(&key, |entry| entry.0) {
        Ok(idx) => decode_lut[idx].1,
        Err(_) => 0,
    }
}

#[wasm_bindgen]
pub fn decode_rg_to_coefficient(r: u8, g: u8) -> u16 {
    let reverse = get_reverse_rg();
    reverse[(r as usize) * 256 + (g as usize)]
}

#[wasm_bindgen]
pub fn get_color_r(v: u16) -> u8 {
    get_color_lut()[v as usize].0
}

#[wasm_bindgen]
pub fn get_color_g(v: u16) -> u8 {
    get_color_lut()[v as usize].1
}

#[wasm_bindgen]
pub fn get_color_b(v: u16) -> u8 {
    get_color_lut()[v as usize].2
}

// ==========================================
// 1. Transformada Reversível Mid/Side (L/R)
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
// Espelhamento Simétrico de Bordas (Boundary Mirroring)
// ==========================================

/// Safe indexing with symmetric edge mirroring to prevent boundary artifacts.
#[inline]
fn get_even(even: &[i16], idx: i32) -> i32 {
    let half = even.len() as i32;
    if idx < 0 {
        even[(-idx - 1).min(half - 1) as usize] as i32
    } else if idx >= half {
        even[(2 * half - idx - 1).max(0) as usize] as i32
    } else {
        even[idx as usize] as i32
    }
}

// ==========================================
// Bijeção 2: Multi-Order Wavelet Predictor Lifting
// ==========================================

/// 1D forward lifting step with selectable predictor order
fn forward_1d(a: &mut [i16], wavelet_type: u32) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    for i in 0..half {
        even[i] = a[2 * i];
        odd[i] = a[2 * i + 1];
    }
    
    // Predict step: odd[i] -= round(P(even))
    for i in 0..half {
        let p_val = match wavelet_type {
            // Order 1 (Linear / CDF 5/3): 2-taps
            0 => {
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                (e0 + e1) / 2
            },
            // Order 3 (Cubic Lagrange / CDF 9/7 proxy): 4-taps
            1 => {
                let e_prev = get_even(&even, i as i32 - 1);
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                let e_next = get_even(&even, i as i32 + 2);
                (-e_prev + 9 * e0 + 9 * e1 - e_next + 8) / 16
            },
            // Order 5 (Quintic Lagrange): 6-taps
            _ => {
                let e_prev2 = get_even(&even, i as i32 - 2);
                let e_prev1 = get_even(&even, i as i32 - 1);
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                let e_next1 = get_even(&even, i as i32 + 2);
                let e_next2 = get_even(&even, i as i32 + 3);
                (3 * e_prev2 - 25 * e_prev1 + 150 * e0 + 150 * e1 - 25 * e_next1 + 3 * e_next2 + 128) / 256
            }
        };
        odd[i] = odd[i].wrapping_sub(p_val as i16);
    }
    
    // Update step: even[i] += round(U(odd)) [CDF 5/3 classic updater]
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

/// 1D inverse lifting step with selectable predictor order
fn inverse_1d(a: &mut [i16], wavelet_type: u32) {
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
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
        let p_val = match wavelet_type {
            // Order 1 (Linear / CDF 5/3): 2-taps
            0 => {
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                (e0 + e1) / 2
            },
            // Order 3 (Cubic Lagrange): 4-taps
            1 => {
                let e_prev = get_even(&even, i as i32 - 1);
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                let e_next = get_even(&even, i as i32 + 2);
                (-e_prev + 9 * e0 + 9 * e1 - e_next + 8) / 16
            },
            // Order 5 (Quintic Lagrange): 6-taps
            _ => {
                let e_prev2 = get_even(&even, i as i32 - 2);
                let e_prev1 = get_even(&even, i as i32 - 1);
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                let e_next1 = get_even(&even, i as i32 + 2);
                let e_next2 = get_even(&even, i as i32 + 3);
                (3 * e_prev2 - 25 * e_prev1 + 150 * e0 + 150 * e1 - 25 * e_next1 + 3 * e_next2 + 128) / 256
            }
        };
        odd[i] = odd[i].wrapping_add(p_val as i16);
    }
    
    for i in 0..half {
        a[2 * i] = even[i];
        a[2 * i + 1] = odd[i];
    }
}

// ==========================================
// 1D Wavelet Packet Decomposition (WPD)
// ==========================================

fn forward_wpd(a: &mut [i16], depth: usize, wavelet_type: u32) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    
    forward_1d(a, wavelet_type);
    
    let half = len / 2;
    forward_wpd(&mut a[0..half], depth - 1, wavelet_type);
    forward_wpd(&mut a[half..len], depth - 1, wavelet_type);
}

fn inverse_wpd(a: &mut [i16], depth: usize, wavelet_type: u32) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    
    let half = len / 2;
    inverse_wpd(&mut a[0..half], depth - 1, wavelet_type);
    inverse_wpd(&mut a[half..len], depth - 1, wavelet_type);
    
    inverse_1d(a, wavelet_type);
}

/// Calculates image width W based on the number of stereo pairs and height H.
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
// Bijeção 3: Codificação Semântica Gray Code
// Transformação da Topologia de Inteiros para Bit-Planes
// ==========================================

#[inline]
pub fn zigzag_encode(val: i32) -> u32 {
    ((val << 1) ^ (val >> 31)) as u32
}

#[inline]
pub fn zigzag_decode(val: u32) -> i32 {
    ((val >> 1) as i32) ^ (-((val & 1) as i32))
}

#[inline]
pub fn gray_encode(val: i32) -> u32 {
    let u_val = zigzag_encode(val);
    u_val ^ (u_val >> 1)
}

#[inline]
pub fn gray_decode(mut val: u32) -> i32 {
    let mut mask = val >> 1;
    while mask != 0 {
        val ^= mask;
        mask >>= 1;
    }
    zigzag_decode(val)
}

// ==========================================
// WASM Entrypoints for Hierarchical Semantic Spectrogram (V2: Single-Pixel Bit-Plane)
// ==========================================

#[wasm_bindgen]
pub fn encode_wavelet_v2_bitplane(data: &[u8], h_custom: usize, wavelet_type: u32) -> Vec<u8> {
    let original_len = data.len() as u32;
    
    // Validate target height H (must be power of 2)
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 {
        h = 1024; // 21.5 Hz frequency resolution
    }
    
    let num_pairs = (data.len() + 3) / 4;
    let mut w = (num_pairs + h - 1) / h;
    if w < 1 { w = 1; }
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // 1. Unpack L/R Little-Endian samples, convert to Mid/Side
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
    
    // 2. Apply 1D Wavelet Packet Decomposition (Frequency is vertical rows)
    let depth = (h as f64).log2() as usize;
    forward_wpd(&mut mid_grid, depth, wavelet_type);
    forward_wpd(&mut side_grid, depth, wavelet_type);
    
    // 3. Hierarchical Bit-Plane Coding in 1 Pixel (32-bits) per Stereo Sample
    let mut output = Vec::with_capacity(16 + grid_size * 4);
    
    // Metadata (Pixel 0 to 3)
    output.extend_from_slice(&original_len.to_be_bytes());
    output.extend_from_slice(&(w as u32).to_be_bytes()); // Restored W physical dimension!
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(wavelet_type as u8);
    output.push(2u8); // Stereo
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            
            // Re-map topography using Gray Code to compress sequential correlation
            let g_m = gray_encode(mid_grid[idx] as i32) as u16;
            let g_s = gray_encode(side_grid[idx] as i32) as u16;
            
            // Use Cubic-Shell color LUT for packing Mid and Side coefficients
            let (r_m, g_m_color, _) = get_color_lut()[g_m as usize];
            let (r_s, g_s_color, _) = get_color_lut()[g_s as usize];
            
            // Inverting the Alpha channel so that silence yields Opaque (255)
            let a_encoded = 255u8.wrapping_sub(g_s_color);

            output.push(r_m);       // R = Mid Red
            output.push(g_m_color); // G = Mid Green
            output.push(r_s);       // B = Side Red
            output.push(a_encoded); // A = Side Green Inverted
        }
    }
    
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v2_bitplane(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 {
        return Err(JsValue::from_str("Invalid encoded data"));
    }
    
    let mut len_bytes = [0u8; 4];
    let mut w_bytes = [0u8; 4];
    let mut h_bytes = [0u8; 4];
    
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    w_bytes.copy_from_slice(&rgba_data[4..8]);
    h_bytes.copy_from_slice(&rgba_data[8..12]);
    
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    let w = u32::from_be_bytes(w_bytes) as usize;
    let h = u32::from_be_bytes(h_bytes) as usize;
    
    let wavelet_type = rgba_data[12] as u32;
    
    let expected_coeff_len = w * h * 4; // 1 pixel (4 bytes) per sample
    if rgba_data.len() < 16 + expected_coeff_len {
        return Err(JsValue::from_str("Invalid encoded data size."));
    }
    
    let grid_size = w * h;
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    for r in 0..h {
        for c in 0..w {
            let offset = 16 + (r * w + c) * 4;
            
            let r_m = rgba_data[offset];
            let g_m_color = rgba_data[offset + 1];
            let r_s = rgba_data[offset + 2];
            let a_encoded = rgba_data[offset + 3];
            
            let g_s_color = 255u8.wrapping_sub(a_encoded);
            
            let g_m = decode_rg_to_coefficient(r_m, g_m_color);
            let g_s = decode_rg_to_coefficient(r_s, g_s_color);
            
            let idx = r * w + c;
            mid_grid[idx] = gray_decode(g_m as u32) as i16;
            side_grid[idx] = gray_decode(g_s as u32) as i16;
        }
    }
    
    let depth = (h as f64).log2() as usize;
    inverse_wpd(&mut mid_grid, depth, wavelet_type);
    inverse_wpd(&mut side_grid, depth, wavelet_type);
    
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
        if original_data.len() < original_len { original_data.push(b1); }
        if original_data.len() < original_len { original_data.push(b2); }
        if original_data.len() < original_len { original_data.push(b3); }
    }
    
    Ok(original_data)
}

// ==========================================
// WASM Entrypoints for Panoramic WPD Spectrogram (V1: Two-Pixel RGB)
// ==========================================

#[wasm_bindgen]
pub fn encode_wavelet_v1_two_pixels(data: &[u8], h_custom: usize, wavelet_type: u32) -> Vec<u8> {
    let original_len = data.len() as u32;
    
    // Validate target height H (must be power of 2)
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 {
        h = 1024; // 21.5 Hz frequency resolution
    }
    
    let w = calculate_grid_width(data.len(), h);
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // 1. Unpack L/R Little-Endian samples, convert to Mid/Side
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
    
    // 2. Apply 1D Wavelet Packet Decomposition (Frequency is vertical rows)
    let depth = (h as f64).log2() as usize;
    forward_wpd(&mut mid_grid, depth, wavelet_type);
    forward_wpd(&mut side_grid, depth, wavelet_type);
    
    // 3. Pack metadata and coefficients into 8-bytes (2 pixels) per sample
    // Output size: 16 bytes metadata + W * H * 2 pixels * 4 bytes/pixel = 16 + W * H * 8 bytes
    let mut output = Vec::with_capacity(16 + grid_size * 8);
    
    // Metadata (Pixel 0 to 3)
    output.extend_from_slice(&original_len.to_be_bytes());
    
    // Save W_png = W * 2 because we use 2 adjacent pixels per sample!
    let w_png = (w * 2) as u32;
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    // Pixel 3: Parameters [Wavelet Type (1B), Channels (1B), Sample Rate (2B)]
    output.push(wavelet_type as u8);
    output.push(2u8); // Stereo
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    // Pack C_M and C_S into 2 adjacent pixels (8 bytes total)
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            
            // Encode using full 16-bit ZigZag headroom (lossless!)
            let u16_m = zigzag_encode(mid_grid[idx] as i32) as u16;
            let u16_s = zigzag_encode(side_grid[idx] as i32) as u16;
            
            // Pixel A (Mid / Mono) - RGB active, A = 255 (opaque)
            output.push((u16_m >> 8) as u8);   // R = high byte
            output.push((u16_m & 0xFF) as u8); // G = low byte
            output.push(0u8);                  // B = unused/neutral
            output.push(255u8);                // A = fully opaque!
            
            // Pixel B (Side / Stereo) - RGB active, A = 255 (opaque)
            output.push((u16_s >> 8) as u8);   // R = high byte
            output.push((u16_s & 0xFF) as u8); // G = low byte
            output.push(0u8);                  // B = unused/neutral
            output.push(255u8);                // A = fully opaque!
        }
    }
    
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v1_two_pixels(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 {
        return Err(JsValue::from_str("Invalid encoded data: too short to contain wavelet metadata header."));
    }
    
    // Read metadata header
    let mut len_bytes = [0u8; 4];
    let mut w_png_bytes = [0u8; 4];
    let mut h_bytes = [0u8; 4];
    
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    w_png_bytes.copy_from_slice(&rgba_data[4..8]);
    h_bytes.copy_from_slice(&rgba_data[8..12]);
    
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    let w_png = u32::from_be_bytes(w_png_bytes) as usize;
    let h = u32::from_be_bytes(h_bytes) as usize;
    
    // Since we use 2 pixels per sample, W_audio is exactly W_png / 2
    let w = w_png / 2;
    
    // Read parameters
    let wavelet_type = rgba_data[12] as u32;
    
    let expected_coeff_len = w * h * 8; // 2 pixels per coefficient
    if rgba_data.len() < 16 + expected_coeff_len {
        return Err(JsValue::from_str("Invalid encoded data size."));
    }
    
    let grid_size = w * h;
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // 1. Unpack 8-byte (2 adjacent pixels) pairs back into i16 coefficients
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            
            // Pixel A is at r * w_png + (c * 2)
            let offset_a = 16 + (r * w_png + (c * 2)) * 4;
            let offset_b = offset_a + 4;
            
            // Read 16-bit Mid coefficient from R and G of Pixel A
            let u16_m = ((rgba_data[offset_a] as u16) << 8) | (rgba_data[offset_a + 1] as u16);
                
            // Read 16-bit Side coefficient from R and G of Pixel B
            let u16_s = ((rgba_data[offset_b] as u16) << 8) | (rgba_data[offset_b + 1] as u16);
                
            mid_grid[idx] = zigzag_decode(u16_m as u32) as i16;
            side_grid[idx] = zigzag_decode(u16_s as u32) as i16;
        }
    }
    
    // 2. Run inverse 1D Wavelet Packet Reconstruction
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
        if original_data.len() < original_len { original_data.push(b1); }
        if original_data.len() < original_len { original_data.push(b2); }
        if original_data.len() < original_len { original_data.push(b3); }
    }
    
    Ok(original_data)
}

// ==========================================
// Backwards-compatible Default Entrypoints (Routing to V2)
// ==========================================

#[wasm_bindgen]
pub fn encode_wavelet(data: &[u8], h_custom: usize, wavelet_type: u32) -> Vec<u8> {
    encode_wavelet_v2_bitplane(data, h_custom, wavelet_type)
}

#[wasm_bindgen]
pub fn decode_wavelet(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    decode_wavelet_v2_bitplane(rgba_data)
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
        return Err(JsValue::from_str("Invalid encoded data size."));
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
    fn test_multi_order_reversibility() {
        let test_cases = vec![
            (vec![100, -50, 200, 300, -400, 500, 600, 700], 0), // CDF 5/3
            (vec![100, -50, 200, 300, -400, 500, 600, 700], 1), // 4-Taps
            (vec![100, -50, 200, 300, -400, 500, 600, 700], 2), // 6-Taps
        ];
        
        for (mut sample, order) in test_cases {
            let original = sample.clone();
            forward_wpd(&mut sample, 2, order);
            inverse_wpd(&mut sample, 2, order);
            assert_eq!(sample, original, "WPD failed for order {}", order);
        }
    }

    #[test]
    fn test_wavelet_pipeline_perfect_identity() {
        let original_audio = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, 0x80,
            0x12, 0x34, 0x56, 0x78
        ];
        
        for order in 0..=2 {
            // Test V2 (default)
            let encoded_rgba_v2 = encode_wavelet(&original_audio, 256, order);
            let decoded_audio_v2 = decode_wavelet(&encoded_rgba_v2).unwrap();
            assert_eq!(original_audio, decoded_audio_v2, "V2 Pipeline failed for order {}", order);

            // Test V1 (two-pixel)
            let encoded_rgba_v1 = encode_wavelet_v1_two_pixels(&original_audio, 256, order);
            let decoded_audio_v1 = decode_wavelet_v1_two_pixels(&encoded_rgba_v1).unwrap();
            assert_eq!(original_audio, decoded_audio_v1, "V1 Pipeline failed for order {}", order);
        }
    }

    #[test]
    fn test_naive_pipeline_perfect_identity() {
        let original_audio = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22];
        let encoded_image = encode_naive(&original_audio);
        assert_eq!(encoded_image.len() % 4, 0);
        let decoded_audio = decode_naive(&encoded_image).unwrap();
        assert_eq!(original_audio, decoded_audio);
    }
}

use wasm_bindgen::prelude::*;
use std::sync::OnceLock;

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
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
    
    for i in 0..half {
        let p_val = match wavelet_type {
            0 => {
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                (e0 + e1) / 2
            },
            1 => {
                let e_prev = get_even(&even, i as i32 - 1);
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                let e_next = get_even(&even, i as i32 + 2);
                (-e_prev + 9 * e0 + 9 * e1 - e_next + 8) / 16
            },
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
    
    for i in 0..half {
        let prev = if i > 0 { odd[i - 1] } else { odd[0] };
        let curr = odd[i];
        let update = ((prev as i32 + curr as i32 + 2) / 4) as i16;
        even[i] = even[i].wrapping_sub(update);
    }
    
    for i in 0..half {
        let p_val = match wavelet_type {
            0 => {
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                (e0 + e1) / 2
            },
            1 => {
                let e_prev = get_even(&even, i as i32 - 1);
                let e0 = get_even(&even, i as i32);
                let e1 = get_even(&even, i as i32 + 1);
                let e_next = get_even(&even, i as i32 + 2);
                (-e_prev + 9 * e0 + 9 * e1 - e_next + 8) / 16
            },
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
// 4. Cubic-Shell Geodesic Snake (V2 LUT)
// ==========================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

struct DecodeEntry {
    key: u64,
    index: u16,
}

static COLOR_LUT: OnceLock<[(u8, u8, u8); 65536]> = OnceLock::new();
static DECODE_LUT: OnceLock<Vec<DecodeEntry>> = OnceLock::new();
static REVERSE_RG: OnceLock<Vec<u16>> = OnceLock::new();

fn generate_geodesic_snake_luts() {
    let mut lut = [(0u8, 0u8, 0u8); 65536];
    let mut idx = 0;
    
    for l in 0..256 {
        let mut pts = Vec::with_capacity(3000);
        for r in 0..=l {
            for g in 0..=l {
                if r == l || g == l {
                    let pair = (r as u8, g as u8);
                    if !pts.contains(&pair) {
                        pts.push(pair);
                    }
                }
            }
        }
        
        pts.sort_unstable_by(|a, b| {
            let key_a = (a.0 as u16 + a.1 as u16) % 2;
            let key_b = (b.0 as u16 + b.1 as u16) % 2;
            key_a.cmp(&key_b).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1))
        });
        
        for p in pts {
            if idx >= 65536 { break; }
            lut[idx] = (p.0, p.1, l as u8);
            idx += 1;
        }
    }
    
    while idx < 65536 {
        lut[idx] = (255, 255, 255);
        idx += 1;
    }
    
    COLOR_LUT.set(lut).ok();
    
    let mut decode_vec = Vec::with_capacity(65536);
    let mut rev_rg = vec![0u16; 256 * 256];
    
    for i in 0..65536 {
        let (r, g, b) = lut[i];
        let key = (r as u64 * r as u64 + g as u64 * g as u64 + b as u64 * b as u64) * 16777216
            + (r as u64 * 65536) + (g as u64 * 256) + b as u64;
            
        decode_vec.push(DecodeEntry { key, index: i as u16 });
        
        let rg_idx = (r as usize * 256) + g as usize;
        rev_rg[rg_idx] = i as u16;
    }
    
    decode_vec.sort_unstable_by_key(|e| e.key);
    DECODE_LUT.set(decode_vec).ok();
    REVERSE_RG.set(rev_rg).ok();
}

fn ensure_luts() {
    if COLOR_LUT.get().is_none() {
        generate_geodesic_snake_luts();
    }
}

#[wasm_bindgen]
pub fn decode_rg_to_coefficient(r: u8, g: u8) -> i16 {
    ensure_luts();
    let rev = REVERSE_RG.get().unwrap();
    let idx = (r as usize * 256) + g as usize;
    let gray_val = rev[idx] as u32;
    gray_decode(gray_val) as i16
}

#[wasm_bindgen]
pub fn decode_color_to_coefficient(r: u8, g: u8, b: u8) -> i16 {
    ensure_luts();
    let dec = DECODE_LUT.get().unwrap();
    let key = (r as u64 * r as u64 + g as u64 * g as u64 + b as u64 * b as u64) * 16777216
        + (r as u64 * 65536) + (g as u64 * 256) + b as u64;
        
    let search = dec.binary_search_by_key(&key, |e| e.key);
    let idx_lut = match search {
        Ok(pos) => dec[pos].index,
        Err(_) => {
            let rev = REVERSE_RG.get().unwrap();
            rev[(r as usize * 256) + g as usize]
        }
    };
    gray_decode(idx_lut as u32) as i16
}

#[wasm_bindgen]
pub fn get_color_r(index: u16) -> u8 {
    ensure_luts();
    COLOR_LUT.get().unwrap()[index as usize].0
}

#[wasm_bindgen]
pub fn get_color_g(index: u16) -> u8 {
    ensure_luts();
    COLOR_LUT.get().unwrap()[index as usize].1
}

#[wasm_bindgen]
pub fn get_color_b(index: u16) -> u8 {
    ensure_luts();
    COLOR_LUT.get().unwrap()[index as usize].2
}

// ==========================================
// 5. V3: Reversible Pure Arithmetic Serpentine Curve
// ==========================================

#[inline]
fn c2(x: i64) -> i64 {
    if x >= 2 { x * (x - 1) / 2 } else { 0 }
}

#[inline]
fn c3(x: i64) -> i64 {
    if x >= 3 {
        x * (x - 1) * (x - 2) / 6
    } else {
        0
    }
}

#[inline]
fn shell_count(q: i64, m: i64) -> i64 {
    c2(q + 2)
        - 3 * c2(q - m + 1)
        + 3 * c2(q - 2 * m)
        - c2(q - 3 * m - 1)
}

#[inline]
fn prefix(q: i64, m: i64) -> i64 {
    if q < 0 {
        return 0;
    }
    c3(q + 3)
        - 3 * c3(q - m + 2)
        + 3 * c3(q - 2 * m + 1)
        - c3(q - 3 * m)
}

#[inline]
fn locate_q(n: u32, m: u32) -> u32 {
    let mut lo = 0;
    let mut hi = 3 * m;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if prefix(mid as i64, m as i64) > n as i64 {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

#[inline]
fn row_range(q: u32, m: u32) -> (u32, u32) {
    (
        q.saturating_sub(2 * m),
        q.min(m),
    )
}

fn row_prefix(q: u32, t: u32, m: u32) -> u32 {
    let q = q as i64;
    let t = t as i64;
    let m = m as i64;
    let r0 = (q - 2 * m).max(0);
    let rmax = q.min(m);

    if t <= r0 {
        return 0;
    }
    let t = t.min(rmax + 1);

    if q <= m {
        return (t * (q + 1) - t * (t - 1) / 2) as u32;
    }

    if q <= 2 * m {
        let s = q - m;
        let mut out = 0i64;

        let a = r0;
        let b = t.min(s);

        if b > a {
            let n = b - a;
            out += n * (2 * m - q + 1)
                + (a + b - 1) * n / 2;
        }

        if t > s {
            let c = s;
            let d = t;
            let n = d - c;

            out += n * (q + 1)
                - (c + d - 1) * n / 2;
        }

        return out as u32;
    }

    let n = t - r0;
    let out = n * (n + 1) / 2;
    out as u32
}

pub fn decode_base(q: u32, r: u32, m: u32) -> Rgb {
    let (r0, rmax) = row_range(q, m);
    let mut lo = r0;
    let mut hi = rmax;

    while lo < hi {
        let mid = (lo + hi) >> 1;
        if row_prefix(q, mid + 1, m) > r {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }

    let rr = lo;
    let before = row_prefix(q, rr, m);
    let j = r - before;

    let g0 = q.saturating_sub(rr + m);
    let g1 = (q - rr).min(m);

    let gg = if ((rr - r0) & 1) == 0 {
        g0 + j
    } else {
        g1 - j
    };

    let bb = q - rr - gg;

    Rgb {
        r: rr as u16,
        g: gg as u16,
        b: bb as u16,
    }
}

#[inline]
pub fn decode_n(n: u32, m: u32) -> Rgb {
    let q = locate_q(n, m);
    let q_sub = (q as i32) - 1;
    let r = n - prefix(q_sub as i64, m as i64) as u32;
    decode_base(q, r, m)
}

#[inline]
pub fn encode_n(p: Rgb, m: u32) -> u32 {
    let q = p.r as u32 + p.g as u32 + p.b as u32;
    let (r0, _rmax) = row_range(q, m);
    let rr = p.r as u32;
    let gg = p.g as u32;
    
    let before = row_prefix(q, rr, m);
    let g0 = q.saturating_sub(rr + m);
    let g1 = (q - rr).min(m);
    
    let j = if ((rr - r0) & 1) == 0 {
        gg.saturating_sub(g0)
    } else {
        g1.saturating_sub(gg)
    };
    
    let local = before + j;
    let q_sub = (q as i32) - 1;
    (prefix(q_sub as i64, m as i64) as u32) + local
}

#[wasm_bindgen]
pub fn wasm_decode_n(n: u32, m: u32) -> Vec<u16> {
    let rgb = decode_n(n, m);
    vec![rgb.r, rgb.g, rgb.b]
}

#[wasm_bindgen]
pub fn wasm_encode_n(r: u16, g: u16, b: u16, m: u32) -> u32 {
    encode_n(Rgb { r, g, b }, m)
}

// ==========================================
// WASM Entrypoints for ALL Versions (Modular sandboxes)
// ==========================================

// Legacy Naive
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
        return Err(JsValue::from_str("Invalid naive data"));
    }
    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    Ok(rgba_data[4..4 + original_len].to_vec())
}

// V1: Two-Pixel Sólido (8-bytes, RGB-only)
#[wasm_bindgen]
pub fn encode_wavelet_v1_two_pixels(data: &[u8], h_custom: usize, wavelet_type: u32) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    let w = calculate_grid_width(data.len(), h);
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
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
    
    let depth = (h as f64).log2() as usize;
    forward_wpd(&mut mid_grid, depth, wavelet_type);
    forward_wpd(&mut side_grid, depth, wavelet_type);
    
    let mut output = Vec::with_capacity(16 + grid_size * 8);
    output.extend_from_slice(&original_len.to_be_bytes());
    let w_png = (w * 2) as u32;
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(wavelet_type as u8);
    output.push(1u8); // Packing Version 1!
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            let u16_m = zigzag_encode(mid_grid[idx] as i32) as u16;
            let u16_s = zigzag_encode(side_grid[idx] as i32) as u16;
            
            output.push((u16_m >> 8) as u8);
            output.push((u16_m & 0xFF) as u8);
            output.push(0u8);
            output.push(255u8);
            
            output.push((u16_s >> 8) as u8);
            output.push((u16_s & 0xFF) as u8);
            output.push(0u8);
            output.push(255u8);
        }
    }
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v1_two_pixels(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 { return Err(JsValue::from_str("Invalid V1 data")); }
    let mut len_bytes = [0u8; 4];
    let mut w_png_bytes = [0u8; 4];
    let mut h_bytes = [0u8; 4];
    
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    w_png_bytes.copy_from_slice(&rgba_data[4..8]);
    h_bytes.copy_from_slice(&rgba_data[8..12]);
    
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    let w_png = u32::from_be_bytes(w_png_bytes) as usize;
    let h = u32::from_be_bytes(h_bytes) as usize;
    let w = w_png / 2;
    
    let wavelet_type = rgba_data[12] as u32;
    let grid_size = w * h;
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            let offset_a = 16 + (r * w_png + (c * 2)) * 4;
            let offset_b = offset_a + 4;
            
            let u16_m = ((rgba_data[offset_a] as u16) << 8) | (rgba_data[offset_a + 1] as u16);
            let u16_s = ((rgba_data[offset_b] as u16) << 8) | (rgba_data[offset_b + 1] as u16);
            
            mid_grid[idx] = zigzag_decode(u16_m as u32) as i16;
            side_grid[idx] = zigzag_decode(u16_s as u32) as i16;
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
        
        original_data.push((u16_l & 0xFF) as u8);
        if original_data.len() < original_len { original_data.push((u16_l >> 8) as u8); }
        if original_data.len() < original_len { original_data.push((u16_r & 0xFF) as u8); }
        if original_data.len() < original_len { original_data.push((u16_r >> 8) as u8); }
    }
    Ok(original_data)
}

// V2: Single-Pixel Bitplane (Gray Code LUT Geodesic)
#[wasm_bindgen]
pub fn encode_wavelet_v2_bitplane(data: &[u8], h_custom: usize, wavelet_type: u32) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    
    let num_pairs = (data.len() + 3) / 4;
    let mut w = (num_pairs + h - 1) / h;
    if w < 1 { w = 1; }
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
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
    
    let depth = (h as f64).log2() as usize;
    forward_wpd(&mut mid_grid, depth, wavelet_type);
    forward_wpd(&mut side_grid, depth, wavelet_type);
    
    let mut output = Vec::with_capacity(16 + grid_size * 4);
    output.extend_from_slice(&original_len.to_be_bytes());
    output.extend_from_slice(&(w as u32).to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(wavelet_type as u8);
    output.push(2u8); // Packing Version 2!
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    ensure_luts();
    let lut = COLOR_LUT.get().unwrap();
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            
            let g_m = gray_encode(mid_grid[idx] as i32) as usize;
            let g_s = gray_encode(side_grid[idx] as i32) as usize;
            
            let (r_m, g_m, b_m) = lut[g_m % 65536];
            let (r_s, g_s, _b_s) = lut[g_s % 65536];
            
            output.push(r_m);
            output.push(g_m);
            output.push(r_s);
            output.push(255u8.wrapping_sub(g_s)); // Alpha inverted side detail
        }
    }
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v2_bitplane(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 { return Err(JsValue::from_str("Invalid V2 data")); }
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
    
    let expected_coeff_len = w * h * 4;
    if rgba_data.len() < 16 + expected_coeff_len { return Err(JsValue::from_str("Invalid V2 size")); }
    
    let grid_size = w * h;
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    ensure_luts();
    let rev = REVERSE_RG.get().unwrap();
    
    for r in 0..h {
        for c in 0..w {
            let offset = 16 + (r * w + c) * 4;
            let r_m = rgba_data[offset] as usize;
            let g_m = rgba_data[offset + 1] as usize;
            let r_s = rgba_data[offset + 2] as usize;
            let a_encoded = rgba_data[offset + 3];
            let g_s = 255u32.wrapping_sub(a_encoded as u32) as usize;
            
            let g_m_idx = rev[r_m * 256 + g_m] as u32;
            let g_s_idx = rev[r_s * 256 + g_s] as u32;
            
            let idx = r * w + c;
            mid_grid[idx] = gray_decode(g_m_idx) as i16;
            side_grid[idx] = gray_decode(g_s_idx) as i16;
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
        
        original_data.push((u16_l & 0xFF) as u8);
        if original_data.len() < original_len { original_data.push((u16_l >> 8) as u8); }
        if original_data.len() < original_len { original_data.push((u16_r & 0xFF) as u8); }
        if original_data.len() < original_len { original_data.push((u16_r >> 8) as u8); }
    }
    Ok(original_data)
}

// ==========================================
// 5. V3: Reversible Pure Arithmetic Serpentine Curve (High-Contrast Scale-Optimized)
// ==========================================

#[inline]
pub fn scale_coordinate(val: u16) -> u8 {
    (((val as f64) * 255.0 / 40.0).round()) as u8
}

#[inline]
pub fn unscale_coordinate(val: u8) -> u16 {
    (((val as f64) * 40.0 / 255.0).round()) as u16
}

// V3: Reversible Pure Arithmetic Serpentine Curve (8-bytes, RGB-only)
#[wasm_bindgen]
pub fn encode_wavelet_v3_serpentine(data: &[u8], h_custom: usize, wavelet_type: u32) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    let w = calculate_grid_width(data.len(), h);
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
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
    
    let depth = (h as f64).log2() as usize;
    forward_wpd(&mut mid_grid, depth, wavelet_type);
    forward_wpd(&mut side_grid, depth, wavelet_type);
    
    let mut output = Vec::with_capacity(16 + grid_size * 8);
    output.extend_from_slice(&original_len.to_be_bytes());
    let w_png = (w * 2) as u32;
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(wavelet_type as u8);
    output.push(3u8); // Packing Version 3!
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            
            let u16_m = zigzag_encode(mid_grid[idx] as i32);
            let u16_s = zigzag_encode(side_grid[idx] as i32);
            
            let rgb_m = decode_n(u16_m, 40);
            let rgb_s = decode_n(u16_s, 40);
            
            output.push(scale_coordinate(rgb_m.r));
            output.push(scale_coordinate(rgb_m.g));
            output.push(scale_coordinate(rgb_m.b));
            output.push(255u8);
            
            output.push(scale_coordinate(rgb_s.r));
            output.push(scale_coordinate(rgb_s.g));
            output.push(scale_coordinate(rgb_s.b));
            output.push(255u8);
        }
    }
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v3_serpentine(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 { return Err(JsValue::from_str("Invalid V3 data")); }
    let mut len_bytes = [0u8; 4];
    let mut w_png_bytes = [0u8; 4];
    let mut h_bytes = [0u8; 4];
    
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    w_png_bytes.copy_from_slice(&rgba_data[4..8]);
    h_bytes.copy_from_slice(&rgba_data[8..12]);
    
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    let w_png = u32::from_be_bytes(w_png_bytes) as usize;
    let h = u32::from_be_bytes(h_bytes) as usize;
    let w = w_png / 2;
    
    let wavelet_type = rgba_data[12] as u32;
    let grid_size = w * h;
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            let offset_a = 16 + (r * w_png + (c * 2)) * 4;
            let offset_b = offset_a + 4;
            
            let rgb_m = Rgb {
                r: unscale_coordinate(rgba_data[offset_a]),
                g: unscale_coordinate(rgba_data[offset_a + 1]),
                b: unscale_coordinate(rgba_data[offset_a + 2]),
            };
            let rgb_s = Rgb {
                r: unscale_coordinate(rgba_data[offset_b]),
                g: unscale_coordinate(rgba_data[offset_b + 1]),
                b: unscale_coordinate(rgba_data[offset_b + 2]),
            };
            
            let u16_m = encode_n(rgb_m, 40);
            let u16_s = encode_n(rgb_s, 40);
            
            mid_grid[idx] = zigzag_decode(u16_m) as i16;
            side_grid[idx] = zigzag_decode(u16_s) as i16;
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
        
        original_data.push((u16_l & 0xFF) as u8);
        if original_data.len() < original_len { original_data.push((u16_l >> 8) as u8); }
        if original_data.len() < original_len { original_data.push((u16_r & 0xFF) as u8); }
        if original_data.len() < original_len { original_data.push((u16_r >> 8) as u8); }
    }
    Ok(original_data)
}

// ==========================================
// 6. Transformada Wavelet Diádica de Mallat (DWT)
// ==========================================

pub fn forward_dwt(a: &mut [i16], depth: usize, wavelet_type: u32) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    
    forward_1d(a, wavelet_type);
    
    let half = len / 2;
    forward_dwt(&mut a[0..half], depth - 1, wavelet_type);
}

pub fn inverse_dwt(a: &mut [i16], depth: usize, wavelet_type: u32) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    
    let half = len / 2;
    inverse_dwt(&mut a[0..half], depth - 1, wavelet_type);
    
    inverse_1d(a, wavelet_type);
}

// V4: Reversible Pure Arithmetic Dyadic Wavelet Transform (8-bytes, RGB-only)
#[wasm_bindgen]
pub fn encode_wavelet_v4_dyadic_dwt(data: &[u8], h_custom: usize, wavelet_type: u32) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    let w = calculate_grid_width(data.len(), h);
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
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
    
    let depth = (h as f64).log2() as usize;
    
    // Apply Forward DWT directly on the entire flat contiguous grids!
    forward_dwt(&mut mid_grid, depth, wavelet_type);
    forward_dwt(&mut side_grid, depth, wavelet_type);
    
    let mut output = Vec::with_capacity(16 + grid_size * 8);
    output.extend_from_slice(&original_len.to_be_bytes());
    let w_png = (w * 2) as u32;
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(wavelet_type as u8);
    output.push(4u8); // Packing Version 4!
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            
            let u16_m = zigzag_encode(mid_grid[idx] as i32);
            let u16_s = zigzag_encode(side_grid[idx] as i32);
            
            let rgb_m = decode_n(u16_m, 40);
            let rgb_s = decode_n(u16_s, 40);
            
            output.push(scale_coordinate(rgb_m.r));
            output.push(scale_coordinate(rgb_m.g));
            output.push(scale_coordinate(rgb_m.b));
            output.push(255u8);
            
            output.push(scale_coordinate(rgb_s.r));
            output.push(scale_coordinate(rgb_s.g));
            output.push(scale_coordinate(rgb_s.b));
            output.push(255u8);
        }
    }
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v4_dyadic_dwt(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 { return Err(JsValue::from_str("Invalid V4 data")); }
    let mut len_bytes = [0u8; 4];
    let mut w_png_bytes = [0u8; 4];
    let mut h_bytes = [0u8; 4];
    
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    w_png_bytes.copy_from_slice(&rgba_data[4..8]);
    h_bytes.copy_from_slice(&rgba_data[8..12]);
    
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    let w_png = u32::from_be_bytes(w_png_bytes) as usize;
    let h = u32::from_be_bytes(h_bytes) as usize;
    let w = w_png / 2;
    
    let wavelet_type = rgba_data[12] as u32;
    let grid_size = w * h;
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    let depth = (h as f64).log2() as usize;
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            let offset_a = 16 + (r * w_png + (c * 2)) * 4;
            let offset_b = offset_a + 4;
            
            let rgb_m = Rgb {
                r: unscale_coordinate(rgba_data[offset_a]),
                g: unscale_coordinate(rgba_data[offset_a + 1]),
                b: unscale_coordinate(rgba_data[offset_a + 2]),
            };
            let rgb_s = Rgb {
                r: unscale_coordinate(rgba_data[offset_b]),
                g: unscale_coordinate(rgba_data[offset_b + 1]),
                b: unscale_coordinate(rgba_data[offset_b + 2]),
            };
            
            let u16_m = encode_n(rgb_m, 40);
            let u16_s = encode_n(rgb_s, 40);
            
            mid_grid[idx] = zigzag_decode(u16_m) as i16;
            side_grid[idx] = zigzag_decode(u16_s) as i16;
        }
    }
    
    // Apply Inverse DWT directly on the entire flat contiguous grids!
    inverse_dwt(&mut mid_grid, depth, wavelet_type);
    inverse_dwt(&mut side_grid, depth, wavelet_type);
    
    let mut original_data = Vec::with_capacity(original_len);
    for i in 0..((original_len + 3) / 4) {
        let m = mid_grid[i];
        let s = side_grid[i];
        let (l, r) = ms_to_lr(m, s);
        let u16_l = l as u16;
        let u16_r = r as u16;
        
        original_data.push((u16_l & 0xFF) as u8);
        if original_data.len() < original_len { original_data.push((u16_l >> 8) as u8); }
        if original_data.len() < original_len { original_data.push((u16_r & 0xFF) as u8); }
        if original_data.len() < original_len { original_data.push((u16_r >> 8) as u8); }
    }
    Ok(original_data)
}

// Map default encode_wavelet/decode_wavelet to point to V2 (Single Pixel Bitplane)
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
pub fn encode_naive_v0(data: &[u8]) -> Vec<u8> {
    encode_naive(data)
}

#[wasm_bindgen]
pub fn decode_naive_v0(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    decode_naive(rgba_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serpentine_arithmetic_bijection() {
        let m = 40;
        for n in 0..10000 {
            let rgb = decode_n(n, m);
            let n_rec = encode_n(rgb, m);
            if n != n_rec {
                println!("DIVERGENCE at index {}: RGB=({:?}), Rec={}", n, rgb, n_rec);
                assert_eq!(n, n_rec);
            }
        }
    }

    #[test]
    fn test_scaling_bijection() {
        for v in 0..=40 {
            let scaled = scale_coordinate(v);
            let unscaled = unscale_coordinate(scaled);
            assert_eq!(v, unscaled, "Scaling failed at value {}", v);
        }
    }

    #[test]
    fn test_v2_pipeline_large() {
        let mut original = vec![0u8; 4000];
        for i in 0..4000 {
            original[i] = (i % 256) as u8;
        }
        let encoded = encode_wavelet_v2_bitplane(&original, 256, 0);
        let decoded = decode_wavelet_v2_bitplane(&encoded).unwrap();
        assert_eq!(original, decoded, "V2 pipeline failed for large audio!");
    }

    #[test]
    fn test_v3_pipeline_large() {
        let mut original = vec![0u8; 4000];
        for i in 0..4000 {
            original[i] = (i % 256) as u8;
        }
        let encoded = encode_wavelet_v3_serpentine(&original, 256, 0);
        let decoded = decode_wavelet_v3_serpentine(&encoded).unwrap();
        assert_eq!(original, decoded, "V3 pipeline failed for large audio!");
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
            let encoded_rgba = encode_wavelet(&original_audio, 256, order);
            let decoded_audio = decode_wavelet(&encoded_rgba).unwrap();
            assert_eq!(original_audio, decoded_audio, "Pipeline failed for order {}", order);
        }
    }
}
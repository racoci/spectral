use wasm_bindgen::prelude::*;
use std::sync::OnceLock;

// V7 Vector Audio Model Modules
pub mod geometry;
pub mod model;
pub mod analysis;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct C32 {
    pub re: f32,
    pub im: f32,
}

impl C32 {
    #[inline(always)]
    pub fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }
    #[inline(always)]
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
    #[inline(always)]
    pub fn abs2(self) -> f32 {
        self.re * self.re + self.im * self.im
    }
    #[inline(always)]
    pub fn mul(self, b: Self) -> Self {
        Self::new(
            self.re * b.re - self.im * b.im,
            self.re * b.im + self.im * b.re,
        )
    }
    #[inline(always)]
    pub fn add_assign(&mut self, b: Self) {
        self.re += b.re;
        self.im += b.im;
    }
    #[inline(always)]
    pub fn scale(self, s: f32) -> Self {
        Self::new(self.re * s, self.im * s)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Grad {
    pub coeff: C32,
    pub freq_hz: f32,
    pub time_s: f32,
    pub confidence: f32,
}

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

#[wasm_bindgen]
pub fn zigzag_encode(val: i32) -> u32 {
    ((val << 1) ^ (val >> 31)) as u32
}

#[wasm_bindgen]
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

static COLOR_LUT: OnceLock<Vec<(u8, u8, u8)>> = OnceLock::new();
static DECODE_LUT: OnceLock<Vec<DecodeEntry>> = OnceLock::new();
static REVERSE_RG: OnceLock<Vec<u16>> = OnceLock::new();

fn generate_geodesic_snake_luts() {
    let mut all_pts = Vec::with_capacity(65536);
    for r in 0..256 {
        for g in 0..256 {
            all_pts.push((r as u8, g as u8));
        }
    }
    
    // Sort all 65,536 points strictly by Euclidean distance (circular rings),
    // and then by alternating serpentine angles to ensure perfect continuity.
    all_pts.sort_unstable_by(|a, b| {
        let dist_a = ((a.0 as f32).powi(2) + (a.1 as f32).powi(2)).sqrt();
        let dist_b = ((b.0 as f32).powi(2) + (b.1 as f32).powi(2)).sqrt();
        
        let ring_a = dist_a.floor() as i32;
        let ring_b = dist_b.floor() as i32;
        
        if ring_a != ring_b {
            ring_a.cmp(&ring_b)
        } else {
            let angle_a = (a.1 as f32).atan2(a.0 as f32);
            let angle_b = (b.1 as f32).atan2(b.0 as f32);
            let is_even = ring_a % 2 == 0;
            if is_even {
                angle_a.partial_cmp(&angle_b).unwrap()
            } else {
                angle_b.partial_cmp(&angle_a).unwrap()
            }
        }
    });
    
    let mut lut = vec![(0u8, 0u8, 0u8); 65536];
    let mut decode_vec = Vec::with_capacity(65536);
    let mut rev_rg = vec![0u16; 256 * 256];
    
    for i in 0..65536 {
        let p = all_pts[i];
        
        // Map blue channel dynamically as the absolute difference between Red and Green
        // to cover yellow, orange, and green while keeping silence perfectly black!
        let b_val = (p.0 as i16 - p.1 as i16).abs() as u8;
        
        lut[i] = (p.0, p.1, b_val);
        
        let key = (p.0 as u64 * p.0 as u64 + p.1 as u64 * p.1 as u64 + b_val as u64 * b_val as u64) * 16777216
            + (p.0 as u64 * 65536) + (p.1 as u64 * 256) + b_val as u64;
            
        decode_vec.push(DecodeEntry { key, index: i as u16 });
        
        let rg_idx = (p.0 as usize * 256) + p.1 as usize;
        rev_rg[rg_idx] = i as u16;
    }
    
    decode_vec.sort_unstable_by_key(|e| e.key);
    COLOR_LUT.set(lut).ok();
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
pub fn decode_rg_to_coefficient_raw(r: u8, g: u8) -> u16 {
    ensure_luts();
    let rev = REVERSE_RG.get().unwrap();
    let idx = (r as usize * 256) + g as usize;
    rev[idx]
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
pub fn decode_color_to_coefficient_raw(r: u8, g: u8, b: u8) -> u16 {
    ensure_luts();
    let dec = DECODE_LUT.get().unwrap();
    let key = (r as u64 * r as u64 + g as u64 * g as u64 + b as u64 * b as u64) * 16777216
        + (r as u64 * 65536) + (g as u64 * 256) + b as u64;

    let search = dec.binary_search_by_key(&key, |e| e.key);
    match search {
        Ok(pos) => dec[pos].index,
        Err(_) => {
            let rev = REVERSE_RG.get().unwrap();
            rev[(r as usize * 256) + g as usize]
        }
    }
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
    
    // Unpack and MS conversion into contiguous vertical blocks
    for c in 0..w {
        for r in 0..h {
            let i = c * h + r;
            let offset = i * 4;
            
            let b0 = if offset < data.len() { data[offset] } else { 0 };
            let b1 = if offset + 1 < data.len() { data[offset + 1] } else { 0 };
            let b2 = if offset + 2 < data.len() { data[offset + 2] } else { 0 };
            let b3 = if offset + 3 < data.len() { data[offset + 3] } else { 0 };
            
            let l_sample = (((b1 as u16) << 8) | (b0 as u16)) as i16;
            let r_sample = (((b3 as u16) << 8) | (b2 as u16)) as i16;
            
            let (m, s) = lr_to_ms(l_sample, r_sample);
            mid_grid[r * w + c] = m;
            side_grid[r * w + c] = s;
        }
    }
    
    let depth = (h as f64).log2() as usize;
    
    // Process columns of size H with Forward DWT
    for c in 0..w {
        let mut col_m = vec![0i16; h];
        let mut col_s = vec![0i16; h];
        for r in 0..h {
            col_m[r] = mid_grid[r * w + c];
            col_s[r] = side_grid[r * w + c];
        }
        
        forward_dwt(&mut col_m, depth, wavelet_type);
        forward_dwt(&mut col_s, depth, wavelet_type);
        
        for r in 0..h {
            mid_grid[r * w + c] = col_m[r];
            side_grid[r * w + c] = col_s[r];
        }
    }
    
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
    
    // Apply Inverse DWT directly on the columns
    for c in 0..w {
        let mut col_m = vec![0i16; h];
        let mut col_s = vec![0i16; h];
        for r in 0..h {
            col_m[r] = mid_grid[r * w + c];
            col_s[r] = side_grid[r * w + c];
        }
        
        inverse_dwt(&mut col_m, depth, wavelet_type);
        inverse_dwt(&mut col_s, depth, wavelet_type);
        
        for r in 0..h {
            mid_grid[r * w + c] = col_m[r];
            side_grid[r * w + c] = col_s[r];
        }
    }
    
    let mut original_data = Vec::with_capacity(original_len);
    for c in 0..w {
        for r in 0..h {
            let i = c * h + r;
            let offset = i * 4;
            if offset >= original_len { break; }
            
            let m = mid_grid[r * w + c];
            let s = side_grid[r * w + c];
            let (l, r_sample) = ms_to_lr(m, s);
            let u16_l = l as u16;
            let u16_r = r_sample as u16;
            
            original_data.push((u16_l & 0xFF) as u8);
            if original_data.len() < original_len { original_data.push((u16_l >> 8) as u8); }
            if original_data.len() < original_len { original_data.push((u16_r & 0xFF) as u8); }
            if original_data.len() < original_len { original_data.push((u16_r >> 8) as u8); }
        }
    }
    Ok(original_data)
}

// ==========================================
// 7. V5: Serpentina Diádica por Lifting CDF 5/3 (Lifting Wavelet Transform)
// ==========================================

pub fn forward_lifting_53(a: &mut [i16], depth: usize) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    // Split
    for i in 0..half {
        even[i] = a[2 * i];
        odd[i] = a[2 * i + 1];
    }
    
    // Predict step: d[i] = o[i] - floor((e[i] + e[i+1]) / 2)
    for i in 0..half {
        let left = even[i] as i32;
        let right = if i + 1 < half { even[i + 1] as i32 } else { even[i] as i32 };
        let pred = (left + right) >> 1;
        odd[i] = odd[i].wrapping_sub(pred as i16);
    }
    
    // Update step: s[i] = e[i] + floor((d[i-1] + d[i] + 2) / 4)
    for i in 0..half {
        let left = if i > 0 { odd[i - 1] as i32 } else { odd[0] as i32 };
        let right = odd[i] as i32;
        let upd = (left + right + 2) >> 2;
        even[i] = even[i].wrapping_add(upd as i16);
    }
    
    // Interleave back into `a` so that low-pass is in the first half and high-pass is in the second half
    for i in 0..half {
        a[i] = even[i];
        a[half + i] = odd[i];
    }
    
    // Recurse ONLY on the low-pass branch (first half)
    forward_lifting_53(&mut a[0..half], depth - 1);
}

pub fn inverse_lifting_53(a: &mut [i16], depth: usize) {
    if depth == 0 { return; }
    let len = a.len();
    if len < 2 { return; }
    let half = len / 2;
    
    // Recurse ONLY on the low-pass branch (first half) first
    inverse_lifting_53(&mut a[0..half], depth - 1);
    
    let mut even = vec![0i16; half];
    let mut odd = vec![0i16; half];
    
    // Extract from `a`
    for i in 0..half {
        even[i] = a[i];
        odd[i] = a[half + i];
    }
    
    // Inverse Update: e[i] = s[i] - floor((d[i-1] + d[i] + 2) / 4)
    for i in 0..half {
        let left = if i > 0 { odd[i - 1] as i32 } else { odd[0] as i32 };
        let right = odd[i] as i32;
        let upd = (left + right + 2) >> 2;
        even[i] = even[i].wrapping_sub(upd as i16);
    }
    
    // Inverse Predict: o[i] = d[i] + floor((e[i] + e[i+1]) / 2)
    for i in 0..half {
        let left = even[i] as i32;
        let right = if i + 1 < half { even[i + 1] as i32 } else { even[i] as i32 };
        let pred = (left + right) >> 1;
        odd[i] = odd[i].wrapping_add(pred as i16);
    }
    
    // Interleave back to original order
    for i in 0..half {
        a[2 * i] = even[i];
        a[2 * i + 1] = odd[i];
    }
}

// V5: Reversible Pure Arithmetic Dyadic Wavelet Transform by Lifting CDF 5/3 (8-bytes, RGB-only)
#[wasm_bindgen]
pub fn encode_wavelet_v5_dyadic_lifting(data: &[u8], h_custom: usize) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    let w = calculate_grid_width(data.len(), h);
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // Unpack and MS conversion into contiguous vertical blocks
    for c in 0..w {
        for r in 0..h {
            let i = c * h + r;
            let offset = i * 4;
            
            let b0 = if offset < data.len() { data[offset] } else { 0 };
            let b1 = if offset + 1 < data.len() { data[offset + 1] } else { 0 };
            let b2 = if offset + 2 < data.len() { data[offset + 2] } else { 0 };
            let b3 = if offset + 3 < data.len() { data[offset + 3] } else { 0 };
            
            let l_sample = (((b1 as u16) << 8) | (b0 as u16)) as i16;
            let r_sample = (((b3 as u16) << 8) | (b2 as u16)) as i16;
            
            let (m, s) = lr_to_ms(l_sample, r_sample);
            mid_grid[r * w + c] = m;
            side_grid[r * w + c] = s;
        }
    }
    
    let depth = (h as f64).log2() as usize;
    
    // Process columns of size H with Forward Lifting 5/3
    for c in 0..w {
        let mut col_m = vec![0i16; h];
        let mut col_s = vec![0i16; h];
        for r in 0..h {
            col_m[r] = mid_grid[r * w + c];
            col_s[r] = side_grid[r * w + c];
        }
        
        forward_lifting_53(&mut col_m, depth);
        forward_lifting_53(&mut col_s, depth);
        
        for r in 0..h {
            mid_grid[r * w + c] = col_m[r];
            side_grid[r * w + c] = col_s[r];
        }
    }
    
    let mut output = Vec::with_capacity(16 + grid_size * 8);
    output.extend_from_slice(&original_len.to_be_bytes());
    let w_png = (w * 2) as u32;
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(0u8); // wavelet_type default
    output.push(5u8); // Packing Version 5!
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
pub fn decode_wavelet_v5_dyadic_lifting(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 { return Err(JsValue::from_str("Invalid V5 data")); }
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
    
    // Process columns of size H with Inverse Lifting 5/3
    for c in 0..w {
        let mut col_m = vec![0i16; h];
        let mut col_s = vec![0i16; h];
        for r in 0..h {
            col_m[r] = mid_grid[r * w + c];
            col_s[r] = side_grid[r * w + c];
        }
        
        inverse_lifting_53(&mut col_m, depth);
        inverse_lifting_53(&mut col_s, depth);
        
        for r in 0..h {
            mid_grid[r * w + c] = col_m[r];
            side_grid[r * w + c] = col_s[r];
        }
    }
    
    let mut original_data = Vec::with_capacity(original_len);
    for c in 0..w {
        for r in 0..h {
            let i = c * h + r;
            let offset = i * 4;
            if offset >= original_len { break; }
            
            let m = mid_grid[r * w + c];
            let s = side_grid[r * w + c];
            let (l, r_sample) = ms_to_lr(m, s);
            let u16_l = l as u16;
            let u16_r = r_sample as u16;
            
            original_data.push((u16_l & 0xFF) as u8);
            if original_data.len() < original_len { original_data.push((u16_l >> 8) as u8); }
            if original_data.len() < original_len { original_data.push((u16_r & 0xFF) as u8); }
            if original_data.len() < original_len { original_data.push((u16_r >> 8) as u8); }
        }
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

// ==========================================
// 8. V6: Dual-Engine Spectrogram (Lifting CDF 5/3 + Gaussian Reassignment)
// ==========================================

#[wasm_bindgen]
pub fn encode_wavelet_v6_reassigned(data: &[u8], h_custom: usize) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    let w = calculate_grid_width(data.len(), h);
    let grid_size = w * h;
    
    let mut mid_grid = vec![0i16; grid_size];
    let mut side_grid = vec![0i16; grid_size];
    
    // Unpack and MS conversion into contiguous vertical blocks (using CDF 5/3 Lifting)
    for c in 0..w {
        for r in 0..h {
            let i = c * h + r;
            let offset = i * 4;
            
            let b0 = if offset < data.len() { data[offset] } else { 0 };
            let b1 = if offset + 1 < data.len() { data[offset + 1] } else { 0 };
            let b2 = if offset + 2 < data.len() { data[offset + 2] } else { 0 };
            let b3 = if offset + 3 < data.len() { data[offset + 3] } else { 0 };
            
            let l_sample = (((b1 as u16) << 8) | (b0 as u16)) as i16;
            let r_sample = (((b3 as u16) << 8) | (b2 as u16)) as i16;
            
            let (m, s) = lr_to_ms(l_sample, r_sample);
            mid_grid[r * w + c] = m;
            side_grid[r * w + c] = s;
        }
    }
    
    let depth = (h as f64).log2() as usize;
    
    // Process columns with Forward Lifting 5/3
    for c in 0..w {
        let mut col_m = vec![0i16; h];
        let mut col_s = vec![0i16; h];
        for r in 0..h {
            col_m[r] = mid_grid[r * w + c];
            col_s[r] = side_grid[r * w + c];
        }
        
        forward_lifting_53(&mut col_m, depth);
        forward_lifting_53(&mut col_s, depth);
        
        for r in 0..h {
            mid_grid[r * w + c] = col_m[r];
            side_grid[r * w + c] = col_s[r];
        }
    }
    
    let mut output = Vec::with_capacity(16 + grid_size * 8);
    output.extend_from_slice(&original_len.to_be_bytes());
    let w_png = (w * 2) as u32;
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(0u8); // wavelet_type
    output.push(6u8); // Packing Version 6!
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
pub fn decode_wavelet_v6_reassigned(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    // Reassigned storage is backward-compatible with V5 dyadic lifting decoder
    decode_wavelet_v5_dyadic_lifting(rgba_data)
}

struct FilterChannel {
    center_freq: f32,
    scale_s: f32,
}

#[wasm_bindgen]
pub fn wasm_generate_v6_spectrogram(rgba_data: &[u8]) -> Result<Vec<f32>, JsValue> {
    if rgba_data.len() < 16 {
        return Err(JsValue::from_str("Invalid input data"));
    }
    let packing_version = rgba_data[13];
    
    // Decode rgba_data back to PCM
    let pcm = match packing_version {
        1 => decode_wavelet_v1_two_pixels(rgba_data)?,
        2 => decode_wavelet_v2_bitplane(rgba_data)?,
        3 => decode_wavelet_v3_serpentine(rgba_data)?,
        4 => decode_wavelet_v4_dyadic_dwt(rgba_data)?,
        5 | 6 => decode_wavelet_v5_dyadic_lifting(rgba_data)?,
        _ => return Err(JsValue::from_str("Unsupported packing version")),
    };
    
    let w_png = u32::from_be_bytes([rgba_data[4], rgba_data[5], rgba_data[6], rgba_data[7]]) as usize;
    let h = u32::from_be_bytes([rgba_data[8], rgba_data[9], rgba_data[10], rgba_data[11]]) as usize;
    let w = if packing_version == 2 { w_png } else { w_png / 2 };
    
    let fs = ((rgba_data[14] as u16) << 8) | (rgba_data[15] as u16);
    let fs_f32 = if fs == 0 { 44100.0 } else { fs as f32 };
    
    let original_len = pcm.len();
    let num_samples = original_len / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = pcm[offset];
        let b1 = pcm[offset + 1];
        let b2 = pcm[offset + 2];
        let b3 = pcm[offset + 3];
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        
        mid_channel[i] = (l + r) * 0.5;
    }
    
    // Power buffer of size W * H
    let mut spec = vec![0.0f32; w * h];
    
    // Precompute 120 geometrically-spaced filter channels (1/12th octave spacing)
    let num_filters = h.min(120); // Match height H (up to 120 filters)
    let cycles = 6.0f32; // Number of cycles to capture in Gaussian window
    let mut channels = Vec::with_capacity(num_filters);
    
    for j in 0..num_filters {
        let step = 1.0f32 / 12.0f32;
        let fc = 20.0f32 * 2.0f32.powf(j as f32 * step);
        channels.push(FilterChannel {
            center_freq: fc,
            scale_s: cycles / fc,
        });
    }
    
    let total_duration_s = num_samples as f32 / fs_f32;
    
    if num_samples < 10 {
        return Ok(spec);
    }
    
    let hop = (num_samples as f32 / w as f32).max(1.0).floor() as usize;
    let ln2 = std::f32::consts::LN_2;
    let two_pi = 2.0 * std::f32::consts::PI;
    
    // Process frames and apply Non-Uniform Constant-Q Phase-Gradient 2D Reassignment
    for c in 0..w {
        let tau_sample = c * hop;
        let tau_s = tau_sample as f32 / fs_f32;
        
        for (j, ch) in channels.iter().enumerate() {
            let a = ch.scale_s;
            let inv_a2 = 1.0 / (a * a);
            
            // Dynamic window half-size (4 * a * fs_hz) capped at 1024 for real-time safety
            let half = ((4.0 * a * fs_f32).ceil() as isize).max(1).min(1024);
            
            let mut c_re = 0.0f32;
            let mut c_im = 0.0f32;
            let mut cg_re = 0.0f32;
            let mut cg_im = 0.0f32;
            let mut ct_re = 0.0f32;
            let mut ct_im = 0.0f32;
            
            for ni in -half..=half {
                let n = tau_sample as isize + ni;
                if n >= 0 && n < num_samples as isize {
                    let val = mid_channel[n as usize];
                    let u = ni as f32 / fs_f32; // relative time coordinate
                    
                    let q = u / a;
                    let g = (-ln2 * q * q).exp();
                    
                    // Analytical derivative of Gaussian window: g'(u) = -2*ln(2)/a^2 * u * g(u)
                    let gp = -(2.0 * ln2 * inv_a2) * u * g;
                    
                    // Complex exponential carriers (relative phase to avoid wrapping and keep stable)
                    let phase = ch.center_freq * two_pi * u;
                    let (s, co) = phase.sin_cos();
                    
                    // Complex multiplication components (using conjugate carrier e^-i*omega*t)
                    c_re  += val * g * co;
                    c_im  -= val * g * s;
                    
                    cg_re += val * gp * co;
                    cg_im -= val * gp * s;
                    
                    let ut = val * u * g;
                    ct_re += ut * co;
                    ct_im -= ut * s;
                }
            }
            
            let eps = 1e-14_f32;
            let denom = (c_re * c_re + c_im * c_im).max(eps);
            
            // Phase-gradient frequency reassignment (using stable conjugate division B * conj(A) / |A|^2)
            // ratio_g = cg * conj(c) / denom
            let ratio_g_im = (cg_im * c_re - cg_re * c_im) / denom;
            let fhat = ch.center_freq - (ratio_g_im / two_pi);
            
            // Phase-gradient time reassignment (using stable conjugate division ct * conj(c) / denom)
            // ratio_t = ct * conj(c) / denom
            let ratio_t_re = (ct_re * c_re + ct_im * c_im) / denom;
            let that = tau_s + ratio_t_re;
            
            // Energy power of the complex coefficient
            let power = c_re * c_re + c_im * c_im;
            if power < 1e-4 { continue; }
            
            // Signal confidence metric
            let conf = (power / (1.0 + power)).sqrt();
            let energy = power * conf;
            
            if fhat >= 20.0 && fhat <= 20000.0 && that >= 0.0 && that <= total_duration_s {
                // Logarithmic frequency Mel-Scale Coordinate [20 Hz, 20 kHz]
                let y_frac = (fhat / 20.0).ln() / 1000.0f32.ln();
                let pixel_y = y_frac * (h - 1) as f32;
                
                // Linear time coordinate matching physical duration
                let x_frac = (that / total_duration_s) * (w - 1) as f32;
                let pixel_x = x_frac;
                
                if pixel_y >= 0.0 && pixel_y <= (h - 1) as f32 && pixel_x >= 0.0 && pixel_x <= (w - 1) as f32 {
                    let y0 = pixel_y.floor() as usize;
                    let y1 = (y0 + 1).min(h - 1);
                    let frac_y = pixel_y - y0 as f32;
                    
                    let x0 = pixel_x.floor() as usize;
                    let x1 = (x0 + 1).min(w - 1);
                    let frac_x = pixel_x - x0 as f32;
                    
                    // Bilinear interpolation on BOTH dimensions (2D Reassigned Bilinear Deposit!)
                    spec[y0 * w + x0] += energy * (1.0 - frac_x) * (1.0 - frac_y);
                    spec[y0 * w + x1] += energy * frac_x * (1.0 - frac_y);
                    spec[y1 * w + x0] += energy * (1.0 - frac_x) * frac_y;
                    spec[y1 * w + x1] += energy * frac_x * frac_y;
                }
            }
        }
    }
    
    Ok(spec)
}

#[wasm_bindgen]
pub fn wasm_track_multiple(rgba_data: &[u8], num_tracks: usize) -> Result<String, JsValue> {
    if rgba_data.len() < 16 {
        return Err(JsValue::from_str("Invalid input data"));
    }
    let packing_version = rgba_data[13];
    
    // 1. Decode rgba_data back to PCM
    let pcm = match packing_version {
        1 => decode_wavelet_v1_two_pixels(rgba_data)?,
        2 => decode_wavelet_v2_bitplane(rgba_data)?,
        3 => decode_wavelet_v3_serpentine(rgba_data)?,
        4 => decode_wavelet_v4_dyadic_dwt(rgba_data)?,
        5 | 6 => decode_wavelet_v5_dyadic_lifting(rgba_data)?,
        _ => return Err(JsValue::from_str("Unsupported packing version")),
    };
    
    let w_png = u32::from_be_bytes([rgba_data[4], rgba_data[5], rgba_data[6], rgba_data[7]]) as usize;
    let h = u32::from_be_bytes([rgba_data[8], rgba_data[9], rgba_data[10], rgba_data[11]]) as usize;
    let w = if packing_version == 2 { w_png } else { w_png / 2 };
    
    let fs = ((rgba_data[14] as u16) << 8) | (rgba_data[15] as u16);
    let fs_f32 = if fs == 0 { 44100.0 } else { fs as f32 };
    
    let original_len = pcm.len();
    let num_samples = original_len / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = pcm[offset];
        let b1 = pcm[offset + 1];
        let b2 = pcm[offset + 2];
        let b3 = pcm[offset + 3];
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        
        mid_channel[i] = (l + r) * 0.5;
    }
    
    // Run Phase-Gradient CQT/Filter-bank analysis
    let num_filters = h.min(120);
    let cycles = 6.0f32;
    let mut channels = Vec::with_capacity(num_filters);
    for j in 0..num_filters {
        let step = 1.0f32 / 12.0f32;
        let fc = 20.0f32 * 2.0f32.powf(j as f32 * step);
        channels.push(FilterChannel {
            center_freq: fc,
            scale_s: cycles / fc,
        });
    }
    
    let total_duration_s = num_samples as f32 / fs_f32;
    if num_samples < 10 {
        return Err(JsValue::from_str("Audio too short for tracking"));
    }
    
    let hop = (num_samples as f32 / w as f32).max(1.0).floor() as usize;
    let ln2 = std::f32::consts::LN_2;
    let two_pi = 2.0 * std::f32::consts::PI;
    
    // Run STFT and compute grads matrix of shape [channels][frames]
    let mut grads = vec![vec![Grad::default(); w]; num_filters];
    
    for c in 0..w {
        let tau_sample = c * hop;
        
        for (j, ch) in channels.iter().enumerate() {
            let a = ch.scale_s;
            let inv_a2 = 1.0 / (a * a);
            let half = ((4.0 * a * fs_f32).ceil() as isize).max(1).min(1024);
            
            let mut c_re = 0.0f32;
            let mut c_im = 0.0f32;
            let mut cg_re = 0.0f32;
            let mut cg_im = 0.0f32;
            
            for ni in -half..=half {
                let n = tau_sample as isize + ni;
                if n >= 0 && n < num_samples as isize {
                    let val = mid_channel[n as usize];
                    let u = ni as f32 / fs_f32;
                    let q = u / a;
                    let g = (-ln2 * q * q).exp();
                    let gp = -(2.0 * ln2 * inv_a2) * u * g;
                    
                    let phase = ch.center_freq * two_pi * u;
                    let (s, co) = phase.sin_cos();
                    
                    c_re  += val * g * co;
                    c_im  -= val * g * s;
                    cg_re += val * gp * co;
                    cg_im -= val * gp * s;
                }
            }
            
            let eps = 1e-14_f32;
            let denom = (c_re * c_re + c_im * c_im).max(eps);
            let ratio_g_im = (cg_im * c_re - cg_re * c_im) / denom;
            let fhat = ch.center_freq - (ratio_g_im / two_pi);
            
            grads[j][c] = Grad {
                coeff: C32::new(c_re, c_im),
                freq_hz: fhat,
                time_s: tau_sample as f32 / fs_f32,
                confidence: (denom / (1.0 + denom)).sqrt(),
            };
        }
    }
    
    // Run multi-track spline tracker
    let cfg = analysis::multi_track::TrackingConfig::default();
    let result = analysis::multi_track::track_multiple(&grads, num_tracks, total_duration_s, &cfg);
    
    // Serialize to JSON string
    let mut json = String::new();
    json.push_str("[\n");
    for (i, track) in result.tracks.iter().enumerate() {
        json.push_str("  {\n");
        json.push_str("    \"segments\": [\n");
        for (j, seg) in track.spline.segments.iter().enumerate() {
            json.push_str(&format!(
                "      {{\"p0\": [{}, {}], \"p1\": [{}, {}], \"p2\": [{}, {}], \"p3\": [{}, {}]}}",
                seg.p0.t, seg.p0.u,
                seg.p1.t, seg.p1.u,
                seg.p2.t, seg.p2.u,
                seg.p3.t, seg.p3.u
            ));
            if j + 1 < track.spline.segments.len() {
                json.push_str(",\n");
            } else {
                json.push_str("\n");
            }
        }
        json.push_str("    ]\n");
        json.push_str("  }");
        if i + 1 < result.tracks.len() {
            json.push_str(",\n");
        } else {
            json.push_str("\n");
        }
    }
    json.push_str("]");
    
    Ok(json)
}

// ==========================================
// 10. V8: Reversible M-Band Polyphase Lifting Spectrogram (Lossless & Semantic)
// ==========================================

fn round_fixed_rust(x: i64, bits: u32) -> i64 {
    let den = 1i64 << bits;
    let half = den / 2;
    if x >= 0 {
        (x + half) / den
    } else {
        -((-x + half) / den)
    }
}

fn filter_fir_rust(e: &[i64], coefs: &[i64; 3]) -> Vec<i64> {
    let len = e.len();
    if len == 0 { return Vec::new(); }
    let mut out = vec![0i64; len];
    for k in 0..len {
        let em2 = e[k.saturating_sub(2)];
        let em1 = e[k.saturating_sub(1)];
        let ec  = e[k];
        let ep1 = e[std::cmp::min(k + 1, len - 1)];
        let ep2 = e[std::cmp::min(k + 2, len - 1)];
        
        let acc = coefs[0] * ec + coefs[1] * (em1 + ep1) + coefs[2] * (em2 + ep2);
        out[k] = round_fixed_rust(acc, 20);
    }
    out
}

fn forward_mband_4_rust(x: &[i64]) -> (Vec<i64>, Vec<i64>, Vec<i64>, Vec<i64>) {
    let len = x.len();
    let half = len / 4;
    let mut e  = Vec::with_capacity(half);
    let mut o1 = Vec::with_capacity(half);
    let mut o2 = Vec::with_capacity(half);
    let mut o3 = Vec::with_capacity(half);
    
    for i in 0..half {
        e.push(x[i * 4]);
        o1.push(x[i * 4 + 1]);
        o2.push(x[i * 4 + 2]);
        o3.push(x[i * 4 + 3]);
    }
    
    let p1 = [300000i64, 100000i64, 20000i64];
    let p2 = [450000i64, 150000i64, 30000i64];
    let p3 = [300000i64, 100000i64, 20000i64];
    let u_coefs = [200000i64, 80000i64, 15000i64];
    
    let pred1 = filter_fir_rust(&e, &p1);
    let pred2 = filter_fir_rust(&e, &p2);
    let pred3 = filter_fir_rust(&e, &p3);
    
    let mut d1 = vec![0i64; half];
    let mut d2 = vec![0i64; half];
    let mut d3 = vec![0i64; half];
    for i in 0..half {
        d1[i] = o1[i] - pred1[i];
        d2[i] = o2[i] - pred2[i];
        d3[i] = o3[i] - pred3[i];
    }
    
    let upd1 = filter_fir_rust(&d1, &u_coefs);
    let upd2 = filter_fir_rust(&d2, &u_coefs);
    let upd3 = filter_fir_rust(&d3, &u_coefs);
    
    let mut s = e.clone();
    for i in 0..half {
        s[i] += upd1[i] + upd2[i] + upd3[i];
    }
    (s, d1, d2, d3)
}

fn inverse_mband_4_rust(s: &[i64], d1: &[i64], d2: &[i64], d3: &[i64], original_len: usize) -> Vec<i64> {
    let half = s.len();
    let p1 = [300000i64, 100000i64, 20000i64];
    let p2 = [450000i64, 150000i64, 30000i64];
    let p3 = [300000i64, 100000i64, 20000i64];
    let u_coefs = [200000i64, 80000i64, 15000i64];
    
    let upd1 = filter_fir_rust(d1, &u_coefs);
    let upd2 = filter_fir_rust(d2, &u_coefs);
    let upd3 = filter_fir_rust(d3, &u_coefs);
    
    let mut e = s.to_vec();
    for i in 0..half {
        e[i] -= upd1[i] + upd2[i] + upd3[i];
    }
    
    let pred1 = filter_fir_rust(&e, &p1);
    let pred2 = filter_fir_rust(&e, &p2);
    let pred3 = filter_fir_rust(&e, &p3);
    
    let mut o1 = vec![0i64; half];
    let mut o2 = vec![0i64; half];
    let mut o3 = vec![0i64; half];
    for i in 0..half {
        o1[i] = d1[i] + pred1[i];
        o2[i] = d2[i] + pred2[i];
        o3[i] = d3[i] + pred3[i];
    }
    
    let mut out = vec![0i64; original_len];
    for i in 0..half {
        if i * 4 < original_len { out[i * 4] = e[i]; }
        if i * 4 + 1 < original_len { out[i * 4 + 1] = o1[i]; }
        if i * 4 + 2 < original_len { out[i * 4 + 2] = o2[i]; }
        if i * 4 + 3 < original_len { out[i * 4 + 3] = o3[i]; }
    }
    out
}

// Forward 1D CDF 5/3 Lifting on standard i64 arrays using strictly i16 wrapping arithmetic
fn forward_lifting_53_i64(x: &[i64]) -> (Vec<i64>, Vec<i64>) {
    let len = x.len();
    let half_e = (len + 1) / 2;
    let half_o = len / 2;
    let mut e = vec![0i16; half_e];
    let mut o = vec![0i16; half_o];
    for i in 0..len {
        if i % 2 == 0 { e[i / 2] = x[i] as i16; }
        else { o[i / 2] = x[i] as i16; }
    }
    
    let mut d = vec![0i16; half_o];
    for n in 0..half_o {
        let left = e[n];
        let right = if n + 1 < half_e { e[n + 1] } else { e[n] };
        let pred = ((left as i32 + right as i32) >> 1) as i16;
        d[n] = o[n].wrapping_sub(pred);
    }
    
    let mut s = vec![0i16; half_e];
    for n in 0..half_e {
        let left = if n > 0 { d[n - 1] } else { d[n] };
        let right = if n < half_o { d[n] } else { d[n - 1] };
        let upd = ((left as i32 + right as i32 + 2) >> 2) as i16;
        s[n] = e[n].wrapping_add(upd);
    }
    
    let s_out = s.into_iter().map(|v| v as i64).collect();
    let d_out = d.into_iter().map(|v| v as i64).collect();
    (s_out, d_out)
}

// Inverse 1D CDF 5/3 Lifting using strictly i16 wrapping arithmetic
fn inverse_lifting_53_i64(s: &[i64], d: &[i64], original_len: usize) -> Vec<i64> {
    let half_e = s.len();
    let half_o = d.len();
    
    let mut e = vec![0i16; half_e];
    for n in 0..half_e {
        let left = if n > 0 { d[n - 1] as i16 } else { d[n] as i16 };
        let right = if n < half_o { d[n] as i16 } else { d[n - 1] as i16 };
        let upd = ((left as i32 + right as i32 + 2) >> 2) as i16;
        e[n] = (s[n] as i16).wrapping_sub(upd);
    }
    
    let mut o = vec![0i16; half_o];
    for n in 0..half_o {
        let left = e[n];
        let right = if n + 1 < half_e { e[n + 1] } else { e[n] };
        let pred = ((left as i32 + right as i32) >> 1) as i16;
        o[n] = (d[n] as i16).wrapping_add(pred);
    }
    
    let mut out = vec![0i64; original_len];
    for i in 0..original_len {
        if i % 2 == 0 { out[i] = e[i / 2] as i64; }
        else { out[i] = o[i / 2] as i64; }
    }
    out
}

// Hierarchical 1-level interleaved CDF 5/3 decomposition
fn forward_packet_4_band(x: &[i64]) -> (Vec<i64>, Vec<i64>, Vec<i64>, Vec<i64>) {
    let (s_full, d_full) = forward_lifting_53_i64(x);
    let half = x.len() / 2;
    let mut s_even = vec![0i64; half / 2];
    let mut s_odd = vec![0i64; half / 2];
    let mut d_even = vec![0i64; half / 2];
    let mut d_odd = vec![0i64; half / 2];
    for i in 0..(half / 2) {
        s_even[i] = s_full[2 * i];
        s_odd[i] = s_full[2 * i + 1];
        d_even[i] = d_full[2 * i];
        d_odd[i] = d_full[2 * i + 1];
    }
    (s_even, s_odd, d_even, d_odd)
}

// Interleaved 1-level reconstruction
fn inverse_packet_4_band(s_even: &[i64], s_odd: &[i64], d_even: &[i64], d_odd: &[i64], original_len: usize) -> Vec<i64> {
    let half = original_len / 2;
    let mut s_full = vec![0i64; half];
    let mut d_full = vec![0i64; half];
    for i in 0..(half / 2) {
        s_full[2 * i] = s_even[i];
        s_full[2 * i + 1] = s_odd[i];
        d_full[2 * i] = d_even[i];
        d_full[2 * i + 1] = d_odd[i];
    }
    inverse_lifting_53_i64(&s_full, &d_full, original_len)
}

#[wasm_bindgen]
pub fn encode_wavelet_v8_mband(data: &[u8], h_custom: usize) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    // Since each grid position holds 2 samples (Mid/Side), we scale down data len accordingly
    let w = calculate_grid_width(data.len() / 2, h);
    let grid_size = w * h;
    
    // We pad the raw PCM Mid/Side data up to the grid boundary (grid_size * 2 samples)
    let num_samples = data.len() / 4;
    let mut mid_input = vec![0i64; grid_size * 2];
    let mut side_input = vec![0i64; grid_size * 2];
    
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = if offset < data.len() { data[offset] } else { 0 };
        let b1 = if offset + 1 < data.len() { data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < data.len() { data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < data.len() { data[offset + 3] } else { 0 };
        
        let l_sample = (((b1 as u16) << 8) | (b0 as u16)) as i16;
        let r_sample = (((b3 as u16) << 8) | (b2 as u16)) as i16;
        
        let (m, s) = lr_to_ms(l_sample, r_sample);
        mid_input[i] = m as i64;
        side_input[i] = s as i64;
    }
    
    // Process entire mono and side sequences through the exactly-bounded 1-level CDF 5/3 Lifting
    let (s_m, d_m) = forward_lifting_53_i64(&mid_input);
    let (s_s, d_s) = forward_lifting_53_i64(&side_input);
    
    let mut output = Vec::with_capacity(24 + grid_size * 16);
    output.extend_from_slice(&original_len.to_be_bytes());
    let w_png = (w * 4) as u32; // 4 pixels per sample point to pack Mid & Side 16-bit coefficients with Alpha=255
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    let rem_bytes = (original_len % 4) as u8;
    output.push(rem_bytes); // byte 12: rem_bytes count!
    output.push(8u8); // byte 13: Packing Version 8!
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    // Store 24-bit odd-byte residuals into metadata bytes 16-18
    let b1_odd = if rem_bytes >= 1 { data[num_samples * 4] } else { 0 };
    let b2_odd = if rem_bytes >= 2 { data[num_samples * 4 + 1] } else { 0 };
    let b3_odd = if rem_bytes == 3 { data[num_samples * 4 + 2] } else { 0 };
    output.push(b1_odd); // byte 16
    output.push(b2_odd); // byte 17
    output.push(b3_odd); // byte 18
    
    // Bytes 19-23: Padding up to 24 bytes
    output.push(0u8);
    output.push(0u8);
    output.push(0u8);
    output.push(0u8);
    output.push(0u8);
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            
            // Map signed 16-bit coefficients with ZigZag encoding to preserve bijection and map silence (0) to black (0)
            let sm_u = zigzag_encode(s_m[idx].clamp(-32768, 32767) as i32) as u16;
            let dm_u = zigzag_encode(d_m[idx].clamp(-32768, 32767) as i32) as u16;
            
            let ss_u = zigzag_encode(s_s[idx].clamp(-32768, 32767) as i32) as u16;
            let ds_u = zigzag_encode(d_s[idx].clamp(-32768, 32767) as i32) as u16;
            
            // Retrieve beautiful, high-contrast Geodesic Snake colors from the 3D RGB Cube!
            // Pixel A: sm_u (Mid Lowpass)
            output.push(get_color_r(sm_u));
            output.push(get_color_g(sm_u));
            output.push(get_color_b(sm_u));
            output.push(255u8);
            
            // Pixel B: dm_u (Mid Highpass)
            output.push(get_color_r(dm_u));
            output.push(get_color_g(dm_u));
            output.push(get_color_b(dm_u));
            output.push(255u8);
            
            // Pixel C: ss_u (Side Lowpass)
            output.push(get_color_r(ss_u));
            output.push(get_color_g(ss_u));
            output.push(get_color_b(ss_u));
            output.push(255u8);
            
            // Pixel D: ds_u (Side Highpass)
            output.push(get_color_r(ds_u));
            output.push(get_color_g(ds_u));
            output.push(get_color_b(ds_u));
            output.push(255u8);
        }
    }
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v8_mband(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 24 {
        return Err(JsValue::from_str("Invalid input data"));
    }
    let original_len = u32::from_be_bytes([rgba_data[0], rgba_data[1], rgba_data[2], rgba_data[3]]) as usize;
    let w_png = u32::from_be_bytes([rgba_data[4], rgba_data[5], rgba_data[6], rgba_data[7]]) as usize;
    let h = u32::from_be_bytes([rgba_data[8], rgba_data[9], rgba_data[10], rgba_data[11]]) as usize;
    
    let w = w_png / 4;
    let grid_size = w * h;
    
    let mut s_m = vec![0i64; grid_size];
    let mut d_m = vec![0i64; grid_size];
    
    let mut s_s = vec![0i64; grid_size];
    let mut d_s = vec![0i64; grid_size];
    
    let rem_bytes = rgba_data[12] as usize;
    let byte1 = rgba_data[16];
    let byte2 = rgba_data[17];
    let byte3 = rgba_data[18];
    
    for r in 0..h {
        for c in 0..w {
            let idx = r * w + c;
            let offset_a = 24 + (r * w_png + c * 4) * 4;
            let offset_b = offset_a + 4;
            let offset_c = offset_a + 8;
            let offset_d = offset_a + 12;
            
            if offset_d + 3 >= rgba_data.len() {
                return Err(JsValue::from_str("Truncated image buffer in V8 decoding"));
            }
            
            // Decode RGB colors back to exact 16-bit coefficients using Geodesic Snake bijection!
            let sm_u = decode_rg_to_coefficient_raw(rgba_data[offset_a], rgba_data[offset_a + 1]);
            let dm_u = decode_rg_to_coefficient_raw(rgba_data[offset_b], rgba_data[offset_b + 1]);
            
            let ss_u = decode_rg_to_coefficient_raw(rgba_data[offset_c], rgba_data[offset_c + 1]);
            let ds_u = decode_rg_to_coefficient_raw(rgba_data[offset_d], rgba_data[offset_d + 1]);
            
            s_m[idx] = zigzag_decode(sm_u as u32) as i64;
            d_m[idx] = zigzag_decode(dm_u as u32) as i64;
            
            s_s[idx] = zigzag_decode(ss_u as u32) as i64;
            d_s[idx] = zigzag_decode(ds_u as u32) as i64;
        }
    }
    
    // Reconstruct through the exactly-bounded 1-level inverse CDF 5/3 Lifting
    let mid_reconstructed = inverse_lifting_53_i64(&s_m, &d_m, grid_size * 2);
    let side_reconstructed = inverse_lifting_53_i64(&s_s, &d_s, grid_size * 2);
    
    let num_samples = original_len / 4;
    let mut pcm = vec![0u8; original_len];
    
    for i in 0..num_samples {
        let m = mid_reconstructed[i] as i16;
        let s = side_reconstructed[i] as i16;
        let (l, r) = ms_to_lr(m, s);
        
        let offset = i * 4;
        pcm[offset]     = (l & 0xFF) as u8;
        pcm[offset + 1] = ((l >> 8) & 0xFF) as u8;
        pcm[offset + 2] = (r & 0xFF) as u8;
        pcm[offset + 3] = ((r >> 8) & 0xFF) as u8;
    }
    
    // Re-inject the odd-byte residuals at the end of the PCM stream
    if rem_bytes >= 1 && num_samples * 4 < original_len {
        pcm[num_samples * 4] = byte1;
    }
    if rem_bytes >= 2 && num_samples * 4 + 1 < original_len {
        pcm[num_samples * 4 + 1] = byte2;
    }
    if rem_bytes == 3 && num_samples * 4 + 2 < original_len {
        pcm[num_samples * 4 + 2] = byte3;
    }
    
    Ok(pcm)
}

#[wasm_bindgen]
pub fn encode_wavelet_v9_reassigned(data: &[u8], h_custom: usize) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    
    // Compute the gorgeous, high-fidelity reassigned spectrogram with target width of 800 columns
    let re_rgba = wasm_calculate_reassigned_spectrogram(data, h, "hann");
    
    let w = 800;
    let grid_size = w * h;
    
    let mut output = Vec::with_capacity(24 + grid_size * 4 + data.len());
    // 24-byte big-endian header
    output.extend_from_slice(&original_len.to_be_bytes());
    output.extend_from_slice(&(w as u32).to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    output.push(0u8); // rem_bytes
    output.push(9u8); // Packing Version 9!
    output.extend_from_slice(&44100u16.to_be_bytes());
    for _ in 0..8 { output.push(0u8); } // padding
    
    // Copy the gorgeous high-fidelity pixels from re_rgba
    output.extend_from_slice(&re_rgba[24..(24 + grid_size * 4)]);
    
    // Append the entire original WAV file exactly as the steganographic payload!
    output.extend_from_slice(data);
    
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v9_reassigned(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 24 {
        return Err(JsValue::from_str("Invalid input data"));
    }
    let original_len = u32::from_be_bytes([rgba_data[0], rgba_data[1], rgba_data[2], rgba_data[3]]) as usize;
    let w = u32::from_be_bytes([rgba_data[4], rgba_data[5], rgba_data[6], rgba_data[7]]) as usize;
    let h = u32::from_be_bytes([rgba_data[8], rgba_data[9], rgba_data[10], rgba_data[11]]) as usize;
    
    let grid_size = w * h;
    
    // The payload start point in standard savePng is at: 24 (metadata header offset in file) + width * (height - 1) * 4
    // Since height = h + 1, height - 1 = h, the offset is exactly 24 + w * h * 4!
    let payload_start = 24 + grid_size * 4;
    if payload_start + original_len > rgba_data.len() {
        return Err(JsValue::from_str("Truncated steganographic payload in V9 decoding"));
    }
    
    let original_wav = rgba_data[payload_start..(payload_start + original_len)].to_vec();
    Ok(original_wav)
}

#[wasm_bindgen]
pub fn wasm_generate_v8_spectrogram(rgba_data: &[u8]) -> Result<Vec<f32>, JsValue> {
    if rgba_data.len() < 24 {
        return Err(JsValue::from_str("Invalid input data"));
    }
    let w_png = u32::from_be_bytes([rgba_data[4], rgba_data[5], rgba_data[6], rgba_data[7]]) as usize;
    let h = u32::from_be_bytes([rgba_data[8], rgba_data[9], rgba_data[10], rgba_data[11]]) as usize;
    let w = w_png / 4;
    let mut spec = vec![0.0f32; w * h];
    
    for r in 0..h {
        for c in 0..w {
            let offset_a = 24 + (r * w_png + c * 4) * 4;
            let offset_b = offset_a + 4;
            
            if offset_b + 3 >= rgba_data.len() { break; }
            
            // Extract highpass detail from Pixel B
            let dm_u = decode_rg_to_coefficient_raw(rgba_data[offset_b], rgba_data[offset_b + 1]);
            let d_val = zigzag_decode(dm_u as u32) as f32;
            
            spec[r * w + c] = d_val * d_val;
        }
    }
    Ok(spec)
}

#[derive(Clone)]
pub struct ParsedWavInfo {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub samples: Vec<f32>,
}

pub fn parse_wav_samples(data: &[u8]) -> ParsedWavInfo {
    if data.len() >= 44 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        let channels = u16::from_le_bytes([data[22], data[23]]);
        let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        let bits_per_sample = u16::from_le_bytes([data[34], data[35]]);
        let pcm_data = &data[44..];

        let mut samples = Vec::new();
        if bits_per_sample == 16 {
            let bytes_per_frame = (channels as usize) * 2;
            if bytes_per_frame > 0 {
                let num_frames = pcm_data.len() / bytes_per_frame;
                samples.reserve(num_frames);

                for f in 0..num_frames {
                    let offset = f * bytes_per_frame;
                    if channels == 1 {
                        let s = i16::from_le_bytes([pcm_data[offset], pcm_data[offset + 1]]) as f32;
                        samples.push(s);
                    } else if channels >= 2 {
                        let l = i16::from_le_bytes([pcm_data[offset], pcm_data[offset + 1]]) as f32;
                        let r = i16::from_le_bytes([pcm_data[offset + 2], pcm_data[offset + 3]]) as f32;
                        samples.push((l + r) * 0.5);
                    }
                }
            }
        } else {
            let num_samples = pcm_data.len() / 2;
            samples.reserve(num_samples);
            for i in 0..num_samples {
                let offset = i * 2;
                if offset + 1 < pcm_data.len() {
                    let s = i16::from_le_bytes([pcm_data[offset], pcm_data[offset + 1]]) as f32;
                    samples.push(s);
                }
            }
        }

        ParsedWavInfo {
            channels,
            sample_rate: if sample_rate > 0 { sample_rate } else { 44100 },
            bits_per_sample,
            samples,
        }
    } else {
        let num_samples = data.len() / 2;
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let offset = i * 2;
            if offset + 1 < data.len() {
                let s = i16::from_le_bytes([data[offset], data[offset + 1]]) as f32;
                samples.push(s);
            }
        }
        ParsedWavInfo {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            samples,
        }
    }
}

#[wasm_bindgen]
pub fn wasm_calculate_reassigned_spectrogram(data: &[u8], h_custom: usize, window_type: &str) -> Vec<u8> {
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    
    // Detect and skip 44-byte WAV header if present
    let pcm_data = if data.len() >= 44 && &data[0..4] == b"RIFF" {
        &data[44..]
    } else {
        data
    };
    
    let w = 800; // Fixed target landscape width of 800 columns
    let grid_size = w * h;
    
    // Decoded PCM Mid channel data
    let num_samples = pcm_data.len() / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = if offset < pcm_data.len() { pcm_data[offset] } else { 0 };
        let b1 = if offset + 1 < pcm_data.len() { pcm_data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < pcm_data.len() { pcm_data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < pcm_data.len() { pcm_data[offset + 3] } else { 0 };
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        mid_channel[i] = (l + r) * 0.5;
    }
    
    let fs_f32 = 44100.0f32;
    let n_stft = 4096;
    
    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    
    // Generate the window h, time-weighted window th, and derivative window dh
    let mut win_h = vec![0.0f32; n_stft];
    let mut win_th = vec![0.0f32; n_stft];
    let mut win_dh = vec![0.0f32; n_stft];
    
    let half_n = (n_stft - 1) as f32 / 2.0;
    
    match window_type {
        "hamming" => {
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = 0.54 - 0.46 * angle.cos();
                win_th[i] = (i as f32 - half_n) * win_h[i];
                win_dh[i] = (0.46 * 2.0 * std::f32::consts::PI / (n_stft - 1) as f32) * angle.sin();
            }
        },
        "gaussian" => {
            let sigma = (n_stft - 1) as f32 / 6.0; // alpha = 3.0
            for i in 0..n_stft {
                let diff = i as f32 - half_n;
                win_h[i] = (-0.5 * (diff / sigma).powi(2)).exp();
                win_th[i] = diff * win_h[i];
                win_dh[i] = -(diff / sigma.powi(2)) * win_h[i];
            }
        },
        "blackman-harris" => {
            let a0 = 0.35875f32;
            let a1 = 0.48829f32;
            let a2 = 0.14128f32;
            let a3 = 0.01168f32;
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = a0 - a1 * angle.cos() + a2 * (2.0 * angle).cos() - a3 * (3.0 * angle).cos();
                win_th[i] = (i as f32 - half_n) * win_h[i];
                win_dh[i] = (2.0 * std::f32::consts::PI / (n_stft - 1) as f32) * 
                           (a1 * angle.sin() - 2.0 * a2 * (2.0 * angle).sin() + 3.0 * a3 * (3.0 * angle).sin());
            }
        },
        _ => { // "hann" as default
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = 0.5 * (1.0 - angle.cos());
                win_th[i] = (i as f32 - half_n) * win_h[i];
                win_dh[i] = (std::f32::consts::PI / (n_stft - 1) as f32) * angle.sin();
            }
        }
    }
    
    let fmin = 20.0f32;
    let fmax = 20000.0f32.min(fs_f32 / 2.0);
    let step = (fmax / fmin).log2() / (h as f32 - 1.0);
    
    let mut reassigned_grid = vec![0.0f32; grid_size];
    let hop = (num_samples as f32 / w as f32).max(1.0).floor() as usize;
    let duration_seconds = num_samples as f32 / fs_f32;
    
    for c in 0..w {
        let start = c * hop;
        let mut buffer_h = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buffer_th = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buffer_dh = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        
        for i in 0..n_stft {
            let idx = start as isize + i as isize - (n_stft as isize / 2);
            if idx >= 0 && idx < num_samples as isize {
                let sample_val = mid_channel[idx as usize];
                buffer_h[i] = Complex::new(sample_val * win_h[i], 0.0);
                buffer_th[i] = Complex::new(sample_val * win_th[i], 0.0);
                buffer_dh[i] = Complex::new(sample_val * win_dh[i], 0.0);
            }
        }
        
        fft.process(&mut buffer_h);
        fft.process(&mut buffer_th);
        fft.process(&mut buffer_dh);
        
        for j in 0..h {
            let fc = fmin * 2.0f32.powf(j as f32 * step);
            let k = (fc * n_stft as f32 / fs_f32).round() as usize;
            let k = k.clamp(1, n_stft / 2 - 1);
            
            let s_h = buffer_h[k];
            let s_th = buffer_th[k];
            let s_dh = buffer_dh[k];
            
            let mag_sq = s_h.re * s_h.re + s_h.im * s_h.im;
            if mag_sq > 1e-2 {
                // Time Reassignment: t_reassigned = t + Re{ S_th * conj(S_h) / |S_h|^2 } (shift in samples)
                let s_th_conj = s_th * s_h.conj();
                let t_shift = s_th_conj.re / mag_sq; // shift in samples
                let c_reassigned = (c as f32 + t_shift / hop as f32).round() as isize;
                
                // Frequency Reassignment: f_reassigned = f - Im{ S_dh * conj(S_h) / |S_h|^2 } * (fs / 2pi)
                let s_dh_conj = s_dh * s_h.conj();
                let omega_shift = s_dh_conj.im / mag_sq; // shift in radians/sample
                let f_reassigned = fc - (omega_shift * fs_f32 / (2.0 * std::f32::consts::PI));
                
                let j_reassigned = ((f_reassigned / fmin).log2() / step).round() as isize;
                
                if c_reassigned >= 0 && c_reassigned < w as isize && j_reassigned >= 0 && j_reassigned < h as isize {
                    let target_idx = j_reassigned as usize * w + c_reassigned as usize;
                    reassigned_grid[target_idx] += mag_sq;
                }
            }
        }
    }
    
    // Normalize and apply 0.3 gamma compression
    let mut max_val = 1e-12f32;
    for i in 0..grid_size {
        if reassigned_grid[i] > max_val { max_val = reassigned_grid[i]; }
    }
    
    // Pack into direct self-contained RGBA buffer with big-endian header!
    let mut output = Vec::with_capacity(24 + grid_size * 4);
    output.extend_from_slice(&0u32.to_be_bytes()); // original_len
    output.extend_from_slice(&(w as u32).to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    for _ in 0..12 { output.push(0u8); } // padding
    
    for r in 0..h {
        for c in 0..w {
            let val = (reassigned_grid[r * w + c] / max_val).powf(0.3);
            let color_idx = (val * 65535.0).round() as u16;
            output.push(get_color_r(color_idx));
            output.push(get_color_g(color_idx));
            output.push(get_color_b(color_idx));
            output.push(255u8);
        }
    }
    output
}

#[wasm_bindgen]
pub fn wasm_calculate_complex_reassigned_spectrogram(data: &[u8], h_custom: usize, window_type: &str) -> Vec<f32> {
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    
    // Detect and skip 44-byte WAV header if present
    let pcm_data = if data.len() >= 44 && &data[0..4] == b"RIFF" {
        &data[44..]
    } else {
        data
    };
    
    // Calculate full width based on intrinsic audio length
    let w = calculate_grid_width(pcm_data.len() / 2, h);
    let grid_size = w * h;
    
    // Decoded PCM Mid channel data
    let num_samples = pcm_data.len() / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = if offset < pcm_data.len() { pcm_data[offset] } else { 0 };
        let b1 = if offset + 1 < pcm_data.len() { pcm_data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < pcm_data.len() { pcm_data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < pcm_data.len() { pcm_data[offset + 3] } else { 0 };
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        mid_channel[i] = (l + r) * 0.5;
    }
    
    let fs_f32 = 44100.0f32;
    let n_stft = 4096;
    
    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    
    // Generate the window h, time-weighted window th, and derivative window dh
    let mut win_h = vec![0.0f32; n_stft];
    let mut win_th = vec![0.0f32; n_stft];
    let mut win_dh = vec![0.0f32; n_stft];
    
    let half_n = (n_stft - 1) as f32 / 2.0;
    
    match window_type {
        "hamming" => {
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = 0.54 - 0.46 * angle.cos();
                win_th[i] = (i as f32 - half_n) * win_h[i];
                win_dh[i] = (0.46 * 2.0 * std::f32::consts::PI / (n_stft - 1) as f32) * angle.sin();
            }
        },
        "gaussian" => {
            let sigma = (n_stft - 1) as f32 / 6.0; // alpha = 3.0
            for i in 0..n_stft {
                let diff = i as f32 - half_n;
                win_h[i] = (-0.5 * (diff / sigma).powi(2)).exp();
                win_th[i] = diff * win_h[i];
                win_dh[i] = -(diff / sigma.powi(2)) * win_h[i];
            }
        },
        "blackman-harris" => {
            let a0 = 0.35875f32;
            let a1 = 0.48829f32;
            let a2 = 0.14128f32;
            let a3 = 0.01168f32;
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = a0 - a1 * angle.cos() + a2 * (2.0 * angle).cos() - a3 * (3.0 * angle).cos();
                win_th[i] = (i as f32 - half_n) * win_h[i];
                win_dh[i] = (2.0 * std::f32::consts::PI / (n_stft - 1) as f32) * 
                           (a1 * angle.sin() - 2.0 * a2 * (2.0 * angle).sin() + 3.0 * a3 * (3.0 * angle).sin());
            }
        },
        _ => { // "hann" as default
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = 0.5 * (1.0 - angle.cos());
                win_th[i] = (i as f32 - half_n) * win_h[i];
                win_dh[i] = (std::f32::consts::PI / (n_stft - 1) as f32) * angle.sin();
            }
        }
    }
    
    let fmin = 20.0f32;
    let fmax = 20000.0f32.min(fs_f32 / 2.0);
    let step = (fmax / fmin).log2() / (h as f32 - 1.0);
    
    let mut reassigned_grid_re = vec![0.0f32; grid_size];
    let mut reassigned_grid_im = vec![0.0f32; grid_size];
    let hop = (num_samples as f32 / w as f32).max(1.0).floor() as usize;
    
    for c in 0..w {
        let start = c * hop;
        let mut buffer_h = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buffer_th = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buffer_dh = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        
        for i in 0..n_stft {
            let idx = start as isize + i as isize - (n_stft as isize / 2);
            if idx >= 0 && idx < num_samples as isize {
                let sample_val = mid_channel[idx as usize];
                buffer_h[i] = Complex::new(sample_val * win_h[i], 0.0);
                buffer_th[i] = Complex::new(sample_val * win_th[i], 0.0);
                buffer_dh[i] = Complex::new(sample_val * win_dh[i], 0.0);
            }
        }
        
        fft.process(&mut buffer_h);
        fft.process(&mut buffer_th);
        fft.process(&mut buffer_dh);
        
        for j in 0..h {
            let fc = fmin * 2.0f32.powf(j as f32 * step);
            let k = (fc * n_stft as f32 / fs_f32).round() as usize;
            let k = k.clamp(1, n_stft / 2 - 1);
            
            let s_h = buffer_h[k];
            let s_th = buffer_th[k];
            let s_dh = buffer_dh[k];
            
            let mag_sq = s_h.re * s_h.re + s_h.im * s_h.im;
            if mag_sq > 1e-2 {
                // Time Reassignment
                let s_th_conj = s_th * s_h.conj();
                let t_shift = s_th_conj.re / mag_sq; // shift in samples
                let c_reassigned = (c as f32 + t_shift / hop as f32).round() as isize;
                
                // Frequency Reassignment
                let s_dh_conj = s_dh * s_h.conj();
                let omega_shift = s_dh_conj.im / mag_sq; // shift in radians/sample
                let f_reassigned = fc - (omega_shift * fs_f32 / (2.0 * std::f32::consts::PI));
                
                let j_reassigned = ((f_reassigned / fmin).log2() / step).round() as isize;
                
                if c_reassigned >= 0 && c_reassigned < w as isize && j_reassigned >= 0 && j_reassigned < h as isize {
                    let target_idx = j_reassigned as usize * w + c_reassigned as usize;
                    // Coherent complex summation instead of power aggregation!
                    reassigned_grid_re[target_idx] += s_h.re;
                    reassigned_grid_im[target_idx] += s_h.im;
                }
            }
        }
    }
    
    let mut complex_grid = vec![0.0f32; grid_size * 2];
    for i in 0..grid_size {
        complex_grid[i * 2] = reassigned_grid_re[i];
        complex_grid[i * 2 + 1] = reassigned_grid_im[i];
    }
    
    complex_grid
}

static LAST_MIRRORED_DENSITY: std::sync::Mutex<Vec<f32>> = std::sync::Mutex::new(Vec::new());

#[derive(Clone, Copy)]
pub struct SpectralQuadruplet {
    pub t: f32,
    pub f: f32,
    pub log_a: f32,
    pub phi: f32,
}

pub struct KdNode {
    pub point: SpectralQuadruplet,
    pub log_f: f32,
    pub axis: u8,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub bbox_t_min: f32,
    pub bbox_t_max: f32,
    pub bbox_log_f_min: f32,
    pub bbox_log_f_max: f32,
}

pub struct SpectralKdTree {
    pub nodes: Vec<KdNode>,
    pub root: Option<usize>,
}

impl SpectralKdTree {
    pub fn build(mut points: Vec<SpectralQuadruplet>) -> Self {
        if points.is_empty() {
            return SpectralKdTree { nodes: Vec::new(), root: None };
        }
        let mut nodes = Vec::with_capacity(points.len());
        let root = Self::build_recursive(&mut points, 0, &mut nodes);
        SpectralKdTree { nodes, root: Some(root) }
    }

    fn build_recursive(
        points: &mut [SpectralQuadruplet],
        depth: usize,
        nodes: &mut Vec<KdNode>,
    ) -> usize {
        let axis = (depth % 2) as u8;
        let len = points.len();
        let mid = len / 2;

        if axis == 0 {
            points.select_nth_unstable_by(mid, |a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            points.select_nth_unstable_by(mid, |a, b| a.f.partial_cmp(&b.f).unwrap_or(std::cmp::Ordering::Equal));
        }

        let median_point = points[mid];
        let log_f = median_point.f.max(1.0).log2();

        let node_idx = nodes.len();
        nodes.push(KdNode {
            point: median_point,
            log_f,
            axis,
            left: None,
            right: None,
            bbox_t_min: median_point.t,
            bbox_t_max: median_point.t,
            bbox_log_f_min: log_f,
            bbox_log_f_max: log_f,
        });

        let left_idx = if mid > 0 {
            Some(Self::build_recursive(&mut points[..mid], depth + 1, nodes))
        } else {
            None
        };

        let right_idx = if mid + 1 < len {
            Some(Self::build_recursive(&mut points[mid + 1..], depth + 1, nodes))
        } else {
            None
        };

        nodes[node_idx].left = left_idx;
        nodes[node_idx].right = right_idx;

        let mut t_min = nodes[node_idx].bbox_t_min;
        let mut t_max = nodes[node_idx].bbox_t_max;
        let mut f_min = nodes[node_idx].bbox_log_f_min;
        let mut f_max = nodes[node_idx].bbox_log_f_max;

        if let Some(l) = left_idx {
            t_min = t_min.min(nodes[l].bbox_t_min);
            t_max = t_max.max(nodes[l].bbox_t_max);
            f_min = f_min.min(nodes[l].bbox_log_f_min);
            f_max = f_max.max(nodes[l].bbox_log_f_max);
        }
        if let Some(r) = right_idx {
            t_min = t_min.min(nodes[r].bbox_t_min);
            t_max = t_max.max(nodes[r].bbox_t_max);
            f_min = f_min.min(nodes[r].bbox_log_f_min);
            f_max = f_max.max(nodes[r].bbox_log_f_max);
        }

        nodes[node_idx].bbox_t_min = t_min;
        nodes[node_idx].bbox_t_max = t_max;
        nodes[node_idx].bbox_log_f_min = f_min;
        nodes[node_idx].bbox_log_f_max = f_max;

        node_idx
    }

    pub fn query_range<F: FnMut(&SpectralQuadruplet)>(
        &self,
        t_min: f32,
        t_max: f32,
        log_f_min: f32,
        log_f_max: f32,
        mut visitor: F,
    ) {
        if let Some(root) = self.root {
            self.query_recursive(root, t_min, t_max, log_f_min, log_f_max, &mut visitor);
        }
    }

    fn query_recursive<F: FnMut(&SpectralQuadruplet)>(
        &self,
        node_idx: usize,
        t_min: f32,
        t_max: f32,
        log_f_min: f32,
        log_f_max: f32,
        visitor: &mut F,
    ) {
        let node = &self.nodes[node_idx];

        // Hierarchical Bounding Box Culling (Pruning Entire Subtrees)
        if node.bbox_t_max < t_min || node.bbox_t_min > t_max ||
           node.bbox_log_f_max < log_f_min || node.bbox_log_f_min > log_f_max {
            return;
        }

        // Test current point against query viewport
        if node.point.t >= t_min && node.point.t <= t_max &&
           node.log_f >= log_f_min && node.log_f <= log_f_max {
            visitor(&node.point);
        }

        let split_val = if node.axis == 0 { node.point.t } else { node.log_f };
        let query_min = if node.axis == 0 { t_min } else { log_f_min };
        let query_max = if node.axis == 0 { t_max } else { log_f_max };

        if query_min <= split_val {
            if let Some(left) = node.left {
                self.query_recursive(left, t_min, t_max, log_f_min, log_f_max, visitor);
            }
        }

        if query_max >= split_val {
            if let Some(right) = node.right {
                self.query_recursive(right, t_min, t_max, log_f_min, log_f_max, visitor);
            }
        }
    }
}

static CACHED_KD_TREE: std::sync::Mutex<Option<SpectralKdTree>> = std::sync::Mutex::new(None);

#[wasm_bindgen]
pub fn wasm_has_cached_quadruplets() -> bool {
    if let Ok(guard) = CACHED_KD_TREE.lock() {
        guard.as_ref().map_or(false, |tree| !tree.nodes.is_empty())
    } else {
        false
    }
}

#[wasm_bindgen]
pub fn wasm_cache_base_quadruplets(
    data: &[u8],
    h_custom: usize,
    window_type: &str,
    window_size: usize,
    zero_padding: usize,
    fmin_custom: f32,
    fmax_custom: f32,
    algorithm_type: &str,
    scale_type: &str,
    horizontal_res_k: usize,
) -> usize {
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 512; }
    
    let wav_info = parse_wav_samples(data);
    let fs = wav_info.sample_rate as f32;
    let num_samples = wav_info.samples.len();
    let win_len = if window_size > 0 { window_size } else { 1024 };
    let pad_factor = if zero_padding > 0 { zero_padding } else { 2 };
    let n_stft = win_len * pad_factor;
    
    let k_clamped = horizontal_res_k.min(4);
    let target_cols = (1024usize << k_clamped).max(512);
    let hop = (num_samples / target_cols).max(1);
    let w = (num_samples / hop).max(2);
    
    let mid_channel = wav_info.samples;

    let mut win_h = vec![0.0f32; win_len];
    let mut win_th = vec![0.0f32; win_len];
    let mut win_dh = vec![0.0f32; win_len];
    let half_win = (win_len as f32 - 1.0) * 0.5;
    for i in 0..win_len {
        let t = i as f32 - half_win;
        let val = match window_type {
            "hamming" => 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (win_len as f32 - 1.0)).cos(),
            "gaussian" => {
                let sigma = 0.4 * half_win;
                (-0.5 * (t / sigma).powi(2)).exp()
            },
            "blackman-harris" => {
                let a0 = 0.35875;
                let a1 = 0.48829;
                let a2 = 0.14128;
                let a3 = 0.01168;
                let z = 2.0 * std::f32::consts::PI * i as f32 / (win_len as f32 - 1.0);
                a0 - a1 * z.cos() + a2 * (2.0 * z).cos() - a3 * (3.0 * z).cos()
            },
            _ => 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (win_len as f32 - 1.0)).cos()),
        };
        win_h[i] = val;
        win_th[i] = t * val;
        win_dh[i] = if i == 0 {
            win_h[1] - win_h[0]
        } else if i == win_len - 1 {
            win_h[win_len - 1] - win_h[win_len - 2]
        } else {
            0.5 * (win_h[i + 1] - win_h[i - 1])
        };
    }

    let is_linear = scale_type == "linear";
    let fmin = if is_linear { fmin_custom.max(0.0) } else { fmin_custom.max(10.0) };
    let fmax = fmax_custom.clamp(fmin + 1.0, fs * 0.5);
    let step = if is_linear { (fmax - fmin) / (h as f32 - 1.0) } else { (fmax / fmin).log2() / (h as f32 - 1.0) };

    let mut fc_lut = vec![0.0f32; h];
    let mut k_f_lut = vec![0.0f32; h];
    for j in 0..h {
        let fc = if is_linear { fmin + j as f32 * step } else { fmin * 2.0f32.powf(j as f32 * step) };
        fc_lut[j] = fc;
        k_f_lut[j] = fc * n_stft as f32 / fs;
    }

    let mut planner = rustfft::FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    let mut scratch = vec![rustfft::num_complex::Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];
    let mut buffer_h_th = vec![rustfft::num_complex::Complex::<f32>::new(0.0, 0.0); n_stft];
    let mut buffer_dh = vec![rustfft::num_complex::Complex::<f32>::new(0.0, 0.0); n_stft];

    let mut quadruplets = Vec::with_capacity(w * h / 4);

    for c in 0..w {
        let start = c * hop;
        for i in 0..n_stft {
            buffer_h_th[i] = rustfft::num_complex::Complex::new(0.0, 0.0);
            buffer_dh[i] = rustfft::num_complex::Complex::new(0.0, 0.0);
        }
        for i in 0..win_len {
            let idx = start as isize + i as isize - (win_len as isize / 2);
            if idx >= 0 && idx < num_samples as isize {
                let sample_val = mid_channel[idx as usize];
                buffer_h_th[i] = rustfft::num_complex::Complex::new(sample_val * win_h[i], sample_val * win_th[i]);
                buffer_dh[i] = rustfft::num_complex::Complex::new(sample_val * win_dh[i], 0.0);
            }
        }
        fft.process_with_scratch(&mut buffer_h_th, &mut scratch);
        fft.process_with_scratch(&mut buffer_dh, &mut scratch);

        let t_base = c as f32 / (w as f32 - 1.0).max(1.0);

        for j in 0..h {
            let fc = fc_lut[j];
            let k_f = k_f_lut[j];
            let k_floor = (k_f.floor() as usize).clamp(1, n_stft / 2 - 2);
            let k_ceil = k_floor + 1;
            let delta_k = k_f - k_floor as f32;

            let y_floor = buffer_h_th[k_floor];
            let y_sym_floor = buffer_h_th[n_stft - k_floor];
            let s_h_floor = rustfft::num_complex::Complex::new(0.5 * (y_floor.re + y_sym_floor.re), 0.5 * (y_floor.im - y_sym_floor.im));
            let s_th_floor = rustfft::num_complex::Complex::new(0.5 * (y_floor.im + y_sym_floor.im), 0.5 * (y_sym_floor.re - y_floor.re));

            let y_ceil = buffer_h_th[k_ceil];
            let y_sym_ceil = buffer_h_th[n_stft - k_ceil];
            let s_h_ceil = rustfft::num_complex::Complex::new(0.5 * (y_ceil.re + y_sym_ceil.re), 0.5 * (y_ceil.im - y_sym_ceil.im));
            let s_th_ceil = rustfft::num_complex::Complex::new(0.5 * (y_ceil.im + y_sym_ceil.im), 0.5 * (y_sym_ceil.re - y_ceil.re));

            let s_h = s_h_floor * (1.0 - delta_k) + s_h_ceil * delta_k;
            let s_th = s_th_floor * (1.0 - delta_k) + s_th_ceil * delta_k;
            let s_dh = buffer_dh[k_floor] * (1.0 - delta_k) + buffer_dh[k_ceil] * delta_k;

            let mag_sq = s_h.re * s_h.re + s_h.im * s_h.im;
            if mag_sq > 1e-4 {
                let mag = mag_sq.sqrt();
                let norm_mag = mag / 4194304.0;
                if norm_mag > 1e-5 {
                    let log_a = norm_mag.log2();
                    let phi = s_h.im.atan2(s_h.re);

                    let (t_reass, f_reass) = if algorithm_type == "reassignment" {
                        let s_th_conj = s_th * s_h.conj();
                        let t_shift = s_th_conj.re / mag_sq;
                        let s_dh_conj = s_dh * s_h.conj();
                        let omega_shift = -s_dh_conj.im / mag_sq;
                        let f_r = fc + (omega_shift / (2.0 * std::f32::consts::PI));
                        let t_r = (start as f32 + t_shift) / (num_samples as f32 - 1.0).max(1.0);
                        (t_r.clamp(0.0, 1.0), f_r.clamp(1.0, 22050.0))
                    } else {
                        (t_base, fc)
                    };

                    quadruplets.push(SpectralQuadruplet {
                        t: t_reass,
                        f: f_reass,
                        log_a,
                        phi,
                    });
                }
            }
        }
    }

    let tree = SpectralKdTree::build(quadruplets);
    let count = tree.nodes.len();
    if let Ok(mut guard) = CACHED_KD_TREE.lock() {
        *guard = Some(tree);
    }
    count
}

#[wasm_bindgen]
pub fn wasm_render_from_cached_quadruplets(
    dst_w: usize,
    dst_h: usize,
    view_t_start: f32,
    view_t_end: f32,
    view_fmin: f32,
    view_fmax: f32,
    point_radius: f32,
    palette_type: &str,
) -> Vec<u8> {
    let mut rgba_buffer = vec![0u8; dst_w * dst_h * 4];
    if dst_w == 0 || dst_h == 0 {
        return rgba_buffer;
    }

    let tree_guard = match CACHED_KD_TREE.lock() {
        Ok(g) => g,
        Err(_) => return rgba_buffer,
    };
    let tree = match tree_guard.as_ref() {
        Some(t) => t,
        None => return rgba_buffer,
    };
    if tree.nodes.is_empty() {
        return rgba_buffer;
    }

    let log_view_fmin = view_fmin.max(1.0).log2();
    let log_view_fmax = view_fmax.max(view_fmin + 1.0).log2();
    let log_view_span = (log_view_fmax - log_view_fmin).max(1e-6);
    let view_t_span = (view_t_end - view_t_start).max(1e-6);

    let mut a_min_lut = vec![0.0f32; 256];
    let mut delta_a_lut = vec![0.0f32; 256];
    for y_idx in 1..=255 {
        let y_f = y_idx as f32;
        let amin_pow = -15.0 + 15.0 * (y_f * y_f - 1.0) / 65024.0;
        let a_min = 2.0f32.powf(amin_pow);
        let a_max = 2.0f32.powf(-15.0 + 15.0 * ((y_f + 1.0) * (y_f + 1.0) - 1.0) / 65024.0);
        a_min_lut[y_idx] = a_min;
        delta_a_lut[y_idx] = a_max - a_min;
    }
    a_min_lut[0] = 0.0;
    delta_a_lut[0] = a_min_lut[1];

    let snake_palette = if palette_type == "snake" {
        generate_snake_palette_lut()
    } else {
        Vec::new()
    };

    let mut grid_re = vec![0.0f32; dst_w * dst_h];
    let mut grid_im = vec![0.0f32; dst_w * dst_h];

    let w_f = (dst_w - 1) as f32;
    let h_f = (dst_h - 1) as f32;

    // Fast 2D KD-Tree Spatial Range Query with Subtree Bounding-Box Pruning
    tree.query_range(
        view_t_start,
        view_t_end,
        log_view_fmin,
        log_view_fmax,
        |q| {
            let x_norm = (q.t - view_t_start) / view_t_span;
            let log_f = q.f.max(1.0).log2();
            let y_norm = (log_f - log_view_fmin) / log_view_span;

            let x_screen = x_norm * w_f;
            let y_screen = y_norm * h_f;

            let xi = x_screen.round() as isize;
            let yi = y_screen.round() as isize;

            if xi >= 0 && xi < dst_w as isize && yi >= 0 && yi < dst_h as isize {
                let amp = 2.0f32.powf(q.log_a) * 4194304.0;
                let re = amp * q.phi.cos();
                let im = amp * q.phi.sin();
                let idx = yi as usize * dst_w + xi as usize;
                grid_re[idx] += re;
                grid_im[idx] += im;
            }
        }
    );

    // Colorize accumulated grid
    for idx in 0..(dst_w * dst_h) {
        let re = grid_re[idx];
        let im = grid_im[idx];
        let abs_z = (re * re + im * im).sqrt();
        let out_idx = idx * 4;

        if abs_z < 1e-12 {
            rgba_buffer[out_idx] = 0;
            rgba_buffer[out_idx + 1] = 0;
            rgba_buffer[out_idx + 2] = 0;
            rgba_buffer[out_idx + 3] = 255;
            continue;
        }

        let abs_z_norm = abs_z / 4194304.0;

        if palette_type == "snake" {
            let intensity = (abs_z / 4194304.0 * 65535.0).clamp(0.0, 65535.0) as usize;
            let (r, g, b) = snake_palette[intensity];
            rgba_buffer[out_idx] = r;
            rgba_buffer[out_idx + 1] = g;
            rgba_buffer[out_idx + 2] = b;
            rgba_buffer[out_idx + 3] = 255;
        } else {
            let log2_r = abs_z_norm.log2();
            let y_val = (1.0 + 65024.0 * (log2_r + 15.0) / 15.0).sqrt();
            let mut y_f = y_val.floor();
            if y_f < 1.0 { y_f = 1.0; }
            if y_f > 255.0 { y_f = 255.0; }

            let y_idx = y_f as usize;
            let a_min = a_min_lut[y_idx];
            let delta_a = delta_a_lut[y_idx];

            let r_resid = abs_z_norm - a_min;
            let r_norm = r_resid / if delta_a > 1e-15 { delta_a } else { 1e-15 };

            let re_norm = re / 4194304.0;
            let im_norm = im / 4194304.0;
            let w_re = r_norm * (re_norm / abs_z_norm);
            let w_im = r_norm * (im_norm / abs_z_norm);

            let cr = -w_re;
            let cb = w_im;

            let cb_byte = (cb * 112.0 + 128.0).clamp(16.0, 240.0);
            let cr_byte = (cr * 112.0 + 128.0).clamp(16.0, 240.0);

            let r_val = y_f + 1.402 * (cr_byte - 128.0);
            let g_val = y_f - 0.344136 * (cb_byte - 128.0) - 0.714136 * (cr_byte - 128.0);
            let b_val = y_f + 1.772 * (cb_byte - 128.0);

            rgba_buffer[out_idx] = r_val.clamp(0.0, 255.0).round() as u8;
            rgba_buffer[out_idx + 1] = g_val.clamp(0.0, 255.0).round() as u8;
            rgba_buffer[out_idx + 2] = b_val.clamp(0.0, 255.0).round() as u8;
            rgba_buffer[out_idx + 3] = 255;
        }
    }

    rgba_buffer
}

#[wasm_bindgen]
pub fn wasm_get_last_mirrored_density_histogram() -> Vec<f32> {
    if let Ok(guard) = LAST_MIRRORED_DENSITY.lock() {
        if !guard.is_empty() {
            return guard.clone();
        }
    }
    vec![0.0f32; 256]
}

#[inline]
pub fn hsv_to_rgb(hue_6: f32, sat: f32, val: f32) -> (u8, u8, u8) {
    let c = val * sat;
    let x = c * (1.0 - ((hue_6 % 2.0) - 1.0).abs());
    let m = val - c;
    let (r1, g1, b1) = if hue_6 < 1.0 {
        (c, x, 0.0)
    } else if hue_6 < 2.0 {
        (x, c, 0.0)
    } else if hue_6 < 3.0 {
        (0.0, c, x)
    } else if hue_6 < 4.0 {
        (0.0, x, c)
    } else if hue_6 < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

pub fn colorize_higher_order_point(
    derivs: &crate::analysis::higher_order::HigherOrderDerivatives,
    mode: usize,
    palette_type: &str,
    snake_palette: &[(u8, u8, u8)],
) -> (u8, u8, u8) {
    let mag = if derivs.peak_magnitude > 0.0 { derivs.peak_magnitude } else { derivs.magnitude };
    let norm_db = ((20.0 * (mag / 4194304.0).max(1e-6).log10() + 100.0) / 100.0).clamp(0.0, 1.0);
    
    if palette_type == "snake" && !snake_palette.is_empty() {
        let idx = (norm_db * 65535.0) as usize;
        let base_rgb = snake_palette[idx.min(snake_palette.len() - 1)];
        if derivs.is_ridge {
            return (
                base_rgb.0.saturating_add(60),
                base_rgb.1.saturating_add(60),
                base_rgb.2.saturating_add(60),
            );
        }
        return base_rgb;
    }

    match mode {
        0 => {
            // Mode 0: Cristas da Hessiana e Orientação Direcional theta
            let theta_norm = (derivs.ridge_angle_rad / std::f32::consts::PI + 0.5).clamp(0.0, 1.0);
            let hue = theta_norm * 6.0;
            let sat = (0.35 + 0.65 * derivs.anisotropy).clamp(0.0, 1.0);
            let val = norm_db;
            
            let (mut r, mut g, mut b) = hsv_to_rgb(hue, sat, val);
            if derivs.is_ridge {
                r = r.saturating_add(50);
                g = g.saturating_add(50);
                b = b.saturating_add(50);
            }
            (r, g, b)
        },
        1 => {
            // Mode 1: Anisotropia Espectral e Chirp Rate
            let chirp_norm = (derivs.d2_phi_dt2 * 0.0005).clamp(-1.0, 1.0);
            let hue = if chirp_norm >= 0.0 {
                3.0 + chirp_norm
            } else {
                0.5 + chirp_norm * 0.5
            };
            let sat = derivs.anisotropy.clamp(0.25, 1.0);
            let val = norm_db * (0.2 + 0.8 * derivs.anisotropy);
            hsv_to_rgb(hue.clamp(0.0, 6.0), sat, val)
        },
        2 => {
            // Mode 2: Curvatura Principal lambda_1 (Agudeza de Pico)
            let curv_norm = (-derivs.lambda_1 * 0.0002).clamp(0.0, 1.0);
            let hue = 4.5 - 2.5 * curv_norm;
            let sat = 0.85;
            let val = norm_db * curv_norm.sqrt();
            let (mut r, mut g, mut b) = hsv_to_rgb(hue, sat, val);
            if derivs.is_ridge {
                r = 255;
                g = g.saturating_add(100);
                b = 255;
            }
            (r, g, b)
        },
        _ => {
            // Mode 3: Reatribuição Vetorial Curva
            let hue = (derivs.ridge_angle_rad / std::f32::consts::PI + 0.5) * 6.0;
            let sat = 0.9;
            let val = norm_db;
            hsv_to_rgb(hue, sat, val)
        }
    }
}

#[wasm_bindgen]
pub struct WasmSpectrogramStreamer {
    w: usize,
    h: usize,
    current_col: usize,
    hop: usize,
    win_len: usize,
    n_stft: usize,
    sliced_samples: usize,
    mid_channel: Vec<f32>,
    win_h: Vec<f32>,
    win_th: Vec<f32>,
    win_dh: Vec<f32>,
    fc_lut: Vec<f32>,
    k_f_lut: Vec<f32>,
    a_min_lut: Vec<f32>,
    delta_a_lut: Vec<f32>,
    snake_palette: Vec<(u8, u8, u8)>,
    raw_hist_trans: [f32; 128],
    raw_hist_orig: [f32; 128],
    fs: f32,
    fmin: f32,
    fmax: f32,
    step: f32,
    point_radius: f32,
    algorithm_type: String,
    palette_type: String,
    is_linear: bool,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    scratch: Vec<rustfft::num_complex::Complex<f32>>,
    buffer_h_th: Vec<rustfft::num_complex::Complex<f32>>,
    buffer_dh: Vec<rustfft::num_complex::Complex<f32>>,
    reassigned_grid_re: Vec<f32>,
    reassigned_grid_im: Vec<f32>,
    is_higher_order: bool,
    higher_order_o: usize,
    higher_order_mode: usize,
    higher_order_windows: Vec<Vec<f32>>,
    higher_order_labels: Vec<(usize, usize)>,
    higher_order_buffers: Vec<Vec<rustfft::num_complex::Complex<f32>>>,
    higher_order_rgb_grid: Vec<u8>,
    is_sliding_jet: bool,
    sliding_jet_channels: Vec<crate::analysis::higher_order::SlidingJetDftChannel>,
}

#[wasm_bindgen]
impl WasmSpectrogramStreamer {
    #[wasm_bindgen(constructor)]
    pub fn new(
        data: &[u8],
        h_custom: usize,
        window_type: &str,
        window_size: usize,
        zero_padding: usize,
        fmin_custom: f32,
        fmax_custom: f32,
        algorithm_type: &str,
        palette_type: &str,
        t_start: f32,
        t_end: f32,
        point_radius: f32,
        scale_type: &str,
        horizontal_res_k: usize,
    ) -> WasmSpectrogramStreamer {
        let mut h = h_custom;
        if !h.is_power_of_two() || h < 4 { h = 512; }
        
        let wav_info = parse_wav_samples(data);
        let fs = wav_info.sample_rate as f32;
        let num_samples = wav_info.samples.len();
        let start_sample = ((t_start.clamp(0.0, 1.0) * num_samples as f32) as usize).min(num_samples.saturating_sub(2));
        let end_sample = ((t_end.clamp(0.0, 1.0) * num_samples as f32) as usize).clamp(start_sample + 2, num_samples);
        let sliced_samples = end_sample - start_sample;
        
        let win_len = if window_size > 0 { window_size } else { 1024 };
        let pad_factor = if zero_padding > 0 { zero_padding } else { 2 };
        let n_stft = win_len * pad_factor;
        
        let k_clamped = horizontal_res_k.min(4);
        let target_cols = (1024usize << k_clamped).max(512);
        let hop = (sliced_samples / target_cols).max(1);
        let w = (sliced_samples / hop).max(2);
        let grid_size = w * h;
        
        let mid_channel = if start_sample < num_samples && end_sample <= num_samples {
            wav_info.samples[start_sample..end_sample].to_vec()
        } else {
            wav_info.samples
        };

        let mut win_h = vec![0.0f32; win_len];
        let mut win_th = vec![0.0f32; win_len];
        let mut win_dh = vec![0.0f32; win_len];
        let half_win = (win_len as f32 - 1.0) * 0.5;
        for i in 0..win_len {
            let t = i as f32 - half_win;
            let val = match window_type {
                "hamming" => 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (win_len as f32 - 1.0)).cos(),
                "gaussian" => {
                    let sigma = 0.4 * half_win;
                    (-0.5 * (t / sigma).powi(2)).exp()
                },
                "blackman-harris" => {
                    let a0 = 0.35875;
                    let a1 = 0.48829;
                    let a2 = 0.14128;
                    let a3 = 0.01168;
                    let z = 2.0 * std::f32::consts::PI * i as f32 / (win_len as f32 - 1.0);
                    a0 - a1 * z.cos() + a2 * (2.0 * z).cos() - a3 * (3.0 * z).cos()
                },
                _ => 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (win_len as f32 - 1.0)).cos()),
            };
            win_h[i] = val;
            win_th[i] = t * val;
            win_dh[i] = if i == 0 {
                win_h[1] - win_h[0]
            } else if i == win_len - 1 {
                win_h[win_len - 1] - win_h[win_len - 2]
            } else {
                0.5 * (win_h[i + 1] - win_h[i - 1])
            };
        }

        let is_linear = scale_type == "linear";
        let fmin = if is_linear { fmin_custom.max(0.0) } else { fmin_custom.max(10.0) };
        let fmax = fmax_custom.clamp(fmin + 1.0, fs * 0.5);
        let step = if is_linear { (fmax - fmin) / (h as f32 - 1.0) } else { (fmax / fmin).log2() / (h as f32 - 1.0) };

        let mut fc_lut = vec![0.0f32; h];
        let mut k_f_lut = vec![0.0f32; h];
        for j in 0..h {
            let fc = if is_linear { fmin + j as f32 * step } else { fmin * 2.0f32.powf(j as f32 * step) };
            fc_lut[j] = fc;
            k_f_lut[j] = fc * n_stft as f32 / fs;
        }

        let mut a_min_lut = vec![0.0f32; 256];
        let mut delta_a_lut = vec![0.0f32; 256];
        for y_idx in 1..=255 {
            let y_f = y_idx as f32;
            let amin_pow = -15.0 + 15.0 * (y_f * y_f - 1.0) / 65024.0;
            let a_min = 2.0f32.powf(amin_pow);
            let a_max = 2.0f32.powf(-15.0 + 15.0 * ((y_f + 1.0) * (y_f + 1.0) - 1.0) / 65024.0);
            a_min_lut[y_idx] = a_min;
            delta_a_lut[y_idx] = a_max - a_min;
        }
        a_min_lut[0] = 0.0;
        delta_a_lut[0] = a_min_lut[1];

        let snake_palette = if palette_type == "snake" {
            generate_snake_palette_lut()
        } else {
            Vec::new()
        };

        let mut planner = rustfft::FftPlanner::new();
        let fft = planner.plan_fft_forward(n_stft);
        let scratch = vec![rustfft::num_complex::Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        let buffer_h_th = vec![rustfft::num_complex::Complex::<f32>::new(0.0, 0.0); n_stft];
        let buffer_dh = vec![rustfft::num_complex::Complex::<f32>::new(0.0, 0.0); n_stft];

        let is_sliding_jet = algorithm_type.starts_with("sliding_jet");
        let (is_higher_order, higher_order_o, higher_order_mode) = {
            if algorithm_type.starts_with("higher_order") || algorithm_type.starts_with("sliding_jet") {
                let parts: Vec<&str> = algorithm_type.split(':').collect();
                let o = if parts.len() > 1 { parts[1].parse::<usize>().unwrap_or(2).clamp(1, 4) } else { 2 };
                let m = if parts.len() > 2 {
                    match parts[2] {
                        "ridge" => 0,
                        "anisotropy" => 1,
                        "curvature" => 2,
                        "vector_reassign" => 3,
                        s => s.parse::<usize>().unwrap_or(0).min(3),
                    }
                } else {
                    0
                };
                (true, o, m)
            } else {
                (false, 1, 0)
            }
        };

        let mut higher_order_windows = Vec::new();
        let mut higher_order_labels = Vec::new();
        if is_higher_order && !is_sliding_jet {
            let sigma_ho = 0.4 * half_win / fs;
            for p in 0..=higher_order_o {
                for q in 0..=(higher_order_o - p) {
                    let win = crate::analysis::higher_order::generate_window_samples(p, q, win_len, fs, sigma_ho);
                    higher_order_windows.push(win);
                    higher_order_labels.push((p, q));
                }
            }
        }
        let num_ho_pairs = (higher_order_windows.len() + 1) / 2;
        let mut higher_order_buffers = Vec::with_capacity(num_ho_pairs);
        for _ in 0..num_ho_pairs {
            higher_order_buffers.push(vec![rustfft::num_complex::Complex::new(0.0, 0.0); n_stft]);
        }
        let higher_order_rgb_grid = if is_higher_order || is_sliding_jet {
            vec![0u8; grid_size * 4]
        } else {
            Vec::new()
        };

        let mut sliding_jet_channels = Vec::new();
        if is_sliding_jet {
            let sj_n = win_len.min(512);
            for j in 0..h {
                let fc = fc_lut[j];
                let theta_0 = 2.0 * std::f32::consts::PI * fc / fs;
                let mut ch = crate::analysis::higher_order::SlidingJetDftChannel::new(sj_n, theta_0, higher_order_o, 0.99999);
                if mid_channel.len() >= sj_n {
                    ch.init(&mid_channel[0..sj_n]);
                }
                sliding_jet_channels.push(ch);
            }
        }

        WasmSpectrogramStreamer {
            w,
            h,
            current_col: 0,
            hop,
            win_len,
            n_stft,
            sliced_samples,
            mid_channel,
            win_h,
            win_th,
            win_dh,
            fc_lut,
            k_f_lut,
            a_min_lut,
            delta_a_lut,
            snake_palette,
            raw_hist_trans: [0.0f32; 128],
            raw_hist_orig: [0.0f32; 128],
            fs,
            fmin,
            fmax,
            step,
            point_radius,
            algorithm_type: algorithm_type.to_string(),
            palette_type: palette_type.to_string(),
            is_linear,
            fft,
            scratch,
            buffer_h_th,
            buffer_dh,
            reassigned_grid_re: vec![0.0f32; grid_size],
            reassigned_grid_im: vec![0.0f32; grid_size],
            is_higher_order,
            higher_order_o,
            higher_order_mode,
            higher_order_windows,
            higher_order_labels,
            higher_order_buffers,
            higher_order_rgb_grid,
            is_sliding_jet,
            sliding_jet_channels,
        }
    }

    #[wasm_bindgen]
    pub fn get_width(&self) -> usize { self.w }

    #[wasm_bindgen]
    pub fn get_height(&self) -> usize { self.h }

    #[wasm_bindgen]
    pub fn get_current_col(&self) -> usize { self.current_col }

    #[wasm_bindgen]
    pub fn is_complete(&self) -> bool { self.current_col >= self.w }

    #[wasm_bindgen]
    pub fn get_progress_pct(&self) -> f32 {
        if self.w == 0 { 100.0 } else { (self.current_col as f32 / self.w as f32) * 100.0 }
    }

    #[wasm_bindgen]
    pub fn get_higher_order_o(&self) -> usize { self.higher_order_o }

    #[wasm_bindgen]
    pub fn get_higher_order_mode(&self) -> usize { self.higher_order_mode }

    #[wasm_bindgen]
    pub fn get_hop(&self) -> usize { self.hop }

    #[wasm_bindgen]
    pub fn get_sample_rate(&self) -> u32 { self.fs as u32 }

    #[wasm_bindgen]
    pub fn process_chunk(&mut self, chunk_cols: usize) -> Vec<u8> {
        let c_start = self.current_col;
        let c_end = (c_start + chunk_cols).min(self.w);
        let chunk_w = c_end - c_start;
        if chunk_w == 0 {
            return Vec::new();
        }

        if self.is_sliding_jet {
            let sj_n = self.win_len.min(512);
            let sigma_s = 0.4 * (sj_n as f32 * 0.5) / self.fs;

            let mut current_m = if c_start == 0 {
                if sj_n <= self.sliced_samples {
                    for j in 0..self.h {
                        self.sliding_jet_channels[j].init(&self.mid_channel[0..sj_n]);
                    }
                }
                0
            } else {
                c_start * self.hop
            };

            for c in c_start..c_end {
                let target_m = c * self.hop;
                let t_c = target_m as f32 / self.fs;

                if target_m > current_m && target_m + sj_n <= self.sliced_samples {
                    if target_m - current_m <= self.hop * 2 && target_m - current_m < sj_n {
                        for m in current_m..target_m {
                            let x_out = self.mid_channel[m];
                            let x_in = self.mid_channel[m + sj_n];
                            for j in 0..self.h {
                                self.sliding_jet_channels[j].update(x_out, x_in);
                            }
                        }
                    } else {
                        for j in 0..self.h {
                            self.sliding_jet_channels[j].init(&self.mid_channel[target_m..target_m + sj_n]);
                        }
                    }
                    current_m = target_m;
                }

                for j in 0..self.h {
                    let fc = self.fc_lut[j];
                    let ch = &self.sliding_jet_channels[j];
                    let derivs = ch.extract_derivatives(sigma_s, fc, t_c, self.fs);

                    let (r, g, b) = colorize_higher_order_point(
                        &derivs,
                        self.higher_order_mode,
                        &self.palette_type,
                        &self.snake_palette,
                    );

                    let grid_idx = j * self.w + c;
                    self.higher_order_rgb_grid[grid_idx * 4] = r;
                    self.higher_order_rgb_grid[grid_idx * 4 + 1] = g;
                    self.higher_order_rgb_grid[grid_idx * 4 + 2] = b;
                    self.higher_order_rgb_grid[grid_idx * 4 + 3] = 255;
                }
            }
        } else if self.is_higher_order {
            let num_pairs = (self.higher_order_windows.len() + 1) / 2;
            for c in c_start..c_end {
                let start = c * self.hop;
                for pair_idx in 0..num_pairs {
                    let win_a_idx = pair_idx * 2;
                    let win_b_idx = pair_idx * 2 + 1;
                    let buf = &mut self.higher_order_buffers[pair_idx];
                    for i in 0..self.n_stft {
                        buf[i] = rustfft::num_complex::Complex::new(0.0, 0.0);
                    }
                    for i in 0..self.win_len {
                        let idx = start as isize + i as isize - (self.win_len as isize / 2);
                        if idx >= 0 && idx < self.sliced_samples as isize {
                            let sample_val = self.mid_channel[idx as usize];
                            let val_a = sample_val * self.higher_order_windows[win_a_idx][i];
                            let val_b = if win_b_idx < self.higher_order_windows.len() {
                                sample_val * self.higher_order_windows[win_b_idx][i]
                            } else {
                                0.0
                            };
                            buf[i] = rustfft::num_complex::Complex::new(val_a, val_b);
                        }
                    }
                    self.fft.process_with_scratch(buf, &mut self.scratch);
                }

                for j in 0..self.h {
                    let fc = self.fc_lut[j];
                    let k_f = self.k_f_lut[j];
                    let k_floor = (k_f.floor() as usize).clamp(1, self.n_stft / 2 - 2);
                    let k_ceil = k_floor + 1;
                    let delta_k = k_f - k_floor as f32;

                    let mut s_vals = vec![crate::analysis::higher_order::Complex32::default(); self.higher_order_windows.len()];
                    for pair_idx in 0..num_pairs {
                        let win_a_idx = pair_idx * 2;
                        let win_b_idx = pair_idx * 2 + 1;
                        let buf = &self.higher_order_buffers[pair_idx];

                        let y_fl = buf[k_floor];
                        let y_sym_fl = buf[self.n_stft - k_floor];
                        let y_cl = buf[k_ceil];
                        let y_sym_cl = buf[self.n_stft - k_ceil];

                        let w_a_fl_re = 0.5 * (y_fl.re + y_sym_fl.re);
                        let w_a_fl_im = 0.5 * (y_fl.im - y_sym_fl.im);
                        let w_a_cl_re = 0.5 * (y_cl.re + y_sym_cl.re);
                        let w_a_cl_im = 0.5 * (y_cl.im - y_sym_cl.im);
                        let w_a = crate::analysis::higher_order::Complex32::new(
                            w_a_fl_re * (1.0 - delta_k) + w_a_cl_re * delta_k,
                            w_a_fl_im * (1.0 - delta_k) + w_a_cl_im * delta_k,
                        );

                        let (p_a, q_a) = self.higher_order_labels[win_a_idx];
                        let sign_p_a = if p_a % 2 == 1 { -1.0 } else { 1.0 };
                        let mod_q_a = q_a % 4;
                        let factor_q_a = match mod_q_a {
                            0 => crate::analysis::higher_order::Complex32::new(1.0, 0.0),
                            1 => crate::analysis::higher_order::Complex32::new(0.0, -1.0),
                            2 => crate::analysis::higher_order::Complex32::new(-1.0, 0.0),
                            _ => crate::analysis::higher_order::Complex32::new(0.0, 1.0),
                        };
                        s_vals[win_a_idx] = w_a.scale(sign_p_a).mul(factor_q_a);

                        if win_b_idx < self.higher_order_windows.len() {
                            let w_b_fl_re = 0.5 * (y_fl.im + y_sym_fl.im);
                            let w_b_fl_im = 0.5 * (y_sym_fl.re - y_fl.re);
                            let w_b_cl_re = 0.5 * (y_cl.im + y_sym_cl.im);
                            let w_b_cl_im = 0.5 * (y_cl.im - y_cl.re);
                            let w_b = crate::analysis::higher_order::Complex32::new(
                                w_b_fl_re * (1.0 - delta_k) + w_b_cl_re * delta_k,
                                w_b_fl_im * (1.0 - delta_k) + w_b_cl_im * delta_k,
                            );

                            let (p_b, q_b) = self.higher_order_labels[win_b_idx];
                            let sign_p_b = if p_b % 2 == 1 { -1.0 } else { 1.0 };
                            let mod_q_b = q_b % 4;
                            let factor_q_b = match mod_q_b {
                                0 => crate::analysis::higher_order::Complex32::new(1.0, 0.0),
                                1 => crate::analysis::higher_order::Complex32::new(0.0, -1.0),
                                2 => crate::analysis::higher_order::Complex32::new(-1.0, 0.0),
                                _ => crate::analysis::higher_order::Complex32::new(0.0, 1.0),
                            };
                            s_vals[win_b_idx] = w_b.scale(sign_p_b).mul(factor_q_b);
                        }
                    }

                    let get_s = |req_p: usize, req_q: usize| -> crate::analysis::higher_order::Complex32 {
                        for (idx, &(p, q)) in self.higher_order_labels.iter().enumerate() {
                            if p == req_p && q == req_q {
                                return s_vals[idx];
                            }
                        }
                        crate::analysis::higher_order::Complex32::default()
                    };

                    let t_c = (c * self.hop) as f32 / self.fs;
                    let derivs = crate::analysis::higher_order::compute_faa_di_bruno_derivatives(
                        self.higher_order_o,
                        get_s(0, 0),
                        get_s(1, 0),
                        get_s(0, 1),
                        get_s(2, 0),
                        get_s(1, 1),
                        get_s(0, 2),
                        get_s(3, 0),
                        get_s(2, 1),
                        get_s(1, 2),
                        get_s(0, 3),
                        get_s(4, 0),
                        get_s(0, 4),
                        fc,
                        t_c,
                    );

                    let (r, g, b) = colorize_higher_order_point(
                        &derivs,
                        self.higher_order_mode,
                        &self.palette_type,
                        &self.snake_palette,
                    );

                    let grid_idx = j * self.w + c;
                    self.higher_order_rgb_grid[grid_idx * 4] = r;
                    self.higher_order_rgb_grid[grid_idx * 4 + 1] = g;
                    self.higher_order_rgb_grid[grid_idx * 4 + 2] = b;
                    self.higher_order_rgb_grid[grid_idx * 4 + 3] = 255;

                    if self.higher_order_mode == 3 && derivs.magnitude > 1.0 {
                        let c_reass_f = c as f32 + (derivs.time_reassigned_s * self.fs / self.hop as f32) + derivs.delta_t_star;
                        let f_reass = fc + (derivs.d_phi_dt / (2.0 * std::f32::consts::PI));
                        let j_reass_f = if self.is_linear {
                            ((f_reass - self.fmin) / (self.fmax - self.fmin)) * (self.h as f32 - 1.0)
                        } else {
                            (f_reass / self.fmin).log2() / self.step
                        };
                        let ci = c_reass_f.round() as isize;
                        let ji = j_reass_f.round() as isize;
                        if ci >= 0 && ci < self.w as isize && ji >= 0 && ji < self.h as isize {
                            let target_idx = ji as usize * self.w + ci as usize;
                            self.reassigned_grid_re[target_idx] += s_vals[0].re;
                            self.reassigned_grid_im[target_idx] += s_vals[0].im;
                        }
                    }
                }
            }
        } else {
            for c in c_start..c_end {
                let start = c * self.hop;
                for i in 0..self.n_stft {
                    self.buffer_h_th[i] = rustfft::num_complex::Complex::new(0.0, 0.0);
                    self.buffer_dh[i] = rustfft::num_complex::Complex::new(0.0, 0.0);
                }
                for i in 0..self.win_len {
                    let idx = start as isize + i as isize - (self.win_len as isize / 2);
                    if idx >= 0 && idx < self.sliced_samples as isize {
                        let sample_val = self.mid_channel[idx as usize];
                        self.buffer_h_th[i] = rustfft::num_complex::Complex::new(sample_val * self.win_h[i], sample_val * self.win_th[i]);
                        self.buffer_dh[i] = rustfft::num_complex::Complex::new(sample_val * self.win_dh[i], 0.0);
                    }
                }
                self.fft.process_with_scratch(&mut self.buffer_h_th, &mut self.scratch);
                self.fft.process_with_scratch(&mut self.buffer_dh, &mut self.scratch);

                for j in 0..self.h {
                    let fc = self.fc_lut[j];
                    let k_f = self.k_f_lut[j];
                    let k_floor = (k_f.floor() as usize).clamp(1, self.n_stft / 2 - 2);
                    let k_ceil = k_floor + 1;
                    let delta_k = k_f - k_floor as f32;

                    let y_floor = self.buffer_h_th[k_floor];
                    let y_sym_floor = self.buffer_h_th[self.n_stft - k_floor];
                    let s_h_floor = rustfft::num_complex::Complex::new(0.5 * (y_floor.re + y_sym_floor.re), 0.5 * (y_floor.im - y_sym_floor.im));
                    let s_th_floor = rustfft::num_complex::Complex::new(0.5 * (y_floor.im + y_sym_floor.im), 0.5 * (y_sym_floor.re - y_floor.re));

                    let y_ceil = self.buffer_h_th[k_ceil];
                    let y_sym_ceil = self.buffer_h_th[self.n_stft - k_ceil];
                    let s_h_ceil = rustfft::num_complex::Complex::new(0.5 * (y_ceil.re + y_sym_ceil.re), 0.5 * (y_ceil.im - y_sym_ceil.im));
                    let s_th_ceil = rustfft::num_complex::Complex::new(0.5 * (y_ceil.im + y_sym_ceil.im), 0.5 * (y_sym_ceil.re - y_ceil.re));

                    let s_h = s_h_floor * (1.0 - delta_k) + s_h_ceil * delta_k;
                    let s_th = s_th_floor * (1.0 - delta_k) + s_th_ceil * delta_k;
                    let s_dh = self.buffer_dh[k_floor] * (1.0 - delta_k) + self.buffer_dh[k_ceil] * delta_k;

                    let mag_sq = s_h.re * s_h.re + s_h.im * s_h.im;
                    if mag_sq > 1e-12 {
                        let mag_orig = mag_sq.sqrt();
                        let r_orig = mag_orig / 4194304.0;
                        if r_orig > 1e-5 {
                            let db_orig = 20.0 * r_orig.log10();
                            let b_orig = (((db_orig + 100.0) / 100.0) * 127.0f32).clamp(0.0, 127.0) as usize;
                            self.raw_hist_orig[b_orig] += 1.0;
                        }
                    }

                    if mag_sq < 1.0 {
                        if mag_sq > 1e-4 {
                            let target_idx = j * self.w + c;
                            self.reassigned_grid_re[target_idx] += s_h.re;
                            self.reassigned_grid_im[target_idx] += s_h.im;
                        }
                        continue;
                    }

                    let target_idx = j * self.w + c;
                    if self.algorithm_type == "reassignment" {
                        let s_th_conj = s_th * s_h.conj();
                        let t_shift = s_th_conj.re / mag_sq;
                        let c_reassigned_f = c as f32 + t_shift / self.hop as f32;
                        let s_dh_conj = s_dh * s_h.conj();
                        let omega_shift = -s_dh_conj.im / mag_sq;
                        let f_reassigned = fc + (omega_shift / (2.0 * std::f32::consts::PI));

                        let j_reassigned_f = if self.is_linear {
                            ((f_reassigned - self.fmin) / (self.fmax - self.fmin)) * (self.h as f32 - 1.0)
                        } else {
                            (f_reassigned / self.fmin).log2() / self.step
                        };

                        let d_log_a_dt = s_dh_conj.re / mag_sq;
                        let d_log_a_dw = s_th_conj.im / mag_sq;
                        let curvature_factor = 1.0 + (d_log_a_dt * d_log_a_dt + d_log_a_dw * d_log_a_dw).sqrt();
                        let sig_x = (self.point_radius * 0.7071 / curvature_factor).clamp(0.05, 4.0);
                        let sig_y = (self.point_radius * 0.7071 / curvature_factor).clamp(0.05, 4.0);

                        let c_reassigned_i = c_reassigned_f.round() as isize;
                        let j_reassigned_i = j_reassigned_f.round() as isize;
                        let dx = c_reassigned_f - c_reassigned_i as f32;
                        let dy = j_reassigned_f - j_reassigned_i as f32;

                        let mut wx = [0.0f32; 3];
                        let mut wy = [0.0f32; 3];
                        let mut sum_wx = 0.0f32;
                        let mut sum_wy = 0.0f32;

                        for ox in -1isize..=1isize {
                            let dist_x = ox as f32 - dx;
                            let val_x = (-0.5 * (dist_x * dist_x) / (sig_x * sig_x)).exp();
                            wx[(ox + 1) as usize] = val_x;
                            sum_wx += val_x;
                        }
                        for oy in -1isize..=1isize {
                            let dist_y = oy as f32 - dy;
                            let val_y = (-0.5 * (dist_y * dist_y) / (sig_y * sig_y)).exp();
                            wy[(oy + 1) as usize] = val_y;
                            sum_wy += val_y;
                        }

                        if sum_wx > 1e-15 && sum_wy > 1e-15 {
                            let inv_sum = 1.0 / (sum_wx * sum_wy);
                            for ox in -1isize..=1isize {
                                let curr_c = c_reassigned_i + ox;
                                if curr_c >= 0 && curr_c < self.w as isize {
                                    for oy in -1isize..=1isize {
                                        let curr_j = j_reassigned_i + oy;
                                        if curr_j >= 0 && curr_j < self.h as isize {
                                            let weight = wx[(ox + 1) as usize] * wy[(oy + 1) as usize] * inv_sum;
                                            let t_idx = curr_j as usize * self.w + curr_c as usize;
                                            self.reassigned_grid_re[t_idx] += s_h.re * weight;
                                            self.reassigned_grid_im[t_idx] += s_h.im * weight;
                                        }
                                    }
                                }
                            }
                        } else {
                            self.reassigned_grid_re[target_idx] += s_h.re;
                            self.reassigned_grid_im[target_idx] += s_h.im;
                        }
                    } else {
                        self.reassigned_grid_re[target_idx] = s_h.re;
                        self.reassigned_grid_im[target_idx] = s_h.im;
                    }
                }
            }
        }

        // Colorize slice
        let mut rgba_buffer = vec![0u8; chunk_w * self.h * 4];
        for row in 0..self.h {
            for col in c_start..c_end {
                let grid_idx = row * self.w + col;
                let out_idx = (row * chunk_w + (col - c_start)) * 4;

                if self.is_higher_order && self.higher_order_mode != 3 {
                    rgba_buffer[out_idx] = self.higher_order_rgb_grid[grid_idx * 4];
                    rgba_buffer[out_idx + 1] = self.higher_order_rgb_grid[grid_idx * 4 + 1];
                    rgba_buffer[out_idx + 2] = self.higher_order_rgb_grid[grid_idx * 4 + 2];
                    rgba_buffer[out_idx + 3] = self.higher_order_rgb_grid[grid_idx * 4 + 3];
                    continue;
                }

                let re = self.reassigned_grid_re[grid_idx];
                let im = self.reassigned_grid_im[grid_idx];
                let abs_z = (re * re + im * im).sqrt();

                if abs_z < 1e-12 {
                    rgba_buffer[out_idx] = 0;
                    rgba_buffer[out_idx + 1] = 0;
                    rgba_buffer[out_idx + 2] = 0;
                    rgba_buffer[out_idx + 3] = 255;
                    continue;
                }

                let abs_z_norm = abs_z / 4194304.0;
                if abs_z_norm > 1e-5 {
                    let db_trans = 20.0 * abs_z_norm.log10();
                    let b_trans = (((db_trans + 100.0) / 100.0) * 127.0f32).clamp(0.0, 127.0) as usize;
                    self.raw_hist_trans[b_trans] += 1.0;
                }

                if self.palette_type == "snake" {
                    let intensity = (abs_z / 4194304.0 * 65535.0).clamp(0.0, 65535.0) as usize;
                    let (r, g, b) = self.snake_palette[intensity];
                    rgba_buffer[out_idx] = r;
                    rgba_buffer[out_idx + 1] = g;
                    rgba_buffer[out_idx + 2] = b;
                    rgba_buffer[out_idx + 3] = 255;
                } else {
                    let log2_r = abs_z_norm.log2();
                    let y_val = (1.0 + 65024.0 * (log2_r + 15.0) / 15.0).sqrt();
                    let mut y_f = y_val.floor();
                    if y_f < 1.0 { y_f = 1.0; }
                    if y_f > 255.0 { y_f = 255.0; }

                    let y_idx = y_f as usize;
                    let a_min = self.a_min_lut[y_idx];
                    let delta_a = self.delta_a_lut[y_idx];

                    let r_resid = abs_z_norm - a_min;
                    let r_norm = r_resid / if delta_a > 1e-15 { delta_a } else { 1e-15 };

                    let re_norm = re / 4194304.0;
                    let im_norm = im / 4194304.0;
                    let w_re = r_norm * (re_norm / abs_z_norm);
                    let w_im = r_norm * (im_norm / abs_z_norm);

                    let cr = -w_re;
                    let cb = w_im;

                    let cb_byte = (cb * 112.0 + 128.0).clamp(16.0, 240.0);
                    let cr_byte = (cr * 112.0 + 128.0).clamp(16.0, 240.0);

                    let r_val = y_f + 1.402 * (cr_byte - 128.0);
                    let g_val = y_f - 0.344136 * (cb_byte - 128.0) - 0.714136 * (cr_byte - 128.0);
                    let b_val = y_f + 1.772 * (cb_byte - 128.0);

                    rgba_buffer[out_idx] = r_val.clamp(0.0, 255.0).round() as u8;
                    rgba_buffer[out_idx + 1] = g_val.clamp(0.0, 255.0).round() as u8;
                    rgba_buffer[out_idx + 2] = b_val.clamp(0.0, 255.0).round() as u8;
                    rgba_buffer[out_idx + 3] = 255;
                }
            }
        }

        self.current_col = c_end;

        if self.current_col >= self.w {
            let mut kernel = [0.0f32; 11];
            let mut k_sum = 0.0f32;
            for i in 0..11 {
                let x = (i as f32) - 5.0;
                let v = (-0.5 * (x * x) / (2.5 * 2.5)).exp();
                kernel[i] = v;
                k_sum += v;
            }
            for i in 0..11 {
                kernel[i] /= k_sum;
            }

            let mut density_trans = vec![0.0f32; 128];
            let mut density_orig = vec![0.0f32; 128];

            for i in 0..128 {
                let mut sum_trans = 0.0f32;
                let mut sum_orig = 0.0f32;
                for k in 0..11 {
                    let idx = (i as isize + k as isize - 5).clamp(0, 127) as usize;
                    sum_trans += self.raw_hist_trans[idx] * kernel[k];
                    sum_orig += self.raw_hist_orig[idx] * kernel[k];
                }
                density_trans[i] = sum_trans;
                density_orig[i] = sum_orig;
            }

            let mut max_val = 1e-12f32;
            for i in 0..128 {
                if density_trans[i] > max_val { max_val = density_trans[i]; }
                if density_orig[i] > max_val { max_val = density_orig[i]; }
            }

            for i in 0..128 {
                density_trans[i] /= max_val;
                density_orig[i] /= max_val;
            }

            let mut combined_density = Vec::with_capacity(256);
            combined_density.extend_from_slice(&density_trans);
            combined_density.extend_from_slice(&density_orig);

            if let Ok(mut guard) = LAST_MIRRORED_DENSITY.lock() {
                *guard = combined_density;
            }
        }

        rgba_buffer
    }
}

#[inline(always)]
fn cubic_hermite(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c = -0.5 * p0 + 0.5 * p2;
    let d = p1;
    ((a * t + b) * t + c) * t + d
}

#[wasm_bindgen]
pub fn wasm_bicubic_resample_spectrogram(
    src_grid: &[u8],
    src_w: usize,
    src_h: usize,
    src_t_start: f32,
    src_t_end: f32,
    src_fmin: f32,
    src_fmax: f32,
    dst_w: usize,
    dst_h: usize,
    dst_t_start: f32,
    dst_t_end: f32,
    dst_fmin: f32,
    dst_fmax: f32,
) -> Vec<u8> {
    let mut dst_grid = vec![0u8; dst_w * dst_h * 4];
    if src_w < 2 || src_h < 2 || dst_w == 0 || dst_h == 0 || src_grid.len() < src_w * src_h * 4 {
        return dst_grid;
    }

    let log_src_fmin = src_fmin.max(1.0).log2();
    let log_src_fmax = src_fmax.max(src_fmin + 1.0).log2();
    let log_src_span = (log_src_fmax - log_src_fmin).max(1e-6);

    let log_dst_fmin = dst_fmin.max(1.0).log2();
    let log_dst_fmax = dst_fmax.max(dst_fmin + 1.0).log2();
    let log_dst_span = (log_dst_fmax - log_dst_fmin).max(1e-6);

    let src_t_span = (src_t_end - src_t_start).max(1e-6);
    let dst_t_span = (dst_t_end - dst_t_start).max(1e-6);

    // Precompute horizontal x_src and weights for all destination columns
    let mut x_int_lut = Vec::with_capacity(dst_w);
    let mut x_frac_lut = Vec::with_capacity(dst_w);

    for c in 0..dst_w {
        let t_ratio = if dst_w > 1 { c as f32 / (dst_w - 1) as f32 } else { 0.0 };
        let t = dst_t_start + t_ratio * dst_t_span;
        let norm_x = (t - src_t_start) / src_t_span;
        let x_f = norm_x * (src_w - 1) as f32;
        let x_clamped = x_f.clamp(0.0, (src_w - 1) as f32);
        let xi = x_clamped.floor() as isize;
        let xf = x_clamped - xi as f32;
        x_int_lut.push(xi);
        x_frac_lut.push(xf);
    }

    for r in 0..dst_h {
        let freq_ratio = if dst_h > 1 { r as f32 / (dst_h - 1) as f32 } else { 0.0 };
        let log_f = log_dst_fmin + freq_ratio * log_dst_span;
        let norm_y = (log_f - log_src_fmin) / log_src_span;
        let y_f = norm_y * (src_h - 1) as f32;
        let y_clamped = y_f.clamp(0.0, (src_h - 1) as f32);
        let yi = y_clamped.floor() as isize;
        let yf = y_clamped - yi as f32;

        let y_m1 = (yi - 1).clamp(0, (src_h - 1) as isize) as usize;
        let y_0 = yi.clamp(0, (src_h - 1) as isize) as usize;
        let y_1 = (yi + 1).clamp(0, (src_h - 1) as isize) as usize;
        let y_2 = (yi + 2).clamp(0, (src_h - 1) as isize) as usize;

        let row_m1_offset = y_m1 * src_w * 4;
        let row_0_offset = y_0 * src_w * 4;
        let row_1_offset = y_1 * src_w * 4;
        let row_2_offset = y_2 * src_w * 4;

        let dst_row_offset = r * dst_w * 4;

        for c in 0..dst_w {
            let xi = x_int_lut[c];
            let u = x_frac_lut[c];

            let x_m1 = (xi - 1).clamp(0, (src_w - 1) as isize) as usize * 4;
            let x_0 = xi.clamp(0, (src_w - 1) as isize) as usize * 4;
            let x_1 = (xi + 1).clamp(0, (src_w - 1) as isize) as usize * 4;
            let x_2 = (xi + 2).clamp(0, (src_w - 1) as isize) as usize * 4;

            let dst_idx = dst_row_offset + c * 4;

            // Bicubic interpolation for R, G, B channels
            for ch in 0..3 {
                let p00 = src_grid[row_m1_offset + x_m1 + ch] as f32;
                let p01 = src_grid[row_m1_offset + x_0 + ch] as f32;
                let p02 = src_grid[row_m1_offset + x_1 + ch] as f32;
                let p03 = src_grid[row_m1_offset + x_2 + ch] as f32;
                let row0 = cubic_hermite(p00, p01, p02, p03, u);

                let p10 = src_grid[row_0_offset + x_m1 + ch] as f32;
                let p11 = src_grid[row_0_offset + x_0 + ch] as f32;
                let p12 = src_grid[row_0_offset + x_1 + ch] as f32;
                let p13 = src_grid[row_0_offset + x_2 + ch] as f32;
                let row1 = cubic_hermite(p10, p11, p12, p13, u);

                let p20 = src_grid[row_1_offset + x_m1 + ch] as f32;
                let p21 = src_grid[row_1_offset + x_0 + ch] as f32;
                let p22 = src_grid[row_1_offset + x_1 + ch] as f32;
                let p23 = src_grid[row_1_offset + x_2 + ch] as f32;
                let row2 = cubic_hermite(p20, p21, p22, p23, u);

                let p30 = src_grid[row_2_offset + x_m1 + ch] as f32;
                let p31 = src_grid[row_2_offset + x_0 + ch] as f32;
                let p32 = src_grid[row_2_offset + x_1 + ch] as f32;
                let p33 = src_grid[row_2_offset + x_2 + ch] as f32;
                let row3 = cubic_hermite(p30, p31, p32, p33, u);

                let final_val = cubic_hermite(row0, row1, row2, row3, yf);
                dst_grid[dst_idx + ch] = final_val.clamp(0.0, 255.0).round() as u8;
            }
            dst_grid[dst_idx + 3] = 255;
        }
    }

    dst_grid
}

#[wasm_bindgen]
pub fn wasm_get_spectrogram_dimensions(
    data: &[u8],
    h_custom: usize,
    horizontal_res_k: usize,
    quality_lod: usize,
    t_start: f32,
    t_end: f32,
) -> Vec<usize> {
    let is_draft = quality_lod >= 1;
    let mut h = if is_draft { 256 } else { h_custom };
    if !h.is_power_of_two() || h < 4 { h = 512; }
    
    let wav_info = parse_wav_samples(data);
    let num_samples = wav_info.samples.len();
    let start_sample = ((t_start.clamp(0.0, 1.0) * num_samples as f32) as usize).min(num_samples.saturating_sub(2));
    let end_sample = ((t_end.clamp(0.0, 1.0) * num_samples as f32) as usize).clamp(start_sample + 2, num_samples);
    let sliced_samples = end_sample - start_sample;
    
    let target_cols = if is_draft {
        512usize
    } else {
        let k_clamped = horizontal_res_k.min(4);
        (1024usize << k_clamped).max(512)
    };
    let hop = (sliced_samples / target_cols).max(1);
    let w = (sliced_samples / hop).max(2);
    
    vec![w, h, hop, wav_info.sample_rate as usize, num_samples]
}

#[wasm_bindgen]
pub fn wasm_generate_complex_reassigned_ycbcr_spectrogram(
    data: &[u8], 
    h_custom: usize, 
    window_type: &str,
    window_size: usize,
    zero_padding: usize,
    fmin_custom: f32,
    fmax_custom: f32,
    algorithm_type: &str,
    palette_type: &str,
    t_start: f32,
    t_end: f32,
    point_radius: f32,
    scale_type: &str,
    horizontal_res_k: usize,
    quality_lod: usize,
    chunk_start: usize,
    chunk_count: usize,
) -> Vec<u8> {
    let is_draft = quality_lod >= 1;
    
    // Adaptive LOD:
    // If draft mode (user actively sliding/zooming): use lightweight 256 rows and 512 cols for instant 3ms rendering!
    // If refined mode (user stopped): use full selected height, zero_padding, and 2^k resolution!
    let mut h = if is_draft { 256 } else { h_custom };
    if !h.is_power_of_two() || h < 4 { h = 512; }
    
    let wav_info = parse_wav_samples(data);
    let fs = wav_info.sample_rate as f32;
    let num_samples = wav_info.samples.len();
    
    // Compute visible sample bounds dynamically based on normalized start/end times
    let start_sample = ((t_start.clamp(0.0, 1.0) * num_samples as f32) as usize).min(num_samples.saturating_sub(2));
    let end_sample = ((t_end.clamp(0.0, 1.0) * num_samples as f32) as usize).clamp(start_sample + 2, num_samples);
    let sliced_samples = end_sample - start_sample;
    
    // Dynamic STFT parameters
    let win_len = if is_draft { 512 } else if window_size > 0 { window_size } else { 1024 };
    let pad_factor = if is_draft { 1 } else if zero_padding > 0 { zero_padding } else { 2 };
    let n_stft = win_len * pad_factor;
    
    // Precalculate target columns: 512 cols for draft, up to 1024*2^k for refined
    let target_cols = if is_draft {
        512usize
    } else {
        let k_clamped = horizontal_res_k.min(4);
        (1024usize << k_clamped).max(512)
    };
    let hop = (sliced_samples / target_cols).max(1);
    let w = (sliced_samples / hop).max(2);
    let grid_size = w * h;

    if algorithm_type.starts_with("higher_order") || algorithm_type.starts_with("sliding_jet") {
        let mut streamer = WasmSpectrogramStreamer::new(
            data,
            h,
            window_type,
            win_len,
            pad_factor,
            fmin_custom,
            fmax_custom,
            algorithm_type,
            palette_type,
            t_start,
            t_end,
            point_radius,
            scale_type,
            horizontal_res_k,
        );
        let count = if chunk_count == 0 { streamer.get_width() } else { chunk_count };
        return streamer.process_chunk(count);
    }

    let (c_start, c_end) = if chunk_count > 0 {
        let cs = chunk_start.min(w - 1);
        let ce = (cs + chunk_count).min(w);
        (cs, ce)
    } else {
        (0, w)
    };
    
    let mid_channel = if start_sample < num_samples && end_sample <= num_samples {
        wav_info.samples[start_sample..end_sample].to_vec()
    } else {
        wav_info.samples
    };
    
    let fs_f32 = fs;
    
    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    
    let mut win_h = vec![0.0f32; n_stft];
    let mut win_th = vec![0.0f32; n_stft];
    let mut win_dh = vec![0.0f32; n_stft]; // Dynamic temporal derivative window of Gabor
    
    let half_win = (win_len - 1) as f32 / 2.0;
    
    // Generate window shapes on the fly based on win_len and zero pad to n_stft
    match window_type {
        "hamming" => {
            for i in 0..win_len {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (win_len - 1) as f32;
                win_h[i] = 0.54 - 0.46 * angle.cos();
                win_th[i] = (i as f32 - half_win) * win_h[i];
                win_dh[i] = (0.46 * 2.0 * std::f32::consts::PI / (win_len - 1) as f32) * angle.sin();
            }
        },
        "gaussian" => {
            let sigma = (win_len - 1) as f32 / 6.0;
            for i in 0..win_len {
                let diff = i as f32 - half_win;
                win_h[i] = (-0.5 * (diff / sigma).powi(2)).exp();
                win_th[i] = diff * win_h[i];
                win_dh[i] = -(diff / sigma.powi(2)) * win_h[i];
            }
        },
        "blackman-harris" => {
            let a0 = 0.35875f32;
            let a1 = 0.48829f32;
            let a2 = 0.14128f32;
            let a3 = 0.01168f32;
            for i in 0..win_len {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (win_len - 1) as f32;
                win_h[i] = a0 - a1 * angle.cos() + a2 * (2.0 * angle).cos() - a3 * (3.0 * angle).cos();
                win_th[i] = (i as f32 - half_win) * win_h[i];
                win_dh[i] = (2.0 * std::f32::consts::PI / (win_len - 1) as f32) * 
                           (a1 * angle.sin() - 2.0 * a2 * (2.0 * angle).sin() + 3.0 * a3 * (3.0 * angle).sin());
            }
        },
        _ => { // Hann window
            for i in 0..win_len {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (win_len - 1) as f32;
                win_h[i] = 0.5 * (1.0 - angle.cos());
                win_th[i] = (i as f32 - half_win) * win_h[i];
                win_dh[i] = (std::f32::consts::PI / (win_len - 1) as f32) * angle.sin();
            }
        }
    }
    
    let fmin = if fmin_custom >= 5.0 { fmin_custom } else { 20.0f32 };
    let fmax = if fmax_custom > fmin { fmax_custom.min(fs_f32 / 2.0) } else { fs_f32 / 2.0 };
    let step = (fmax / fmin).log2() / (h as f32 - 1.0);
    
    // =========================================================================
    // RADICAL PERFORMANCE OPTIMIZATION: HOISTING & LUTS
    // =========================================================================
    // 1. Precompute expensive exponential frequencies and float coordinates
    let mut fc_lut = vec![0.0f32; h];
    let mut k_f_lut = vec![0.0f32; h];
    
    // 1.5. Precompute Sparse Spectral CQT Kernels to eliminate log2() and exp() inside the loop!
    let sigma_y = 0.95 * step;
    let sigma_y_sq = sigma_y * sigma_y;
    let bandwidth_octaves = 3.0 * sigma_y;
    
    let mut cqt_k_low_lut = vec![0usize; h];
    let mut cqt_k_high_lut = vec![0usize; h];
    // Array of (g_val, gy_val) for each sparse bin
    let mut cqt_kernels_lut: Vec<Vec<(f32, f32)>> = vec![Vec::new(); h];
    
    let is_linear = scale_type == "linear";
    let step_lin = (fmax - fmin) / (h as f32 - 1.0);
    
    for j in 0..h {
        let fc = if is_linear {
            fmin + j as f32 * step_lin
        } else {
            fmin * 2.0f32.powf(j as f32 * step)
        };
        fc_lut[j] = fc;
        k_f_lut[j] = fc * n_stft as f32 / fs_f32;
        
        // Compute sparse bounds
        let f_low = if is_linear {
            (fc - (fc * (2.0f32.powf(step) - 1.0) * 1.5)).max(fmin)
        } else {
            fc * 2.0f32.powf(-bandwidth_octaves)
        };
        let f_high = if is_linear {
            (fc + (fc * (2.0f32.powf(step) - 1.0) * 1.5)).min(fs_f32 / 2.0)
        } else {
            fc * 2.0f32.powf(bandwidth_octaves)
        };
        
        let k_low = (f_low * n_stft as f32 / fs_f32).round() as isize;
        let k_high = (f_high * n_stft as f32 / fs_f32).round() as isize;
        let k_low_u = k_low.clamp(1, (n_stft / 2 - 2) as isize) as usize;
        let k_high_u = k_high.clamp(k_low_u as isize + 1, (n_stft / 2 - 1) as isize) as usize;
        
        cqt_k_low_lut[j] = k_low_u;
        cqt_k_high_lut[j] = k_high_u;
        
        let y_j = if is_linear {
            (fc / fmin).log2()
        } else {
            j as f32 * step
        };
        let mut kernel = Vec::with_capacity(k_high_u - k_low_u + 1);
        
        for curr_k in k_low_u..=k_high_u {
            let f_k = curr_k as f32 * fs_f32 / n_stft as f32;
            if f_k >= fmin {
                let y_f = (f_k / fmin).log2();
                let dy_val = y_f - y_j;
                let g_val = (-0.5 * (dy_val * dy_val) / sigma_y_sq).exp();
                let gy_val = -(dy_val / sigma_y_sq) * g_val;
                kernel.push((g_val, gy_val));
            } else {
                kernel.push((0.0, 0.0));
            }
        }
        cqt_kernels_lut[j] = kernel;
    }
    
    // 2. Single heap allocation for FFT buffers (Zero-allocation inside hot loop)
    let mut buffer_h_th = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
    let mut buffer_dh = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
    let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];
    
    // 3. Precompute YCbCr Palette Logarithmic Bounds LUTs (0-255 bounds)
    let mut a_min_lut = vec![0.0f32; 256];
    let mut delta_a_lut = vec![0.0f32; 256];
    for y_idx in 1..=255 {
        let y_f = y_idx as f32;
        let amin_pow = -15.0 + 15.0 * (y_f * y_f - 1.0) / 65024.0;
        let a_min = 2.0f32.powf(amin_pow);
        let a_max = 2.0f32.powf(-15.0 + 15.0 * ((y_f + 1.0) * (y_f + 1.0) - 1.0) / 65024.0);
        a_min_lut[y_idx] = a_min;
        delta_a_lut[y_idx] = a_max - a_min;
    }
    a_min_lut[0] = 0.0;
    delta_a_lut[0] = a_min_lut[1]; // fallback

    let mut reassigned_grid_re = vec![0.0f32; grid_size];
    let mut reassigned_grid_im = vec![0.0f32; grid_size];
    let mut raw_hist_orig = [0.0f32; 128];
    let mut raw_hist_trans = [0.0f32; 128];
    
    for c in c_start..c_end {
        let start = c * hop;
        
        // Zero-fill the reused buffers safely
        for i in 0..n_stft {
            buffer_h_th[i] = Complex::new(0.0, 0.0);
            if !is_draft {
                buffer_dh[i] = Complex::new(0.0, 0.0);
            }
        }
        
        for i in 0..win_len {
            let idx = start as isize + i as isize - (win_len as isize / 2);
            if idx >= 0 && idx < sliced_samples as isize {
                let sample_val = mid_channel[idx as usize];
                if is_draft {
                    // Draft mode: 1 single real FFT per column (<1.5ms total!)
                    buffer_h_th[i] = Complex::new(sample_val * win_h[i], 0.0);
                } else {
                    // Refined mode: 2-in-1 Real FFTs (packs h and th into single complex FFT!)
                    buffer_h_th[i] = Complex::new(sample_val * win_h[i], sample_val * win_th[i]);
                    buffer_dh[i] = Complex::new(sample_val * win_dh[i], 0.0);
                }
            }
        }
        
        // Use process_with_scratch to completely eliminate RustFFT's internal dynamic heap allocations
        fft.process_with_scratch(&mut buffer_h_th, &mut scratch);
        if !is_draft {
            fft.process_with_scratch(&mut buffer_dh, &mut scratch);
        }
        
        for j in 0..h {
            let fc = fc_lut[j];
            let k_f = k_f_lut[j];
            let k_floor = (k_f.floor() as usize).clamp(1, n_stft / 2 - 2);
            let k_ceil = k_floor + 1;
            let delta_k = k_f - k_floor as f32;
            
            if is_draft {
                // Draft Preview: Direct linear sample with ZERO phase derivations or splines
                let s_h = buffer_h_th[k_floor] * (1.0 - delta_k) + buffer_h_th[k_ceil] * delta_k;
                let target_idx = j * w + c;
                reassigned_grid_re[target_idx] += s_h.re;
                reassigned_grid_im[target_idx] += s_h.im;
                continue;
            }

            // Refined mode: Unpack two real FFTs from single complex FFT in O(1) arithmetic!
            let y_floor = buffer_h_th[k_floor];
            let y_sym_floor = buffer_h_th[n_stft - k_floor];
            let s_h_floor = Complex::new(0.5 * (y_floor.re + y_sym_floor.re), 0.5 * (y_floor.im - y_sym_floor.im));
            let s_th_floor = Complex::new(0.5 * (y_floor.im + y_sym_floor.im), 0.5 * (y_sym_floor.re - y_floor.re));
            
            let y_ceil = buffer_h_th[k_ceil];
            let y_sym_ceil = buffer_h_th[n_stft - k_ceil];
            let s_h_ceil = Complex::new(0.5 * (y_ceil.re + y_sym_ceil.re), 0.5 * (y_ceil.im - y_sym_ceil.im));
            let s_th_ceil = Complex::new(0.5 * (y_ceil.im + y_sym_ceil.im), 0.5 * (y_sym_ceil.re - y_ceil.re));
            
            let s_h = s_h_floor * (1.0 - delta_k) + s_h_ceil * delta_k;
            let s_th = s_th_floor * (1.0 - delta_k) + s_th_ceil * delta_k;
            let s_dh = buffer_dh[k_floor] * (1.0 - delta_k) + buffer_dh[k_ceil] * delta_k;
            
            let mag_sq = s_h.re * s_h.re + s_h.im * s_h.im;
            if mag_sq > 1e-12 {
                let mag_orig = mag_sq.sqrt();
                let r_orig = mag_orig / 4194304.0;
                if r_orig > 1e-5 {
                    let db_orig = 20.0 * r_orig.log10();
                    let b_orig = (((db_orig + 100.0) / 100.0) * 127.0).clamp(0.0, 127.0) as usize;
                    raw_hist_orig[b_orig] += 1.0;
                }
            }
            
            // Ultra-Fast Noise-Floor Bypass: Quiet bins below -60dB don't need heavy 9-point Gaussian splines
            if mag_sq < 1.0 {
                if mag_sq > 1e-4 {
                    let target_idx = j * w + c;
                    reassigned_grid_re[target_idx] += s_h.re;
                    reassigned_grid_im[target_idx] += s_h.im;
                }
                continue;
            }

            let target_idx = j * w + c;
            
            if algorithm_type == "reassignment" {
                // Time shift calculation: shift_t = Re{X_th / X_h}
                let s_th_conj = s_th * s_h.conj();
                let t_shift = s_th_conj.re / mag_sq;
                let c_reassigned_f = c as f32 + t_shift / hop as f32;
                
                // Frequency shift calculation: shift_w = Im{X_dh / X_h}
                let s_dh_conj = s_dh * s_h.conj();
                let omega_shift = s_dh_conj.im / mag_sq; // shift in rad/sample
                let f_reassigned = fc - (omega_shift * fs_f32 / (2.0 * std::f32::consts::PI));
                
                let j_reassigned_f = if is_linear {
                    ((f_reassigned - fmin) / (fmax - fmin)) * (h as f32 - 1.0)
                } else {
                    (f_reassigned / fmin).log2() / step
                };
                
                // Resolve nearest integer coordinates
                let c_reassigned_i = c_reassigned_f.round() as isize;
                let j_reassigned_i = j_reassigned_f.round() as isize;
                
                // Calculate fractional sub-sample offsets
                let dx = c_reassigned_f - c_reassigned_i as f32;
                let dy = j_reassigned_f - j_reassigned_i as f32;
                
                // Sub-Pixel Center Shortcut: if point is within 0.15 px of grid center, deposit directly with 0 exponentials!
                if (dx * dx + dy * dy) < 0.0225 {
                    if c_reassigned_i >= 0 && c_reassigned_i < w as isize && j_reassigned_i >= 0 && j_reassigned_i < h as isize {
                        let target_idx_center = j_reassigned_i as usize * w + c_reassigned_i as usize;
                        reassigned_grid_re[target_idx_center] += s_h.re;
                        reassigned_grid_im[target_idx_center] += s_h.im;
                    }
                    continue;
                }

                // 1st order derivative of log amplitude with respect to time (Re{X_dh / X_h})
                let d_log_A_dt = s_dh_conj.re / mag_sq;
                
                // 1st order derivative of log amplitude with respect to frequency (Im{X_th / X_h})
                let d_log_A_dw = s_th_conj.im / mag_sq;
                
                let sig_t = (point_radius * 0.5 / (1.0 + d_log_A_dt.abs())).clamp(0.1, 4.0);
                let sig_f = (point_radius * 0.5 / (1.0 + d_log_A_dw.abs())).clamp(0.1, 4.0);
                
                let inv_2_sig_t_sq = 0.5 / (sig_t * sig_t);
                let inv_2_sig_f_sq = 0.5 / (sig_f * sig_f);
                
                // Fast Separable Gaussian (Only 6 exp calls instead of 9!)
                let wx_m1 = (-(-1.0 - dx) * (-1.0 - dx) * inv_2_sig_t_sq).exp();
                let wx_0  = (-(-dx * dx) * inv_2_sig_t_sq).exp();
                let wx_p1 = (-( 1.0 - dx) * ( 1.0 - dx) * inv_2_sig_t_sq).exp();
                
                let wy_m1 = (-(-1.0 - dy) * (-1.0 - dy) * inv_2_sig_f_sq).exp();
                let wy_0  = (-(-dy * dy) * inv_2_sig_f_sq).exp();
                let wy_p1 = (-( 1.0 - dy) * ( 1.0 - dy) * inv_2_sig_f_sq).exp();
                
                let sum_x = wx_m1 + wx_0 + wx_p1;
                let sum_y = wy_m1 + wy_0 + wy_p1;
                let inv_sum = 1.0 / ((sum_x * sum_y).max(1e-12));
                
                let wx = [wx_m1, wx_0, wx_p1];
                let wy = [wy_m1, wy_0, wy_p1];
                
                for ox in -1isize..=1isize {
                    let curr_c = c_reassigned_i + ox;
                    if curr_c < 0 || curr_c >= w as isize { continue; }
                    let w_x_val = wx[(ox + 1) as usize];
                    
                    for oy in -1isize..=1isize {
                        let curr_j = j_reassigned_i + oy;
                        if curr_j >= 0 && curr_j < h as isize {
                            let target_idx_gauss = curr_j as usize * w + curr_c as usize;
                            let norm_w = (w_x_val * wy[(oy + 1) as usize]) * inv_sum;
                            reassigned_grid_re[target_idx_gauss] += s_h.re * norm_w;
                            reassigned_grid_im[target_idx_gauss] += s_h.im * norm_w;
                        }
                    }
                }
            } else if algorithm_type == "cqt" {
                    // Constant-Q Log-Gaussian Spectral Jet (fCQT-Jet) - ZERO ALLOCATION / O(1) LUT HOT LOOP!
                    let k_low = cqt_k_low_lut[j];
                    let k_high = cqt_k_high_lut[j];
                    let kernel = &cqt_kernels_lut[j];
                    
                    let mut cqt_re = 0.0f32;
                    let mut cqt_im = 0.0f32;
                    let mut cqty_re = 0.0f32;
                    let mut cqty_im = 0.0f32;
                    let mut weight_sum = 0.0f32;
                    
                    // Sum over the support of the log-Gaussian filter using pre-computed LUT values!
                    let phase_multiplier = 2.0 * std::f32::consts::PI * (c * hop) as f32 / n_stft as f32;
                    
                    for (i, curr_k) in (k_low..=k_high).enumerate() {
                        let (g_val, gy_val) = kernel[i];
                        
                        if g_val > 1e-6 {
                            let y_k = buffer_h_th[curr_k];
                            let y_sym = buffer_h_th[n_stft - curr_k];
                            let fft_bin = Complex::new(0.5 * (y_k.re + y_sym.re), 0.5 * (y_k.im - y_sym.im));
                            
                            // Compute the continuous phase-shifted sum for sample index n = c * hop
                            // e^(i 2pi k n / N)
                            let angle = curr_k as f32 * phase_multiplier;
                            let phase_shifter = Complex::new(angle.cos(), angle.sin());
                            
                            let shifted_bin = fft_bin * phase_shifter;
                            
                            cqt_re += shifted_bin.re * g_val;
                            cqt_im += shifted_bin.im * g_val;
                            
                            cqty_re += shifted_bin.re * gy_val;
                            cqty_im += shifted_bin.im * gy_val;
                            
                            weight_sum += g_val;
                        }
                    }
                    
                    if weight_sum > 1e-15 {
                        let cqt_coeff = Complex::new(cqt_re / n_stft as f32, cqt_im / n_stft as f32);
                        let cqty_coeff = Complex::new(cqty_re / n_stft as f32, cqty_im / n_stft as f32);
                        
                        let abs_cqt = (cqt_coeff.re * cqt_coeff.re + cqt_coeff.im * cqt_coeff.im).sqrt();
                        
                        if abs_cqt > 1e-12 {
                            // Compute exact analytical derivatives of log-amplitude and phase with respect to y!
                            // ratio = C_y / C
                            let ratio = cqty_coeff * cqt_coeff.conj() / (abs_cqt * abs_cqt);
                            let d_log_A_dy = ratio.re;
                            let d_phi_dy = ratio.im;
                            
                            // We can use these derivatives to sharpen the visualization reassigning along the Y axis!
                            // Frequency reassigned coordinate:
                            let j_reassigned_f = if is_linear {
                                let y_j = (fc / fmin).log2();
                                let y_reassigned = y_j + d_phi_dy * step;
                                let f_reassigned = fmin * 2.0f32.powf(y_reassigned);
                                ((f_reassigned - fmin) / (fmax - fmin)) * (h as f32 - 1.0)
                            } else {
                                j as f32 + d_phi_dy * step
                            };
                            
                            // We can also perform 1D Gaussian sharpening spread along the Y-axis scaled by point_radius!
                            let sig_f = (point_radius * 0.5 / (1.0 + d_log_A_dy.abs())).clamp(0.05, 5.0);
                            
                            let j_reassigned_i = j_reassigned_f.round() as isize;
                            let dy = j_reassigned_f - j_reassigned_i as f32;
                            
                            // Distribute complex energy along the Y axis using 1D Gaussian spread of 3 bins
                            let mut w_sum = 0.0f32;
                            let mut w_vals = [0.0f32; 3];
                            
                            for oy in -1isize..=1isize {
                                let dist_y = oy as f32 - dy;
                                let w_val = (-0.5 * (dist_y * dist_y) / (sig_f * sig_f)).exp();
                                w_vals[(oy + 1) as usize] = w_val;
                                w_sum += w_val;
                            }
                            
                            if w_sum > 1e-15 {
                                for oy in -1isize..=1isize {
                                    let curr_j = j_reassigned_i + oy;
                                    if curr_j >= 0 && curr_j < h as isize {
                                        let target_idx_cqt = curr_j as usize * w + c;
                                        let norm_w = w_vals[(oy + 1) as usize] / w_sum;
                                        reassigned_grid_re[target_idx_cqt] += cqt_coeff.re * norm_w * 4194304.0; // scale back
                                        reassigned_grid_im[target_idx_cqt] += cqt_coeff.im * norm_w * 4194304.0;
                                    }
                                }
                            }
                        } else {
                            reassigned_grid_re[target_idx] = cqt_coeff.re * 4194304.0;
                            reassigned_grid_im[target_idx] = cqt_coeff.im * 4194304.0;
                        }
                    }
                } else {
                    // Standard Smooth Log Spectrogram
                    reassigned_grid_re[target_idx] = s_h.re;
                    reassigned_grid_im[target_idx] = s_h.im;
                }
        }
    }
    
    // Perform Colorization on the CPU
    let chunk_w = c_end - c_start;
    let mut rgba_buffer = vec![0u8; chunk_w * h * 4];
    
    // Prepare Snake Palette table internally if needed
    let mut snake_palette = Vec::new();
    if palette_type == "snake" {
        snake_palette = generate_snake_palette_lut();
    }
    
    for row in 0..h {
        for col in c_start..c_end {
            let grid_idx = row * w + col;
            let out_idx = (row * chunk_w + (col - c_start)) * 4;
            
            let re = reassigned_grid_re[grid_idx];
            let im = reassigned_grid_im[grid_idx];
            let abs_z = (re * re + im * im).sqrt();
            
            if abs_z < 1e-12 {
                rgba_buffer[out_idx] = 0;
                rgba_buffer[out_idx + 1] = 0;
                rgba_buffer[out_idx + 2] = 0;
                rgba_buffer[out_idx + 3] = 255;
                continue;
            }

            let abs_z_norm = abs_z / 4194304.0;
            if abs_z_norm > 1e-5 {
                let db_trans = 20.0 * abs_z_norm.log10();
                let b_trans = (((db_trans + 100.0) / 100.0) * 127.0).clamp(0.0, 127.0) as usize;
                raw_hist_trans[b_trans] += 1.0;
            }
            
            if palette_type == "snake" {
                // Geodesic Snake (Intensidade) colorizer
                // Map magnitude to 16-bit space
                let intensity = (abs_z / 4194304.0 * 65535.0).clamp(0.0, 65535.0) as usize;
                let (r, g, b) = snake_palette[intensity];
                rgba_buffer[out_idx] = r;
                rgba_buffer[out_idx + 1] = g;
                rgba_buffer[out_idx + 2] = b;
                rgba_buffer[out_idx + 3] = 255;
            } else {
                // YCbCr Magnitude-Phase Complex colorizer using zero-allocation O(1) LUT!
                let log2_r = abs_z_norm.log2();
                let y_val = (1.0 + 65024.0 * (log2_r + 15.0) / 15.0).sqrt();
                let mut y_f = y_val.floor();
                if y_f < 1.0 { y_f = 1.0; }
                if y_f > 255.0 { y_f = 255.0; }
                
                let y_idx = y_f as usize;
                let a_min = a_min_lut[y_idx];
                let delta_a = delta_a_lut[y_idx];
                
                let r_resid = abs_z_norm - a_min;
                let r_norm = r_resid / if delta_a > 1e-15 { delta_a } else { 1e-15 };
                
                let re_norm = re / 4194304.0;
                let im_norm = im / 4194304.0;
                let w_re = r_norm * (re_norm / abs_z_norm);
                let w_im = r_norm * (im_norm / abs_z_norm);
                
                let cr = -w_re;
                let cb = w_im;
                
                let cb_byte = (cb * 112.0 + 128.0).clamp(16.0, 240.0);
                let cr_byte = (cr * 112.0 + 128.0).clamp(16.0, 240.0);
                
                let r_val = y_f + 1.402 * (cr_byte - 128.0);
                let g_val = y_f - 0.344136 * (cb_byte - 128.0) - 0.714136 * (cr_byte - 128.0);
                let b_val = y_f + 1.772 * (cb_byte - 128.0);
                
                rgba_buffer[out_idx] = r_val.clamp(0.0, 255.0).round() as u8;
                rgba_buffer[out_idx + 1] = g_val.clamp(0.0, 255.0).round() as u8;
                rgba_buffer[out_idx + 2] = b_val.clamp(0.0, 255.0).round() as u8;
                rgba_buffer[out_idx + 3] = 255;
            }
        }
    }

    if c_end >= w {
        // Gaussian Smoothing KDE filter for mirrored continuous density plot
        let mut kernel = [0.0f32; 11];
        let mut k_sum = 0.0f32;
        for i in 0..11 {
            let x = (i as f32) - 5.0;
            let v = (-0.5 * (x * x) / (2.5 * 2.5)).exp();
            kernel[i] = v;
            k_sum += v;
        }
        for i in 0..11 {
            kernel[i] /= k_sum;
        }

        let mut density_trans = vec![0.0f32; 128];
        let mut density_orig = vec![0.0f32; 128];

        for i in 0..128 {
            let mut sum_trans = 0.0f32;
            let mut sum_orig = 0.0f32;
            for k in 0..11 {
                let idx = (i as isize + k as isize - 5).clamp(0, 127) as usize;
                sum_trans += raw_hist_trans[idx] * kernel[k];
                sum_orig += raw_hist_orig[idx] * kernel[k];
            }
            density_trans[i] = sum_trans;
            density_orig[i] = sum_orig;
        }

        let mut max_val = 1e-12f32;
        for i in 0..128 {
            if density_trans[i] > max_val { max_val = density_trans[i]; }
            if density_orig[i] > max_val { max_val = density_orig[i]; }
        }

        for i in 0..128 {
            density_trans[i] /= max_val;
            density_orig[i] /= max_val;
        }

        let mut combined_density = Vec::with_capacity(256);
        combined_density.extend_from_slice(&density_trans);
        combined_density.extend_from_slice(&density_orig);

        if let Ok(mut guard) = LAST_MIRRORED_DENSITY.lock() {
            *guard = combined_density;
        }
    }
    
    rgba_buffer
}

#[wasm_bindgen]
pub struct WasmHigherOrderPointResult {
    pub magnitude: f32,
    pub phase: f32,
    pub d_log_a_dt: f32,
    pub d_log_a_dw: f32,
    pub d_phi_dt: f32,
    pub d_phi_dw: f32,
    pub d2_phi_dt2: f32,
    pub freq_inst_hz: f32,
    pub time_reassigned_s: f32,
    pub hessian_det: f32,
    pub hessian_trace: f32,
    pub lambda_1: f32,
    pub lambda_2: f32,
    pub ridge_angle_rad: f32,
    pub anisotropy: f32,
    pub delta_t_star: f32,
    pub delta_w_star: f32,
    pub peak_magnitude: f32,
    pub is_ridge: bool,
    pub bandwidth_3db_hz: f32,
    pub duration_3db_s: f32,
    pub chirp_rate: f32,
    pub delta_t_inflex: f32,
}

#[wasm_bindgen]
pub fn wasm_analyze_higher_order_point(
    data: &[u8],
    sample_rate: f32,
    target_time_s: f32,
    target_freq_hz: f32,
    window_size: usize,
    max_order: usize,
) -> WasmHigherOrderPointResult {
    let pcm_data = if data.len() >= 44 && &data[0..4] == b"RIFF" {
        &data[44..]
    } else {
        data
    };
    let num_samples = pcm_data.len() / 4;
    let fs = if sample_rate > 0.0 { sample_rate } else { 44100.0 };
    let win_len = if window_size > 0 { window_size } else { 1024 };
    let center_sample = ((target_time_s * fs) as isize).clamp(0, (num_samples - 1) as isize);

    let mut frame = vec![0.0f32; win_len];
    let half_win = (win_len as isize) / 2;
    for i in 0..win_len {
        let idx = center_sample - half_win + i as isize;
        if idx >= 0 && (idx as usize) < num_samples {
            let offset = idx as usize * 4;
            let b0 = pcm_data[offset] as u16;
            let b1 = pcm_data[offset + 1] as u16;
            let b2 = pcm_data[offset + 2] as u16;
            let b3 = pcm_data[offset + 3] as u16;
            let l = ((b1 << 8) | b0) as i16 as f32;
            let r = ((b3 << 8) | b2) as i16 as f32;
            frame[i] = (l + r) * 0.5;
        }
    }

    let sigma_s = 0.4 * (win_len as f32 * 0.5) / fs;
    let order = max_order.clamp(1, 4);
    let mut higher_order_windows = Vec::new();
    let mut higher_order_labels = Vec::new();
    for p in 0..=order {
        for q in 0..=(order - p) {
            let win = crate::analysis::higher_order::generate_window_samples(p, q, win_len, fs, sigma_s);
            higher_order_windows.push(win);
            higher_order_labels.push((p, q));
        }
    }

    let mut coeffs = vec![crate::analysis::higher_order::Complex32::default(); higher_order_windows.len()];
    let half = (win_len as f32 - 1.0) * 0.5;
    for (idx, win) in higher_order_windows.iter().enumerate() {
        let mut acc = crate::analysis::higher_order::Complex32::default();
        for n in 0..win_len {
            let u = (n as f32 - half) / fs;
            let phase = -2.0 * std::f32::consts::PI * target_freq_hz * u;
            let (s, c) = phase.sin_cos();
            let val = frame[n] * win[n];
            acc.re += val * c;
            acc.im += val * s;
        }
        let (p, q) = higher_order_labels[idx];
        let sign_p = if p % 2 == 1 { -1.0 } else { 1.0 };
        let mod_q = q % 4;
        let factor_q = match mod_q {
            0 => crate::analysis::higher_order::Complex32::new(1.0, 0.0),
            1 => crate::analysis::higher_order::Complex32::new(0.0, -1.0),
            2 => crate::analysis::higher_order::Complex32::new(-1.0, 0.0),
            _ => crate::analysis::higher_order::Complex32::new(0.0, 1.0),
        };
        coeffs[idx] = acc.scale(sign_p).mul(factor_q);
    }

    let get_s = |req_p: usize, req_q: usize| -> crate::analysis::higher_order::Complex32 {
        for (idx, &(p, q)) in higher_order_labels.iter().enumerate() {
            if p == req_p && q == req_q {
                return coeffs[idx];
            }
        }
        crate::analysis::higher_order::Complex32::default()
    };

    let d = crate::analysis::higher_order::compute_faa_di_bruno_derivatives(
        order,
        get_s(0, 0),
        get_s(1, 0),
        get_s(0, 1),
        get_s(2, 0),
        get_s(1, 1),
        get_s(0, 2),
        get_s(3, 0),
        get_s(2, 1),
        get_s(1, 2),
        get_s(0, 3),
        get_s(4, 0),
        get_s(0, 4),
        target_freq_hz,
        target_time_s,
    );

    WasmHigherOrderPointResult {
        magnitude: d.magnitude,
        phase: d.phase,
        d_log_a_dt: d.d_log_a_dt,
        d_log_a_dw: d.d_log_a_dw,
        d_phi_dt: d.d_phi_dt,
        d_phi_dw: d.d_phi_dw,
        d2_phi_dt2: d.d2_phi_dt2,
        freq_inst_hz: d.freq_inst_hz,
        time_reassigned_s: d.time_reassigned_s,
        hessian_det: d.hessian_det,
        hessian_trace: d.hessian_trace,
        lambda_1: d.lambda_1,
        lambda_2: d.lambda_2,
        ridge_angle_rad: d.ridge_angle_rad,
        anisotropy: d.anisotropy,
        delta_t_star: d.delta_t_star,
        delta_w_star: d.delta_w_star,
        peak_magnitude: d.peak_magnitude,
        is_ridge: d.is_ridge,
        bandwidth_3db_hz: d.bandwidth_3db_hz,
        duration_3db_s: d.duration_3db_s,
        chirp_rate: d.d2_phi_dt2,
        delta_t_inflex: d.delta_t_inflex,
    }
}

// Utility to generate Geodesic Snake LUT inside Rust for 100% self-contained speeds
fn generate_snake_palette_lut() -> Vec<(u8, u8, u8)> {
    let mut palette = vec![(0u8, 0u8, 0u8); 65536];
    for i in 0..65536 {
        // Simple mathematical serpentine geodesic snake emulation
        let r = (i % 256) as u8;
        let g = ((i / 256) % 256) as u8;
        let b = (r as i32 - g as i32).abs() as u8;
        palette[i] = (r, g, b);
    }
    palette
}

#[wasm_bindgen]
pub fn wasm_synthesize_spectrogram_to_wav(
    rgba_grid: &[u8],
    width: usize,
    height: usize,
    fmin_custom: f32,
    fmax_custom: f32,
    scale_type: &str,
    window_size: usize,
    zero_padding: usize,
    start_col: usize,
    end_col: usize,
    hop_custom: usize,
    sample_rate_custom: u32,
) -> Vec<u8> {
    if width == 0 || height == 0 || rgba_grid.len() < width * height * 4 {
        return create_empty_wav();
    }

    let sample_rate = if sample_rate_custom > 0 { sample_rate_custom } else { 44100 };
    let fs = sample_rate as f32;
    let win_len = if window_size > 0 { window_size } else { 1024 };
    let pad_factor = if zero_padding > 0 { zero_padding } else { 4 };
    let n_stft = win_len * pad_factor;
    let hop = if hop_custom > 0 { hop_custom } else { 64 };

    let c_start = start_col.min(width - 1);
    let c_end = end_col.clamp(c_start + 1, width);
    let num_cols = c_end - c_start;

    let output_len = num_cols * hop + win_len;
    let mut output_samples = vec![0.0f32; output_len];
    let mut window_sum = vec![0.0f32; output_len];

    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let ifft = planner.plan_fft_inverse(n_stft);

    // Synthesis Hann window
    let mut win_syn = vec![0.0f32; win_len];
    for i in 0..win_len {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / (win_len - 1) as f32;
        win_syn[i] = 0.5 * (1.0 - angle.cos());
    }

    let fmin = if fmin_custom >= 5.0 { fmin_custom } else { 20.0f32 };
    let fmax = if fmax_custom > fmin { fmax_custom.min(fs / 2.0) } else { fs / 2.0 };
    let is_linear = scale_type == "linear";
    let step_log = (fmax / fmin).log2() / (height as f32 - 1.0);
    let step_lin = (fmax - fmin) / (height as f32 - 1.0);

    // Precompute a_min and delta_a LUTs
    let mut a_min_lut = vec![0.0f32; 256];
    let mut delta_a_lut = vec![0.0f32; 256];
    for y_idx in 1..=255 {
        let y_f = y_idx as f32;
        let amin_pow = -15.0 + 15.0 * (y_f * y_f - 1.0) / 65024.0;
        let a_min = 2.0f32.powf(amin_pow);
        let a_max = 2.0f32.powf(-15.0 + 15.0 * ((y_f + 1.0) * (y_f + 1.0) - 1.0) / 65024.0);
        a_min_lut[y_idx] = a_min;
        delta_a_lut[y_idx] = a_max - a_min;
    }
    delta_a_lut[0] = a_min_lut[1];

    let mut fft_buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];

    for col in c_start..c_end {
        let col_offset = col - c_start;
        let t_offset = col_offset * hop;

        for i in 0..n_stft {
            fft_buffer[i] = Complex::new(0.0, 0.0);
        }

        for j in 0..height {
            let px_idx = (j * width + col) * 4;
            let r = rgba_grid[px_idx] as f32;
            let g = rgba_grid[px_idx + 1] as f32;
            let b = rgba_grid[px_idx + 2] as f32;

            let y_val = 0.299 * r + 0.587 * g + 0.114 * b;
            if y_val < 1.0 {
                continue;
            }

            let cr = (r - y_val) / 1.402;
            let cb = (b - y_val) / 1.772;

            let w_re = -cr / 112.0;
            let w_im = cb / 112.0;
            let phase = w_im.atan2(w_re);
            let w_mag = (w_re * w_re + w_im * w_im).sqrt();

            let y_idx = (y_val.round() as usize).clamp(1, 255);
            let a_min = a_min_lut[y_idx];
            let delta_a = delta_a_lut[y_idx];

            let magnitude = (a_min + w_mag * delta_a) * 4194304.0;
            let z = Complex::new(magnitude * phase.cos(), magnitude * phase.sin());

            let fc = if is_linear {
                fmin + j as f32 * step_lin
            } else {
                fmin * 2.0f32.powf(j as f32 * step_log)
            };

            let k = (fc * n_stft as f32 / fs).round() as usize;
            if k > 0 && k < n_stft / 2 {
                fft_buffer[k] += z;
                fft_buffer[n_stft - k] += z.conj(); // Hermitian symmetry
            }
        }

        ifft.process(&mut fft_buffer);

        for i in 0..win_len {
            if t_offset + i < output_len {
                let sample_val = fft_buffer[i].re / n_stft as f32;
                output_samples[t_offset + i] += sample_val * win_syn[i];
                window_sum[t_offset + i] += win_syn[i] * win_syn[i];
            }
        }
    }

    // Normalize overlap-add
    for i in 0..output_len {
        if window_sum[i] > 1e-4 {
            output_samples[i] /= window_sum[i];
        }
    }

    // Find peak and normalize
    let peak = output_samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let scale_factor = if peak > 1.0 { 32760.0 / peak } else { 32760.0 };

    // Encode to 16-bit Mono WAV
    let num_pcm = output_len;
    let byte_rate = sample_rate * 2;
    let block_align = 2u16;
    let bits_per_sample = 16u16;
    let data_chunk_size = (num_pcm * 2) as u32;
    let file_size = 36 + data_chunk_size;

    let mut wav = Vec::with_capacity(44 + num_pcm * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // Mono
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_chunk_size.to_le_bytes());

    for s in output_samples {
        let pcm_val = (s * scale_factor).clamp(-32767.0, 32767.0) as i16;
        wav.extend_from_slice(&pcm_val.to_le_bytes());
    }

    wav
}

fn create_empty_wav() -> Vec<u8> {
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&36u32.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&44100u32.to_le_bytes());
    wav.extend_from_slice(&88200u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&0u32.to_le_bytes());
    wav
}

#[wasm_bindgen]
pub fn wasm_calculate_log_spectrogram(data: &[u8], h_custom: usize, window_type: &str) -> Vec<u8> {
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    
    // Detect and skip 44-byte WAV header if present
    let pcm_data = if data.len() >= 44 && &data[0..4] == b"RIFF" {
        &data[44..]
    } else {
        data
    };
    
    let w = 800; // Fixed target landscape width of 800 columns
    let grid_size = w * h;
    
    // Decoded PCM Mid channel data
    let num_samples = pcm_data.len() / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = if offset < pcm_data.len() { pcm_data[offset] } else { 0 };
        let b1 = if offset + 1 < pcm_data.len() { pcm_data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < pcm_data.len() { pcm_data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < pcm_data.len() { pcm_data[offset + 3] } else { 0 };
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        mid_channel[i] = (l + r) * 0.5;
    }
    
    let fs_f32 = 44100.0f32;
    let n_stft = 4096;
    
    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    
    // Generate the window h
    let mut win_h = vec![0.0f32; n_stft];
    match window_type {
        "hamming" => {
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = 0.54 - 0.46 * angle.cos();
            }
        },
        "gaussian" => {
            let sigma = (n_stft - 1) as f32 / 6.0; // alpha = 3.0
            let half_n = (n_stft - 1) as f32 / 2.0;
            for i in 0..n_stft {
                let diff = i as f32 - half_n;
                win_h[i] = (-0.5 * (diff / sigma).powi(2)).exp();
            }
        },
        "blackman-harris" => {
            let a0 = 0.35875f32;
            let a1 = 0.48829f32;
            let a2 = 0.14128f32;
            let a3 = 0.01168f32;
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = a0 - a1 * angle.cos() + a2 * (2.0 * angle).cos() - a3 * (3.0 * angle).cos();
            }
        },
        _ => { // "hann" as default
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = 0.5 * (1.0 - angle.cos());
            }
        }
    }
    
    let fmin = 20.0f32;
    let fmax = 20000.0f32.min(fs_f32 / 2.0);
    let step = (fmax / fmin).log2() / (h as f32 - 1.0);
    
    let mut log_grid = vec![0.0f32; grid_size];
    let hop = (num_samples as f32 / w as f32).max(1.0).floor() as usize;
    
    for c in 0..w {
        let start = c * hop;
        let mut buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        
        for i in 0..n_stft {
            let idx = start as isize + i as isize - (n_stft as isize / 2);
            if idx >= 0 && idx < num_samples as isize {
                buffer[i] = Complex::new(mid_channel[idx as usize] * win_h[i], 0.0);
            }
        }
        
        fft.process(&mut buffer);
        
        for j in 0..h {
            let fc = fmin * 2.0f32.powf(j as f32 * step);
            let k_frac = fc * n_stft as f32 / fs_f32;
            let k_floor = k_frac.floor() as usize;
            let k_ceil = k_frac.ceil() as usize;
            let k_floor = k_floor.clamp(0, n_stft / 2 - 1);
            let k_ceil = k_ceil.clamp(0, n_stft / 2 - 1);
            
            let mag_floor = (buffer[k_floor].re * buffer[k_floor].re + buffer[k_floor].im * buffer[k_floor].im).sqrt();
            let mag_ceil = (buffer[k_ceil].re * buffer[k_ceil].re + buffer[k_ceil].im * buffer[k_ceil].im).sqrt();
            
            // Linear interpolation between floor and ceil FFT bins
            let t = k_frac - k_frac.floor();
            let interpolated_mag = mag_floor * (1.0 - t) + mag_ceil * t;
            
            log_grid[j * w + c] = interpolated_mag * interpolated_mag; // Power spectrum
        }
    }
    
    // Normalize and apply 0.3 gamma compression
    let mut max_val = 1e-12f32;
    for i in 0..grid_size {
        if log_grid[i] > max_val { max_val = log_grid[i]; }
    }
    
    // Pack into direct self-contained RGBA buffer with big-endian header!
    let mut output = Vec::with_capacity(24 + grid_size * 4);
    output.extend_from_slice(&0u32.to_be_bytes()); // original_len
    output.extend_from_slice(&(w as u32).to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    for _ in 0..12 { output.push(0u8); } // padding
    
    for r in 0..h {
        for c in 0..w {
            let val = (log_grid[r * w + c] / max_val).powf(0.3);
            let color_idx = (val * 65535.0).round() as u16;
            output.push(get_color_r(color_idx));
            output.push(get_color_g(color_idx));
            output.push(get_color_b(color_idx));
            output.push(255u8);
        }
    }
    output
}

#[wasm_bindgen]
pub fn wasm_get_complex_spectrogram_width(data: &[u8], h_custom: usize) -> usize {
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    let pcm_data = if data.len() >= 44 && &data[0..4] == b"RIFF" {
        &data[44..]
    } else {
        data
    };
    calculate_grid_width(pcm_data.len() / 2, h)
}

#[wasm_bindgen]
pub fn wasm_generate_complex_spectrogram(data: &[u8], h_custom: usize, window_type: &str) -> Vec<f32> {
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    
    let pcm_data = if data.len() >= 44 && &data[0..4] == b"RIFF" {
        &data[44..]
    } else {
        data
    };
    
    let w = 800; // Match high-fidelity landscape resolution
    let grid_size = w * h;
    
    // Decoded PCM Mid channel data
    let num_samples = pcm_data.len() / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = if offset < pcm_data.len() { pcm_data[offset] } else { 0 };
        let b1 = if offset + 1 < pcm_data.len() { pcm_data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < pcm_data.len() { pcm_data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < pcm_data.len() { pcm_data[offset + 3] } else { 0 };
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        mid_channel[i] = (l + r) * 0.5;
    }
    
    let fs_f32 = 44100.0f32;
    let n_stft = 4096;
    
    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    
    // Generate the window h
    let mut win_h = vec![0.0f32; n_stft];
    match window_type {
        "hamming" => {
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = 0.54 - 0.46 * angle.cos();
            }
        },
        "gaussian" => {
            let sigma = (n_stft - 1) as f32 / 6.0; // alpha = 3.0
            let half_n = (n_stft - 1) as f32 / 2.0;
            for i in 0..n_stft {
                let diff = i as f32 - half_n;
                win_h[i] = (-0.5 * (diff / sigma).powi(2)).exp();
            }
        },
        "blackman-harris" => {
            let a0 = 0.35875f32;
            let a1 = 0.48829f32;
            let a2 = 0.14128f32;
            let a3 = 0.01168f32;
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = a0 - a1 * angle.cos() + a2 * (2.0 * angle).cos() - a3 * (3.0 * angle).cos();
            }
        },
        _ => { // "hann" as default
            for i in 0..n_stft {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
                win_h[i] = 0.5 * (1.0 - angle.cos());
            }
        }
    }
    
    let fmin = 20.0f32;
    let fmax = 20000.0f32.min(fs_f32 / 2.0);
    let step = (fmax / fmin).log2() / (h as f32 - 1.0);
    
    let mut complex_grid = vec![0.0f32; grid_size * 2]; // Alternating [Real, Imag]
    let hop = (num_samples as f32 / w as f32).max(1.0).floor() as usize;
    
    for c in 0..w {
        let start = c * hop;
        let mut buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        
        for i in 0..n_stft {
            let idx = start as isize + i as isize - (n_stft as isize / 2);
            if idx >= 0 && idx < num_samples as isize {
                buffer[i] = Complex::new(mid_channel[idx as usize] * win_h[i], 0.0);
            }
        }
        
        fft.process(&mut buffer);
        
        for j in 0..h {
            let fc = fmin * 2.0f32.powf(j as f32 * step);
            let k_frac = fc * n_stft as f32 / fs_f32;
            let k_floor = k_frac.floor() as usize;
            let k_ceil = k_frac.ceil() as usize;
            let k_floor = k_floor.clamp(0, n_stft / 2 - 1);
            let k_ceil = k_ceil.clamp(0, n_stft / 2 - 1);
            
            // Linear interpolation of complex coefficients
            let val_floor = buffer[k_floor];
            let val_ceil = buffer[k_ceil];
            
            let t = k_frac - k_frac.floor();
            let interpolated_re = val_floor.re * (1.0 - t) + val_ceil.re * t;
            let interpolated_im = val_floor.im * (1.0 - t) + val_ceil.im * t;
            
            let out_idx = (j * w + c) * 2;
            complex_grid[out_idx]     = interpolated_re;
            complex_grid[out_idx + 1] = interpolated_im;
        }
    }
    
    complex_grid
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct ReassignmentComparison {
    pub avg_time_error: f32,
    pub max_time_error: f32,
    pub avg_freq_error: f32,
    pub max_freq_error: f32,
    pub evaluated_points: u32,
    pub sample_ratio: f32,
    pub sample_grad: f32,
    pub sample_raw_grad: f32,
}

#[wasm_bindgen]
pub fn wasm_compare_reassignment_methods(data: &[u8]) -> ReassignmentComparison {
    // Detect and skip 44-byte WAV header if present
    let pcm_data = if data.len() >= 44 && &data[0..4] == b"RIFF" {
        &data[44..]
    } else {
        data
    };
    
    let num_samples = pcm_data.len() / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = if offset < pcm_data.len() { pcm_data[offset] } else { 0 };
        let b1 = if offset + 1 < pcm_data.len() { pcm_data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < pcm_data.len() { pcm_data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < pcm_data.len() { pcm_data[offset + 3] } else { 0 };
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        mid_channel[i] = (l + r) * 0.5;
    }
    
    let n_stft = 4096;
    let half_n = (n_stft - 1) as f32 / 2.0;
    
    // Generate Hann analysis, time, and derivative windows
    let mut win_h = vec![0.0f32; n_stft];
    let mut win_th = vec![0.0f32; n_stft];
    let mut win_dh = vec![0.0f32; n_stft];
    for i in 0..n_stft {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / (n_stft - 1) as f32;
        win_h[i] = 0.5 * (1.0 - angle.cos());
        win_th[i] = (i as f32 - half_n) * win_h[i];
        win_dh[i] = (std::f32::consts::PI / (n_stft - 1) as f32) * angle.sin();
    }
    
    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    
    let mut sum_diff_t = 0.0f64;
    let mut sum_diff_k = 0.0f64;
    let mut max_diff_t = 0.0f32;
    let mut max_diff_k = 0.0f32;
    let mut count = 0u32;
    
    let mut s_ratio = 0.0f32;
    let mut s_grad = 0.0f32;
    let mut s_raw_grad = 0.0f32;
    
    // We sample 150 sliding window frames in the middle region to avoid edge boundaries
    let start_frame = (num_samples / 2).max(n_stft) as isize;
    let end_frame = (start_frame + 150).min(num_samples as isize - n_stft as isize) as isize;
    
    for c in start_frame..end_frame {
        // 1. Compute normal STFT, time-weighted, and derivative STFT at center frame c
        let mut buf_h = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buf_th = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buf_dh = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        for i in 0..n_stft {
            let sample_idx = c + i as isize - (n_stft as isize / 2);
            let val = mid_channel[sample_idx as usize];
            buf_h[i] = Complex::new(val * win_h[i], 0.0);
            buf_th[i] = Complex::new(val * win_th[i], 0.0);
            buf_dh[i] = Complex::new(val * win_dh[i], 0.0);
        }
        fft.process(&mut buf_h);
        fft.process(&mut buf_th);
        fft.process(&mut buf_dh);
        
        // 2. Compute normal STFT shifted by +1 sample and -1 sample
        let mut buf_h_p1 = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buf_h_m1 = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        for i in 0..n_stft {
            let sample_idx_p1 = c + 1 + i as isize - (n_stft as isize / 2);
            let sample_idx_m1 = c - 1 + i as isize - (n_stft as isize / 2);
            buf_h_p1[i] = Complex::new(mid_channel[sample_idx_p1 as usize] * win_h[i], 0.0);
            buf_h_m1[i] = Complex::new(mid_channel[sample_idx_m1 as usize] * win_h[i], 0.0);
        }
        fft.process(&mut buf_h_p1);
        fft.process(&mut buf_h_m1);
        
        // Compare values bin-by-bin for active spectral lines (k in 10..500)
        for k in 10..500 {
            let s_h = buf_h[k];
            let mag_sq = s_h.re * s_h.re + s_h.im * s_h.im;
            
            // Only compare meaningful points with sufficient energy level to avoid floating-point noise around zeros
            if mag_sq > 5000000.0 {
                // A. Time Reassignment (Re{ S_th * conj(S_h) / |S_h|^2 }) in samples
                let s_th = buf_th[k];
                let shift_t_ratio = (s_th * s_h.conj()).re / mag_sq;
                
                // B. Time Reassignment from Phase Gradient w.r.t frequency (central difference k+1 and k-1)
                let s_h_kp1 = buf_h[k + 1];
                let s_h_km1 = buf_h[k - 1];
                let grad_k_phi = (s_h_kp1 * s_h_km1.conj()).arg() / 2.0;
                let shift_t_grad = - grad_k_phi * (n_stft as f32 / (2.0 * std::f32::consts::PI)) + 0.5;
                
                if count == 100 {
                    s_ratio = shift_t_ratio;
                    s_grad = shift_t_grad;
                    s_raw_grad = grad_k_phi;
                }
                
                // C. Frequency Reassignment (-Im{ S_dh * conj(S_h) / |S_h|^2 } * (N / 2pi)) in bins
                let s_dh = buf_dh[k];
                let shift_k_ratio = - (s_dh * s_h.conj()).im / mag_sq * (n_stft as f32 / (2.0 * std::f32::consts::PI));
                
                // D. Frequency Reassignment from Phase Gradient w.r.t time: dPhi/dT * (N / 2pi) - k
                let s_h_p1 = buf_h_p1[k];
                let s_h_m1 = buf_h_m1[k];
                let grad_tau_phi = (s_h_p1 * s_h_m1.conj()).arg(); // phase difference between c+1 and c-1
                let shift_k_grad = (grad_tau_phi / 2.0) * (n_stft as f32 / (2.0 * std::f32::consts::PI)) - k as f32;
                
                let diff_t = (shift_t_ratio - shift_t_grad).abs();
                let diff_k = (shift_k_ratio - shift_k_grad).abs();
                
                sum_diff_t += diff_t as f64;
                sum_diff_k += diff_k as f64;
                
                if diff_t > max_diff_t { max_diff_t = diff_t; }
                if diff_k > max_diff_k { max_diff_k = diff_k; }
                
                count += 1;
            }
        }
    }
    
    ReassignmentComparison {
        avg_time_error: if count > 0 { (sum_diff_t / count as f64) as f32 } else { 0.0 },
        max_time_error: max_diff_t,
        avg_freq_error: if count > 0 { (sum_diff_k / count as f64) as f32 } else { 0.0 },
        max_freq_error: max_diff_k,
        evaluated_points: count,
        sample_ratio: s_ratio,
        sample_grad: s_grad,
        sample_raw_grad: s_raw_grad,
    }
}

#[wasm_bindgen]
pub fn encode_wavelet_v7_cqt(data: &[u8], h_custom: usize) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    let w = calculate_grid_width(data.len(), h);
    let grid_size = w * h;
    
    // Decoded PCM Mid/Side data
    let num_samples = data.len() / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = if offset < data.len() { data[offset] } else { 0 };
        let b1 = if offset + 1 < data.len() { data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < data.len() { data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < data.len() { data[offset + 3] } else { 0 };
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        mid_channel[i] = (l + r) * 0.5;
    }
    
    let fs_f32 = 44100.0f32;
    let n_stft = 4096;
    
    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    
    // Constant relative standard deviation Q = 12 bins/octave (approx. 0.0833)
    let sigma = 1.0f32 / 12.0f32;
    
    // Logarithmic center frequencies spanning from 20 Hz to 20000 Hz
    let fmin = 20.0f32;
    let fmax = 20000.0f32.min(fs_f32 / 2.0);
    let step = (fmax / fmin).log2() / (h as f32 - 1.0);
    
    let mut c_ref_real = vec![0.0f32; h * w];
    let mut c_ref_imag = vec![0.0f32; h * w];
    
    let hop = (num_samples as f32 / w as f32).max(1.0).floor() as usize;
    
    // Process frames using STFT Hann-windowing and Gaussian warping
    for c in 0..w {
        let start = c * hop;
        let mut buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        
        for i in 0..n_stft {
            let idx = start as isize + i as isize - (n_stft as isize / 2);
            if idx >= 0 && idx < num_samples as isize {
                let w_val = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n_stft as f32 - 1.0)).cos());
                buffer[i] = Complex::new(mid_channel[idx as usize] * w_val, 0.0);
            }
        }
        
        fft.process(&mut buffer);
        
        // Warp linear frequency bins to logarithmic CQT bins
        for j in 0..h {
            let fc = fmin * 2.0f32.powf(j as f32 * step);
            
            let mut sum_re = 0.0f32;
            let mut sum_im = 0.0f32;
            let mut sum_w = 0.0f32;
            
            // Dynamic window centering with 3-sigma octave boundaries to ensure perfect low-frequency overlap
            let k_start = ((fc * 0.80f32) * n_stft as f32 / fs_f32).round() as isize;
            let k_end = ((fc * 1.25f32) * n_stft as f32 / fs_f32).round() as isize;
            let k_start = k_start.max(1) as usize;
            let k_end = k_end.min((n_stft / 2 - 1) as isize) as usize;
            
            for k in k_start..=k_end {
                let fk = k as f32 * fs_f32 / n_stft as f32;
                let d = (fk / fc).log2();
                let w_val = (-0.5 * (d / sigma).powi(2)).exp();
                
                sum_re += buffer[k].re * w_val;
                sum_im += buffer[k].im * w_val;
                sum_w += w_val;
            }
            
            if sum_w > 1e-12 {
                c_ref_real[j * w + c] = sum_re / sum_w;
                c_ref_imag[j * w + c] = sum_im / sum_w;
            }
        }
    }
    
    // Pack into output pixels
    let mut output = Vec::with_capacity(16 + grid_size * 8);
    output.extend_from_slice(&original_len.to_be_bytes());
    let w_png = (w * 2) as u32;
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(0u8); // unused
    output.push(7u8); // Packing Version 7!
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    // Find maximum power in CQT matrix to scale dynamically and preserve rich dynamic range
    let mut max_power = 1e-5f32;
    for r in 0..h {
        for c in 0..w {
            let power = c_ref_real[r * w + c] * c_ref_real[r * w + c] + c_ref_imag[r * w + c] * c_ref_imag[r * w + c];
            if power > max_power { max_power = power; }
        }
    }
    
    for r in 0..h {
        for c in 0..w {
            let power = c_ref_real[r * w + c] * c_ref_real[r * w + c] + c_ref_imag[r * w + c] * c_ref_imag[r * w + c];
            
            // Apply exact V6 gamma compression (gamma 0.3) for stunning thermal dynamics
            let ratio = (power / max_power).powf(0.3);
            let color_index = (ratio * 65535.0) as u16;
            
            // Retrieve beautiful Geodesic Snake colors from the 3D RGB Cube
            let r_val = get_color_r(color_index);
            let g_val = get_color_g(color_index);
            let b_val = get_color_b(color_index);
            
            // Pixel A (Mid CQT color)
            output.push(r_val);
            output.push(g_val);
            output.push(b_val);
            output.push(255u8);
            
            // Pixel B (Side CQT color, kept identical for visually smooth continuous curves)
            output.push(r_val);
            output.push(g_val);
            output.push(b_val);
            output.push(255u8);
        }
    }
    
    // Steganographically append the exact original WAV bytes right at the end!
    output.extend_from_slice(data);
    output
}

#[wasm_bindgen]
pub fn encode_stft_cqt_v8_layout(data: &[u8], h_custom: usize) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut h = h_custom;
    if !h.is_power_of_two() || h < 4 { h = 1024; }
    
    // Detect and skip 44-byte WAV header if present
    let pcm_data = if data.len() >= 44 && &data[0..4] == b"RIFF" {
        &data[44..]
    } else {
        data
    };
    
    let w = calculate_grid_width(pcm_data.len() / 2, h);
    let grid_size = w * h;
    let w_cqt = w * 2; // 2 consecutive temporal frames packed horizontally per grid position!
    
    // Decoded PCM Mid/Side data
    let num_samples = pcm_data.len() / 4;
    let mut mid_channel = vec![0.0f32; num_samples];
    let mut side_channel = vec![0.0f32; num_samples];
    for i in 0..num_samples {
        let offset = i * 4;
        let b0 = if offset < pcm_data.len() { pcm_data[offset] } else { 0 };
        let b1 = if offset + 1 < pcm_data.len() { pcm_data[offset + 1] } else { 0 };
        let b2 = if offset + 2 < pcm_data.len() { pcm_data[offset + 2] } else { 0 };
        let b3 = if offset + 3 < pcm_data.len() { pcm_data[offset + 3] } else { 0 };
        
        let l = (((b1 as u16) << 8) | (b0 as u16)) as i16 as f32;
        let r = (((b3 as u16) << 8) | (b2 as u16)) as i16 as f32;
        mid_channel[i] = (l + r) * 0.5;
        side_channel[i] = (l - r) * 0.5;
    }
    
    let fs_f32 = 44100.0f32;
    let n_stft = 4096;
    
    use rustfft::{FftPlanner, num_complex::Complex};
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_stft);
    
    let sigma = 1.0f32 / 12.0f32;
    let fmin = 20.0f32;
    let fmax = 20000.0f32.min(fs_f32 / 2.0);
    let step = (fmax / fmin).log2() / (h as f32 - 1.0);
    
    let mut c_mid_real = vec![0.0f32; h * w_cqt];
    let mut c_mid_imag = vec![0.0f32; h * w_cqt];
    let mut c_side_real = vec![0.0f32; h * w_cqt];
    let mut c_side_imag = vec![0.0f32; h * w_cqt];
    
    let hop = (num_samples as f32 / w_cqt as f32).max(1.0).floor() as usize;
    
    for c_cqt in 0..w_cqt {
        let start = c_cqt * hop;
        let mut buffer_mid = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        let mut buffer_side = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
        
        for i in 0..n_stft {
            let idx = start as isize + i as isize - (n_stft as isize / 2);
            if idx >= 0 && idx < num_samples as isize {
                let w_val = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n_stft as f32 - 1.0)).cos());
                buffer_mid[i] = Complex::new(mid_channel[idx as usize] * w_val, 0.0);
                buffer_side[i] = Complex::new(side_channel[idx as usize] * w_val, 0.0);
            }
        }
        
        fft.process(&mut buffer_mid);
        fft.process(&mut buffer_side);
        
        for j in 0..h {
            let fc = fmin * 2.0f32.powf(j as f32 * step);
            let k_start = ((fc * 0.80f32) * n_stft as f32 / fs_f32).round() as isize;
            let k_end = ((fc * 1.25f32) * n_stft as f32 / fs_f32).round() as isize;
            let k_start = k_start.max(1) as usize;
            let k_end = k_end.min((n_stft / 2 - 1) as isize) as usize;
            
            let mut sum_mid_re = 0.0f32;
            let mut sum_mid_im = 0.0f32;
            let mut sum_side_re = 0.0f32;
            let mut sum_side_im = 0.0f32;
            let mut sum_w = 0.0f32;
            
            for k in k_start..=k_end {
                let fk = k as f32 * fs_f32 / n_stft as f32;
                let d = (fk / fc).log2();
                let w_val = (-0.5 * (d / sigma).powi(2)).exp();
                
                sum_mid_re += buffer_mid[k].re * w_val;
                sum_mid_im += buffer_mid[k].im * w_val;
                sum_side_re += buffer_side[k].re * w_val;
                sum_side_im += buffer_side[k].im * w_val;
                sum_w += w_val;
            }
            
            if sum_w > 1e-12 {
                c_mid_real[j * w_cqt + c_cqt] = sum_mid_re / sum_w;
                c_mid_imag[j * w_cqt + c_cqt] = sum_mid_im / sum_w;
                c_side_real[j * w_cqt + c_cqt] = sum_side_re / sum_w;
                c_side_imag[j * w_cqt + c_cqt] = sum_side_im / sum_w;
            }
        }
    }
    
    // Find max power for normalization
    let mut max_power_mid = 1e-12f32;
    let mut max_power_side = 1e-12f32;
    for i in 0..(h * w_cqt) {
        let p_mid = c_mid_real[i] * c_mid_real[i] + c_mid_imag[i] * c_mid_imag[i];
        let p_side = c_side_real[i] * c_side_real[i] + c_side_imag[i] * c_side_imag[i];
        if p_mid > max_power_mid { max_power_mid = p_mid; }
        if p_side > max_power_side { max_power_side = p_side; }
    }
    
    // Pack into exactly 4 pixels per frame:
    let mut output = Vec::with_capacity(24 + grid_size * 16);
    output.extend_from_slice(&original_len.to_be_bytes());
    let w_png = (w * 4) as u32;
    output.extend_from_slice(&w_png.to_be_bytes());
    output.extend_from_slice(&(h as u32).to_be_bytes());
    
    output.push(0u8); // rem_bytes
    output.push(8u8); // packing version 8 to match layout!
    output.extend_from_slice(&44100u16.to_be_bytes());
    
    // Bytes 16-23: Padding zeroes
    for _ in 0..8 { output.push(0u8); }
    
    for r in 0..h {
        for c in 0..w {
            let idx0 = r * w_cqt + (c * 2);
            let idx1 = idx0 + 1;
            
            let p_mid0 = c_mid_real[idx0] * c_mid_real[idx0] + c_mid_imag[idx0] * c_mid_imag[idx0];
            let p_mid1 = c_mid_real[idx1] * c_mid_real[idx1] + c_mid_imag[idx1] * c_mid_imag[idx1];
            
            let p_side0 = c_side_real[idx0] * c_side_real[idx0] + c_side_imag[idx0] * c_side_imag[idx0];
            let p_side1 = c_side_real[idx1] * c_side_real[idx1] + c_side_imag[idx1] * c_side_imag[idx1];
            
            let ratio_mid0 = (p_mid0 / max_power_mid).powf(0.3);
            let ratio_mid1 = (p_mid1 / max_power_mid).powf(0.3);
            
            let ratio_side0 = (p_side0 / max_power_side).powf(0.3);
            let ratio_side1 = (p_side1 / max_power_side).powf(0.3);
            
            // Map positive CQT intensity to the same Geodesic Snake scale: [0, 65535] (0 maps exactly to black)
            let sm_u0 = (ratio_mid0 * 65535.0) as u16;
            let sm_u1 = (ratio_mid1 * 65535.0) as u16;
            
            let ss_u0 = (ratio_side0 * 65535.0) as u16;
            let ss_u1 = (ratio_side1 * 65535.0) as u16;
            
            // Pixel A: sm_u0
            output.push(get_color_r(sm_u0));
            output.push(get_color_g(sm_u0));
            output.push(get_color_b(sm_u0));
            output.push(255u8);
            
            // Pixel B: sm_u1
            output.push(get_color_r(sm_u1));
            output.push(get_color_g(sm_u1));
            output.push(get_color_b(sm_u1));
            output.push(255u8);
            
            // Pixel C: ss_u0
            output.push(get_color_r(ss_u0));
            output.push(get_color_g(ss_u0));
            output.push(get_color_b(ss_u0));
            output.push(255u8);
            
            // Pixel D: ss_u1
            output.push(get_color_r(ss_u1));
            output.push(get_color_g(ss_u1));
            output.push(get_color_b(ss_u1));
            output.push(255u8);
        }
    }
    output
}

#[wasm_bindgen]
pub fn decode_wavelet_v7_cqt(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 16 {
        return Err(JsValue::from_str("Invalid input data"));
    }
    let original_len = u32::from_be_bytes([rgba_data[0], rgba_data[1], rgba_data[2], rgba_data[3]]) as usize;
    let w_png = u32::from_be_bytes([rgba_data[4], rgba_data[5], rgba_data[6], rgba_data[7]]) as usize;
    let h = u32::from_be_bytes([rgba_data[8], rgba_data[9], rgba_data[10], rgba_data[11]]) as usize;
    let grid_size = w_png * h;
    
    let payload_offset = 16 + grid_size * 4;
    if payload_offset + original_len > rgba_data.len() {
        return Err(JsValue::from_str("Corrupted steganographic payload"));
    }
    
    let pcm = rgba_data[payload_offset .. payload_offset + original_len].to_vec();
    Ok(pcm)
}

#[wasm_bindgen]
pub fn wasm_generate_v7_spectrogram(rgba_data: &[u8]) -> Result<Vec<f32>, JsValue> {
    if rgba_data.len() < 16 {
        return Err(JsValue::from_str("Invalid input data"));
    }
    let w_png = u32::from_be_bytes([rgba_data[4], rgba_data[5], rgba_data[6], rgba_data[7]]) as usize;
    let h = u32::from_be_bytes([rgba_data[8], rgba_data[9], rgba_data[10], rgba_data[11]]) as usize;
    let w = w_png / 2;
    let mut spec = vec![0.0f32; w * h];
    
    for r in 0..h {
        for c in 0..w {
            let idx_a = r * w_png + (c * 2);
            let offset_a = 16 + idx_a * 4;
            
            if offset_a + 3 >= rgba_data.len() { break; }
            
            let r_val = rgba_data[offset_a];
            let g_val = rgba_data[offset_a + 1];
            let b_val = rgba_data[offset_a + 2];
            
            // Decode color back to original coefficient index
            let coef = decode_color_to_coefficient(r_val, g_val, b_val);
            let ratio = coef as f32 / 65535.0;

            // Decompress power (gamma 0.3 inverse is 1.0 / 0.3) with absolute value to prevent NaN on negative ratios
            spec[r * w + c] = ratio.abs().powf(1.0 / 0.3);
        }
    }
    Ok(spec)
}

fn lift_forward_float(v: &[f32], p: &[f32; 3], u_coefs: &[f32; 3]) -> Vec<f32> {
    let len = v.len();
    let half_e = (len + 1) / 2;
    let half_o = len / 2;
    let mut e = vec![0.0f32; half_e];
    let mut o = vec![0.0f32; half_o];
    for i in 0..len {
        if i % 2 == 0 { e[i / 2] = v[i]; }
        else { o[i / 2] = v[i]; }
    }
    
    let mut pred = vec![0.0f32; half_o];
    for k in 0..half_o {
        let em2 = e[k.saturating_sub(2)];
        let em1 = e[k.saturating_sub(1)];
        let ec  = e[k];
        let ep1 = e[std::cmp::min(k + 1, half_e - 1)];
        let ep2 = e[std::cmp::min(k + 2, half_e - 1)];
        
        pred[k] = p[0] * ec + p[1] * (em1 + ep1) + p[2] * (em2 + ep2);
    }
    
    let mut d = vec![0.0f32; half_o];
    for k in 0..half_o {
        d[k] = o[k] - pred[k];
    }
    
    let mut upd = vec![0.0f32; half_e];
    for k in 0..half_e {
        let dm2 = d[k.saturating_sub(2)];
        let dm1 = d[k.saturating_sub(1)];
        let dc  = d[k];
        let dp1 = d[std::cmp::min(k + 1, half_o - 1)];
        let dp2 = d[std::cmp::min(k + 2, half_o - 1)];
        
        upd[k] = u_coefs[0] * dc + u_coefs[1] * (dm1 + dp1) + u_coefs[2] * (dm2 + dp2);
    }
    
    let mut s = e.clone();
    for k in 0..half_e {
        s[k] += upd[k];
    }
    
    let mut out = vec![0.0f32; len];
    for i in 0..half_e { out[i * 2] = s[i]; }
    for i in 0..half_o { out[i * 2 + 1] = d[i]; }
    out
}

fn evaluate_cqt_lifting_rmse(p: &[f32; 3], u_coefs: &[f32; 3]) -> f32 {
    let n_test = 61;
    let center = 30;
    let mut cur = vec![0.0f32; n_test];
    cur[center] = 1.0;
    
    // Apply 3 stages of forward float lifting
    for _ in 0..3 {
        cur = lift_forward_float(&cur, p, u_coefs);
    }
    
    // Rescale response
    let mut max_val = 1e-30f32;
    for &v in &cur {
        if v.abs() > max_val { max_val = v.abs(); }
    }
    
    let target_sigma_bins = 1.65 / 2.355;
    let mut error = 0.0f32;
    
    for i in 0..n_test {
        let val = cur[i].abs() / max_val;
        let x_idx = i as f32 - center as f32;
        let gauss_target = (-0.5 * (x_idx / target_sigma_bins).powi(2)).exp();
        
        let yd = 20.0 * (val.max(1e-8)).log10();
        let gd = 20.0 * (gauss_target.max(1e-8)).log10();
        
        // Weight emphasizing the main lobe
        let w = (-0.5 * (x_idx / 12.0).powi(2)).exp();
        error += w * (yd - gd).powi(2);
    }
    error
}

#[wasm_bindgen]
pub fn wasm_optimize_cqt_lifting() -> Vec<f32> {
    let mut p = [0.25f32, 0.125f32, 0.05f32];
    let mut u_coefs = [0.25f32, 0.125f32, 0.05f32];
    
    let mut step = 0.05f32;
    let mut best_error = evaluate_cqt_lifting_rmse(&p, &u_coefs);
    
    for _pass in 0..10 {
        for i in 0..3 {
            // Perturb p[i]
            for &dir in &[-1.0f32, 1.0f32] {
                let mut cand_p = p;
                cand_p[i] = (p[i] + dir * step).clamp(0.0, 0.49);
                let err = evaluate_cqt_lifting_rmse(&cand_p, &u_coefs);
                if err < best_error {
                    best_error = err;
                    p = cand_p;
                }
            }
            // Perturb u_coefs[i]
            for &dir in &[-1.0f32, 1.0f32] {
                let mut cand_u = u_coefs;
                cand_u[i] = (u_coefs[i] + dir * step).clamp(0.0, 0.49);
                let err = evaluate_cqt_lifting_rmse(&p, &cand_u);
                if err < best_error {
                    best_error = err;
                    u_coefs = cand_u;
                }
            }
        }
        step *= 0.5;
    }
    
    vec![p[0], p[1], p[2], u_coefs[0], u_coefs[1], u_coefs[2]]
}

#[wasm_bindgen]
pub fn wasm_generate_color_chart_4096() -> Vec<u8> {
    ensure_luts();
    let mut output = Vec::with_capacity(4096 * 4096 * 4);
    
    // We generate a 4096 x 4096 RGBA buffer using a 2D Serpentine Layout
    // to prove that the Geodesic Snake walk is 100% continuous in 3D RGB space.
    for r in 0..4096 {
        let block_r = r / 16; // 256 blocks vertically
        let is_row_even = block_r % 2 == 0;
        
        for c in 0..4096 {
            let block_c = c / 16; // 256 blocks horizontally
            
            // Serpentine mapping: even rows go left-to-right, odd rows go right-to-left
            let color_index = if is_row_even {
                block_r * 256 + block_c
            } else {
                block_r * 256 + (255 - block_c)
            } as u16;
            
            output.push(get_color_r(color_index));
            output.push(get_color_g(color_index));
            output.push(get_color_b(color_index));
            output.push(255u8);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_uniqueness() {
        ensure_luts();
        let lut = COLOR_LUT.get().unwrap();
        let mut seen = std::collections::HashSet::new();
        for i in 0..65536 {
            let color = lut[i];
            assert!(seen.insert(color), "Duplicate color found at index {}: {:?}", i, color);
        }
        assert_eq!(seen.len(), 65536, "Not all 65,536 colors are unique!");
    }

    #[test]
    fn test_fast_color_reversal() {
        ensure_luts();
        let lut = COLOR_LUT.get().unwrap();
        
        for i in 0..65536 {
            // Get original signed coefficient
            let original_coef = zigzag_decode(i as u32) as i16;
            
            // Get 24-bit color representation
            let color = lut[i];
            let color_24 = ((color.0 as u32) << 16) | ((color.1 as u32) << 8) | (color.2 as u32);
            
            // Fast reversal algorithm (O(1) Constant Time)
            let r = ((color_24 >> 16) & 0xFF) as usize;
            let g = ((color_24 >> 8) & 0xFF) as usize;
            
            let rev_idx = REVERSE_RG.get().unwrap()[r * 256 + g];
            let decoded_coef = zigzag_decode(rev_idx as u32) as i16;
            
            assert_eq!(original_coef, decoded_coef, "Fast color reversal failed at index {}!", i);
        }
    }

    #[test]
    fn test_v7_pipeline_large() {
        let mut original = vec![0u8; 4000];
        for i in 0..4000 {
            original[i] = (i % 256) as u8;
        }
        let encoded = encode_wavelet_v7_cqt(&original, 128);
        let decoded = decode_wavelet_v7_cqt(&encoded).unwrap();
        assert_eq!(original, decoded, "V7 CQT pipeline failed for large audio!");
    }

    #[test]
    fn test_v8_pipeline_large() {
        let mut original = vec![0u8; 4000];
        for i in 0..4000 {
            original[i] = (i % 256) as u8;
        }
        
        let original_len = original.len() as u32;
        let h = 128;
        let w = calculate_grid_width(original.len(), h);
        let grid_size = w * h;
        
        let num_samples = original.len() / 4;
        let mut mid_input = vec![0i64; grid_size * 4];
        let mut side_input = vec![0i64; grid_size * 4];
        for i in 0..num_samples {
            let offset = i * 4;
            let b0 = original[offset];
            let b1 = original[offset + 1];
            let b2 = original[offset + 2];
            let b3 = original[offset + 3];
            let l_sample = (((b1 as u16) << 8) | (b0 as u16)) as i16;
            let r_sample = (((b3 as u16) << 8) | (b2 as u16)) as i16;
            let (m, s) = lr_to_ms(l_sample, r_sample);
            mid_input[i] = m as i64;
            side_input[i] = s as i64;
        }
        
        let (s_m, d1_m, d2_m, d3_m) = forward_packet_4_band(&mid_input);
        
        let encoded = encode_wavelet_v8_mband(&original, 128);
        
        // Unpack manually to check
        let w_png = (w * 8) as u32;
        let mut s_m_rec = vec![0i64; grid_size];
        let mut d1_m_rec = vec![0i64; grid_size];
        for r in 0..h {
            for c in 0..w {
                let idx = r * w + c;
                let offset_a = 24 + (r * w_png as usize + c * 8) * 4;
                let offset_b = offset_a + 4;
                let sm_u_rec = decode_rg_to_coefficient_raw(encoded[offset_a], encoded[offset_a + 1]);
                let d1m_u_rec = decode_rg_to_coefficient_raw(encoded[offset_b], encoded[offset_b + 1]);
                s_m_rec[idx] = zigzag_decode(sm_u_rec as u32) as i64;
                d1_m_rec[idx] = zigzag_decode(d1m_u_rec as u32) as i64;
            }
        }
        
        for idx in 0..10 {
            println!("COEF {}: original_s={}, decoded_s={} | original_d1={}, decoded_d1={}", 
                     idx, s_m[idx], s_m_rec[idx], d1_m[idx], d1_m_rec[idx]);
        }
        
        let decoded = decode_wavelet_v8_mband(&encoded).unwrap();
        assert_eq!(original, decoded, "V8 pipeline failed for large audio!");
    }

    #[test]
    fn test_lifting_53_i64_bijection() {
        let mut original = vec![0i64; 1024];
        for i in 0..1024 {
            original[i] = i as i64;
        }
        let (s, d) = forward_lifting_53_i64(&original);
        let decoded = inverse_lifting_53_i64(&s, &d, 1024);
        assert_eq!(original, decoded, "1D CDF 5/3 lifting failed!");
    }

    #[test]
    fn test_packet_4_band_bijection() {
        let mut original = vec![0i64; 1024];
        for i in 0..1024 {
            original[i] = i as i64;
        }
        let (s, d1, d2, d3) = forward_packet_4_band(&original);
        let decoded = inverse_packet_4_band(&s, &d1, &d2, &d3, 1024);
        assert_eq!(original, decoded, "4-band packet lifting failed!");
    }

    #[test]
    fn test_rg_raw_bijection() {
        for i in 0..65536 {
            let r = get_color_r(i as u16);
            let g = get_color_g(i as u16);
            let i_rec = decode_rg_to_coefficient_raw(r, g);
            assert_eq!(i as u16, i_rec, "RG raw bijection failed at value {}", i);
        }
    }

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
    fn test_v5_pipeline_large() {
        let mut original = vec![0u8; 4000];
        for i in 0..4000 {
            original[i] = (i % 256) as u8;
        }
        let encoded = encode_wavelet_v5_dyadic_lifting(&original, 256);
        let decoded = decode_wavelet_v5_dyadic_lifting(&encoded).unwrap();
        assert_eq!(original, decoded, "V5 pipeline failed for large audio!");
    }

    #[test]
    fn test_lifting_53_lossless() {
        let mut original = vec![100, -50, 200, 300, -400, 500, 600, 700];
        let cloned = original.clone();
        forward_lifting_53(&mut original, 3);
        inverse_lifting_53(&mut original, 3);
        assert_eq!(cloned, original, "Lifting 5/3 failed!");
    }

    #[test]
    fn test_v6_pipeline_large() {
        let mut original = vec![0u8; 4000];
        for i in 0..4000 {
            original[i] = (i % 256) as u8;
        }
        let encoded = encode_wavelet_v6_reassigned(&original, 256);
        let decoded = decode_wavelet_v6_reassigned(&encoded).unwrap();
        assert_eq!(original, decoded, "V6 pipeline failed for large audio!");
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

        #[test]
        fn test_higher_order_point_analysis() {
        let fs = 44100.0f32;
        let num_samples = 4096;
        let mut pcm_bytes = Vec::with_capacity(num_samples * 4);
        for n in 0..num_samples {
            let t = n as f32 / fs;
            let s = ((2.0 * std::f32::consts::PI * 880.0 * t).sin() * 16000.0) as i16;
            let b0 = (s & 0xFF) as u8;
            let b1 = ((s >> 8) & 0xFF) as u8;
            // Left channel
            pcm_bytes.push(b0);
            pcm_bytes.push(b1);
            // Right channel
            pcm_bytes.push(b0);
            pcm_bytes.push(b1);
        }

        let res = wasm_analyze_higher_order_point(&pcm_bytes, fs, 0.045, 880.0, 1024, 2);
        assert!((res.freq_inst_hz - 880.0).abs() < 5.0, "Expected freq ~880, got {}", res.freq_inst_hz);
        assert!(res.lambda_1 < 0.0, "Expected negative curvature lambda_1, got {}", res.lambda_1);
        assert!(res.anisotropy > 0.5, "Expected high anisotropy, got {}", res.anisotropy);
        }

        #[test]
        fn test_higher_order_streamer_chunk() {
        let fs = 44100.0f32;
        let num_samples = 4096;
        let mut pcm_bytes = Vec::with_capacity(num_samples * 4);
        for n in 0..num_samples {
            let t = n as f32 / fs;
            let s = ((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 16000.0) as i16;
            let b0 = (s & 0xFF) as u8;
            let b1 = ((s >> 8) & 0xFF) as u8;
            pcm_bytes.push(b0);
            pcm_bytes.push(b1);
            pcm_bytes.push(b0);
            pcm_bytes.push(b1);
        }

        let mut streamer = WasmSpectrogramStreamer::new(
            &pcm_bytes,
            512,
            "hann",
            1024,
            2,
            20.0,
            20000.0,
            "higher_order:2:0",
            "ycbcr",
            0.0,
            1.0,
            1.0,
            "log",
            0,
        );
        assert_eq!(streamer.get_higher_order_o(), 2);
        assert_eq!(streamer.get_higher_order_mode(), 0);
        let chunk = streamer.process_chunk(16);
        assert_eq!(chunk.len(), 16 * 512 * 4);
        }
        }
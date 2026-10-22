use std::f32::consts::PI;

pub const M_BITS: u32 = 14;
pub const TOTAL_FREQ: u32 = 1 << M_BITS; // 16384
pub const RANS_L: u32 = 1 << 16; // 65536
pub const LIMIT_BASE: u32 = (RANS_L >> M_BITS) << 8; // 1024

/// Modelo Laplaciano geométrico de 14 bits para rANS
pub struct LaplacianModel {
    pub r: f32,
    pub max_symbol: i32,
    pub num_symbols: usize,
    pub freqs: Vec<u32>,
    pub cum: Vec<u32>,
}

impl LaplacianModel {
    pub fn new(r: f32, max_symbol: i32) -> Self {
        let safe_r = r.clamp(0.05, 0.95);
        let num_symbols = (2 * max_symbol + 1) as usize;
        let mut weights = Vec::with_capacity(num_symbols);
        let mut unnorm_sum = 0.0f32;

        for k in -max_symbol..=max_symbol {
            let w = safe_r.powi(k.abs());
            weights.push(w);
            unnorm_sum += w;
        }

        let mut freqs = Vec::with_capacity(num_symbols);
        let mut sum = 0u32;
        for i in 0..num_symbols {
            let f = ((weights[i] / unnorm_sum) * (TOTAL_FREQ as f32)).round().max(1.0) as u32;
            freqs.push(f);
            sum += f;
        }

        // Ajusta centro para fechar soma exata
        let center_idx = max_symbol as usize;
        if sum != TOTAL_FREQ {
            freqs[center_idx] = (freqs[center_idx] as i32 + (TOTAL_FREQ as i32 - sum as i32)).max(1) as u32;
        }

        let mut cum = Vec::with_capacity(num_symbols + 1);
        cum.push(0);
        for i in 0..num_symbols {
            cum.push(cum[i] + freqs[i]);
        }

        Self {
            r: safe_r,
            max_symbol,
            num_symbols,
            freqs,
            cum,
        }
    }
}

pub struct RansEncoder {
    pub x: u32,
    pub out: Vec<u8>,
}

impl RansEncoder {
    pub fn new() -> Self {
        Self {
            x: RANS_L,
            out: Vec::with_capacity(2048),
        }
    }

    pub fn encode_symbol(&mut self, symbol: i32, model: &LaplacianModel) {
        let clamped = symbol.clamp(-model.max_symbol, model.max_symbol);
        let s = (clamped + model.max_symbol) as usize;
        let f = model.freqs[s];
        let c = model.cum[s];

        while self.x >= LIMIT_BASE * f {
            self.out.push((self.x & 0xff) as u8);
            self.x >>= 8;
        }

        self.x = ((self.x / f) << M_BITS) + (self.x % f) + c;
    }

    pub fn finish(mut self) -> Vec<u8> {
        for i in 0..4 {
            self.out.push(((self.x >> (i * 8)) & 0xff) as u8);
        }
        self.out
    }
}

pub struct RansDecoder<'a> {
    pub x: u32,
    pub buffer: &'a [u8],
    pub ptr: i32,
}

impl<'a> RansDecoder<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        let mut ptr = buffer.len() as i32 - 1;
        let mut x = 0u32;
        for _ in 0..4 {
            let b = if ptr >= 0 { buffer[ptr as usize] as u32 } else { 0 };
            x = (x << 8) | b;
            ptr -= 1;
        }
        Self { x, buffer, ptr }
    }

    pub fn decode_symbol(&mut self, model: &LaplacianModel) -> i32 {
        let slot = self.x & (TOTAL_FREQ - 1);
        let mut s = 0usize;
        while s + 1 < model.cum.len() && model.cum[s + 1] <= slot {
            s += 1;
        }

        let symbol = s as i32 - model.max_symbol;
        let f = model.freqs[s];
        let c = model.cum[s];

        self.x = f * (self.x >> M_BITS) + slot - c;

        while self.x < RANS_L && self.ptr >= 0 {
            let b = self.buffer[self.ptr as usize] as u32;
            self.x = (self.x << 8) | b;
            self.ptr -= 1;
        }

        symbol
    }
}

pub fn quantize_dead_zone(value: f32, delta: f32, eps: f32) -> f32 {
    let diff = delta * (value / (delta + eps)).tanh();
    let res = value - diff;
    if res.abs() < 1e-5 { 0.0 } else { res }
}

pub struct RdoEngine {
    pub frame_size: usize,
    pub num_latents: usize,
    pub basis: Vec<Vec<f32>>,
}

impl RdoEngine {
    pub fn new(frame_size: usize, num_latents: usize) -> Self {
        let mut basis = Vec::with_capacity(num_latents);
        for k in 0..num_latents {
            let mut vec = Vec::with_capacity(frame_size);
            let factor = if k == 0 { (1.0 / frame_size as f32).sqrt() } else { (2.0 / frame_size as f32).sqrt() };
            for n in 0..frame_size {
                vec.push(factor * (PI * k as f32 * (n as f32 + 0.5) / frame_size as f32).cos());
            }
            basis.push(vec);
        }
        Self { frame_size, num_latents, basis }
    }

    pub fn encode_audio(&self, pcm_data: &[u8], lambda: f32, dead_zone: f32, lossless: bool) -> Vec<u8> {
        let is_wav = pcm_data.len() >= 44 && &pcm_data[0..4] == b"RIFF";
        let header_len = if is_wav { 44 } else { 0 };
        let payload = &pcm_data[header_len..];
        let total_samples = payload.len() / 2;

        let mut samples_i16 = Vec::with_capacity(total_samples);
        for i in 0..total_samples {
            let low = payload[i * 2] as u16;
            let high = payload[i * 2 + 1] as u16;
            let val = (low | (high << 8)) as i16;
            samples_i16.push(val);
        }

        let num_frames = (total_samples + self.frame_size - 1) / self.frame_size;
        let mut out = Vec::with_capacity(payload.len());

        // Header do bitstream DRED (16 bytes)
        out.extend_from_slice(b"DRED");
        out.push(if lossless { 1 } else { 0 });
        out.extend_from_slice(&(self.frame_size as u16).to_le_bytes());
        out.push(self.num_latents as u8);
        out.extend_from_slice(&(total_samples as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 4]); // Reservado

        // Copia header WAV para reconstrução idêntica
        if is_wav {
            out.extend_from_slice(&pcm_data[0..44]);
        }

        let latent_model = LaplacianModel::new(0.65, 512);
        let residual_high_model = LaplacianModel::new(0.60, 128);

        for f_idx in 0..num_frames {
            let start = f_idx * self.frame_size;
            let end = (start + self.frame_size).min(total_samples);
            let frame_len = end - start;

            let mut frame_f32 = vec![0.0f32; self.frame_size];
            for (idx, &s) in samples_i16[start..end].iter().enumerate() {
                frame_f32[idx] = s as f32 / 32768.0;
            }

            // 1. Projeção na base ortogonal
            let mut latents = vec![0.0f32; self.num_latents];
            for k in 0..self.num_latents {
                let mut dot = 0.0f32;
                for n in 0..self.frame_size {
                    dot += frame_f32[n] * self.basis[k][n];
                }
                latents[k] = dot;
            }

            // 2. RDO e quantização com dead-zone
            let mut q_latents = vec![0i32; self.num_latents];
            let mut dequant = vec![0.0f32; self.num_latents];

            for k in 0..self.num_latents {
                let z = latents[k];
                let z_dz = quantize_dead_zone(z, dead_zone, 1e-4);
                let q = (z_dz * 40.0).round().clamp(-512.0, 512.0) as i32;
                if q == 0 {
                    continue;
                }

                let cost_bits = 2.0 + (q.abs() as f32) * 0.62;
                let dist_red = z_dz * z_dz;
                let marginal_cost = -dist_red + lambda * cost_bits * 2.0;

                if marginal_cost < 0.0 {
                    q_latents[k] = q;
                    dequant[k] = q as f32 / 40.0;
                }
            }

            // Codifica latentes via rANS
            let mut enc_lat = RansEncoder::new();
            for &q in q_latents.iter().rev() {
                enc_lat.encode_symbol(q, &latent_model);
            }
            let bytes_lat = enc_lat.finish();
            out.extend_from_slice(&(bytes_lat.len() as u16).to_le_bytes());
            out.extend_from_slice(&bytes_lat);

            // Se lossless, calcula reconstrução e codifica resíduo inteiro sem perda
            if lossless {
                let mut recon = vec![0.0f32; self.frame_size];
                for k in 0..self.num_latents {
                    if dequant[k] != 0.0 {
                        for n in 0..self.frame_size {
                            recon[n] += dequant[k] * self.basis[k][n];
                        }
                    }
                }

                let mut high_bytes = Vec::with_capacity(frame_len);
                let mut low_bytes = Vec::with_capacity(frame_len);

                for (idx, &s) in samples_i16[start..end].iter().enumerate() {
                    let pred_i16 = (recon[idx] * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
                    let diff = s.wrapping_sub(pred_i16);
                    let h = (diff >> 8) as i32;
                    let l = (diff & 0xFF) as u8;
                    high_bytes.push(h);
                    low_bytes.push(l);
                }

                // Codifica bytes altos com rANS
                let mut enc_high = RansEncoder::new();
                for &h in high_bytes.iter().rev() {
                    enc_high.encode_symbol(h, &residual_high_model);
                }
                let bytes_high = enc_high.finish();

                out.extend_from_slice(&(bytes_high.len() as u32).to_le_bytes());
                out.extend_from_slice(&bytes_high);
                out.extend_from_slice(&low_bytes);
            }
        }

        out
    }

    pub fn decode_audio(&self, compressed_data: &[u8]) -> Vec<u8> {
        if compressed_data.len() < 16 || &compressed_data[0..4] != b"DRED" {
            return Vec::new();
        }

        let lossless = compressed_data[4] == 1;
        let frame_size = u16::from_le_bytes([compressed_data[5], compressed_data[6]]) as usize;
        let num_latents = compressed_data[7] as usize;
        let total_samples = u32::from_le_bytes([
            compressed_data[8], compressed_data[9], compressed_data[10], compressed_data[11]
        ]) as usize;

        let has_wav = compressed_data.len() > 16 + 44;
        let mut ptr = 16;
        let mut wav_header = Vec::new();

        if has_wav {
            wav_header.extend_from_slice(&compressed_data[ptr..ptr + 44]);
            ptr += 44;
        }

        let num_frames = (total_samples + frame_size - 1) / frame_size;
        let mut recovered_i16 = Vec::with_capacity(total_samples);

        let latent_model = LaplacianModel::new(0.65, 512);
        let residual_high_model = LaplacianModel::new(0.60, 128);

        for f_idx in 0..num_frames {
            let start = f_idx * frame_size;
            let end = (start + frame_size).min(total_samples);
            let frame_len = end - start;

            // Lê tamanho e bytes de latentes
            let lat_len = u16::from_le_bytes([compressed_data[ptr], compressed_data[ptr + 1]]) as usize;
            ptr += 2;
            let lat_bytes = &compressed_data[ptr..ptr + lat_len];
            ptr += lat_len;

            let mut dec_lat = RansDecoder::new(lat_bytes);
            let mut dequant = vec![0.0f32; num_latents];
            for k in 0..num_latents {
                let q = dec_lat.decode_symbol(&latent_model);
                dequant[k] = q as f32 / 40.0;
            }

            // Reconstrução
            let mut recon = vec![0.0f32; frame_size];
            for k in 0..num_latents {
                if dequant[k] != 0.0 {
                    for n in 0..frame_size {
                        recon[n] += dequant[k] * self.basis[k][n];
                    }
                }
            }

            if lossless {
                let high_len = u32::from_le_bytes([
                    compressed_data[ptr], compressed_data[ptr + 1], compressed_data[ptr + 2], compressed_data[ptr + 3]
                ]) as usize;
                ptr += 4;
                let high_bytes = &compressed_data[ptr..ptr + high_len];
                ptr += high_len;

                let low_bytes = &compressed_data[ptr..ptr + frame_len];
                ptr += frame_len;

                let mut dec_high = RansDecoder::new(high_bytes);
                for n in 0..frame_len {
                    let h = dec_high.decode_symbol(&residual_high_model);
                    let l = low_bytes[n];
                    let diff = ((h as i16) << 8) | (l as i16);

                    let pred = (recon[n] * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
                    let s = pred.wrapping_add(diff);
                    recovered_i16.push(s);
                }
            } else {
                for n in 0..frame_len {
                    let pred = (recon[n] * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
                    recovered_i16.push(pred);
                }
            }
        }

        let mut out = Vec::with_capacity(wav_header.len() + recovered_i16.len() * 2);
        out.extend_from_slice(&wav_header);
        for s in recovered_i16 {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    pub fn compute_pareto(&self, pcm_data: &[u8]) -> String {
        let is_wav = pcm_data.len() >= 44 && &pcm_data[0..4] == b"RIFF";
        let header_len = if is_wav { 44 } else { 0 };
        let payload = &pcm_data[header_len..];
        let total_samples = payload.len() / 2;
        if total_samples == 0 {
            return "[]".to_string();
        }

        let mut frame_f32 = vec![0.0f32; self.frame_size];
        let n_eval = total_samples.min(self.frame_size);
        for i in 0..n_eval {
            let low = payload[i * 2] as u16;
            let high = payload[i * 2 + 1] as u16;
            frame_f32[i] = ((low | (high << 8)) as i16) as f32 / 32768.0;
        }

        let lambdas = [0.0001, 0.0005, 0.002, 0.01, 0.05];
        let mut points = Vec::new();

        for &lam in &lambdas {
            let mut latents = vec![0.0f32; self.num_latents];
            for k in 0..self.num_latents {
                let mut dot = 0.0f32;
                for n in 0..self.frame_size {
                    dot += frame_f32[n] * self.basis[k][n];
                }
                latents[k] = dot;
            }

            let mut active_count = 0;
            let mut rate_bits = 0.0f32;
            let mut dequant = vec![0.0f32; self.num_latents];

            for k in 0..self.num_latents {
                let z = latents[k];
                let z_dz = quantize_dead_zone(z, 0.08, 1e-4);
                let q = (z_dz * 40.0).round() as i32;
                if q == 0 { continue; }

                let cost_bits = 2.0 + (q.abs() as f32) * 0.62;
                let dist_red = z_dz * z_dz;
                let marginal_cost = -dist_red + lam * cost_bits * 2.0;

                if marginal_cost < 0.0 {
                    dequant[k] = q as f32 / 40.0;
                    active_count += 1;
                    rate_bits += cost_bits;
                }
            }

            let mut recon = vec![0.0f32; self.frame_size];
            for k in 0..self.num_latents {
                if dequant[k] != 0.0 {
                    for n in 0..self.frame_size {
                        recon[n] += dequant[k] * self.basis[k][n];
                    }
                }
            }

            let mut sse = 0.0f32;
            for n in 0..self.frame_size {
                let diff = frame_f32[n] - recon[n];
                sse += diff * diff;
            }
            let rmse = (sse / self.frame_size as f32).sqrt();
            let rate_kbps = (rate_bits * (48000.0 / self.frame_size as f32)) / 1000.0;

            points.push(format!(
                "{{\"lambda\":{},\"rate_kbps\":{:.2},\"rmse\":{:.6},\"active_dims\":{}}}",
                lam, rate_kbps, rmse, active_count
            ));
        }

        format!("[{}]", points.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdo_codec_roundtrip() {
        let engine = RdoEngine::new(512, 64);
        let mut pcm = vec![0u8; 44 + 512 * 2];
        pcm[0..4].copy_from_slice(b"RIFF");
        for i in 0..512 {
            let s = ((i as f32 * 0.1).sin() * 15000.0) as i16;
            let bytes = s.to_le_bytes();
            pcm[44 + i * 2] = bytes[0];
            pcm[44 + i * 2 + 1] = bytes[1];
        }

        let compressed = engine.encode_audio(&pcm, 0.005, 0.08, true);
        let decompressed = engine.decode_audio(&compressed);

        assert_eq!(pcm.len(), decompressed.len());
        assert_eq!(pcm, decompressed);
    }
}


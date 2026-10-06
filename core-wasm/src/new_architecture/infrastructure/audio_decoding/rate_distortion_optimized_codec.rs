//! Codec de Jato com Otimização Taxa-Distorção (RDO-Jet)
//!
//! Protocolo binário RDOJ com quantização por zona morta (dead-zone)
//! para transmissão compacta de coeficientes e streaming de LOD.

pub struct RateDistortionOptimizedCodec;

impl RateDistortionOptimizedCodec {
    /// Comprime um vetor contínuo de amostras em pacote binário compacto RDOJ
    pub fn compress_frame(samples: &[f32], rate_distortion_lambda: f32) -> Vec<u8> {
        let delta = rate_distortion_lambda.max(0.001);
        let mut sparse_entries = Vec::new();

        for (idx, &val) in samples.iter().enumerate() {
            let abs_v = val.abs();
            if abs_v > delta * 0.5 {
                let sign = if val >= 0.0 { 1.0f32 } else { -1.0f32 };
                let q = sign * ((abs_v - delta * 0.5) / delta + 1.0).floor();
                let q_i16 = q.clamp(-32767.0, 32767.0) as i16;
                if q_i16 != 0 {
                    sparse_entries.push((idx as u32, q_i16));
                }
            }
        }

        let num_entries = sparse_entries.len() as u32;
        let mut packet = Vec::with_capacity(16 + sparse_entries.len() * 6);
        // Header: "RDOJ" (4B) + original_len (4B) + num_entries (4B) + delta (4B)
        packet.extend_from_slice(b"RDOJ");
        packet.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        packet.extend_from_slice(&num_entries.to_le_bytes());
        packet.extend_from_slice(&delta.to_le_bytes());

        for (idx, q_val) in sparse_entries {
            packet.extend_from_slice(&idx.to_le_bytes());
            packet.extend_from_slice(&q_val.to_le_bytes());
        }

        packet
    }

    /// Descomprime um pacote binário RDOJ para o vetor de amostras reconstruído
    pub fn decompress_frame(packet: &[u8]) -> Vec<f32> {
        if packet.len() < 16 || &packet[0..4] != b"RDOJ" {
            return Vec::new();
        }

        let original_len = u32::from_le_bytes([packet[4], packet[5], packet[6], packet[7]]) as usize;
        let num_entries = u32::from_le_bytes([packet[8], packet[9], packet[10], packet[11]]) as usize;
        let delta = f32::from_le_bytes([packet[12], packet[13], packet[14], packet[15]]);

        let mut output = vec![0.0f32; original_len];
        let mut offset = 16;

        for _ in 0..num_entries {
            if offset + 6 > packet.len() {
                break;
            }
            let idx = u32::from_le_bytes([packet[offset], packet[offset + 1], packet[offset + 2], packet[offset + 3]]) as usize;
            let q_val = i16::from_le_bytes([packet[offset + 4], packet[offset + 5]]) as f32;
            if idx < original_len {
                output[idx] = q_val * delta;
            }
            offset += 6;
        }

        output
    }
}

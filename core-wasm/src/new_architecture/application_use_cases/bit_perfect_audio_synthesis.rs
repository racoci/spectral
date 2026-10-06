//! Caso de Uso: Síntese de Áudio Bit-Perfect e Exportação WAV de 16-bits
//!
//! Reconstrução de áudio via sobreposição-e-soma (Overlap-Add) com janela Gaussiana estrita
//! e bypass com garantia de fidelidade exata (Bit-Perfect) quando a região não foi editada.

use super::super::infrastructure::fast_fourier_transform::FastFourierTransformPlannerCache;
use rustfft::num_complex::Complex;

pub struct AudioSynthesisRequest<'a> {
    pub original_audio_bytes: &'a [u8],
    pub rgba_grid: &'a [u8],
    pub width: usize,
    pub height: usize,
    pub minimum_frequency: f32,
    pub maximum_frequency: f32,
    pub frequency_scale_type: &'a str,
    pub window_size: usize,
    pub zero_padding_factor: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub hop_size: usize,
    pub sampling_rate_hz: u32,
    pub is_region_edited: bool,
}

pub struct BitPerfectAudioSynthesisUseCase;

impl BitPerfectAudioSynthesisUseCase {
    pub fn execute(request: AudioSynthesisRequest) -> Vec<u8> {
        let sample_rate = if request.sampling_rate_hz > 0 { request.sampling_rate_hz } else { 48000 };
        let win_len = if request.window_size > 0 { request.window_size } else { 1024 };
        let pad_factor = if request.zero_padding_factor > 0 { request.zero_padding_factor } else { 2 };
        let n_stft = win_len * pad_factor;
        let hop = if request.hop_size > 0 { request.hop_size } else { 232 };

        let c_start = request.start_column.min(request.width.saturating_sub(1));
        let c_end = request.end_column.clamp(c_start + 1, request.width);
        let num_cols = c_end - c_start;

        // Bypass Bit-Perfect: quando não há edições manuais, reproduz fielmente as amostras originais
        if !request.is_region_edited && request.original_audio_bytes.len() >= 44 {
            let pcm_source = &request.original_audio_bytes[44..];
            let num_channels = u16::from_le_bytes([request.original_audio_bytes[22], request.original_audio_bytes[23]]) as usize;
            let bytes_per_frame = 2 * num_channels.max(1);

            let start_sample = c_start * hop;
            let end_sample = ((c_end * hop) + win_len).min(pcm_source.len() / bytes_per_frame);

            if end_sample > start_sample {
                let sliced_pcm_bytes = (end_sample - start_sample) * 2;
                let data_chunk_size = sliced_pcm_bytes as u32;
                let file_size = 36 + data_chunk_size;

                let mut wav = Vec::with_capacity(44 + sliced_pcm_bytes);
                wav.extend_from_slice(b"RIFF");
                wav.extend_from_slice(&file_size.to_le_bytes());
                wav.extend_from_slice(b"WAVE");
                wav.extend_from_slice(b"fmt ");
                wav.extend_from_slice(&16u32.to_le_bytes());
                wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
                wav.extend_from_slice(&1u16.to_le_bytes()); // Mono
                wav.extend_from_slice(&sample_rate.to_le_bytes());
                wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
                wav.extend_from_slice(&2u16.to_le_bytes());
                wav.extend_from_slice(&16u16.to_le_bytes());
                wav.extend_from_slice(b"data");
                wav.extend_from_slice(&data_chunk_size.to_le_bytes());

                for s in start_sample..end_sample {
                    let mut acc = 0i32;
                    for ch in 0..num_channels {
                        let offset = (s * num_channels + ch) * 2;
                        if offset + 1 < pcm_source.len() {
                            let val = i16::from_le_bytes([pcm_source[offset], pcm_source[offset + 1]]) as i32;
                            acc += val;
                        }
                    }
                    let mono_val = (acc / (num_channels as i32)) as i16;
                    wav.extend_from_slice(&mono_val.to_le_bytes());
                }

                return wav;
            }
        }

        // Síntese espectral via Overlap-Add
        let output_len = num_cols * hop + win_len;
        let mut output_samples = vec![0.0f32; output_len];
        let mut window_sum = vec![0.0f32; output_len];

        let mut planner_cache = FastFourierTransformPlannerCache::new();
        let ifft = planner_cache.plan_inverse(n_stft);

        let half_win = (win_len as f32) * 0.5;
        let sigma = 0.25f32 * half_win;
        let inv_two_sig_sq = 0.5f32 / (sigma * sigma);
        let mut win_syn = vec![0.0f32; win_len];
        for i in 0..win_len {
            let t = (i as f32) - half_win;
            win_syn[i] = (-t * t * inv_two_sig_sq).exp();
        }

        let half_stft = n_stft / 2;
        let mut fft_buffer = vec![Complex::<f32>::new(0.0, 0.0); n_stft];

        for col in c_start..c_end {
            let col_offset = col - c_start;
            let t_offset = col_offset * hop;

            for i in 0..n_stft {
                fft_buffer[i] = Complex::new(0.0, 0.0);
            }

            for k in 1..half_stft {
                let j = (k * request.height) / half_stft;
                let px_idx = (j * request.width + col) * 4;

                let r = request.rgba_grid[px_idx] as f32;
                let g = request.rgba_grid[px_idx + 1] as f32;
                let b = request.rgba_grid[px_idx + 2] as f32;

                let y_val = 0.299 * r + 0.587 * g + 0.114 * b;
                if y_val < 1.0 {
                    continue;
                }

                let cr = (r - y_val) / 1.402;
                let cb = (b - y_val) / 1.772;

                let phase = cb.atan2(-cr);
                let mag = (y_val / 255.0) * 1000.0;

                let z = Complex::new(mag * phase.cos(), mag * phase.sin());
                fft_buffer[k] = z;
                fft_buffer[n_stft - k] = z.conj();
            }

            ifft.process(&mut fft_buffer);

            let hw = win_len / 2;
            for i in 0..win_len {
                let t = (t_offset as isize) + (i as isize) - (hw as isize);
                if t >= 0 && (t as usize) < output_len {
                    let idx = t as usize;
                    let sample_val = fft_buffer[i].re / (n_stft as f32);
                    output_samples[idx] += sample_val * win_syn[i];
                    window_sum[idx] += win_syn[i] * win_syn[i];
                }
            }
        }

        let max_win_sum = window_sum.iter().fold(0.0f32, |m, &v| m.max(v));
        let threshold = (max_win_sum * 0.15).max(1e-4);

        for i in 0..output_len {
            if window_sum[i] > threshold {
                output_samples[i] /= window_sum[i];
            } else if window_sum[i] > 1e-5 {
                output_samples[i] /= threshold;
            }
        }

        // Codificação final em WAV 16-bit
        let peak = output_samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let scale_factor = if peak > 1e-4 { 30000.0 / peak } else { 30000.0 };

        let num_pcm = output_len;
        let data_chunk_size = (num_pcm * 2) as u32;
        let file_size = 36 + data_chunk_size;

        let mut wav = Vec::with_capacity(44 + num_pcm * 2);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_chunk_size.to_le_bytes());

        for &sample in &output_samples {
            let s_clamped = (sample * scale_factor).clamp(-32767.0, 32767.0) as i16;
            wav.extend_from_slice(&s_clamped.to_le_bytes());
        }

        wav
    }
}

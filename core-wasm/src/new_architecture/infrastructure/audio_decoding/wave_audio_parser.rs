//! Leitura e Validação de Arquivos de Áudio PCM WAV
//!
//! Parser estrito e de zero cópia do cabeçalho RIFF WAV (PCM 16-bit).

pub struct WaveAudioInformation {
    pub channels: usize,
    pub sampling_rate_hz: u32,
    pub samples: Vec<f32>,
}

pub struct WaveAudioParser;

impl WaveAudioParser {
    pub fn parse_pcm_16bit(data: &[u8]) -> Option<WaveAudioInformation> {
        if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
            return None;
        }

        let num_channels = u16::from_le_bytes([data[22], data[23]]) as usize;
        let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        let bits_per_sample = u16::from_le_bytes([data[34], data[35]]) as usize;

        if bits_per_sample != 16 || num_channels == 0 {
            return None;
        }

        let pcm_bytes = &data[44..];
        let bytes_per_sample = 2 * num_channels;
        let total_samples = pcm_bytes.len() / bytes_per_sample;

        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let mut acc = 0.0f32;
            for ch in 0..num_channels {
                let offset = (i * num_channels + ch) * 2;
                if offset + 1 < pcm_bytes.len() {
                    let s16 = i16::from_le_bytes([pcm_bytes[offset], pcm_bytes[offset + 1]]) as f32;
                    acc += s16 / 32768.0;
                }
            }
            samples.push(acc / (num_channels as f32));
        }

        Some(WaveAudioInformation {
            channels: num_channels,
            sampling_rate_hz: sample_rate,
            samples,
        })
    }
}

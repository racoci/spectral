import type { ScalarFieldModel } from './taylor_jet_math';
import { evaluateBaseFunction } from './taylor_jet_math';

export interface OrbitOptions {
  sampleRate?: number;
  durationSeconds?: number;
  fundamentalHz?: number;
  orbitRadiusX?: number;
  orbitRadiusY?: number;
}

export interface SpectralOptions {
  sampleRate?: number;
  durationSeconds?: number;
  fundamentalHz?: number;
  numHarmonics?: number;
}

/**
 * Avalia o valor do campo escalar no ponto cartesiano (x, y) incluindo deformações de brushes.
 */
function sampleFieldAt(field: ScalarFieldModel, x: number, y: number): number {
  let val = evaluateBaseFunction(
    field.baseFunction.type,
    x,
    y,
    field.baseFunction.gain,
    field.baseFunction.bias,
    field.baseFunction.sigma
  );

  // Aplica contribuição de cada camada de brush
  for (const b of field.brushLayers) {
    for (let pIdx = 0; pIdx < b.points.length; pIdx++) {
      const pt = b.points[pIdx];
      const weight = b.profile1D[pIdx] !== undefined ? b.profile1D[pIdx] : 1.0;
      const d2 = (x - pt.x) * (x - pt.x) + (y - pt.y) * (y - pt.y);
      const s2 = 2.0 * b.radius * b.radius;

      if (d2 <= b.radius * b.radius * 4.0) {
        const delta = Math.exp(-d2 / s2) * b.intensity * weight;
        if (b.operator === 'sub') {
          val -= delta;
        } else {
          val += delta;
        }
      }
    }
  }

  return val;
}

/**
 * Modo A: Sintetiza áudio via varredura de órbita periódica fechada no campo f(x, y).
 */
export function synthesizeOrbitWavetable(
  field: ScalarFieldModel,
  options?: OrbitOptions
): Float32Array {
  const sampleRate = options?.sampleRate || 48000;
  const duration = options?.durationSeconds || 1.0;
  const f0 = options?.fundamentalHz || 220.0;
  const rx = options?.orbitRadiusX || 0.8;
  const ry = options?.orbitRadiusY || 0.8;

  const totalSamples = Math.floor(sampleRate * duration);
  const output = new Float32Array(totalSamples);
  const anc = field.anchor;

  // Envelope trapezoidal suave de ataque e decaimento
  const fadeLen = Math.floor(0.01 * sampleRate); // 10ms fade

  for (let n = 0; n < totalSamples; n++) {
    const t = n / sampleRate;
    const theta = 2.0 * Math.PI * f0 * t;

    const px = anc.x + rx * Math.cos(theta);
    const py = anc.y + ry * Math.sin(theta);

    let sampleVal = sampleFieldAt(field, px, py);

    // Fade-in / Fade-out
    if (n < fadeLen) {
      sampleVal *= n / fadeLen;
    } else if (n > totalSamples - fadeLen) {
      sampleVal *= (totalSamples - n) / fadeLen;
    }

    output[n] = sampleVal;
  }

  // Normalização de pico suave para garantir conformidade
  normalizePeak(output, 0.95);
  return output;
}

/**
 * Modo B: Sintetiza áudio interpretando o campo como superfície de magnitude espectral DDSP.
 */
export function synthesizeSpectralSurface(
  field: ScalarFieldModel,
  options?: SpectralOptions
): Float32Array {
  const sampleRate = options?.sampleRate || 48000;
  const duration = options?.durationSeconds || 1.0;
  const f0 = options?.fundamentalHz || 220.0;
  const K = options?.numHarmonics || 8;

  const totalSamples = Math.floor(sampleRate * duration);
  const output = new Float32Array(totalSamples);
  const dom = field.domain;

  const fadeLen = Math.floor(0.01 * sampleRate);

  for (let n = 0; n < totalSamples; n++) {
    const t = n / sampleRate;
    const normT = n / totalSamples;
    const mapX = dom.xMin + normT * (dom.xMax - dom.xMin);

    let sampleVal = 0.0;

    for (let k = 1; k <= K; k++) {
      const normK = (k - 1) / Math.max(1, K - 1);
      const mapY = dom.yMin + normK * (dom.yMax - dom.yMin);

      // Amplitude proporcional ao valor positivo do campo
      const rawAmp = sampleFieldAt(field, mapX, mapY);
      const amp = Math.max(0.0, rawAmp) * (1.0 / Math.sqrt(k));

      sampleVal += amp * Math.sin(2.0 * Math.PI * k * f0 * t);
    }

    if (n < fadeLen) {
      sampleVal *= n / fadeLen;
    } else if (n > totalSamples - fadeLen) {
      sampleVal *= (totalSamples - n) / fadeLen;
    }

    output[n] = sampleVal;
  }

  normalizePeak(output, 0.95);
  return output;
}

function normalizePeak(samples: Float32Array, targetPeak = 0.95) {
  let peak = 1e-9;
  for (let i = 0; i < samples.length; i++) {
    const abs = Math.abs(samples[i]);
    if (abs > peak) peak = abs;
  }

  if (peak > targetPeak) {
    const scale = targetPeak / peak;
    for (let i = 0; i < samples.length; i++) {
      samples[i] *= scale;
    }
  }
}

/**
 * Codifica o sinal PCM Float32 em um arquivo WAV de 16-bit mono padrão RIFF.
 */
export function encodeScalarFieldWav(samples: Float32Array, sampleRate: number): Uint8Array {
  const n = samples.length;
  const buffer = new Uint8Array(44 + n * 2);

  // RIFF chunk
  buffer[0] = 0x52; // 'R'
  buffer[1] = 0x49; // 'I'
  buffer[2] = 0x46; // 'F'
  buffer[3] = 0x46; // 'F'
  const chunkSize = 36 + n * 2;
  buffer[4] = chunkSize & 0xff;
  buffer[5] = (chunkSize >> 8) & 0xff;
  buffer[6] = (chunkSize >> 16) & 0xff;
  buffer[7] = (chunkSize >> 24) & 0xff;

  // WAVE
  buffer[8] = 0x57; // 'W'
  buffer[9] = 0x41; // 'A'
  buffer[10] = 0x56; // 'V'
  buffer[11] = 0x45; // 'E'

  // fmt subchunk
  buffer[12] = 0x66; // 'f'
  buffer[13] = 0x6d; // 'm'
  buffer[14] = 0x74; // 't'
  buffer[15] = 0x20; // ' '
  buffer[16] = 16;
  buffer[17] = 0;
  buffer[18] = 0;
  buffer[19] = 0;

  buffer[20] = 1; // AudioFormat (1 = PCM)
  buffer[21] = 0;
  buffer[22] = 1; // NumChannels (1 = Mono)
  buffer[23] = 0;

  // SampleRate
  buffer[24] = sampleRate & 0xff;
  buffer[25] = (sampleRate >> 8) & 0xff;
  buffer[26] = (sampleRate >> 16) & 0xff;
  buffer[27] = (sampleRate >> 24) & 0xff;

  const byteRate = sampleRate * 1 * 2;
  buffer[28] = byteRate & 0xff;
  buffer[29] = (byteRate >> 8) & 0xff;
  buffer[30] = (byteRate >> 16) & 0xff;
  buffer[31] = (byteRate >> 24) & 0xff;

  buffer[32] = 2; // BlockAlign
  buffer[33] = 0;
  buffer[34] = 16; // BitsPerSample
  buffer[35] = 0;

  // data subchunk
  buffer[36] = 0x64; // 'd'
  buffer[37] = 0x61; // 'a'
  buffer[38] = 0x74; // 't'
  buffer[39] = 0x61; // 'a'
  const dataSize = n * 2;
  buffer[40] = dataSize & 0xff;
  buffer[41] = (dataSize >> 8) & 0xff;
  buffer[42] = (dataSize >> 16) & 0xff;
  buffer[43] = (dataSize >> 24) & 0xff;

  for (let i = 0; i < n; i++) {
    const s = Math.max(-1.0, Math.min(1.0, samples[i]));
    const s16 = Math.round(s * 32767);
    const offset = 44 + i * 2;
    buffer[offset] = s16 & 0xff;
    buffer[offset + 1] = (s16 >> 8) & 0xff;
  }

  return buffer;
}

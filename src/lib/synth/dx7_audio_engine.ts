import type { Dx7Patch, Dx7Operator } from './dx7_parser';
import { dx7RateToSeconds, dx7LevelToLinear } from './dx7_parser';

/**
 * Topologia de roteamento dos 32 algoritmos clássicos do Yamaha DX7.
 */
export interface AlgorithmRouting {
  carriers: number[]; // IDs dos operadores que são portadoras diretas (1 a 6)
  modulators: Record<number, number[]>; // Mapa: opId -> lista de operadores que modulam este operador
  feedbackOp: number; // ID do operador com loop de feedback
}

/**
 * Tabela canônica de algoritmos do DX7 (1 a 32).
 */
export const DX7_ALGORITHMS: Record<number, AlgorithmRouting> = {
  1: {
    carriers: [1, 3],
    modulators: { 1: [2], 2: [], 3: [4], 4: [5], 5: [6], 6: [] },
    feedbackOp: 6
  },
  2: {
    carriers: [1, 3],
    modulators: { 1: [2], 2: [6], 3: [4], 4: [5], 5: [], 6: [] },
    feedbackOp: 2
  },
  3: {
    carriers: [1, 4],
    modulators: { 1: [2, 3], 2: [], 3: [], 4: [5], 5: [6], 6: [] },
    feedbackOp: 6
  },
  4: {
    carriers: [1, 4],
    modulators: { 1: [2], 2: [], 3: [], 4: [5, 6], 5: [], 6: [] },
    feedbackOp: 6
  },
  5: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  6: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 5
  },
  7: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [6], 5: [], 6: [] },
    feedbackOp: 6
  },
  8: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 4
  },
  9: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [6], 3: [4], 4: [], 5: [], 6: [] },
    feedbackOp: 2
  },
  10: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4, 6], 4: [], 5: [], 6: [] },
    feedbackOp: 3
  },
  11: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  12: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 2
  },
  13: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  14: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  15: {
    carriers: [1, 3, 5],
    modulators: { 1: [2], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 2
  },
  16: {
    carriers: [1],
    modulators: { 1: [2, 3, 4], 2: [], 3: [], 4: [5], 5: [6], 6: [] },
    feedbackOp: 6
  },
  17: {
    carriers: [1],
    modulators: { 1: [2, 3, 4], 2: [6], 3: [], 4: [], 5: [], 6: [] },
    feedbackOp: 2
  },
  18: {
    carriers: [1],
    modulators: { 1: [2, 3], 2: [], 3: [4, 5], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  19: {
    carriers: [1, 4, 5],
    modulators: { 1: [2, 3], 2: [], 3: [6], 4: [], 5: [], 6: [] },
    feedbackOp: 6
  },
  20: {
    carriers: [1, 2, 4],
    modulators: { 1: [], 2: [3], 3: [4], 4: [5, 6], 5: [], 6: [] },
    feedbackOp: 3
  },
  21: {
    carriers: [1, 2, 4],
    modulators: { 1: [], 2: [3], 3: [], 4: [5, 6], 5: [], 6: [] },
    feedbackOp: 3
  },
  22: {
    carriers: [1, 3, 4, 5],
    modulators: { 1: [2], 2: [], 3: [], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  23: {
    carriers: [1, 3, 4, 5],
    modulators: { 1: [2], 2: [6], 3: [], 4: [], 5: [], 6: [] },
    feedbackOp: 6
  },
  24: {
    carriers: [1, 3, 4, 5],
    modulators: { 1: [2], 2: [], 3: [], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  25: {
    carriers: [1, 3, 4, 5],
    modulators: { 1: [2], 2: [], 3: [], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  26: {
    carriers: [1, 2, 3, 5],
    modulators: { 1: [], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 6
  },
  27: {
    carriers: [1, 2, 3, 5],
    modulators: { 1: [], 2: [], 3: [4], 4: [], 5: [6], 6: [] },
    feedbackOp: 3
  },
  28: {
    carriers: [1, 2, 3, 6],
    modulators: { 1: [], 2: [], 3: [], 4: [], 5: [], 6: [5] },
    feedbackOp: 5
  },
  29: {
    carriers: [1, 2, 3, 4],
    modulators: { 1: [], 2: [], 3: [], 4: [5, 6], 5: [], 6: [] },
    feedbackOp: 6
  },
  30: {
    carriers: [1, 2, 3, 4],
    modulators: { 1: [], 2: [], 3: [], 4: [5], 5: [6], 6: [] },
    feedbackOp: 5
  },
  31: {
    carriers: [1, 2, 3, 4, 5],
    modulators: { 1: [], 2: [], 3: [], 4: [], 5: [], 6: [] },
    feedbackOp: 6
  },
  32: {
    // 32: Síntese puramente aditiva (todas as 6 são portadoras paralelas!)
    carriers: [1, 2, 3, 4, 5, 6],
    modulators: { 1: [], 2: [], 3: [], 4: [], 5: [], 6: [] },
    feedbackOp: 6
  }
};

/**
 * Avalia o envelope de 4 taxas e 4 níveis do DX7 para um tempo contínuo t (em segundos).
 */
export function evaluateDx7Envelope(op: Dx7Operator, t: number, noteDuration: number): number {
  const t1 = dx7RateToSeconds(op.rates[0]);
  const t2 = dx7RateToSeconds(op.rates[1]);
  const t3 = dx7RateToSeconds(op.rates[2]);
  const t4 = dx7RateToSeconds(op.rates[3]);

  const l0 = 0.0;
  const l1 = dx7LevelToLinear(op.levels[0]);
  const l2 = dx7LevelToLinear(op.levels[1]);
  const l3 = dx7LevelToLinear(op.levels[2]);
  const l4 = dx7LevelToLinear(op.levels[3]);

  // Se estiver dentro da duração da nota
  if (t < noteDuration) {
    if (t < t1) {
      // Estágio 1 (Ataque): 0 -> L1
      const progress = t / Math.max(1e-4, t1);
      return l0 + (l1 - l0) * progress;
    } else if (t < t1 + t2) {
      // Estágio 2 (Decay 1): L1 -> L2
      const progress = (t - t1) / Math.max(1e-4, t2);
      return l1 + (l2 - l1) * progress;
    } else if (t < t1 + t2 + t3) {
      // Estágio 3 (Decay 2): L2 -> L3
      const progress = (t - t1 - t2) / Math.max(1e-4, t3);
      return l2 + (l3 - l2) * progress;
    } else {
      // Estágio Sustain: mantém L3
      return l3;
    }
  } else {
    // Estágio 4 (Release): L3 -> L4
    const relT = t - noteDuration;
    if (relT < t4) {
      const progress = relT / Math.max(1e-4, t4);
      return l3 + (l4 - l3) * progress;
    }
    return l4;
  }
}

/**
 * Converte razão harmônica do operador DX7 para multiplicador float.
 */
export function getOperatorFrequencyMultiplier(op: Dx7Operator): number {
  if (op.mode === 'fixed') {
    // Modo fixo em Hz
    return 1.0;
  }
  // Coarse 0 = 0.5x, 1 = 1.0x, etc.
  const coarse = op.freqCoarse === 0 ? 0.5 : op.freqCoarse;
  const fine = op.freqFine / 100.0;
  const detuneOffset = op.detune * 0.001;
  return coarse * (1.0 + fine) + detuneOffset;
}

export interface SynthOptions {
  sampleRate?: number;
  duration?: number; // segundos totais
  noteDuration?: number; // tempo de sustentação antes do release
  fundamentalHz?: number; // frequência base (ex: 440 Hz)
  velocity?: number; // 0.0 a 1.0
}

/**
 * Sintetizador FM Yamaha DX7 de 6 Operadores completo em TypeScript.
 * Retorna amostras PCM Float32Array normalizadas [-1.0, 1.0].
 */
export function synthesizeDx7Patch(patch: Dx7Patch, options?: SynthOptions): Float32Array {
  const sampleRate = options?.sampleRate || 48000;
  const totalDuration = options?.duration || 2.0;
  const noteDuration = options?.noteDuration || Math.max(0.1, totalDuration - 0.4);
  const fundamental = options?.fundamentalHz || 440.0;
  const velocity = options?.velocity !== undefined ? options.velocity : 0.85;

  const totalSamples = Math.floor(sampleRate * totalDuration);
  const output = new Float32Array(totalSamples);

  const routing = DX7_ALGORITHMS[patch.algorithm] || DX7_ALGORITHMS[1];
  const feedbackOpId = routing.feedbackOp;
  // Escala canônica do DX7: dobra a cada nível de 1 a 7
  const feedbackScale = patch.feedback > 0 ? Math.pow(2.0, patch.feedback - 7) * Math.PI : 0.0;

  // Acumuladores de fase para os 6 operadores
  const phases = new Float64Array(7); // índices 1 a 6
  // Amostras anteriores para o feedback
  let fbY1 = 0.0;
  let fbY2 = 0.0;

  // Pré-computa multiplicadores de frequência dos operadores
  const opMults = new Float64Array(7);
  const opOutputGains = new Float32Array(7);

  for (const op of patch.operators) {
    opMults[op.id] = getOperatorFrequencyMultiplier(op);
    // Ganho do operador escalado por sensibilidade de velocidade
    const baseLevel = dx7LevelToLinear(op.outputLevel);
    const velFactor = 1.0 - (1.0 - velocity) * (op.velSens / 7.0);
    opOutputGains[op.id] = baseLevel * Math.max(0.0, velFactor);
  }

  // Amostras calculadas no instante atual para cada operador (1..6)
  const opOutputs = new Float64Array(7);

  // Ordem de avaliação topológica (moduladores primeiro, portadoras por último)
  // No DX7, processar de Op 6 para baixo (6 down to 1) cobre quase todas as dependências causais
  const evalOrder = [6, 5, 4, 3, 2, 1];

  for (let n = 0; n < totalSamples; n++) {
    const t = n / sampleRate;

    // Calcula saída instantânea de cada operador
    for (const opId of evalOrder) {
      const op = patch.operators.find((o) => o.id === opId);
      if (!op) continue;

      const f_hz = fundamental * opMults[opId];
      const omega = (2.0 * Math.PI * f_hz) / sampleRate;
      phases[opId] += omega;

      // Envelope atual do operador
      const env = evaluateDx7Envelope(op, t, noteDuration);
      const amp = env * opOutputGains[opId];

      // Modulação de fase vinda de outros operadores
      let modPhase = 0.0;
      const modSourceIds = routing.modulators[opId] || [];
      for (const mId of modSourceIds) {
        // Índice de modulação proporcional à amplitude do modulador
        modPhase += opOutputs[mId] * 4.0;
      }

      // Adiciona feedback se este for o operador com loop
      if (opId === feedbackOpId && feedbackScale > 0) {
        const fbAvg = (fbY1 + fbY2) * 0.5;
        modPhase += fbAvg * feedbackScale;
      }

      // Calcula o seno do operador
      const rawSine = Math.sin(phases[opId] + modPhase);
      const currentOpVal = rawSine * amp;
      opOutputs[opId] = currentOpVal;

      if (opId === feedbackOpId) {
        fbY2 = fbY1;
        fbY1 = rawSine;
      }
    }

    // Soma a contribuição de todas as portadoras designadas pelo algoritmo
    let sampleVal = 0.0;
    for (const carrierId of routing.carriers) {
      sampleVal += opOutputs[carrierId];
    }

    output[n] = sampleVal;
  }

  // Normalização de pico suave para evitar clipping
  let peak = 1e-9;
  for (let i = 0; i < totalSamples; i++) {
    const abs = Math.abs(output[i]);
    if (abs > peak) peak = abs;
  }

  const targetPeak = 0.95;
  if (peak > targetPeak) {
    const scale = targetPeak / peak;
    for (let i = 0; i < totalSamples; i++) {
      output[i] *= scale;
    }
  }

  return output;
}

/**
 * Codifica o sinal PCM Float32 em um arquivo WAV de 16-bit mono padrão RIFF.
 */
export function encodeWav16Mono(samples: Float32Array, sampleRate: number): Uint8Array {
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
  buffer[16] = 16; // Subchunk size (16 for PCM)
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

  // ByteRate = SampleRate * NumChannels * BitsPerSample/8
  const byteRate = sampleRate * 1 * 2;
  buffer[28] = byteRate & 0xff;
  buffer[29] = (byteRate >> 8) & 0xff;
  buffer[30] = (byteRate >> 16) & 0xff;
  buffer[31] = (byteRate >> 24) & 0xff;

  // BlockAlign = NumChannels * BitsPerSample/8
  buffer[32] = 2;
  buffer[33] = 0;
  // BitsPerSample
  buffer[34] = 16;
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

  // PCM samples em 16-bit little-endian
  for (let i = 0; i < n; i++) {
    const s = Math.max(-1.0, Math.min(1.0, samples[i]));
    const s16 = Math.round(s * 32767);
    const offset = 44 + i * 2;
    buffer[offset] = s16 & 0xff;
    buffer[offset + 1] = (s16 >> 8) & 0xff;
  }

  return buffer;
}

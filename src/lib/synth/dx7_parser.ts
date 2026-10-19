/**
 * DX7 SysEx Parser em TypeScript puro.
 * Decodifica bancos e vozes binárias (.syx) do Yamaha DX7 / Dexed.
 */

export interface Dx7Operator {
  id: number; // 1 a 6
  rates: [number, number, number, number]; // 0 a 99 (R1, R2, R3, R4)
  levels: [number, number, number, number]; // 0 a 99 (L1, L2, L3, L4)
  klsBreakPoint: number;
  klsLeftDepth: number;
  klsRightDepth: number;
  klsCurves: number;
  rateScaling: number;
  amSens: number;
  velSens: number;
  outputLevel: number; // 0 a 99
  mode: 'ratio' | 'fixed';
  freqCoarse: number;
  freqFine: number;
  detune: number; // -7 a +7
}

export interface Dx7PitchEnvelope {
  rates: [number, number, number, number];
  levels: [number, number, number, number];
}

export interface Dx7Patch {
  name: string;
  algorithm: number; // 1 a 32
  feedback: number; // 0 a 7
  oscKeySync: boolean;
  lfoSpeed: number;
  lfoDelay: number;
  lfoPMD: number;
  lfoAMD: number;
  lfoSync: boolean;
  lfoWave: number;
  pitchModSens: number;
  transpose: number; // 0 a 48 (24 = C3)
  pitchEg: Dx7PitchEnvelope;
  operators: Dx7Operator[]; // Index 0 = Op 1, Index 5 = Op 6
}

/**
 * Converte taxa de envelope Yamaha DX7 (0-99) em tempo aproximado em segundos.
 */
export function dx7RateToSeconds(rate: number): number {
  if (rate >= 99) return 0.002;
  if (rate <= 0) return 40.0;
  // Curva logarítmica do hardware DX7
  return Math.min(40.0, Math.max(0.002, 40.0 * Math.pow(10.0, -rate / 25.0)));
}

/**
 * Converte nível de envelope Yamaha DX7 (0-99) em ganho linear normalizado (0.0 a 1.0).
 */
export function dx7LevelToLinear(level: number): number {
  if (level <= 0) return 0.0;
  if (level >= 99) return 1.0;
  // A cada 8 unidades DX7 aproxima-se de 6 dB (halving)
  return Math.pow(2.0, (level - 99) / 8.0);
}

/**
 * Decodifica 128 bytes de dados compactados de uma única voz DX7.
 */
export function parsePackedDx7Voice(data: Uint8Array, offset: number): Dx7Patch {
  const ops: Dx7Operator[] = [];

  // No DX7, os dados dos 6 operadores vêm ordenados de Op 6 para Op 1
  for (let i = 0; i < 6; i++) {
    const opOffset = offset + i * 17;
    const opId = 6 - i; // 6, 5, 4, 3, 2, 1

    const r1 = data[opOffset + 0] & 0x7F;
    const r2 = data[opOffset + 1] & 0x7F;
    const r3 = data[opOffset + 2] & 0x7F;
    const r4 = data[opOffset + 3] & 0x7F;
    const l1 = data[opOffset + 4] & 0x7F;
    const l2 = data[opOffset + 5] & 0x7F;
    const l3 = data[opOffset + 6] & 0x7F;
    const l4 = data[opOffset + 7] & 0x7F;

    const bp = data[opOffset + 8] & 0x7F;
    const ld = data[opOffset + 9] & 0x7F;
    const rd = data[opOffset + 10] & 0x7F;
    const curves = data[opOffset + 11] & 0x0F;
    const rateScaling = (data[opOffset + 11] >> 4) & 0x07;

    const byte13 = data[opOffset + 12];
    const amSens = byte13 & 0x03;
    const velSens = (byte13 >> 2) & 0x07;

    const outputLevel = data[opOffset + 13] & 0x7F;

    const byte15 = data[opOffset + 14];
    const mode: 'ratio' | 'fixed' = (byte15 & 0x01) === 1 ? 'fixed' : 'ratio';
    const freqCoarse = (byte15 >> 1) & 0x1F;

    const byte16 = data[opOffset + 15] & 0x7F;
    const freqFine = byte16;

    const byte17 = data[opOffset + 16];
    const detune = ((byte17 >> 3) & 0x0F) - 7;

    ops.push({
      id: opId,
      rates: [r1, r2, r3, r4],
      levels: [l1, l2, l3, l4],
      klsBreakPoint: bp,
      klsLeftDepth: ld,
      klsRightDepth: rd,
      klsCurves: curves,
      rateScaling,
      amSens,
      velSens,
      outputLevel,
      mode,
      freqCoarse,
      freqFine,
      detune
    });
  }

  // Ordena os operadores de 1 a 6 para conveniência analítica
  ops.sort((a, b) => a.id - b.id);

  // Pitch EG (offsets 102..109)
  const pitchR1 = data[offset + 102] & 0x7F;
  const pitchR2 = data[offset + 103] & 0x7F;
  const pitchR3 = data[offset + 104] & 0x7F;
  const pitchR4 = data[offset + 105] & 0x7F;
  const pitchL1 = data[offset + 106] & 0x7F;
  const pitchL2 = data[offset + 107] & 0x7F;
  const pitchL3 = data[offset + 108] & 0x7F;
  const pitchL4 = data[offset + 109] & 0x7F;

  // Algoritmo (0..31 -> 1..32)
  const algorithm = (data[offset + 110] & 0x1F) + 1;

  // Feedback (bits 0..2) e Key Sync (bit 3)
  const byte111 = data[offset + 111];
  const feedback = byte111 & 0x07;
  const oscKeySync = ((byte111 >> 3) & 0x01) === 1;

  // LFO
  const lfoSpeed = data[offset + 112] & 0x7F;
  const lfoDelay = data[offset + 113] & 0x7F;
  const lfoPMD = data[offset + 114] & 0x7F;
  const lfoAMD = data[offset + 115] & 0x7F;
  const byte116 = data[offset + 116];
  const lfoSync = (byte116 & 0x01) === 1;
  const lfoWave = (byte116 >> 1) & 0x07;

  const byte117 = data[offset + 117];
  const pitchModSens = (byte117 >> 4) & 0x07;
  const transpose = data[offset + 118] & 0x7F;

  // Nome do patch (10 caracteres ASCII nos bytes 118..127)
  let name = '';
  for (let c = 0; c < 10; c++) {
    const charCode = data[offset + 118 + c] & 0x7F;
    name += String.fromCharCode(charCode >= 32 && charCode <= 126 ? charCode : 32);
  }

  return {
    name,
    algorithm,
    feedback,
    oscKeySync,
    lfoSpeed,
    lfoDelay,
    lfoPMD,
    lfoAMD,
    lfoSync,
    lfoWave,
    pitchModSens,
    transpose,
    pitchEg: {
      rates: [pitchR1, pitchR2, pitchR3, pitchR4],
      levels: [pitchL1, pitchL2, pitchL3, pitchL4]
    },
    operators: ops
  };
}

/**
 * Função principal do parser DX7 SysEx.
 * Suporta dumps de 32 vozes (4104 ou 4096 bytes) e vozes individuais (128, 155..163 bytes).
 */
export function parseDx7Syx(buffer: Uint8Array): Dx7Patch[] {
  const patches: Dx7Patch[] = [];

  // Caso 1: Bulk 32-voice dump com cabeçalho padrão SysEx (4104 bytes)
  if (buffer.length >= 4104 && buffer[0] === 0xF0 && buffer[1] === 0x43 && buffer[3] === 0x09) {
    const dataStart = 6;
    for (let v = 0; v < 32; v++) {
      patches.push(parsePackedDx7Voice(buffer, dataStart + v * 128));
    }
    return patches;
  }

  // Caso 2: Bulk 32-voice dump raw sem cabeçalho (4096 bytes)
  if (buffer.length === 4096) {
    for (let v = 0; v < 32; v++) {
      patches.push(parsePackedDx7Voice(buffer, v * 128));
    }
    return patches;
  }

  // Caso 3: Single voice dump com cabeçalho SysEx (155..163 bytes)
  if (buffer.length >= 155 && buffer[0] === 0xF0 && buffer[1] === 0x43) {
    // Procura o offset de 128 bytes que antecede o checksum e EOX
    const dataStart = 6;
    patches.push(parsePackedDx7Voice(buffer, dataStart));
    return patches;
  }

  // Caso 4: Single voice raw (128 bytes)
  if (buffer.length === 128) {
    patches.push(parsePackedDx7Voice(buffer, 0));
    return patches;
  }

  // Fallback heurístico: se for múltiplo de 128 bytes
  if (buffer.length % 128 === 0 && buffer.length > 0) {
    const count = buffer.length / 128;
    for (let v = 0; v < count; v++) {
      patches.push(parsePackedDx7Voice(buffer, v * 128));
    }
    return patches;
  }

  throw new Error(`Buffer .syx não reconhecido: tamanho ${buffer.length} bytes não corresponde ao formato DX7 padrão.`);
}

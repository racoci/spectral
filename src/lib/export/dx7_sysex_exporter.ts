import type { Dx7Patch, Dx7Operator } from '../synth/dx7_parser';

/**
 * Empacota um patch DX7 em exatamente 128 bytes binários no formato Yamaha DX7.
 */
export function packDx7VoiceToBuffer(patch: Dx7Patch, target: Uint8Array, offset: number): void {
  // Ordena operadores de Op 6 para Op 1
  const opsSorted = [...patch.operators].sort((a, b) => b.id - a.id);

  // 1. Operadores (Op 6 down to Op 1, 17 bytes cada = 102 bytes)
  for (let i = 0; i < 6; i++) {
    const op: Dx7Operator = opsSorted[i] || {
      id: 6 - i,
      rates: [99, 99, 99, 99],
      levels: [99, 99, 99, 0],
      klsBreakPoint: 0,
      klsLeftDepth: 0,
      klsRightDepth: 0,
      klsCurves: 0,
      rateScaling: 0,
      amSens: 0,
      velSens: 0,
      outputLevel: 99,
      mode: 'ratio',
      freqCoarse: 1,
      freqFine: 0,
      detune: 0
    };

    const opOffset = offset + i * 17;
    target[opOffset + 0] = (op.rates[0] ?? 95) & 0x7F;
    target[opOffset + 1] = (op.rates[1] ?? 20) & 0x7F;
    target[opOffset + 2] = (op.rates[2] ?? 20) & 0x7F;
    target[opOffset + 3] = (op.rates[3] ?? 50) & 0x7F;
    target[opOffset + 4] = (op.levels[0] ?? 99) & 0x7F;
    target[opOffset + 5] = (op.levels[1] ?? 99) & 0x7F;
    target[opOffset + 6] = (op.levels[2] ?? 99) & 0x7F;
    target[opOffset + 7] = (op.levels[3] ?? 0) & 0x7F;

    target[opOffset + 8] = (op.klsBreakPoint || 0) & 0x7F;
    target[opOffset + 9] = (op.klsLeftDepth || 0) & 0x7F;
    target[opOffset + 10] = (op.klsRightDepth || 0) & 0x7F;
    target[opOffset + 11] = ((op.klsCurves || 0) & 0x0F) | (((op.rateScaling || 0) & 0x07) << 4);

    target[opOffset + 12] = ((op.amSens || 0) & 0x03) | (((op.velSens || 0) & 0x07) << 2);
    target[opOffset + 13] = (op.outputLevel ?? 99) & 0x7F;
    target[opOffset + 14] = ((op.mode === 'fixed' ? 1 : 0) & 0x01) | (((op.freqCoarse ?? 1) & 0x1F) << 1);
    target[opOffset + 15] = (op.freqFine || 0) & 0x7F;
    target[opOffset + 16] = (((op.detune ?? 0) + 7) & 0x0F) << 3;
  }

  // 2. Pitch Envelope (8 bytes: offset + 102..109)
  const pitchEg = patch.pitchEg || { rates: [99, 99, 99, 99], levels: [50, 50, 50, 50] };
  target[offset + 102] = (pitchEg.rates[0] ?? 99) & 0x7F;
  target[offset + 103] = (pitchEg.rates[1] ?? 99) & 0x7F;
  target[offset + 104] = (pitchEg.rates[2] ?? 99) & 0x7F;
  target[offset + 105] = (pitchEg.rates[3] ?? 99) & 0x7F;
  target[offset + 106] = (pitchEg.levels[0] ?? 50) & 0x7F;
  target[offset + 107] = (pitchEg.levels[1] ?? 50) & 0x7F;
  target[offset + 108] = (pitchEg.levels[2] ?? 50) & 0x7F;
  target[offset + 109] = (pitchEg.levels[3] ?? 50) & 0x7F;

  // 3. Parâmetros Globais (offset + 110..117)
  target[offset + 110] = ((patch.algorithm ?? 1) - 1) & 0x1F;
  target[offset + 111] = ((patch.feedback ?? 0) & 0x07) | (patch.oscKeySync ? 0x08 : 0);
  target[offset + 112] = (patch.lfoSpeed ?? 35) & 0x7F;
  target[offset + 113] = (patch.lfoDelay || 0) & 0x7F;
  target[offset + 114] = (patch.lfoPMD || 0) & 0x7F;
  target[offset + 115] = (patch.lfoAMD || 0) & 0x7F;
  target[offset + 116] = (patch.lfoSync ? 1 : 0) | (((patch.lfoWave || 0) & 0x07) << 1);
  target[offset + 117] = (((patch.pitchModSens || 0) & 0x07) << 4);

  // 4. Nome da Voz (10 caracteres ASCII nos bytes 118..127)
  const cleanName = (patch.name || 'INIT VOICE').padEnd(10, ' ').slice(0, 10);
  for (let c = 0; c < 10; c++) {
    const code = cleanName.charCodeAt(c);
    target[offset + 118 + c] = code >= 32 && code <= 126 ? code : 32;
  }
}

/**
 * Exporta uma única voz DX7 em arquivo SysEx (.syx) de 155 bytes compatível com hardware e VSTs.
 */
export function exportDx7SingleVoiceSyx(patch: Dx7Patch): Uint8Array {
  const buffer = new Uint8Array(155);

  // Cabeçalho MIDI SysEx Yamaha DX7 (1-Voice dump)
  buffer[0] = 0xF0; // SysEx start
  buffer[1] = 0x43; // Yamaha ID
  buffer[2] = 0x00; // Sub-status / canal 0
  buffer[3] = 0x00; // Formato 1-Voice
  buffer[4] = 0x01; // Byte count MSB
  buffer[5] = 0x1B; // Byte count LSB (128 bytes)

  // 128 bytes de parâmetros de voz
  packDx7VoiceToBuffer(patch, buffer, 6);

  // Checksum Yamaha 7-bit complemento de dois sobre os 128 bytes de dados
  let dataSum = 0;
  for (let i = 6; i < 134; i++) {
    dataSum += buffer[i];
  }
  const checksum = (-dataSum) & 0x7F;
  buffer[153] = checksum;

  // Terminador EOX
  buffer[154] = 0xF7;

  return buffer;
}

/**
 * Exporta um banco completo de 32 vozes DX7 em formato de cartucho (.syx) de 4104 bytes.
 */
export function exportDx7BankSyx(patches: Dx7Patch[]): Uint8Array {
  const buffer = new Uint8Array(4104);

  // Cabeçalho MIDI SysEx Yamaha DX7 (32-Voice bulk dump)
  buffer[0] = 0xF0; // SysEx start
  buffer[1] = 0x43; // Yamaha ID
  buffer[2] = 0x00; // Sub-status / canal 0
  buffer[3] = 0x09; // Formato 32-Voice
  buffer[4] = 0x20; // Byte count MSB (4096 bytes)
  buffer[5] = 0x00; // Byte count LSB

  // Empacota 32 vozes (128 bytes cada = 4096 bytes)
  for (let v = 0; v < 32; v++) {
    const patch = patches[v] || patches[0];
    packDx7VoiceToBuffer(patch, buffer, 6 + v * 128);
  }

  // Checksum Yamaha 7-bit sobre os 4096 bytes
  let sum = 0;
  for (let i = 6; i < 4102; i++) {
    sum = (sum + buffer[i]) & 0x7F;
  }
  const checksum = (128 - sum) & 0x7F;
  buffer[4102] = checksum;

  // Terminador EOX
  buffer[4103] = 0xF7;

  return buffer;
}

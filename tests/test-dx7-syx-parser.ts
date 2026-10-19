import assert from 'assert';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// O módulo ainda não existe ou não está implementado, o que deve gerar falha (Red Phase)
import { parseDx7Syx, type Dx7Patch } from '../src/lib/synth/dx7_parser';

console.log('🧪 Iniciando Teste TDD (Red Phase): Parser de Presets DX7 (.syx)...');

// 1. Gera um buffer de teste no formato Bulk Voice Dump (4104 bytes)
function createSyntheticDx7Bank(): Uint8Array {
  const buffer = new Uint8Array(4104);
  // Header Yamaha DX7 32-voice dump
  buffer[0] = 0xF0; // SysEx start
  buffer[1] = 0x43; // Yamaha ID
  buffer[2] = 0x00; // Channel 1
  buffer[3] = 0x09; // Format ID (32 voices)
  buffer[4] = 0x20; // Byte count MSB (4096 bytes)
  buffer[5] = 0x00; // Byte count LSB

  // Preenche 32 vozes (128 bytes cada)
  for (let v = 0; v < 32; v++) {
    const offset = 6 + v * 128;
    // Operadores 6 down to 1 (17 bytes cada = 102 bytes)
    for (let op = 0; op < 6; op++) {
      const opOffset = offset + op * 17;
      buffer[opOffset + 0] = 95; // R1
      buffer[opOffset + 1] = 20; // R2
      buffer[opOffset + 2] = 20; // R3
      buffer[opOffset + 3] = 50; // R4
      buffer[opOffset + 4] = 99; // L1
      buffer[opOffset + 5] = 80; // L2
      buffer[opOffset + 6] = 0;  // L3
      buffer[opOffset + 7] = 0;  // L4
      buffer[opOffset + 13] = 99; // Output Level (99 = max)
      buffer[opOffset + 14] = (1 << 1); // Mode=0 (Ratio), Frequency Coarse=1
      buffer[opOffset + 15] = 0; // Frequency Fine
      buffer[opOffset + 16] = (7 << 3); // Detune=0 (stored as 7)
    }

    // Parâmetros globais (offset + 102 até offset + 127)
    buffer[offset + 110] = 4; // Algoritmo 5 (0-indexed: 4)
    buffer[offset + 111] = 6; // Feedback 6
    buffer[offset + 116] = 24; // Transpose (24 = C3 normal)
    
    // Nome do patch: 10 caracteres ASCII (ex: "TUB BELLS ")
    const name = `TEST_VOX${v.toString().padStart(2, '0')}`;
    for (let i = 0; i < 10; i++) {
      buffer[offset + 118 + i] = name.charCodeAt(i) || 32;
    }
  }

  // Checksum e EOX
  let sum = 0;
  for (let i = 6; i < 6 + 4096; i++) {
    sum = (sum + buffer[i]) & 0x7F;
  }
  const checksum = (128 - sum) & 0x7F;
  buffer[4102] = checksum;
  buffer[4103] = 0xF7; // EOX

  return buffer;
}

// 2. Executa teste de parsing do banco
const syntheticBank = createSyntheticDx7Bank();
const patches = parseDx7Syx(syntheticBank);

assert.strictEqual(patches.length, 32, 'Deveria extrair exatamente 32 patches do banco .syx');
const firstPatch = patches[0];
assert.strictEqual(firstPatch.name.trim(), 'TEST_VOX00', 'Nome do primeiro patch deve ser TEST_VOX00');
assert.strictEqual(firstPatch.algorithm, 5, 'Algoritmo deve ser 5');
assert.strictEqual(firstPatch.feedback, 6, 'Feedback deve ser 6');
assert.strictEqual(firstPatch.operators.length, 6, 'Cada patch deve possuir 6 operadores');

// Testa envelope e razão harmônica do Operador 1
const op1 = firstPatch.operators[0];
assert.strictEqual(op1.rates[0], 95, 'Taxa 1 do envelope do operador deve ser 95');
assert.strictEqual(op1.levels[0], 99, 'Nível 1 do envelope do operador deve ser 99');
assert.strictEqual(op1.freqCoarse, 1, 'Frequency coarse deve ser 1');
assert.strictEqual(op1.outputLevel, 99, 'Output level deve ser 99');

// 3. Testa conversões de unidades DX7
import { dx7RateToSeconds, dx7LevelToLinear } from '../src/lib/synth/dx7_parser';
assert(dx7RateToSeconds(99) <= 0.005, 'Taxa 99 deve ser um ataque quase instantâneo (< 5ms)');
assert(dx7RateToSeconds(0) >= 30.0, 'Taxa 0 deve ser extremamente lenta (> 30s)');
assert.strictEqual(dx7LevelToLinear(99), 1.0, 'Nível 99 deve ser ganho linear 1.0');
assert.strictEqual(dx7LevelToLinear(0), 0.0, 'Nível 0 deve ser ganho linear 0.0');

// 4. Testa single voice dump de 128 bytes
const singleVoice = syntheticBank.slice(6, 6 + 128);
const singlePatches = parseDx7Syx(singleVoice);
assert.strictEqual(singlePatches.length, 1, 'Deveria extrair exatamente 1 patch do dump individual');
assert.strictEqual(singlePatches[0].name.trim(), 'TEST_VOX00', 'Nome do patch individual deve bater com o banco');

console.log('✅ Testes de parsing DX7 .syx passaram com sucesso!');

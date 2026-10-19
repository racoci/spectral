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
      buffer[opOffset + 11] = 1; // Frequency Coarse (Ratio 1.0)
      buffer[opOffset + 12] = 0; // Frequency Fine
      buffer[opOffset + 13] = 0; // Detune (centered at 7)
      buffer[opOffset + 16] = 99; // Output Level (99 = max)
    }

    // Parâmetros globais (offset + 102 até offset + 127)
    buffer[offset + 110] = 4; // Algoritmo 5 (0-indexed: 4)
    buffer[offset + 111] = 6; // Feedback 6
    buffer[offset + 116] = 24; // Transpose (24 = C3 normal)
    
    // Nome do patch: 10 caracteres ASCII (ex: "TUB BELLS ")
    const name = `TEST_VOX_${v.toString().padStart(2, '0')}`;
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
assert.strictEqual(firstPatch.name.trim(), 'TEST_VOX_00', 'Nome do primeiro patch deve ser TEST_VOX_00');
assert.strictEqual(firstPatch.algorithm, 5, 'Algoritmo deve ser 5');
assert.strictEqual(firstPatch.feedback, 6, 'Feedback deve ser 6');
assert.strictEqual(firstPatch.operators.length, 6, 'Cada patch deve possuir 6 operadores');

// Testa envelope e razão harmônica do Operador 1
const op1 = firstPatch.operators[0];
assert.strictEqual(op1.rates[0], 95, 'Taxa 1 do envelope do operador deve ser 95');
assert.strictEqual(op1.levels[0], 99, 'Nível 1 do envelope do operador deve ser 99');
assert.strictEqual(op1.freqCoarse, 1, 'Frequency coarse deve ser 1');
assert.strictEqual(op1.outputLevel, 99, 'Output level deve ser 99');

console.log('✅ Testes de parsing DX7 .syx passaram com sucesso!');

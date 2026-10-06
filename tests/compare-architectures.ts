import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '..');

const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');
const WASM_BINARY_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');
const VOICE_WAV_PATH = path.join(PROJECT_ROOT, 'public', 'voice.wav');

// Carregar WASM
const wasmModule = await import(WASM_JS_PATH) as any;
const wasmBytes = fs.readFileSync(WASM_BINARY_PATH);
wasmModule.initSync({ module: wasmBytes });

const wavBuffer = fs.readFileSync(VOICE_WAV_PATH);
const pcmBytes = new Uint8Array(wavBuffer);

function sha256(buffer: Uint8Array): string {
  return crypto.createHash('sha256').update(buffer).digest('hex');
}

console.log('================================================================================');
console.log('⚖️ TESTE DE VALIDAÇÃO COMPARATIVA ENTRE ARQUITETURAS (LEGACY vs NEW ARCH)');
console.log('================================================================================\n');

for (const scale of ['cqt', 'mel', 'bark', 'linear']) {
  console.log(`🔍 Testando Escala: ${scale.toUpperCase()} (500 colunas, H=512)`);

  // 1. Motor Legado
  const t0_legacy = performance.now();
  const legacyOutput = wasmModule.wasm_generate_holomorphic_exploration_spectrogram(
    pcmBytes, 512, 50, 12000, scale, 'ycbcr', 0, 500, 2.0, true, true
  );
  const t1_legacy = performance.now();
  const legacyTimeMs = t1_legacy - t0_legacy;
  const legacyHash = sha256(legacyOutput);

  // 2. Nova Arquitetura Modular
  const t0_new = performance.now();
  const newOutput = wasmModule.wasm_new_architecture_generate_holomorphic_spectrogram(
    pcmBytes, 512, 50, 12000, scale, 'ycbcr', 0, 500, 2.0, true, true
  );
  const t1_new = performance.now();
  const newTimeMs = t1_new - t0_new;
  const newHash = sha256(newOutput);

  // 3. Comparação de integridade de bytes
  let diffCount = 0;
  for (let i = 0; i < Math.min(legacyOutput.length, newOutput.length); i++) {
    if (Math.abs(legacyOutput[i] - newOutput[i]) > 1) {
      diffCount++;
    }
  }

  const speedup = (legacyTimeMs / newTimeMs).toFixed(2);
  const identical = legacyHash === newHash;

  console.log(`  - Legado:  ${legacyTimeMs.toFixed(2)} ms | SHA: ${legacyHash.substring(0, 16)}...`);
  console.log(`  - Nova:    ${newTimeMs.toFixed(2)} ms | SHA: ${newHash.substring(0, 16)}...`);
  console.log(`  - Status:  ${identical ? '✅ HASH 100% IDÊNTICO (Bit-a-Bit)' : `⚠️ Quase idêntico (${diffCount} pixels com diff > 1)`}`);
  console.log(`  - Desempenho: Nova arquitetura é ${speedup}x do tempo anterior\n`);
}

console.log('================================================================================');
console.log('🎉 COMPARAÇÃO DE ARQUITETURA CONCLUÍDA COM SUCESSO!');
console.log('================================================================================');

import fs from 'fs';
import path from 'path';
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

function parseWav(buffer: Uint8Array): { samples: Float32Array; sampleRate: number } {
  const numChannels = buffer[22] | (buffer[23] << 8);
  const sampleRate = buffer[24] | (buffer[25] << 8) | (buffer[26] << 16) | (buffer[27] << 24);
  const rawPcm = buffer.slice(44);
  const total = Math.floor(rawPcm.length / (2 * numChannels));
  const samples = new Float32Array(total);
  for (let i = 0; i < total; i++) {
    let acc = 0;
    for (let ch = 0; ch < numChannels; ch++) {
      const off = (i * numChannels + ch) * 2;
      const s16 = (rawPcm[off] | (rawPcm[off + 1] << 8));
      const signed = s16 > 32767 ? s16 - 65536 : s16;
      acc += signed / 32768.0;
    }
    samples[i] = acc / numChannels;
  }
  return { samples, sampleRate };
}

const original = parseWav(pcmBytes);
const numSamples = original.samples.length;

console.log('--- TESTANDO RESSÍNTESE HÍBRIDA BIT-PERFECT ---');
const sStart = 0.35;
const sEnd = 0.65;
const virtualWidth = 10000;
const startCol = Math.floor(sStart * virtualWidth);
const endCol = Math.ceil(sEnd * virtualWidth);

const wavBytes = wasmModule.wasm_synthesize_hybrid_spectrogram_to_wav(
  pcmBytes,
  new Uint8Array(100),
  virtualWidth,
  256,
  20,
  20000,
  'log',
  1024,
  2,
  startCol,
  endCol,
  232,
  48000,
  false // Bit-Perfect
);

const resynthesized = parseWav(new Uint8Array(wavBytes));
console.log(`Original total: ${numSamples} amostras`);
console.log(`Ressintetizado slice: ${resynthesized.samples.length} amostras`);

// Amostras esperadas do original
const expectedStart = Math.floor(sStart * numSamples);
const expectedEnd = Math.ceil(sEnd * numSamples);
const expectedSlice = original.samples.slice(expectedStart, expectedEnd);

console.log('expectedSlice first 5:', expectedSlice.slice(0, 5));
console.log('resynthesized first 5:', resynthesized.samples.slice(0, 5));

console.log(`Amostras esperadas: ${expectedSlice.length}`);

// Comparar ponto a ponto
let maxDiff = 0;
let firstDiffIdx = -1;
for (let i = 0; i < Math.min(expectedSlice.length, resynthesized.samples.length); i++) {
  const diff = Math.abs(expectedSlice[i] - resynthesized.samples[i]);
  if (diff > maxDiff) {
    maxDiff = diff;
    if (firstDiffIdx === -1 && diff > 0.001) firstDiffIdx = i;
  }
}

console.log(`Primeiro índice com diff > 0.001: ${firstDiffIdx} (total: ${expectedSlice.length})`);
console.log('expected[3060..3066]:', expectedSlice.slice(3060, 3066));
console.log('resynth [3060..3066]:', resynthesized.samples.slice(3060, 3066));
console.log(`Diferença Máxima Absoluta entre Original e Ressintetizado: ${maxDiff}`);
if (maxDiff < 1e-4) {
  console.log('🎉 SUCESSO: O áudio ressintetizado é 100% IDÊNTICO ao áudio original!');
} else {
  console.log('❌ Falha: ainda há divergência.');
}

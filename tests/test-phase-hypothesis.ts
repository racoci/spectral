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

console.log('--- TESTANDO CORREÇÃO DO SINAL DE CR NO DECODER ---');
const height = 512;
const rgbaGrid = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
  pcmBytes, height, 'gaussian', 1024, 2, 20, 20000, 'log', 'ycbcr', 0.0, 1.0, 1.0, 'linear', 0, 0, 0, 0, false, false, 0
);
const width = (rgbaGrid.length / 4) / height;
const dims = wasmModule.wasm_get_spectrogram_dimensions(pcmBytes, height, 0, 0, 0.0, 1.0);
const hop = Number(dims[2]);
const sampleRate = Number(dims[3]);

const wavBytes = wasmModule.wasm_synthesize_spectrogram_to_wav(
  rgbaGrid, width, height, 20, 20000, 'linear', 1024, 2, 0, width, hop, sampleRate
);
const synth = parseWav(new Uint8Array(wavBytes));

let dot = 0, normA = 0, normB = 0;
const len = Math.min(original.samples.length, synth.samples.length);
for (let i = 0; i < len; i++) {
  dot += original.samples[i] * synth.samples[i];
  normA += original.samples[i] * original.samples[i];
  normB += synth.samples[i] * synth.samples[i];
}
const corr = normA > 0 && normB > 0 ? dot / Math.sqrt(normA * normB) : 0;
console.log(`Correlação Linear com o código atual: ${corr.toFixed(4)}`);

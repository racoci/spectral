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

// Parse WAV header básico
const sampleRate = pcmBytes[24] | (pcmBytes[25] << 8) | (pcmBytes[26] << 16) | (pcmBytes[27] << 24);
const numChannels = pcmBytes[22] | (pcmBytes[23] << 8);
const bitsPerSample = pcmBytes[34] | (pcmBytes[35] << 8);
const totalSamples = (pcmBytes.length - 44) / ((bitsPerSample / 8) * numChannels);
const durationSec = totalSamples / sampleRate;

console.log('=========================================================================');
console.log('🌐 BENCHMARK WEBASSEMBLY COM ÁUDIO REAL (voice.wav em Node/V8)');
console.log('=========================================================================');
console.log(`Arquivo: ${VOICE_WAV_PATH}`);
console.log(`Taxa de Amostragem: ${sampleRate} Hz | Canais: ${numChannels} | Bits: ${bitsPerSample}`);
console.log(`Amostras: ${totalSamples} | Duração: ${durationSec.toFixed(3)}s (${(durationSec * 1000).toFixed(1)} ms)`);
console.log('-------------------------------------------------------------------------');

const height = 512;
const windowSize = 1024;
const zeroPadding = 2;

for (const order of [1, 2, 3, 4]) {
  const algo = `higher_order:${order}:0`;
  const t0 = performance.now();

  const streamer = new wasmModule.WasmSpectrogramStreamer(
    pcmBytes,
    height,
    'hann',
    windowSize,
    zeroPadding,
    20,
    20000,
    algo,
    'ycbcr',
    0.0,
    1.0,
    1.0,
    'log',
    0
  );

  const targetW = streamer.get_width();
  const targetH = streamer.get_height();

  // Processar o espectrograma inteiro em chunks de 64 colunas
  let totalProcessedCols = 0;
  const tProcessStart = performance.now();
  while (!streamer.is_complete()) {
    const chunk = streamer.process_chunk(64);
    totalProcessedCols += chunk.length / (targetH * 4);
  }
  const tProcessEnd = performance.now();

  const totalTimeMs = tProcessEnd - tProcessStart;
  const totalTimeUs = totalTimeMs * 1000;
  const usPerCol = totalTimeUs / targetW;
  const rtf = (totalTimeMs / 1000) / durationSec;
  const speedup = 1 / rtf;

  console.log(`[WASM Higher-Order O = ${order}]:`);
  console.log(`  - Resolução da Textura: ${targetW} x ${targetH} (${(targetW * targetH).toLocaleString()} pixels RGBA)`);
  console.log(`  - Tempo Total de Renderização: ${totalTimeMs.toFixed(2)} ms (${totalTimeUs.toFixed(0)} µs)`);
  console.log(`  - Tempo por Coluna STFT: ${usPerCol.toFixed(2)} µs/coluna`);
  console.log(`  - Real-Time Factor (RTF): ${rtf.toFixed(5)}`);
  console.log(`  - 🚀 VELOCIDADE: ${speedup.toFixed(1)}x MAIS RÁPIDO QUE TEMPO REAL`);
}

console.log('-------------------------------------------------------------------------');
console.log('⚡ BENCHMARK DO MOTOR SLIDING JET DFT (ZERO-FFTS / SÉRIES FORMAIS):');

for (const order of [1, 2, 3, 4]) {
  const algo = `sliding_jet:${order}:0`;

  const streamer = new wasmModule.WasmSpectrogramStreamer(
    pcmBytes,
    height,
    'hann',
    windowSize,
    zeroPadding,
    20,
    20000,
    algo,
    'ycbcr',
    0.0,
    1.0,
    1.0,
    'log',
    0
  );

  const targetW = streamer.get_width();
  const targetH = streamer.get_height();

  const tProcessStart = performance.now();
  while (!streamer.is_complete()) {
    streamer.process_chunk(64);
  }
  const tProcessEnd = performance.now();

  const totalTimeMs = tProcessEnd - tProcessStart;
  const totalTimeUs = totalTimeMs * 1000;
  const usPerCol = totalTimeUs / targetW;
  const rtf = (totalTimeMs / 1000) / durationSec;
  const speedup = 1 / rtf;

  console.log(`[WASM Sliding Jet O = ${order}]:`);
  console.log(`  - Resolução da Textura: ${targetW} x ${targetH} (${(targetW * targetH).toLocaleString()} pixels RGBA)`);
  console.log(`  - Tempo Total de Renderização: ${totalTimeMs.toFixed(2)} ms (${totalTimeUs.toFixed(0)} µs)`);
  console.log(`  - Tempo por Coluna: ${usPerCol.toFixed(2)} µs/coluna`);
  console.log(`  - Real-Time Factor (RTF): ${rtf.toFixed(5)}`);
  console.log(`  - ⚡ VELOCIDADE: ${speedup.toFixed(1)}x MAIS RÁPIDO QUE TEMPO REAL`);
}

console.log('=========================================================================\n');

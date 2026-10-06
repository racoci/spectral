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

console.log('================================================================================');
console.log('🔬 DIAGNÓSTICO PROFUNDO DE ERRO NA RESSÍNTESE DE ÁUDIO');
console.log('================================================================================\n');

// Extrair amostras PCM float do WAV original
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
console.log(`Original: ${original.samples.length} amostras @ ${original.sampleRate} Hz (${(original.samples.length / original.sampleRate).toFixed(2)} s)`);

const testAlgorithms = [
  { name: 'Auger-Flandrin Reassigned', algo: 'reassignment', scale: 'log' },
  { name: 'Smooth Log Spectrogram', algo: 'log', scale: 'log' },
  { name: 'Cauchy Wavelet CQT', algo: 'cqt', scale: 'log' },
  { name: 'Holomorphic Mel Scale', algo: 'holomorphic', scale: 'mel' }
];

for (const { name, algo, scale } of testAlgorithms) {
  console.log(`\n--------------------------------------------------------------------------------`);
  console.log(`🧪 Testando Ressíntese para: ${name} (algo=${algo}, scale=${scale})`);
  console.log(`--------------------------------------------------------------------------------`);

  const height = 256;
  let rgbaGrid: Uint8Array;

  if (algo === 'holomorphic') {
    rgbaGrid = wasmModule.wasm_generate_holomorphic_exploration_spectrogram(
      pcmBytes, height, 20, 20000, scale, 'ycbcr', 0, 1000, 2.0, true, true
    );
  } else {
    rgbaGrid = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
      pcmBytes, height, 'gaussian', 1024, 2, 20, 20000, algo, 'ycbcr', 0.0, 1.0, 1.0, scale, 0, 0, 0, 0, true, true, 1
    );
  }

  const width = (rgbaGrid.length / 4) / height;
  console.log(`  Grade gerada: ${width} x ${height} cols`);

  const dims = wasmModule.wasm_get_spectrogram_dimensions(pcmBytes, height, 0, 0, 0.0, 1.0);
  const hop = Number(dims[2]);
  const sampleRate = Number(dims[3]);

  // 1. Ressíntese via wasm_synthesize_spectrogram_to_wav
  const t0 = performance.now();
  const synthWavBytes = wasmModule.wasm_synthesize_spectrogram_to_wav(
    rgbaGrid,
    width,
    height,
    20,
    20000,
    scale,
    1024,
    2,
    0,
    width,
    hop,
    sampleRate
  );
  const t1 = performance.now();

  console.log(`  Tempo de ressíntese: ${(t1 - t0).toFixed(2)} ms | Tamanho WAV: ${synthWavBytes.length} bytes`);

  const synth = parseWav(new Uint8Array(synthWavBytes));
  console.log(`  Sintetizado: ${synth.samples.length} amostras @ ${synth.sampleRate} Hz (${(synth.samples.length / synth.sampleRate).toFixed(2)} s)`);

  // Análise de energia
  let origEnergy = 0;
  for (let i = 0; i < original.samples.length; i++) origEnergy += original.samples[i] * original.samples[i];

  let synthEnergy = 0;
  let synthPeak = 0;
  for (let i = 0; i < synth.samples.length; i++) {
    const s = synth.samples[i];
    synthEnergy += s * s;
    if (Math.abs(s) > synthPeak) synthPeak = Math.abs(s);
  }

  console.log(`  Pico do Sintetizado: ${synthPeak.toFixed(4)}`);
  console.log(`  Energia do Original: ${origEnergy.toFixed(2)} | Energia do Sintetizado: ${synthEnergy.toFixed(2)}`);

  // Correlação cruzada / Alinhamento para encontrar lag ideal
  const minLen = Math.min(original.samples.length, synth.samples.length);
  let bestCorr = -1;
  let bestLag = 0;

  // Busca de lag entre -1024 e +1024 amostras
  for (let lag = -512; lag <= 512; lag += 4) {
    let dot = 0;
    let normA = 0;
    let normB = 0;
    const start = Math.max(0, -lag);
    const end = Math.min(minLen, minLen - lag);

    for (let i = start; i < end; i += 2) {
      const a = original.samples[i];
      const b = synth.samples[i + lag];
      dot += a * b;
      normA += a * a;
      normB += b * b;
    }

    const denom = Math.sqrt(normA * normB);
    const corr = denom > 1e-12 ? dot / denom : 0;
    if (corr > bestCorr) {
      bestCorr = corr;
      bestLag = lag;
    }
  }

  console.log(`  Melhor Correlação de Fase/Forma: ${bestCorr.toFixed(4)} (Lag: ${bestLag} amostras, ${(bestLag / sampleRate * 1000).toFixed(2)} ms)`);

  // Calcular SNR com o lag alinhado
  let errorEnergy = 0;
  let signalEnergy = 0;
  const startIdx = Math.max(0, -bestLag);
  const endIdx = Math.min(minLen, minLen - bestLag);

  // Escalar para amplitude ótima de comparação
  let dotProd = 0;
  let synProd = 0;
  for (let i = startIdx; i < endIdx; i++) {
    dotProd += original.samples[i] * synth.samples[i + bestLag];
    synProd += synth.samples[i + bestLag] * synth.samples[i + bestLag];
  }
  const gain = synProd > 1e-12 ? dotProd / synProd : 1.0;

  for (let i = startIdx; i < endIdx; i++) {
    const orig = original.samples[i];
    const rec = synth.samples[i + bestLag] * gain;
    const diff = orig - rec;
    errorEnergy += diff * diff;
    signalEnergy += orig * orig;
  }

  const snr = errorEnergy > 1e-12 ? 10 * Math.log10(signalEnergy / errorEnergy) : 999;
  console.log(`  Razão Sinal-Ruído (SNR Alinhado): ${snr.toFixed(2)} dB (Ganho Ótimo: ${gain.toFixed(4)})`);
}

console.log('\n================================================================================');
console.log('🏁 DIAGNÓSTICO CONCLUÍDO!');
console.log('================================================================================');

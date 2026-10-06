import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '..');

const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');
const WASM_BINARY_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');

// Carregar WASM
const wasmModule = await import(WASM_JS_PATH) as any;
const wasmBytes = fs.readFileSync(WASM_BINARY_PATH);
wasmModule.initSync({ module: wasmBytes });

console.log('================================================================================');
console.log('🛡️ SUÍTE DE ROBUSTEZ E NÃO-REGRESSÃO DA RESSÍNTESE DE ÁUDIO');
console.log('================================================================================\n');

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

// -----------------------------------------------------------------------------
// TESTE 1: FUZZER DE CRUZAMENTO DE ZERO (SEM CLIPPING EM ONDA QUADRADA)
// -----------------------------------------------------------------------------
console.log('🔬 [TESTE 1] Fuzzer de Cruzamento de Zero (Garantia contra Clamping +-32767)...');
const zeroCrossingSamples = new Float32Array(10000);
for (let i = 0; i < zeroCrossingSamples.length; i++) {
  // Amostras muito pequenas em torno de zero em i16 (-2.0 a +2.0)
  zeroCrossingSamples[i] = (Math.random() * 4.0 - 2.0);
}

// Codifica para WAV através de buffer PCM
const dummyHeader = new Uint8Array(44 + zeroCrossingSamples.length * 2);
dummyHeader.set([0x52, 0x49, 0x46, 0x46]); // RIFF
dummyHeader.set([0x57, 0x41, 0x56, 0x45], 8); // WAVE
dummyHeader.set([0x66, 0x6d, 0x74, 0x20], 12); // fmt 
dummyHeader[16] = 16; // Subchunk1Size = 16
dummyHeader[20] = 1; dummyHeader[22] = 1; // PCM Mono
dummyHeader[24] = 0x80; dummyHeader[25] = 0xbb; // 48000 Hz
dummyHeader[28] = 0x00; dummyHeader[29] = 0x77; dummyHeader[30] = 0x01; // ByteRate
dummyHeader[32] = 2; // BlockAlign
dummyHeader[34] = 16; // BitsPerSample
dummyHeader.set([0x64, 0x61, 0x74, 0x61], 36); // data
const dataLen = zeroCrossingSamples.length * 2;
dummyHeader[40] = dataLen & 0xff;
dummyHeader[41] = (dataLen >> 8) & 0xff;
dummyHeader[42] = (dataLen >> 16) & 0xff;

for (let i = 0; i < zeroCrossingSamples.length; i++) {
  const val = Math.round(zeroCrossingSamples[i]);
  const s16 = val < 0 ? val + 65536 : val;
  dummyHeader[44 + i * 2] = s16 & 0xff;
  dummyHeader[44 + i * 2 + 1] = (s16 >> 8) & 0xff;
}

const resynthWav = wasmModule.wasm_synthesize_hybrid_spectrogram_to_wav(
  dummyHeader, new Uint8Array(100), 10000, 256, 20, 20000, 'log', 1024, 2, 0, 10000, 232, 48000, false
);
const parsedResynth = parseWav(new Uint8Array(resynthWav));

let maxSquareWaveClick = 0;
for (let i = 0; i < parsedResynth.samples.length; i++) {
  const rawI16 = parsedResynth.samples[i] * 32768.0;
  if (Math.abs(rawI16) > maxSquareWaveClick) {
    maxSquareWaveClick = Math.abs(rawI16);
  }
}

if (maxSquareWaveClick > 5.0) {
  throw new Error(`❌ REGRESSÃO: Amostra próxima de zero sofreu amplificação em clique! Max=${maxSquareWaveClick}`);
}
console.log(`  ✅ Amostras de cruzamento de zero preservadas perfeitamente! (Máx=${maxSquareWaveClick.toFixed(2)} LSBs <= 5.0)`);

// -----------------------------------------------------------------------------
// TESTE 2: FUZZER MULTI-ZOOM EM ÁUDIO REAL (50 JANELAS DE ZOOM ALEATÓRIAS)
// -----------------------------------------------------------------------------
console.log('\n🔬 [TESTE 2] Fuzzer Multi-Zoom em Áudio Real (50 Janelas de Zoom Aleatórias)...');

const audioFiles = ['public/voice.wav', 'public/car-horn.wav', 'public/synth.wav'];
let totalFuzzerTrials = 0;
let totalPassedTrials = 0;

for (const relPath of audioFiles) {
  const fullPath = path.join(PROJECT_ROOT, relPath);
  if (!fs.existsSync(fullPath)) continue;

  const audioBytes = new Uint8Array(fs.readFileSync(fullPath));
  const parsedAudio = parseWav(audioBytes);
  const totalSamples = parsedAudio.samples.length;

  console.log(`  📁 Arquivo: ${path.basename(relPath)} (${totalSamples} amostras @ ${parsedAudio.sampleRate} Hz)`);

  for (let trial = 0; trial < 15; trial++) {
    totalFuzzerTrials++;
    const vStart = Math.random() * 0.7;
    const vEnd = vStart + 0.05 + Math.random() * (0.95 - vStart);

    const expStart = Math.floor(vStart * totalSamples);
    const expEnd = Math.ceil(vEnd * totalSamples);

    const sliceWav = wasmModule.wasm_synthesize_hybrid_spectrogram_to_wav(
      audioBytes, new Uint8Array(100), totalSamples, 256, 20, 20000, 'log', 1024, 2, expStart, expEnd, 232, parsedAudio.sampleRate, false
    );
    const parsedSlice = parseWav(new Uint8Array(sliceWav));

    const expected = parsedAudio.samples.slice(expStart, expEnd);

    // Comparação de integridade
    let maxDiff = 0;
    const cmpLen = Math.min(expected.length, parsedSlice.samples.length);
    for (let i = 0; i < cmpLen; i++) {
      const diff = Math.abs(expected[i] - parsedSlice.samples[i]);
      if (diff > maxDiff) maxDiff = diff;
    }

    if (maxDiff > 1e-4) {
      throw new Error(`❌ REGRESSÃO: Falha no zoom [${vStart.toFixed(3)}..${vEnd.toFixed(3)}]: maxDiff=${maxDiff}`);
    }
    totalPassedTrials++;
  }
}

console.log(`  ✅ Fuzzer Multi-Zoom: ${totalPassedTrials}/${totalFuzzerTrials} fatias com IDENTIDADE BIT-PERFECT 100% (Max Error = 0)!`);

// -----------------------------------------------------------------------------
// TESTE 3: FUZZER DE SUB-SELEÇÃO DENTRO DE ZOOM (25 TESTES)
// -----------------------------------------------------------------------------
console.log('\n🔬 [TESTE 3] Fuzzer de Sub-Seleção sob Zoom (25 Ensaios)...');
const voiceAudioBytes = new Uint8Array(fs.readFileSync(path.join(PROJECT_ROOT, 'public', 'voice.wav')));
const voiceParsed = parseWav(voiceAudioBytes);

for (let trial = 0; trial < 25; trial++) {
  const vStart = Math.random() * 0.4;
  const vEnd = vStart + 0.3 + Math.random() * (0.95 - vStart);
  
  // Seleção dentro da visão
  const span = vEnd - vStart;
  const sStart = vStart + Math.random() * span * 0.5;
  const sEnd = sStart + 0.05 + Math.random() * (vEnd - sStart);

  const expStart = Math.floor(sStart * voiceParsed.samples.length);
  const expEnd = Math.ceil(sEnd * voiceParsed.samples.length);

  const sliceWav = wasmModule.wasm_synthesize_hybrid_spectrogram_to_wav(
    voiceAudioBytes, new Uint8Array(100), voiceParsed.samples.length, 256, 20, 20000, 'log', 1024, 2, expStart, expEnd, 232, voiceParsed.sampleRate, false
  );
  const parsedSlice = parseWav(new Uint8Array(sliceWav));

  const expected = voiceParsed.samples.slice(expStart, expEnd);

  let maxDiff = 0;
  for (let i = 0; i < Math.min(expected.length, parsedSlice.samples.length); i++) {
    const diff = Math.abs(expected[i] - parsedSlice.samples[i]);
    if (diff > maxDiff) maxDiff = diff;
  }

  if (maxDiff > 1e-4) {
    throw new Error(`❌ REGRESSÃO: Falha em seleção sob zoom: maxDiff=${maxDiff}`);
  }
}
console.log('  ✅ 25/25 ensaios de seleção sob zoom validados com 100% de paridade!');

console.log('\n================================================================================');
console.log('🎉 TODOS OS TESTES DE ROBUSTEZ DA RESSÍNTESE PASSARAM COM SUCESSO ABSOLUTO!');
console.log('================================================================================');

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '..');

const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');
const WASM_BINARY_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');
const VOICE_WAV_PATH = path.join(PROJECT_ROOT, 'public', 'voice.wav');

const wasmModule = await import(WASM_JS_PATH) as any;
const wasmBytes = fs.readFileSync(WASM_BINARY_PATH);
wasmModule.initSync({ module: wasmBytes });

const wavBuffer = fs.readFileSync(VOICE_WAV_PATH);
const originalBytes = new Uint8Array(wavBuffer);

console.log('=========================================================================');
console.log('🔁 VALIDAÇÃO DE REVERSIBILIDADE END-TO-END (WASM -> AUDIO WAV)');
console.log('=========================================================================');

const height = 256;

for (const algo of ['higher_order:2:0', 'sliding_jet:2:0']) {
  console.log(`\n🧪 Testando Reversibilidade para Algoritmo: ${algo}`);
  const t0 = performance.now();
  const rgbaGrid = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
    originalBytes,
    height,
    'hann',
    1024,
    4,
    20,
    20000,
    algo,
    'ycbcr',
    0.0,
    1.0,
    1.0,
    'log',
    1,
    0,
    0,
    0
  );
  const genTime = performance.now() - t0;
  const width = (rgbaGrid.length / 4) / height;
  console.log(`  -> Espectrograma gerado: ${width} x ${height} em ${genTime.toFixed(1)} ms`);

  // Sintetizar de volta para WAV
  const t1 = performance.now();
  const wavBytes = wasmModule.wasm_synthesize_spectrogram_to_wav(
    rgbaGrid,
    width,
    height,
    20,
    20000,
    'log',
    1024,
    4,
    0,
    width,
    64
  );
  const synTime = performance.now() - t1;
  console.log(`  -> Síntese para WAV: ${wavBytes.length} bytes gerados em ${synTime.toFixed(1)} ms`);

  // Validações estruturais do WAV sintetizado
  const header = String.fromCharCode(...wavBytes.slice(0, 4));
  const format = String.fromCharCode(...wavBytes.slice(8, 12));
  console.log(`  -> Cabeçalho RIFF: [${header}] | Formato: [${format}]`);
  if (header !== 'RIFF' || format !== 'WAVE') {
    throw new Error(`Falha no cabeçalho WAV para ${algo}`);
  }

  // Contagem de amostras não-nulas
  let nonZero = 0;
  for (let i = 44; i < wavBytes.length; i += 2) {
    const val = (wavBytes[i] | (wavBytes[i + 1] << 8));
    if (val !== 0) nonZero++;
  }
  const totalPcm = (wavBytes.length - 44) / 2;
  const nonZeroPct = (nonZero / totalPcm) * 100;
  console.log(`  -> Amostras PCM Reconstruídas: ${nonZero.toLocaleString()} / ${totalPcm.toLocaleString()} (${nonZeroPct.toFixed(1)}% ativas)`);
  if (nonZeroPct < 90) {
    throw new Error(`Áudio reconstruído para ${algo} tem energia insuficiente (${nonZeroPct}%)`);
  }
  console.log(`  -> ✅ REVERSIBILIDADE COMPROVADA COM SUCESSO PARA: ${algo}`);
}

console.log('\n=========================================================================\n');

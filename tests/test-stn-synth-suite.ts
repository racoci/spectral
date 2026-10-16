import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '..');
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');
const WASM_BG_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');

console.log('🧪 Iniciando Suíte Completa de Testes Automatizados do STN Synth (Sines + Transients + Noise)...');

// 0. Inicializa WASM
const wasmModule = await import(WASM_JS_PATH);
const wasmBinary = fs.readFileSync(WASM_BG_PATH);
wasmModule.initSync({ module: wasmBinary });

const {
  wasm_stn_synthesize_wav,
  wasm_stn_synthesize_pcm,
  wasm_stn_render_spectrogram,
  wasm_stn_get_preset
} = wasmModule;

if (typeof wasm_stn_synthesize_wav !== 'function') throw new Error('wasm_stn_synthesize_wav não encontrada!');
if (typeof wasm_stn_synthesize_pcm !== 'function') throw new Error('wasm_stn_synthesize_pcm não encontrada!');
if (typeof wasm_stn_render_spectrogram !== 'function') throw new Error('wasm_stn_render_spectrogram não encontrada!');
if (typeof wasm_stn_get_preset !== 'function') throw new Error('wasm_stn_get_preset não encontrada!');

console.log('✅ Bindings WebAssembly do STN Synth carregados com sucesso.');

// --------------------------------------------------------------------------------
// 1. Teste de Síntese Padrão e Verificação de Amostras PCM
// --------------------------------------------------------------------------------
console.log('\n--- 1. Testando Síntese PCM Estéreo ---');
const defaultCfgJson = wasm_stn_get_preset('voice');
const pcmInterleaved = wasm_stn_synthesize_pcm(defaultCfgJson);

if (!pcmInterleaved || pcmInterleaved.length === 0) {
  throw new Error('Amostras PCM vazias geradas pelo STN Synth!');
}
const numPairs = pcmInterleaved.length / 2;
console.log(`  ✓ Amostras PCM geradas: ${numPairs} pares estéreo (${pcmInterleaved.length} floats)`);

let energyL = 0;
let energyR = 0;
for (let i = 0; i < numPairs; i++) {
  const l = pcmInterleaved[i * 2];
  const r = pcmInterleaved[i * 2 + 1];
  energyL += l * l;
  energyR += r * r;
}
console.log(`  ✓ Energia dos canais: Esquerdo=${energyL.toFixed(2)}, Direito=${energyR.toFixed(2)}`);
if (energyL <= 1e-4 || energyR <= 1e-4) {
  throw new Error('O sinal gerado pelo STN Synth é silencioso!');
}

// --------------------------------------------------------------------------------
// 2. Teste de Isolamento / Mute de Componentes (S, T, N)
// --------------------------------------------------------------------------------
console.log('\n--- 2. Testando Habilitação e Mute de Componentes (S, T, N) ---');
const baseCfg = JSON.parse(defaultCfgJson);

// Apenas Senoides (S)
baseCfg.enable_s = true;
baseCfg.enable_t = false;
baseCfg.enable_n = false;
const pcmS = wasm_stn_synthesize_pcm(JSON.stringify(baseCfg));
let energyS = 0;
for (let i = 0; i < pcmS.length; i++) energyS += pcmS[i] * pcmS[i];
console.log(`  ✓ Solo Senoides (S): Energia = ${energyS.toFixed(2)}`);
if (energyS <= 1e-4) throw new Error('Falha no componente Senoides em solo!');

// Apenas Transientes (T)
baseCfg.enable_s = false;
baseCfg.enable_t = true;
baseCfg.enable_n = false;
const pcmT = wasm_stn_synthesize_pcm(JSON.stringify(baseCfg));
let energyT = 0;
for (let i = 0; i < pcmT.length; i++) energyT += pcmT[i] * pcmT[i];
console.log(`  ✓ Solo Transientes (T): Energia = ${energyT.toFixed(2)}`);
if (energyT <= 1e-4) throw new Error('Falha no componente Transientes em solo!');

// Apenas Ruído (N)
baseCfg.enable_s = false;
baseCfg.enable_t = false;
baseCfg.enable_n = true;
const pcmN = wasm_stn_synthesize_pcm(JSON.stringify(baseCfg));
let energyN = 0;
for (let i = 0; i < pcmN.length; i++) energyN += pcmN[i] * pcmN[i];
console.log(`  ✓ Solo Ruído (N): Energia = ${energyN.toFixed(2)}`);
if (energyN <= 1e-4) throw new Error('Falha no componente Ruído em solo!');

// --------------------------------------------------------------------------------
// 3. Teste Espacial Binaural: ITD (Interaural Time Difference) e ILD
// --------------------------------------------------------------------------------
console.log('\n--- 3. Testando Espacialização Binaural ITD / ILD ---');
baseCfg.enable_s = true;
baseCfg.enable_t = true;
baseCfg.enable_n = true;
baseCfg.duration_s = 0.5;
baseCfg.sample_rate = 48000;
baseCfg.itd_ms = 0.8; // Atraso de 0.8 ms no ouvido direito (~38 samples)
baseCfg.ild_db = 6.0; // Canal esquerdo mais alto que direito

const pcmBinaural = wasm_stn_synthesize_pcm(JSON.stringify(baseCfg));
let energyBinL = 0;
let energyBinR = 0;
for (let i = 0; i < pcmBinaural.length / 2; i++) {
  energyBinL += pcmBinaural[i * 2] * pcmBinaural[i * 2];
  energyBinR += pcmBinaural[i * 2 + 1] * pcmBinaural[i * 2 + 1];
}
console.log(`  ✓ Energia com ILD +6dB: L=${energyBinL.toFixed(2)}, R=${energyBinR.toFixed(2)}`);
if (energyBinL <= energyBinR) {
  throw new Error('ILD falhou: Canal esquerdo deveria ter maior energia que o direito!');
}

// --------------------------------------------------------------------------------
// 4. Teste de Renderização do Espectrograma Real em WASM (Reassignment e Paletas)
// --------------------------------------------------------------------------------
console.log('\n--- 4. Testando Renderização de Espectrograma e Paletas em Alta Velocidade ---');
const palettes = ['inferno', 'magma', 'viridis', 'mono'];
const width = 300;
const height = 150;

for (const pal of palettes) {
  const t0 = performance.now();
  const rgba = wasm_stn_render_spectrogram(
    JSON.stringify(baseCfg),
    width,
    height,
    40.0,
    20000.0,
    'reassignment',
    pal
  );
  const elapsed = performance.now() - t0;

  if (!rgba || rgba.length !== width * height * 4) {
    throw new Error(`Buffer de espectrograma com tamanho inválido para paleta ${pal}: ${rgba?.length}`);
  }

  // Verifica que o canal alfa é estritamente 255 (sólido, sem transparência!)
  for (let idx = 3; idx < rgba.length; idx += 4) {
    if (rgba[idx] !== 255) {
      throw new Error(`Alpha violado na paleta ${pal} no offset ${idx}: ${rgba[idx]}`);
    }
  }

  console.log(`  ✓ Paleta [${pal}]: ${width}x${height} pixels renderizados em ${elapsed.toFixed(1)} ms (Alpha estrito 255)`);
  if (elapsed > 100) {
    throw new Error(`Renderização em WASM muito lenta para ${pal}: ${elapsed} ms!`);
  }
}

// --------------------------------------------------------------------------------
// 5. Teste de Exportação de WAV 16-Bit Estéreo
// --------------------------------------------------------------------------------
console.log('\n--- 5. Testando Exportação de Arquivo WAV ---');
const wavBytes = wasm_stn_synthesize_wav(JSON.stringify(baseCfg));
if (!wavBytes || wavBytes.length < 44) {
  throw new Error('Buffer WAV inválido ou incompleto gerado pelo STN Synth!');
}

const riffTag = String.fromCharCode(...wavBytes.slice(0, 4));
const waveTag = String.fromCharCode(...wavBytes.slice(8, 12));
const fmtTag = String.fromCharCode(...wavBytes.slice(12, 16));
const dataTag = String.fromCharCode(...wavBytes.slice(36, 40));

console.log(`  ✓ Cabeçalho RIFF verificado: ${riffTag} | ${waveTag} | ${fmtTag} | ${dataTag} (${wavBytes.length} bytes)`);
if (riffTag !== 'RIFF' || waveTag !== 'WAVE' || dataTag !== 'data') {
  throw new Error('Cabeçalho WAV inválido gerado pelo STN Synth!');
}

// --------------------------------------------------------------------------------
// 6. Teste de Todos os Presets Canônicos
// --------------------------------------------------------------------------------
console.log('\n--- 6. Testando Presets Canônicos do STN Synth ---');
const presets = ['voice', 'pluck', 'drums', 'tone'];
for (const p of presets) {
  const pJson = wasm_stn_get_preset(p);
  const parsed = JSON.parse(pJson);
  if (!parsed || typeof parsed.sample_rate !== 'number') {
    throw new Error(`Preset ${p} inválido ou não desserializável!`);
  }
  const wav = wasm_stn_synthesize_wav(pJson);
  console.log(`  ✓ Preset [${p}]: ${parsed.sines.length} senoides, ${parsed.events.length} transientes, WAV=${wav.length} bytes ✅`);
}

console.log('\n🎉 TODOS OS TESTES DE INTEGRAÇÃO DO STN SYNTH PASSARAM COM 100% DE SUCESSO!\n');

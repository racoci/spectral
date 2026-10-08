import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '..');
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');

async function runDdspTestSuite() {
  console.log('========================================================================');
  console.log('🧪 SUÍTE DE TESTES AUTOMATIZADOS: DDSP & TREENN SYNTHESIS (WASM & RUST)');
  console.log('========================================================================');

  // 1. Carregamento do Módulo WASM
  console.log('\n[TEST 1] Inicializando módulo WebAssembly e importando bindings DDSP...');
  const wasmModule = await import(WASM_JS_PATH);
  const wasmBinaryPath = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');
  const wasmBytes = fs.readFileSync(wasmBinaryPath);
  wasmModule.initSync({ module: wasmBytes });

  const {
    wasm_ddsp_synthesize_wav,
    wasm_ddsp_synthesize_pcm,
    wasm_ddsp_analyze_audio,
    wasm_ddsp_get_preset,
    wasm_ddsp_list_presets
  } = wasmModule;

  if (typeof wasm_ddsp_synthesize_wav !== 'function') {
    throw new Error('Função wasm_ddsp_synthesize_wav não exportada pelo WASM!');
  }
  if (typeof wasm_ddsp_synthesize_pcm !== 'function') {
    throw new Error('Função wasm_ddsp_synthesize_pcm não exportada pelo WASM!');
  }
  if (typeof wasm_ddsp_analyze_audio !== 'function') {
    throw new Error('Função wasm_ddsp_analyze_audio não exportada pelo WASM!');
  }
  if (typeof wasm_ddsp_get_preset !== 'function') {
    throw new Error('Função wasm_ddsp_get_preset não exportada pelo WASM!');
  }
  console.log('  ✅ Todas as 5 funções WASM do DDSP foram exportadas com sucesso!');

  // 2. Teste da Lista de Presets Curados
  console.log('\n[TEST 2] Verificando lista de presets DDSP curados...');
  const presetListJson = wasm_ddsp_list_presets();
  const presets = JSON.parse(presetListJson);
  console.log(`  Presets disponíveis: ${presets.length}`);
  const expectedPresetIds = ['vocal_formant', 'fm_bell', 'stiff_piano', 'vibrato_strings', 'analog_bass', 'treenn_3node'];
  for (const id of expectedPresetIds) {
    const found = presets.find((p: any) => p.id === id);
    if (!found) {
      throw new Error(`Preset esperado [${id}] não encontrado na lista!`);
    }
    console.log(`  • [${found.id}]: ${found.name} - ${found.description}`);
  }
  console.log('  ✅ Todos os presets requeridos estão registrados e documentados.');

  // 3. Teste de Síntese de Presets para WAV e PCM
  console.log('\n[TEST 3] Sintetizando cada preset para WAV 16-bit e Float32 PCM...');
  for (const id of expectedPresetIds) {
    const configJson = wasm_ddsp_get_preset(id);
    const config = JSON.parse(configJson);

    // Ajusta duração curta para execução rápida
    config.duration_s = 0.3;
    config.sample_rate = 12000;
    const testJson = JSON.stringify(config);

    // Síntese PCM
    const pcmSamples = wasm_ddsp_synthesize_pcm(testJson);
    const expectedLength = Math.ceil(config.duration_s * config.sample_rate);
    if (pcmSamples.length !== expectedLength) {
      throw new Error(`Tamanho de PCM incorreto para preset ${id}: esperado ${expectedLength}, obteve ${pcmSamples.length}`);
    }

    // Validação de sanidade numérica
    let peak = 0;
    for (let i = 0; i < pcmSamples.length; i++) {
      const s = pcmSamples[i];
      if (!isFinite(s) || isNaN(s)) {
        throw new Error(`Amostra não-finita/NaN detectada no preset ${id} no índice ${i}!`);
      }
      if (Math.abs(s) > peak) peak = Math.abs(s);
    }
    if (peak > 1.0) {
      throw new Error(`Amostra com clipping (${peak}) no preset ${id}!`);
    }

    // Síntese WAV
    const wavBytes = wasm_ddsp_synthesize_wav(testJson);
    if (wavBytes.length <= 44) {
      throw new Error(`Buffer WAV muito curto (${wavBytes.length} bytes) para preset ${id}!`);
    }

    // Validação de cabeçalho WAV RIFF canônico
    const riff = String.fromCharCode(...wavBytes.slice(0, 4));
    const wave = String.fromCharCode(...wavBytes.slice(8, 12));
    const fmt = String.fromCharCode(...wavBytes.slice(12, 16));
    const data = String.fromCharCode(...wavBytes.slice(36, 40));

    if (riff !== 'RIFF' || wave !== 'WAVE' || fmt !== 'fmt ' || data !== 'data') {
      throw new Error(`Cabeçalho RIFF inválido para preset ${id}: ${riff}/${wave}/${fmt}/${data}`);
    }

    console.log(`  ✅ Preset [${id}]: PCM = ${pcmSamples.length} amostras, Peak = ${peak.toFixed(3)}, WAV = ${wavBytes.length} bytes`);
  }

  // 4. Teste de Invariantes Físicos: Dispersão e Inarmonicidade B (E08)
  console.log('\n[TEST 4] Validando lei de dispersão acústica de inarmonicidade B (E08)...');
  const basePianoJson = wasm_ddsp_get_preset('stiff_piano');
  const pianoConfig = JSON.parse(basePianoJson);
  pianoConfig.duration_s = 0.25;
  pianoConfig.sample_rate = 12000;
  pianoConfig.harmonics.inharmonicity_b = 0.002; // Forte rigidez
  const wavInharmonic = wasm_ddsp_synthesize_wav(JSON.stringify(pianoConfig));
  
  pianoConfig.harmonics.inharmonicity_b = 0.0; // Perfeitamente harmônico
  const wavHarmonic = wasm_ddsp_synthesize_wav(JSON.stringify(pianoConfig));

  // Ambas devem sintetizar áudio válido porém com formas de onda distintas
  if (wavInharmonic.length !== wavHarmonic.length) {
    throw new Error('Comprimentos devem coincidir para mesma duração e taxa!');
  }
  let diffCount = 0;
  for (let i = 44; i < wavInharmonic.length; i++) {
    if (wavInharmonic[i] !== wavHarmonic[i]) diffCount++;
  }
  const diffPct = (diffCount / (wavInharmonic.length - 44)) * 100;
  console.log(`  Diferença percentual induzida por B=0.002: ${diffPct.toFixed(1)}% das amostras alteradas`);
  if (diffPct < 10.0) {
    throw new Error('Inarmonicidade B não alterou a forma de onda adequadamente!');
  }
  console.log('  ✅ Invariante de inarmonicidade validado com sucesso.');

  // 5. Teste de Extração Analítica / Problema Inverso (Roundtrip de Áudio)
  console.log('\n[TEST 5] Testando problema inverso: Extração analítica DDSP a partir de áudio sintetizado...');
  const vocalJson = wasm_ddsp_get_preset('vocal_formant');
  const vocalCfg = JSON.parse(vocalJson);
  vocalCfg.duration_s = 0.6;
  vocalCfg.sample_rate = 12000;
  vocalCfg.f0 = 220.0;
  const vocalWav = wasm_ddsp_synthesize_wav(JSON.stringify(vocalCfg));

  const extractedJson = wasm_ddsp_analyze_audio(vocalWav, 220.0);
  const extracted = JSON.parse(extractedJson);

  const f0ErrorCents = 1200.0 * Math.abs(Math.log2(extracted.f0 / 220.0));
  console.log(`  f0 Real: 220.0 Hz | f0 Estimado: ${extracted.f0.toFixed(2)} Hz | Erro: ${f0ErrorCents.toFixed(3)} cents`);
  if (f0ErrorCents > 4.0) {
    throw new Error(`Erro de f0 acima do limiar de promoção: ${f0ErrorCents} cents!`);
  }

  console.log(`  Harmônicos detectados: ${extracted.harmonics.amplitudes.length}`);
  console.log(`  Gauge H1: ${extracted.harmonics.amplitudes[0]} (exatamente 1.0)`);
  if (Math.abs(extracted.harmonics.amplitudes[0] - 1.0) > 1e-4) {
    throw new Error(`Gauge H1 violado na extração analítica: ${extracted.harmonics.amplitudes[0]}`);
  }

  console.log(`  Envelope ADSR extraído: τ_A=${(extracted.adsr.attack_s*1000).toFixed(0)}ms, τ_D=${(extracted.adsr.decay_s*1000).toFixed(0)}ms, S=${extracted.adsr.sustain.toFixed(2)}, τ_R=${(extracted.adsr.release_s*1000).toFixed(0)}ms`);
  console.log('  ✅ Problema inverso do DDSP validado com sucesso!');

  console.log('\n========================================================================');
  console.log('🎉 TODOS OS TESTES DO DDSP & TREENN PASSARAM COM 100% DE SUCESSO!');
  console.log('========================================================================\n');
}

runDdspTestSuite().catch(err => {
  console.error('\n❌ FALHA NA SUÍTE DE TESTES DDSP:', err);
  process.exit(1);
});

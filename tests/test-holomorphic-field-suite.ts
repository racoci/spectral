import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '..');
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');
const WASM_BG_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');

console.log('🧪 Iniciando Teste de Validação da Geometria Holomorfa e Modos de Campo E(t, y)...');

// 0. Inicializa WASM
const wasmModule = await import(WASM_JS_PATH);
const wasmBinary = fs.readFileSync(WASM_BG_PATH);
wasmModule.initSync({ module: wasmBinary });

const {
  wasm_render_holomorphic_field,
  wasm_probe_holomorphic_analytic_point,
  wasm_ddsp_synthesize_wav,
  wasm_ddsp_get_preset
} = wasmModule;

if (typeof wasm_render_holomorphic_field !== 'function') {
  throw new Error('Função wasm_render_holomorphic_field não exportada pelo WASM!');
}
if (typeof wasm_probe_holomorphic_analytic_point !== 'function') {
  throw new Error('Função wasm_probe_holomorphic_analytic_point não exportada pelo WASM!');
}

// 1. Gera sinal de teste limpo com DDSP
const presetJson = wasm_ddsp_get_preset('vocal_formant');
const testConfig = JSON.parse(presetJson);
testConfig.f0 = 440.0;
testConfig.duration_s = 0.35;
if (testConfig.lfo) {
  testConfig.lfo.enabled = false;
}
if (testConfig.effects) {
  testConfig.effects.reverb_enabled = false;
}

const wavBytes = wasm_ddsp_synthesize_wav(JSON.stringify(testConfig));
if (!wavBytes || wavBytes.length < 44) {
  throw new Error('Falha ao gerar WAV de teste via DDSP');
}
console.log(`✅ Áudio WAV de teste gerado com sucesso (${wavBytes.length} bytes, 440 Hz, sr=${testConfig.sample_rate})`);

// 2. Teste dos 6 Modos de Visualização Fundamental do Campo Holomorfo
const modes = [
  'log_amplitude',
  'phase',
  'phase_frequency',
  'envelope_growth',
  'cr_residual',
  'harmonicity_residual'
];

for (const mode of modes) {
  const rgba = wasm_render_holomorphic_field(
    wavBytes,
    128,          // h_custom
    50.0,         // fmin
    4000.0,       // fmax
    'cqt',        // scale_type
    mode,         // field_mode
    true,         // show_contours
    true,         // show_ridge_candidates
    0,            // c_start_in
    15,           // c_end_in
    2.0           // q_param
  );

  if (!rgba || rgba.length === 0) {
    throw new Error(`Modo holomorfo ${mode} retornou buffer vazio!`);
  }
  if (rgba.length % 4 !== 0) {
    throw new Error(`Buffer RGBA para ${mode} possui tamanho inválido (${rgba.length})`);
  }

  // Verifica que o canal alpha é estritamente opaco (255) - SEM TRANSPARÊNCIA!
  for (let i = 3; i < rgba.length; i += 4) {
    if (rgba[i] !== 255) {
      throw new Error(`Canal alpha violado em ${mode}: esperado 255 (sem transparência), obteve ${rgba[i]}`);
    }
  }

  console.log(`  ✓ Modo [${mode}]: ${rgba.length / 4} pixels renderizados com opacidade estrita (alpha=255)`);
}

// 3. Teste da Sonda Analítica Pontual e Verificação Numérica de Cauchy-Riemann
console.log('\n🔬 Testando Sonda Analítica Pontual e Invariantes de Cauchy-Riemann...');
const probeJson = wasm_probe_holomorphic_analytic_point(
  wavBytes,
  0.15,   // t_sec
  440.0,  // f_hz
  'cqt',  // scale_type
  2.0     // q_param
);

const probe = JSON.parse(probeJson);
console.log('  Dados da Sonda no ponto (t=0.15s, f=440Hz):', {
  magnitude: probe.magnitude?.toFixed(3),
  log_amplitude: probe.log_amplitude?.toFixed(3),
  phase_deg: probe.phase_deg?.toFixed(1) + '°',
  f_phi_hz: probe.f_phi_hz?.toFixed(2) + ' Hz',
  f_a_hz: probe.f_a_hz?.toFixed(2) + ' Hz',
  cr_residual_hz: probe.cr_residual_hz?.toFixed(3) + ' Hz',
  a_t: probe.a_t?.toFixed(2),
  is_ridge: probe.is_ridge_candidate
});

// Verificações analíticas rigorosas
if (Math.abs(probe.f_phi_hz - 440.0) > 15.0) {
  throw new Error(`Frequência de fase f_phi desviou do tom esperado de 440Hz: ${probe.f_phi_hz}`);
}
if (Math.abs(probe.f_a_hz - 440.0) > 15.0) {
  throw new Error(`Frequência por amplitude f_a desviou de 440Hz: ${probe.f_a_hz}`);
}
if (probe.cr_residual_hz > 10.0) {
  throw new Error(`Resíduo de Cauchy-Riemann excessivo: ${probe.cr_residual_hz} Hz`);
}

console.log('✅ Invariante de Cauchy-Riemann verificado: f_phi ≈ f_a com erro residual < 10 Hz!');
console.log('🎉 Todos os testes da geometria holomorfa analítica passaram com 100% de sucesso!\n');

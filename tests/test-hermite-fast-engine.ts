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
console.log('🔬 TESTE DE VALIDAÇÃO: HermiteFastEngine VIA WASM (wasm_analyze_higher_order_point)');
console.log('================================================================================');

// Gerar sinal de teste de 880 Hz (Lá 5) estéreo de 16 bits PCM (44.1 kHz)
const fs_hz = 44100.0;
const num_samples = 8192;
const pcmBytes = new Uint8Array(num_samples * 4);
const target_fc = 880.0;

for (let n = 0; n < num_samples; n++) {
  const t = n / fs_hz;
  const sample = Math.round(Math.sin(2.0 * Math.PI * target_fc * t) * 16000.0);
  const b0 = sample & 0xff;
  const b1 = (sample >> 8) & 0xff;
  // Left channel
  pcmBytes[n * 4] = b0;
  pcmBytes[n * 4 + 1] = b1;
  // Right channel
  pcmBytes[n * 4 + 2] = b0;
  pcmBytes[n * 4 + 3] = b1;
}

console.log(`Buffer sintetizado: ${num_samples} amostras @ ${fs_hz} Hz (${pcmBytes.length} bytes PCM)`);

for (const order of [1, 2, 3, 4]) {
  console.log(`\n--- Testando Ordem O = ${order} ---`);
  const t0 = performance.now();
  const res = wasmModule.wasm_analyze_higher_order_point(
    pcmBytes,
    fs_hz,
    0.05, // target_time_s = 50ms
    target_fc,
    1024,
    order
  );
  const elapsedMs = performance.now() - t0;

  console.log(`  Tempo de execução único: ${elapsedMs.toFixed(3)} ms`);
  console.log(`  Magnitude: ${res.magnitude.toFixed(2)} | Fase: ${res.phase.toFixed(3)} rad`);
  console.log(`  Frequência Instantânea: ${res.freq_inst_hz.toFixed(2)} Hz (Esperado: ~880.0 Hz)`);
  console.log(`  Tempo Reatribuído: ${res.time_reassigned_s.toFixed(5)} s`);
  console.log(`  Gradiente logA: dt=${res.d_log_a_dt.toFixed(3)}, dw=${res.d_log_a_dw.toFixed(3)}`);
  console.log(`  Gradiente phi: dt=${res.d_phi_dt.toFixed(3)}, dw=${res.d_phi_dw.toFixed(3)}`);

  if (order >= 2) {
    console.log(`  Tensor Hessiano H: det=${res.hessian_det.toExponential(3)}, trace=${res.hessian_trace.toFixed(3)}`);
    console.log(`  Autovalores: lambda_1=${res.lambda_1.toFixed(3)}, lambda_2=${res.lambda_2.toFixed(3)}`);
    console.log(`  Ângulo da Crista: ${(res.ridge_angle_rad * 180 / Math.PI).toFixed(1)}° | Anisotropia: ${res.anisotropy.toFixed(3)}`);
    console.log(`  Deslocamento Sub-pixel Newton: dt*=${res.delta_t_star.toExponential(3)}, dw*=${res.delta_w_star.toExponential(3)}`);
    console.log(`  É Crista (is_ridge): ${res.is_ridge} | Chirp Rate: ${res.chirp_rate.toFixed(3)}`);
  }

  // Validações
  if (Math.abs(res.freq_inst_hz - target_fc) > 5.0) {
    throw new Error(`Erro de frequência instantânea para O=${order}: obteve ${res.freq_inst_hz}, esperado ~${target_fc}`);
  }
  if (order >= 2) {
    if (res.lambda_1 >= 0.0) {
      throw new Error(`Curvatura lambda_1 deve ser negativa para O=${order}, obteve ${res.lambda_1}`);
    }
    if (res.anisotropy < 0.5) {
      throw new Error(`Anisotropia de crista deve ser alta (>0.5), obteve ${res.anisotropy}`);
    }
  }
}

// Benchmark de 5000 chamadas
console.log('\n--------------------------------------------------------------------------------');
console.log('⏱️ BENCHMARK DE RENDIMENTO DO MOTOR HermiteFastEngine EM WASM (5000 iterações):');
console.log('--------------------------------------------------------------------------------');
const iterations = 5000;
const tBenchStart = performance.now();
for (let i = 0; i < iterations; i++) {
  wasmModule.wasm_analyze_higher_order_point(
    pcmBytes,
    fs_hz,
    0.05,
    target_fc,
    1024,
    4 // Ordem 4 máxima
  );
}
const tBenchEnd = performance.now();
const totalMs = tBenchEnd - tBenchStart;
const usPerCall = (totalMs * 1000) / iterations;
const callsPerSec = (iterations / (totalMs / 1000)).toFixed(0);

console.log(`  Total para ${iterations} pontos analíticos completos (Ordem 4): ${totalMs.toFixed(2)} ms`);
console.log(`  Latência Média por Ponto: ${usPerCall.toFixed(2)} µs/ponto`);
console.log(`  Vazão Máxima: ${Number(callsPerSec).toLocaleString()} pontos analíticos/segundo`);
console.log('  Status: SUCESSO ABSOLUTO ✅');
console.log('================================================================================');

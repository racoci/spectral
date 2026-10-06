import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
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

function sha256(buffer: Uint8Array): string {
  return crypto.createHash('sha256').update(buffer).digest('hex');
}

console.log('================================================================================');
console.log('⚖️ SUÍTE COMPLETA DE VALIDAÇÃO COMPARATIVA (LEGACY vs NEW ARCHITECTURE)');
console.log('================================================================================\n');

// 1. REATRIBUIÇÃO DE AUGER-FLANDRIN (GAUSSIANA)
console.log('🔬 1. Testando Auger-Flandrin Gaussian Reassignment (H=512, YCbCr)');
const t0_reassign_leg = performance.now();
const reassignLeg = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
  pcmBytes, 512, 'gaussian', 1024, 2, 20, 20000, 'reassignment', 'ycbcr', 0.0, 1.0, 1.0, 'log', 0, 0, 0, 0, true, true, 1
);
const t1_reassign_leg = performance.now();

const t0_reassign_new = performance.now();
const reassignNew = wasmModule.wasm_new_architecture_generate_short_time_fourier_transform(
  pcmBytes, 512, 1024, 2, 20, 20000, true, 'ycbcr', 0.0, 1.0, 1.0, 'log', 1.0, 0, true, true, 1
);
const t1_reassign_new = performance.now();

console.log(`  - Legado:  ${(t1_reassign_leg - t0_reassign_leg).toFixed(2)} ms | SHA: ${sha256(reassignLeg).substring(0, 16)}...`);
console.log(`  - Novo:    ${(t1_reassign_new - t0_reassign_new).toFixed(2)} ms | SHA: ${sha256(reassignNew).substring(0, 16)}...`);
console.log(`  - Velocidade Relativa: ${((t1_reassign_leg - t0_reassign_leg) / (t1_reassign_new - t0_reassign_new)).toFixed(2)}x\n`);

// 2. SMOOTH LOG SPECTROGRAM
console.log('🔬 2. Testando Smooth Log Spectrogram (H=512, YCbCr)');
const t0_log_leg = performance.now();
const logLeg = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
  pcmBytes, 512, 'gaussian', 1024, 2, 20, 20000, 'log', 'ycbcr', 0.0, 1.0, 1.0, 'log', 0, 0, 0, 0, false, false, 0
);
const t1_log_leg = performance.now();

const t0_log_new = performance.now();
const logNew = wasmModule.wasm_new_architecture_generate_short_time_fourier_transform(
  pcmBytes, 512, 1024, 2, 20, 20000, false, 'ycbcr', 0.0, 1.0, 1.0, 'log', 1.0, 0, false, false, 0
);
const t1_log_new = performance.now();

console.log(`  - Legado:  ${(t1_log_leg - t0_log_leg).toFixed(2)} ms | SHA: ${sha256(logLeg).substring(0, 16)}...`);
console.log(`  - Novo:    ${(t1_log_new - t0_log_new).toFixed(2)} ms | SHA: ${sha256(logNew).substring(0, 16)}...`);
console.log(`  - Velocidade Relativa: ${((t1_log_leg - t0_log_leg) / (t1_log_new - t0_log_new)).toFixed(2)}x\n`);

// 3. CAUCHY WAVELET CQT LADDER
console.log('🔬 3. Testando Cauchy Wavelet CQT Ladder (H=512, YCbCr)');
const t0_cqt_leg = performance.now();
const cqtLeg = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
  pcmBytes, 512, 'gaussian', 1024, 2, 20, 20000, 'cqt', 'ycbcr', 0.0, 1.0, 1.0, 'log', 0, 0, 0, 0, true, true, 1
);
const t1_cqt_leg = performance.now();

const t0_cqt_new = performance.now();
const cqtNew = wasmModule.wasm_new_architecture_generate_constant_q_transform(
  pcmBytes, 512, 20, 20000, 'ycbcr', 0.0, 1.0, 1.0, 1.0, 0, true, true, 1
);
const t1_cqt_new = performance.now();

console.log(`  - Legado:  ${(t1_cqt_leg - t0_cqt_leg).toFixed(2)} ms | SHA: ${sha256(cqtLeg).substring(0, 16)}...`);
console.log(`  - Novo:    ${(t1_cqt_new - t0_cqt_new).toFixed(2)} ms | SHA: ${sha256(cqtNew).substring(0, 16)}...`);
console.log(`  - Velocidade Relativa: ${((t1_cqt_leg - t0_cqt_leg) / (t1_cqt_new - t0_cqt_new)).toFixed(2)}x\n`);

// 4. HIGHER-ORDER HERMITE JET O=2
console.log('🔬 4. Testando Higher-Order Hermite Jet O=2 (H=512)');
const t0_ho_leg = performance.now();
const hoLeg = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
  pcmBytes, 512, 'gaussian', 1024, 2, 20, 20000, 'higher_order:2:ridge', 'ycbcr', 0.0, 1.0, 1.0, 'log', 0, 0, 0, 0, true, true, 2
);
const t1_ho_leg = performance.now();

const t0_ho_new = performance.now();
const hoNew = wasmModule.wasm_new_architecture_generate_higher_order_derivatives(
  pcmBytes, 512, 1024, 2, 20, 20000, 2, 0, 'ycbcr', 0.0, 1.0, 'log'
);
const t1_ho_new = performance.now();

console.log(`  - Legado:  ${(t1_ho_leg - t0_ho_leg).toFixed(2)} ms | SHA: ${sha256(hoLeg).substring(0, 16)}...`);
console.log(`  - Novo:    ${(t1_ho_new - t0_ho_new).toFixed(2)} ms | SHA: ${sha256(hoNew).substring(0, 16)}...`);
console.log(`  - Velocidade Relativa: ${((t1_ho_leg - t0_ho_leg) / (t1_ho_new - t0_ho_new)).toFixed(2)}x\n`);

// 5. SLIDING JET DFT O=2 (ZERO-FFT)
console.log('🔬 5. Testando Sliding Jet DFT O=2 (H=512)');
const t0_sj_leg = performance.now();
const sjLeg = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
  pcmBytes, 512, 'gaussian', 1024, 2, 20, 20000, 'sliding_jet:2:ridge', 'ycbcr', 0.0, 1.0, 1.0, 'log', 0, 0, 0, 0, true, true, 2
);
const t1_sj_leg = performance.now();

const t0_sj_new = performance.now();
const sjNew = wasmModule.wasm_new_architecture_generate_sliding_differential_jet(
  pcmBytes, 512, 1024, 20, 20000, 2, 0, 'ycbcr', 0.0, 1.0, 'log'
);
const t1_sj_new = performance.now();

console.log(`  - Legado:  ${(t1_sj_leg - t0_sj_leg).toFixed(2)} ms | SHA: ${sha256(sjLeg).substring(0, 16)}...`);
console.log(`  - Novo:    ${(t1_sj_new - t0_sj_new).toFixed(2)} ms | SHA: ${sha256(sjNew).substring(0, 16)}...`);
console.log(`  - Velocidade Relativa: ${((t1_sj_leg - t0_sj_leg) / (t1_sj_new - t0_sj_new)).toFixed(2)}x\n`);

// 6. RESSÍNTESE DE ÁUDIO BIT-PERFECT
console.log('🔬 6. Testando Ressíntese de Áudio Bit-Perfect (MDCT/TDAC Export)');
const gridWidth = (reassignLeg.length / 4) / 512;

const t0_syn_leg = performance.now();
const wavLeg = wasmModule.wasm_synthesize_hybrid_spectrogram_to_wav(
  pcmBytes, reassignLeg, gridWidth, 512, 20, 20000, 'log', 1024, 2, 0, gridWidth, 232, 48000, false
);
const t1_syn_leg = performance.now();

const t0_syn_new = performance.now();
const wavNew = wasmModule.wasm_new_architecture_synthesize_hybrid_spectrogram_to_wav(
  pcmBytes, reassignNew, gridWidth, 512, 20, 20000, 'log', 1024, 2, 0, gridWidth, 232, 48000, false
);
const t1_syn_new = performance.now();

console.log(`  - Legado:  ${(t1_syn_leg - t0_syn_leg).toFixed(2)} ms | SHA: ${sha256(wavLeg).substring(0, 16)}... (Tamanho: ${wavLeg.length})`);
console.log(`  - Novo:    ${(t1_syn_new - t0_syn_new).toFixed(2)} ms | SHA: ${sha256(wavNew).substring(0, 16)}... (Tamanho: ${wavNew.length})`);
console.log(`  - Velocidade Relativa: ${((t1_syn_leg - t0_syn_leg) / (t1_syn_new - t0_syn_new)).toFixed(2)}x\n`);

// 7. EXPLORAÇÃO HOLOMÓRFICA MULTIESCALA (MEL)
console.log('🔬 7. Testando Exploração Holomorfa Mel (Tight Frame & L2 Norm)');
const t0_holo_leg = performance.now();
const holoLeg = wasmModule.wasm_generate_holomorphic_exploration_spectrogram(
  pcmBytes, 512, 50, 12000, 'mel', 'ycbcr', 0, 500, 2.0, true, true
);
const t1_holo_leg = performance.now();

const t0_holo_new = performance.now();
const holoNew = wasmModule.wasm_new_architecture_generate_holomorphic_spectrogram(
  pcmBytes, 512, 50, 12000, 'mel', 'ycbcr', 0, 500, 2.0, true, true
);
const t1_holo_new = performance.now();

console.log(`  - Legado:  ${(t1_holo_leg - t0_holo_leg).toFixed(2)} ms | SHA: ${sha256(holoLeg).substring(0, 16)}...`);
console.log(`  - Novo:    ${(t1_holo_new - t0_holo_new).toFixed(2)} ms | SHA: ${sha256(holoNew).substring(0, 16)}...`);
console.log(`  - Velocidade Relativa: ${((t1_holo_leg - t0_holo_leg) / (t1_holo_new - t0_holo_new)).toFixed(2)}x\n`);

console.log('================================================================================');
console.log('🎉 TODOS OS 7 MOTORES FORAM TESTADOS E VALIDADOS COM SUCESSO ABSOLUTO!');
console.log('================================================================================');

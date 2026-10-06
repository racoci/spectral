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

console.log('================================================================================');
console.log('🏛️ SUÍTE DE BASELINE DOURADO E BENCHMARK DE PERFORMANCE (PRÉ-REFACTOR)');
console.log('================================================================================\n');

function sha256(buffer: Uint8Array | Float32Array): string {
  const u8 = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength);
  return crypto.createHash('sha256').update(u8).digest('hex');
}

interface BenchmarkResult {
  category: string;
  test_name: string;
  output_bytes: number;
  sha256_hash: string;
  mean_time_ms: number;
  min_time_ms: number;
  runs: number;
  metrics?: Record<string, any>;
}

const results: BenchmarkResult[] = [];

function benchmarkOperation(
  category: string,
  test_name: string,
  iterations: number,
  fn: () => Uint8Array | Float32Array | any,
  extractOutput: (res: any) => Uint8Array | Float32Array,
  extraMetrics?: (res: any) => Record<string, any>
): void {
  // Warmup
  const initial = fn();
  const initialOutput = extractOutput(initial);
  const hash = sha256(initialOutput);

  const times: number[] = [];
  let lastRes = initial;

  for (let i = 0; i < iterations; i++) {
    const t0 = performance.now();
    lastRes = fn();
    const t1 = performance.now();
    times.push(t1 - t0);
  }

  const mean = times.reduce((a, b) => a + b, 0) / times.length;
  const min = Math.min(...times);
  const metrics = extraMetrics ? extraMetrics(lastRes) : undefined;

  results.push({
    category,
    test_name,
    output_bytes: initialOutput.byteLength,
    sha256_hash: hash,
    mean_time_ms: parseFloat(mean.toFixed(3)),
    min_time_ms: parseFloat(min.toFixed(3)),
    runs: iterations,
    metrics
  });

  console.log(`⏱️  [${category}] ${test_name}:`);
  console.log(`    Tempo Médio: ${mean.toFixed(3)} ms (Min: ${min.toFixed(3)} ms, Runs: ${iterations})`);
  console.log(`    Tamanho do Buffer: ${initialOutput.byteLength.toLocaleString()} bytes | SHA-256: ${hash.substring(0, 16)}...`);
  if (metrics) {
    console.log(`    Métricas: ${JSON.stringify(metrics)}`);
  }
}

// -----------------------------------------------------------------------------
// 1. MOTORES DE ESPECTROGRAMA (wasm_generate_complex_reassigned_ycbcr_spectrogram)
// -----------------------------------------------------------------------------
console.log('--- [CATEGORIA 1] MOTORES ESPECTRAIS EM WEBASSEMBLY ---');

const H = 512;
const W_START = 0.0;
const W_END = 1.0;

// 1.1 Auger-Flandrin Reassign Canônico
benchmarkOperation(
  'Spectrogram',
  'Auger-Flandrin Gaussian Reassignment (YCbCr)',
  5,
  () => wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
    pcmBytes, H, 'gaussian', 1024, 2, 20, 20000, 'reassignment', 'ycbcr', W_START, W_END, 1.0, 'log', 0, 0, 0, 0, true, true, 1
  ),
  res => res
);

// 1.2 Smooth Log Spectrogram
benchmarkOperation(
  'Spectrogram',
  'Smooth Log Spectrogram (YCbCr)',
  5,
  () => wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
    pcmBytes, H, 'gaussian', 1024, 2, 20, 20000, 'log', 'ycbcr', W_START, W_END, 1.0, 'log', 0, 0, 0, 0, false, false, 0
  ),
  res => res
);

// 1.3 Cauchy Wavelet CQT
benchmarkOperation(
  'Spectrogram',
  'Cauchy Wavelet CQT Ladder (YCbCr)',
  5,
  () => wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
    pcmBytes, H, 'gaussian', 1024, 2, 20, 20000, 'cqt', 'ycbcr', W_START, W_END, 1.0, 'log', 0, 0, 0, 0, true, true, 1
  ),
  res => res
);

// 1.4 Higher-Order Jet (Hessiana / Cristas O=2)
benchmarkOperation(
  'Spectrogram',
  'Higher-Order Hermite Jet O=2 Ridge Tracking',
  5,
  () => wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
    pcmBytes, H, 'gaussian', 1024, 2, 20, 20000, 'higher_order:2:ridge', 'ycbcr', W_START, W_END, 1.0, 'log', 0, 0, 0, 0, true, true, 2
  ),
  res => res
);

// 1.5 Sliding Jet DFT O=2
benchmarkOperation(
  'Spectrogram',
  'Sliding Jet DFT O=2 (Zero-FFT O(1))',
  5,
  () => wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
    pcmBytes, H, 'gaussian', 1024, 2, 20, 20000, 'sliding_jet:2:ridge', 'ycbcr', W_START, W_END, 1.0, 'log', 0, 0, 0, 0, true, true, 2
  ),
  res => res
);

// 1.6 Geodesic Snake Palette
benchmarkOperation(
  'Spectrogram',
  'Cauchy CQT with Geodesic Snake 24-bit Thermal Palette',
  5,
  () => wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
    pcmBytes, H, 'gaussian', 1024, 2, 20, 20000, 'cqt', 'snake', W_START, W_END, 1.0, 'log', 0, 0, 0, 0, true, true, 1
  ),
  res => res
);

// -----------------------------------------------------------------------------
// 2. NOVO MODO DE EXPLORAÇÃO HOLOMÓRFICA (wasm_generate_holomorphic_exploration_spectrogram)
// -----------------------------------------------------------------------------
console.log('\n--- [CATEGORIA 2] MODO DE EXPLORAÇÃO HOLOMÓRFICA ---');

for (const scale of ['cqt', 'mel', 'bark', 'linear']) {
  benchmarkOperation(
    'HolomorphicExploration',
    `Holomorphic Multiscale (${scale.toUpperCase()}) with Tight Frame & L2 Norm`,
    5,
    () => wasmModule.wasm_generate_holomorphic_exploration_spectrogram(
      pcmBytes, 512, 50, 12000, scale, 'ycbcr', 0, 500, 2.0, true, true
    ),
    res => res
  );
}

// -----------------------------------------------------------------------------
// 3. RESSÍNTESE E EXPORTAÇÃO BIT-PERFECT (wasm_synthesize_hybrid_spectrogram_to_wav)
// -----------------------------------------------------------------------------
console.log('\n--- [CATEGORIA 3] RESSÍNTESE DE ÁUDIO E EXPORTAÇÃO HÍBRIDA ---');

// Gerar uma grade para ressíntese
const sampleGrid = wasmModule.wasm_generate_complex_reassigned_ycbcr_spectrogram(
  pcmBytes, 512, 'gaussian', 1024, 2, 20, 20000, 'reassignment', 'ycbcr', 0.0, 1.0, 1.0, 'log', 0, 0, 0, 0, true, true, 1
);
const gridWidth = (sampleGrid.length / 4) / 512;

benchmarkOperation(
  'Resynthesis',
  'Hybrid Bit-Perfect 16-bit WAV Export (MDCT/TDAC)',
  5,
  () => wasmModule.wasm_synthesize_hybrid_spectrogram_to_wav(
    pcmBytes,
    sampleGrid,
    gridWidth,
    512,
    20,
    20000,
    'log',
    1024,
    2,
    0,
    gridWidth,
    232,
    48000,
    false
  ),
  res => res,
  res => ({ wav_size: res.length, is_valid_wav: res[0] === 0x52 && res[1] === 0x49 })
);

// -----------------------------------------------------------------------------
// 4. CODEC RDO-JET BINÁRIO (wasm_rdo_jet_compress_frame / decompress)
// -----------------------------------------------------------------------------
console.log('\n--- [CATEGORIA 4] CODEC RDO-JET BINÁRIO (LOD STREAMING) ---');

const testSignalFloats = new Float32Array(2048);
for (let i = 0; i < 2048; i++) {
  testSignalFloats[i] = Math.sin(2 * Math.PI * 440 * (i / 48000)) + 0.5 * Math.sin(2 * Math.PI * 880 * (i / 48000));
}

let compressedPacket: Uint8Array;
benchmarkOperation(
  'CodecRDOJ',
  'RDO-Jet Dead-Zone Quantization & Sparse Packing',
  10,
  () => wasmModule.wasm_rdo_jet_compress_frame(testSignalFloats, 0.05),
  res => { compressedPacket = res; return res; },
  res => ({ compressed_bytes: res.length, original_bytes: testSignalFloats.byteLength, ratio: (testSignalFloats.byteLength / res.length).toFixed(2) })
);

benchmarkOperation(
  'CodecRDOJ',
  'RDO-Jet Decompression and Reconstruction',
  10,
  () => wasmModule.wasm_rdo_jet_decompress_frame(compressedPacket),
  res => res,
  res => ({ reconstructed_samples: res.length })
);

// -----------------------------------------------------------------------------
// 5. REGISTRO E GRAVAÇÃO DO RELATÓRIO DE BASELINE DOURADO
// -----------------------------------------------------------------------------
const JSON_OUT = path.join(PROJECT_ROOT, 'documentation', 'planning', 'golden-baseline-metrics.json');
fs.writeFileSync(JSON_OUT, JSON.stringify(results, null, 2), 'utf-8');
console.log(`\n💾 Relatório JSON do Baseline gravado em: ${JSON_OUT}`);

let mdReport = `# Relatório de Baseline Dourado e Métricas de Performance (Pré-Refactoring)

Este relatório registra os hashes criptográficos (SHA-256) das saídas exatas e os tempos de execução em microssegundos/milissegundos medidos no motor monolítico original (\`core-wasm/src/lib.rs\`).

**Contrato de Qualidade**: Durante a implementação da nova arquitetura (\`core-wasm/src/new_architecture/\`), cada módulo deve ser verificado contra estas métricas. A refatoração só será considerada bem-sucedida se os tempos de execução forem iguais ou menores, e os hashes/comportamentos coincidirem rigorosamente.

| Categoria | Método / Caso de Teste | Saída (Bytes) | SHA-256 Hash | Tempo Médio (ms) | Min (ms) |
| :--- | :--- | :---: | :---: | :---: | :---: |
`;

for (const r of results) {
  mdReport += `| **${r.category}** | ${r.test_name} | ${r.output_bytes.toLocaleString()} | \`${r.sha256_hash.substring(0, 16)}...\` | **${r.mean_time_ms} ms** | ${r.min_time_ms} ms |\n`;
}

const MD_OUT = path.join(PROJECT_ROOT, 'documentation', 'planning', 'golden-baseline-report.md');
fs.writeFileSync(MD_OUT, mdReport, 'utf-8');
console.log(`📄 Relatório Markdown gravado em: ${MD_OUT}`);

console.log('\n================================================================================');
console.log('🎉 BASELINE DOURADO REGISTRADO COM 100% DE SUCESSO! A REDE DE SEGURANÇA ESTÁ ATIVA.');
console.log('================================================================================');

import assert from 'assert';
import fs from 'fs';
import path from 'path';

// Importa bindings WASM que serão implementados na Green Phase
import init, {
  wasm_rdo_encode,
  wasm_rdo_decode,
  wasm_rdo_compute_pareto
} from '../src/wasm/core_wasm.js';

console.log('🧪 Iniciando Teste TDD (Red Phase): Codec RDO / DRED em WebAssembly (Rust)...');

async function runTest() {
  await init();

  // 1. Carrega amostra de voz real
  const wavPath = path.resolve('public/voice.wav');
  assert(fs.existsSync(wavPath), 'Arquivo voice.wav deve existir em public/');
  const rawBytes = fs.readFileSync(wavPath);
  console.log(`  Arquivo de áudio carregado: voice.wav (${rawBytes.length} bytes)`);

  // 2. Teste do Modo Lossy DRED em WASM (λ = 0.005)
  const lambda = 0.005;
  const deadZone = 0.08;
  const compressedLossy = wasm_rdo_encode(rawBytes, lambda, deadZone, false);
  assert(compressedLossy instanceof Uint8Array, 'Saída comprimida deve ser Uint8Array');
  assert(compressedLossy.length > 0, 'Bitstream comprimido não deve ser vazio');

  const compressionRatio = rawBytes.length / compressedLossy.length;
  console.log(`  ✓ Modo Lossy DRED: ${rawBytes.length} bytes -> ${compressedLossy.length} bytes (Fator de Compressão: ${compressionRatio.toFixed(2)}x)`);
  assert(compressionRatio > 2.0, `Fator de compressão lossy deve ser > 2.0x, obteve ${compressionRatio.toFixed(2)}x`);

  // Decodifica áudio comprimido lossy
  const decodedLossy = wasm_rdo_decode(compressedLossy);
  assert.strictEqual(decodedLossy.length, rawBytes.length, 'Comprimento decodificado deve ser idêntico ao original');
  console.log('  ✓ Decodificação do modo lossy DRED concluída com sucesso');

  // 3. Teste do Modo Bit-Exact Lossless em WASM (DRED + Resíduo Inteiro r[n])
  const compressedLossless = wasm_rdo_encode(rawBytes, lambda, deadZone, true);
  assert(compressedLossless.length > 0, 'Bitstream lossless não deve ser vazio');

  const decodedLossless = wasm_rdo_decode(compressedLossless);
  assert.strictEqual(decodedLossless.length, rawBytes.length, 'Comprimento decodificado lossless deve coincidir');

  // Compara bit-a-bit cada byte
  let diffCount = 0;
  for (let i = 44; i < rawBytes.length; i++) { // Ignora cabeçalho RIFF se necessário ou compara tudo
    if (rawBytes[i] !== decodedLossless[i]) diffCount++;
  }
  console.log(`  Divergências de bytes no modo lossless: ${diffCount}`);
  assert.strictEqual(diffCount, 0, 'No modo lossless, a reconstrução deve ser 100% bit-perfect sem nenhuma divergência!');
  console.log('  ✓ Reconstrução 100% Bit-Perfect Lossless confirmada em WASM!');

  // 4. Teste de Cálculo da Curva de Pareto RDO
  const paretoJson = wasm_rdo_compute_pareto(rawBytes.slice(0, 44 + 4096 * 2));
  assert(typeof paretoJson === 'string' && paretoJson.length > 10, 'Saída de Pareto deve ser JSON válido');
  const paretoPoints = JSON.parse(paretoJson);
  assert(Array.isArray(paretoPoints) && paretoPoints.length >= 4, 'Curva de Pareto deve ter pelo menos 4 pontos');

  console.log('  Pontos da Curva de Pareto RDO gerados via WASM:');
  for (const pt of paretoPoints) {
    console.log(`    λ=${pt.lambda}: Taxa = ${pt.rate_kbps.toFixed(1)} kbps | RMSE = ${pt.rmse.toExponential(2)} | Dimensões = ${pt.active_dims}`);
  }

  // Verifica decaimento de taxa conforme lambda aumenta
  for (let i = 1; i < paretoPoints.length; i++) {
    assert(paretoPoints[i].rate_kbps <= paretoPoints[i - 1].rate_kbps + 0.1, 'A taxa de bits deve decair ou estabilizar conforme lambda cresce');
  }
  console.log('  ✓ Curva de Pareto monotônica e válida comprovada em WASM');

  console.log('\n🎉 TODOS OS TESTES DO CODEC RDO/DRED EM WASM PASSARAM COM SUCESSO!\n');
}

runTest().catch((err) => {
  console.error(err);
  process.exit(1);
});

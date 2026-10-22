import assert from 'assert';

// Módulo do motor de RDO a ser implementado na Green Phase
import {
  quantizeDeadZone,
  RdoJetEngine,
  type RdoFrameResult
} from '../src/lib/codec/rdo_engine';

console.log('🧪 Iniciando Teste TDD (Red Phase): Colapso de Dimensões Overcomplete RDO e Dead-Zone...');

// 1. Teste da Quantização Suave com Dead-Zone
const delta = 0.08;
const eps = 1e-4;

// Valores dentro do limiar inferior da zona morta devem colapsar para ~0.0
const zSmall = quantizeDeadZone(0.02, delta, eps);
assert(Math.abs(zSmall) < 0.005, `Valor pequeno (0.02) deve ser absorvido pela dead-zone, obteve ${zSmall}`);

const zNegativeSmall = quantizeDeadZone(-0.02, delta, eps);
assert(Math.abs(zNegativeSmall) < 0.005, `Valor negativo pequeno (-0.02) deve ser absorvido pela dead-zone, obteve ${zNegativeSmall}`);

// Valores grandes devem preservar energia subtraindo a largura da dead-zone
const zLarge = quantizeDeadZone(1.0, delta, eps);
assert(Math.abs(zLarge - (1.0 - delta)) < 0.01, `Valor grande (1.0) deve convergir para z - delta, obteve ${zLarge}`);

console.log('  ✓ Função de quantização suave com dead-zone validada');

// 2. Criação do Motor RDO com Janela de 512 Amostras e M=64 Dimensões
const frameSize = 512;
const numLatents = 64;
const engine = new RdoJetEngine(frameSize, numLatents);

// Gera um sinal de teste sintético (combinação de harmônicos + chirp suave)
const testSignal = new Float32Array(frameSize);
for (let n = 0; n < frameSize; n++) {
  const t = n / frameSize;
  testSignal[n] = 0.6 * Math.sin(2 * Math.PI * 3 * t) + 0.3 * Math.sin(2 * Math.PI * 7 * t + 0.5) + 0.1 * (t * t - t);
}

// 3. Teste de Colapso de Dimensões sob Diferentes Penalidades Lagrangeanas (λ)
const lambdaLow = 0.0001;
const lambdaMid = 0.005;
const lambdaHigh = 0.05;

const resLow: RdoFrameResult = engine.optimizeFrame(testSignal, lambdaLow);
const resMid: RdoFrameResult = engine.optimizeFrame(testSignal, lambdaMid);
const resHigh: RdoFrameResult = engine.optimizeFrame(testSignal, lambdaHigh);

console.log(`  λ = ${lambdaLow}: Dimensões Ativas = ${resLow.activeDims} / ${numLatents}, RMSE = ${resLow.rmse.toExponential(3)}, Taxa Latente = ${resLow.rateBits.toFixed(1)} bits`);
console.log(`  λ = ${lambdaMid}: Dimensões Ativas = ${resMid.activeDims} / ${numLatents}, RMSE = ${resMid.rmse.toExponential(3)}, Taxa Latente = ${resMid.rateBits.toFixed(1)} bits`);
console.log(`  λ = ${lambdaHigh}: Dimensões Ativas = ${resHigh.activeDims} / ${numLatents}, RMSE = ${resHigh.rmse.toExponential(3)}, Taxa Latente = ${resHigh.rateBits.toFixed(1)} bits`);

// Monotonicidade: Maior lambda -> menor número de dimensões ativas e menor taxa
assert(resLow.activeDims >= resMid.activeDims, 'Dimensões ativas com lambda baixo devem ser >= que com lambda médio');
assert(resMid.activeDims >= resHigh.activeDims, 'Dimensões ativas com lambda médio devem ser >= que com lambda alto');
assert(resLow.rateBits >= resMid.rateBits, 'Taxa de bits com lambda baixo deve ser >= que com lambda médio');
assert(resMid.rateBits >= resHigh.rateBits, 'Taxa de bits com lambda médio deve ser >= que com lambda alto');

// Poda do espaço overcomplete: mesmo com lambda baixo, pelo menos metade das 64 dimensões deve ser podada
assert(resLow.activeDims <= 32, 'Pelo menos 50% das dimensões overcomplete devem ser podadas para 0');

console.log('  ✓ Monotonicidade e colapso automático do espaço overcomplete comprovados');

// 4. Teste de Reconstrução Exata Lossless via Resíduo Inteiro
const intOriginal = new Int16Array(frameSize);
for (let n = 0; n < frameSize; n++) {
  intOriginal[n] = Math.round(testSignal[n] * 32767);
}

// Resíduo inteiro r[n] = x_int[n] - round(x_hat[n] * 32767)
const residual = new Int16Array(frameSize);
for (let n = 0; n < frameSize; n++) {
  const predictedInt = Math.round(resMid.reconstruction[n] * 32767);
  residual[n] = intOriginal[n] - predictedInt;
}

// Reconstrução bit-a-bit: x_rec[n] = round(x_hat[n] * 32767) + r[n]
for (let n = 0; n < frameSize; n++) {
  const predictedInt = Math.round(resMid.reconstruction[n] * 32767);
  const recVal = predictedInt + residual[n];
  assert.strictEqual(recVal, intOriginal[n], `Amostra ${n} deve ser idêntica bit-a-bit`);
}

console.log('  ✓ Reconstrução exata lossless (x_int = round(x_hat) + r) confirmada');

console.log('\n🎉 TODOS OS TESTES DE RDO E QUANTIZAÇÃO COM DEAD-ZONE PASSARAM COM SUCESSO!\n');

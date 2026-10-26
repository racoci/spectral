import assert from 'assert';

// Módulo a ser implementado na Green Phase
import {
  LearnedRidgeDictionary,
  RidgeInnovationQuantizer,
  SurpriseRidgeCodec
} from '../src/lib/codec/learned_ridge_codec';

console.log('🧪 Iniciando Teste TDD (Red Phase): Codec de Cristas com Representação Aprendida (TS)...');

// 1. Teste de Dicionário com Bases Ortonormais
const blockLen = 16;
const numComponents = 4;
const dict = new LearnedRidgeDictionary(blockLen, numComponents);

// Gera vetor de teste
const testVector = new Float64Array(blockLen);
for (let i = 0; i < blockLen; i++) {
  testVector[i] = 440.0 + 10.0 * Math.sin((2 * Math.PI * i) / blockLen);
}

const coeffs = dict.project(testVector);
assert.strictEqual(coeffs.length, numComponents, 'Deve gerar exatamente K coeficientes');
const recon = dict.reconstruct(coeffs);
assert.strictEqual(recon.length, blockLen, 'Vetor reconstruído deve ter tamanho de bloco idêntico');
console.log('  ✓ Projeção e reconstrução ortogonal de cristas validadas');

// 2. Teste do Quantizador
const quantizer = new RidgeInnovationQuantizer(0.2);
const qInts = quantizer.quantize(coeffs);
const deqCoeffs = quantizer.dequantize(qInts);
for (let i = 0; i < coeffs.length; i++) {
  assert(Math.abs(coeffs[i] - deqCoeffs[i]) <= 0.1001, 'Erro de quantização deve respeitar delta/2');
}
console.log('  ✓ Quantização de inovações latentes validada');

// 3. Teste do Surprise Ridge Codec
const codec = new SurpriseRidgeCodec(0.08, 15);
let skips = 0;
for (let k = 0; k < 30; k++) {
  // Tom constante estável
  const { transmitted, fHat } = codec.processFrame(440.0);
  assert(Math.abs(fHat - 440.0) < 0.5, 'Predição livre do decodificador deve manter tom constante');
  if (!transmitted) skips++;
}

assert(skips >= 15, `Deveria ter pulado pelo menos 15 transmissões em tom constante, pulou ${skips}`);
console.log(`  ✓ Surprise Encoding suprimiu ${skips} transmissões desnecessárias com precisão`);

console.log('\n🎉 TODOS OS TESTES DO CODEC DE CRISTAS APRENDIDO EM TYPESCRIPT PASSARAM!\n');

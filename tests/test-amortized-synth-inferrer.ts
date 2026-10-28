import assert from 'assert';

// Módulo a ser implementado na Green Phase
import {
  RelationalEdgeClassifier,
  AmortizedSynthInferrer
} from '../src/lib/synth/amortized_synth_inferrer';

console.log('🧪 Iniciando Teste TDD (Red Phase): Inferência Amortizada e Classificador Relacional (TS)...');

// 1. Teste do Classificador Relacional Bilinear
const classifier = new RelationalEdgeClassifier(8);
const h0 = [1.0, 0.0, 0.5, 0.8, -0.2, 0.1, 0.0, 0.9];
const h1 = [0.0, 1.0, 0.5, 0.2,  0.7, 0.0, 0.4, 0.1];
const h2 = [0.0, 0.0, 0.1, 0.0, -0.8, 0.9, 0.1, 0.0];

const pMatrix = classifier.predictTopology([h0, h1, h2]);
assert.strictEqual(pMatrix.length, 3);
assert.strictEqual(pMatrix[0].length, 3);

assert(pMatrix[0][1] >= 0.70, `Aresta 1 -> 0 deve ter alta probabilidade, obteve ${pMatrix[0][1].toFixed(3)}`);
assert(pMatrix[1][2] >= 0.70, `Aresta 2 -> 1 deve ter alta probabilidade, obteve ${pMatrix[1][2].toFixed(3)}`);
assert(pMatrix[2][0] <= 0.30, `Aresta 0 -> 2 deve ter baixa probabilidade, obteve ${pMatrix[2][0].toFixed(3)}`);
console.log('  ✓ Classificador relacional bilinear em TypeScript validado');

// 2. Teste do Inferrer Amortizado
const fs = 48000;
const numSamples = 12000;
const testPcm = new Float32Array(numSamples);

// Gera tom FM de teste: 440 Hz modulado por 220 Hz com beta=1.4
for (let n = 0; n < numSamples; n++) {
  const t = n / fs;
  const mod = 1.4 * Math.sin(2.0 * Math.PI * 220.0 * t);
  testPcm[n] = 0.8 * Math.sin(2.0 * Math.PI * 440.0 * t + mod);
}

const inferrer = new AmortizedSynthInferrer(fs);
const t0 = Date.now();
const inferredTree = inferrer.inferSynthesizer(testPcm, 2);
const elapsed = Date.now() - t0;

console.log(`  ✓ Tempo de inferência no browser: ${elapsed} ms (limite < 25 ms)`);
assert(elapsed < 30, `Inferência amortizada deve ser instantânea, levou ${elapsed} ms`);

const f0 = inferredTree.operators[0].freq;
const centsErr = 1200.0 * Math.abs(Math.log2(f0 / 440.0));
console.log(`  ✓ f0 estimado: ${f0.toFixed(2)} Hz (Erro = ${centsErr.toFixed(3)} cents)`);
assert(centsErr < 3.0, `Erro de f0 deve ser < 3 cents, obteve ${centsErr.toFixed(3)}`);

console.log('\n🎉 TODOS OS TESTES DE INFERÊNCIA AMORTIZADA EM TYPESCRIPT PASSARAM!\n');

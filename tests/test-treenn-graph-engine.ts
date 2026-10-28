import assert from 'assert';

// Módulo a ser implementado na Green Phase
import {
  TreeNNCausalGraph,
  type TreeNNOperatorConfig,
  type TreeNNDict
} from '../src/lib/synth/treenn_graph_engine';

console.log('🧪 Iniciando Teste TDD (Red Phase): Motor Causal TreeNN e Grafo de Síntese (TS)...');

// 1. Configurar Grafo de 3 Operadores
const ops: TreeNNOperatorConfig[] = [
  { freq: 440.0, baseAmplitude: 0.8, isCarrier: true },
  { freq: 220.0, baseAmplitude: 0.6, isCarrier: false },
  { freq: 5.0,   baseAmplitude: 1.0, isCarrier: false }
];

// Matriz W: Op 1 modula Op 0 (1.5) e Op 2 modula Op 1 (0.8)
const W: number[][] = [
  [0.0,  1.5, 0.0],
  [0.0,  0.4, 0.8],
  [0.0,  0.0, 0.0]
];

const outputWeights = [1.0, 0.0, 0.0];

const graph = new TreeNNCausalGraph(ops, W, outputWeights, 48000);
const pcm = graph.synthesize(0.25); // 12000 amostras

assert.strictEqual(pcm.length, 12000, 'Deveria gerar exatamente 12000 amostras PCM');

// Verifica ausência de NaN e saturação suave
let peak = 0;
let energy = 0;
for (let i = 0; i < pcm.length; i++) {
  const sample = pcm[i];
  assert(!isNaN(sample), `Amostra ${i} não pode ser NaN`);
  assert(isFinite(sample), `Amostra ${i} não pode ser infinita`);
  const abs = Math.abs(sample);
  if (abs > peak) peak = abs;
  energy += sample * sample;
}

assert(peak > 0.05, 'Deveria conter sinal audível');
assert(peak <= 1.5, 'Saturação suave deve limitar pico a <= 1.5');
console.log(`  ✓ Síntese estéreo estável: Amostras=${pcm.length}, Pico=${peak.toFixed(4)}, RMS=${Math.sqrt(energy / pcm.length).toFixed(4)}`);

// 2. Teste de Serialização JSON Canônica
const dictData: TreeNNDict = graph.toDict();
assert.strictEqual(dictData.num_operators, 3);
assert.strictEqual(dictData.W[0][1], 1.5);

const reloaded = TreeNNCausalGraph.fromDict(dictData);
assert.strictEqual(reloaded.ops[0].freq, 440.0);
assert.strictEqual(reloaded.W[0][1], 1.5);
console.log('  ✓ Serialização e restauração do grafo TreeNN validadas');

console.log('\n🎉 TODOS OS TESTES DO MOTOR TREENN EM TYPESCRIPT PASSARAM COM SUCESSO!\n');

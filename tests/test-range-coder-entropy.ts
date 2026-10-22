import assert from 'assert';

// Módulo do codificador de faixa e modelo de entropia a ser implementado na Green Phase
import {
  GeometricLaplacianModel,
  RangeEncoder,
  RangeDecoder,
  encodeSymbols,
  decodeSymbols
} from '../src/lib/codec/range_coder';

console.log('🧪 Iniciando Teste TDD (Red Phase): Range Coder Bit-Exact e Modelo de Entropia Laplaciano...');

// 1. Teste do Modelo Probabilístico Laplaciano Geométrico do DRED
const r = 0.65; // Parâmetro de decaimento do DRED
const maxSymbol = 8;
const model = new GeometricLaplacianModel(r, maxSymbol);

// A soma das probabilidades normalizadas deve ser exatamente 16384 (escala de 14 bits)
const totalFreq = model.cumulativeFreq[model.cumulativeFreq.length - 1];
assert.strictEqual(totalFreq, 16384, 'A soma cumulativa das frequências deve ser exatamente 2^14 = 16384');

// O símbolo 0 (centro) deve ter a maior probabilidade
const prob0 = model.getSymbolFrequency(0);
const prob1 = model.getSymbolFrequency(1);
const prob2 = model.getSymbolFrequency(2);
assert(prob0 > prob1 && prob1 > prob2, 'Probabilidades devem decair monotonicamente com |k|');
console.log(`  ✓ Modelo Laplaciano: P(0)=${prob0}, P(1)=${prob1}, P(2)=${prob2} (Escala 16384)`);

// 2. Teste de Entropia Teórica de Shannon
const theoreticalEntropy = model.computeTheoreticalEntropy();
assert(theoreticalEntropy > 1.0 && theoreticalEntropy < 4.0, 'Entropia de Shannon deve estar em intervalo realista');
console.log(`  ✓ Entropia Teórica de Shannon calculada: ${theoreticalEntropy.toFixed(3)} bits/símbolo`);

// 3. Teste de Ida e Volta (Roundtrip) Bit-Exact do Range Coder
const testSymbols = [0, 0, 1, 0, -1, 0, 0, 2, -1, 0, 0, 0, 3, -2, 0, 1, 0, 0, -3, 0];
console.log(`  Sequência de teste (${testSymbols.length} símbolos):`, testSymbols);

const bitstream = encodeSymbols(testSymbols, model);
assert(bitstream instanceof Uint8Array, 'O bitstream gerado deve ser um Uint8Array');
assert(bitstream.length > 0, 'O bitstream não deve ser vazio');

const decodedSymbols = decodeSymbols(bitstream, testSymbols.length, model);
assert.strictEqual(decodedSymbols.length, testSymbols.length, 'Comprimento decodificado deve coincidir');
assert.deepStrictEqual(decodedSymbols, testSymbols, 'Reconstrução bit-a-bit do Range Coder deve ser 100% idêntica');

console.log(`  ✓ Bitstream gerado: ${bitstream.length} bytes (${(bitstream.length * 8 / testSymbols.length).toFixed(2)} bits/símbolo)`);
console.log('  ✓ Reconstrução exata bit-a-bit confirmada com 100% de simetria!');

// 4. Teste de Compressão de Sequência Longa com Muitos Zeros (Esparsidade Típica DRED)
const sparseSymbols: number[] = [];
for (let i = 0; i < 500; i++) {
  // 80% zeros, 15% +/-1, 5% +/-2
  const rand = Math.random();
  if (rand < 0.80) sparseSymbols.push(0);
  else if (rand < 0.95) sparseSymbols.push(rand < 0.875 ? 1 : -1);
  else sparseSymbols.push(rand < 0.975 ? 2 : -2);
}

const sparseBitstream = encodeSymbols(sparseSymbols, model);
const sparseDecoded = decodeSymbols(sparseBitstream, sparseSymbols.length, model);
assert.deepStrictEqual(sparseDecoded, sparseSymbols, 'Sequência esparsa longa deve ser decodificada sem perdas');

const bitsPerSymbol = (sparseBitstream.length * 8) / sparseSymbols.length;
console.log(`  ✓ Compressão esparsa: ${sparseSymbols.length} símbolos comprimidos em ${sparseBitstream.length} bytes (${bitsPerSymbol.toFixed(2)} bits/símbolo)`);
assert(bitsPerSymbol < 2.5, 'Taxa de compressão em sequência esparsa deve ser inferior a 2.5 bits/símbolo');

console.log('\n🎉 TODOS OS TESTES DO RANGE CODER E MODELO LAPLACIANO PASSARAM COM SUCESSO!\n');

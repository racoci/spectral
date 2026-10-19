import assert from 'assert';
import { DX7_TUB_BELLS, DX7_E_PIANO_1, DX7_SLAP_BASS } from '../src/lib/synth/dx7_presets';
import { emitLeanCode, parseLeanCode, locateLeanParameterRange } from '../src/lib/synth/lean_ast_codec';

console.log('🧪 Iniciando Teste TDD (Red Phase): Emissor e Parser de AST Lean 4...');

// 1. Testa emissão de código Lean 4 a partir do preset Tubular Bells
const leanCode = emitLeanCode(DX7_TUB_BELLS);
assert(typeof leanCode === 'string' && leanCode.length > 100, 'Código Lean 4 deve ser uma string não vazia');
assert(leanCode.includes('def tubularBells : FmPatch :='), 'Código Lean 4 deve conter declaração def com tipo FmPatch');
assert(leanCode.includes('name := "TUB BELLS"'), 'Deve conter o nome do patch com syntax Lean 4 :=');
assert(leanCode.includes('algorithm := 1'), 'Deve conter o algoritmo com syntax Lean 4');
assert(leanCode.includes('feedback := 7'), 'Deve conter o feedback com syntax Lean 4');
assert(leanCode.includes('operators := #['), 'Deve conter o array de operadores Lean 4 #[]');

// 2. Testa roundtrip simétrico: Parse(Emit(patch)) == patch
const parsedPatch = parseLeanCode(leanCode);
assert.strictEqual(parsedPatch.name, DX7_TUB_BELLS.name, 'Nome deve ser idêntico');
assert.strictEqual(parsedPatch.algorithm, DX7_TUB_BELLS.algorithm, 'Algoritmo deve ser idêntico');
assert.strictEqual(parsedPatch.feedback, DX7_TUB_BELLS.feedback, 'Feedback deve ser idêntico');
assert.strictEqual(parsedPatch.operators.length, 6, 'Deve reconstruir 6 operadores');

for (let i = 0; i < 6; i++) {
  const origOp = DX7_TUB_BELLS.operators[i];
  const parsedOp = parsedPatch.operators[i];
  assert.strictEqual(parsedOp.id, origOp.id, `Op ${i + 1}: ID deve ser idêntico`);
  assert.strictEqual(parsedOp.outputLevel, origOp.outputLevel, `Op ${i + 1}: OutputLevel deve ser idêntico`);
  assert.strictEqual(parsedOp.freqCoarse, origOp.freqCoarse, `Op ${i + 1}: FreqCoarse deve ser idêntico`);
  assert.strictEqual(parsedOp.freqFine, origOp.freqFine, `Op ${i + 1}: FreqFine deve ser idêntico`);
  assert.strictEqual(parsedOp.detune, origOp.detune, `Op ${i + 1}: Detune deve ser idêntico`);
  assert.deepStrictEqual(parsedOp.rates, origOp.rates, `Op ${i + 1}: Rates devem ser idênticos`);
  assert.deepStrictEqual(parsedOp.levels, origOp.levels, `Op ${i + 1}: Levels devem ser idênticos`);
}

// 3. Testa roundtrip em outros presets canônicos
for (const preset of [DX7_E_PIANO_1, DX7_SLAP_BASS]) {
  const code = emitLeanCode(preset);
  const back = parseLeanCode(code);
  assert.strictEqual(back.name, preset.name);
  assert.strictEqual(back.algorithm, preset.algorithm);
  assert.strictEqual(back.feedback, preset.feedback);
}

// 4. Testa localização cirúrgica de parâmetros [from, to] para widgets CodeMirror
const rangeFeedback = locateLeanParameterRange(leanCode, 'feedback');
assert(rangeFeedback !== null, 'Deveria localizar o parâmetro feedback no código Lean');
const sliceFeedback = leanCode.slice(rangeFeedback.from, rangeFeedback.to);
assert.strictEqual(sliceFeedback, '7', 'Trecho localizado no texto deve ser exatamente "7"');

const rangeOp1Level = locateLeanParameterRange(leanCode, 'op.1.outputLevel');
assert(rangeOp1Level !== null, 'Deveria localizar o parâmetro op.1.outputLevel');
const sliceOp1Level = leanCode.slice(rangeOp1Level.from, rangeOp1Level.to);
assert.strictEqual(sliceOp1Level, '99', 'Trecho localizado no texto deve ser exatamente "99"');

console.log('✅ Todos os testes de emissão, parsing e localização AST Lean 4 passaram com 100% de sucesso!');

import assert from 'assert';

// Módulo codec Lean 4 a ser implementado na Green Phase
import {
  emitLeanScalarField,
  parseLeanScalarField,
  locateScalarFieldParamRange
} from '../src/lib/scalar_field/scalar_field_codec';
import type { ScalarFieldModel } from '../src/lib/scalar_field/taylor_jet_math';

console.log('🧪 Iniciando Teste TDD (Red Phase): Codec e Roundtrip de Campos Escalares em Lean 4...');

// 1. Cria um modelo de teste de campo escalar com ponto de ancoragem e jatos
const testField: ScalarFieldModel = {
  domain: { xMin: -2.5, xMax: 2.5, yMin: -2.5, yMax: 2.5 },
  baseFunction: { type: 'saddle', gain: 1.5, bias: 0.25, sigma: 1.0 },
  anchor: { x: 0.75, y: -0.5 },
  maxOrder: 4,
  brushLayers: [
    {
      id: 'stroke_1',
      operator: 'add',
      radius: 0.2,
      intensity: 0.8,
      points: [
        { x: -0.5, y: 0.0 },
        { x: 0.0, y: 0.5 },
        { x: 0.5, y: 0.2 }
      ],
      profile1D: [0.0, 0.8, 0.0]
    }
  ]
};

// 2. Emite código Lean 4
const leanCode = emitLeanScalarField(testField);
assert(typeof leanCode === 'string' && leanCode.length > 50, 'Código emitido deve ser string não vazia');
assert(leanCode.includes('structure ScalarField2D') || leanCode.includes('def myScalarField : ScalarField2D :='), 'Deve conter declaração Lean 4 de ScalarField2D');
assert(leanCode.includes('baseType := "saddle"'), 'Deve conter tipo base');
assert(leanCode.includes('gain := 1.5'), 'Deve conter gain 1.5');
assert(leanCode.includes('bias := 0.25'), 'Deve conter bias 0.25');

// 3. Testa Roundtrip simétrico: Parse(Emit(field)) == field
const parsed = parseLeanScalarField(leanCode);
assert.strictEqual(parsed.baseFunction.type, testField.baseFunction.type, 'Tipo base deve ser idêntico');
assert.strictEqual(parsed.baseFunction.gain, testField.baseFunction.gain, 'Ganho deve ser idêntico');
assert.strictEqual(parsed.baseFunction.bias, testField.baseFunction.bias, 'Bias deve ser idêntico');
assert.strictEqual(parsed.anchor.x, testField.anchor.x, 'Âncora x deve ser idêntica');
assert.strictEqual(parsed.anchor.y, testField.anchor.y, 'Âncora y deve ser idêntica');
assert.strictEqual(parsed.maxOrder, testField.maxOrder, 'MaxOrder deve ser idêntico');
assert.strictEqual(parsed.brushLayers.length, testField.brushLayers.length, 'Número de camadas de brush deve ser idêntico');

// 4. Testa localização cirúrgica de parâmetros [from, to] para CodeMirror
const rangeGain = locateScalarFieldParamRange(leanCode, 'base.gain');
assert(rangeGain !== null, 'Deveria localizar o parâmetro base.gain');
const sliceGain = leanCode.slice(rangeGain.from, rangeGain.to);
assert.strictEqual(sliceGain, '1.5', 'Fatia localizada no texto deve ser exatamente "1.5"');

const rangeAnchorX = locateScalarFieldParamRange(leanCode, 'anchor.x');
assert(rangeAnchorX !== null, 'Deveria localizar anchor.x');
const sliceAnchorX = leanCode.slice(rangeAnchorX.from, rangeAnchorX.to);
assert.strictEqual(sliceAnchorX, '0.75', 'Fatia localizada deve ser exatamente "0.75"');

console.log('✅ Testes de Roundtrip e Localização Cirúrgica de Campos Escalares em Lean 4 passaram!');

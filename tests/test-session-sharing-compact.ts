import assert from 'assert';

// Módulo a ser implementado na Green Phase
import {
  serializeToHashQuery,
  parseFromHashQuery,
  type SpectralPackage
} from '../src/lib/sharing/session_serializer';

console.log('🧪 Iniciando Teste TDD (Red Phase): Serialização de Sessão em Formato Compacto...');

// 1. Teste de Pacote STN Synth
const stnPkg: SpectralPackage = {
  v: 1,
  tool: 'stn',
  title: 'Vocal Pluck Spatial 3D',
  params: {
    sGain: -3.5,
    tGain: 2.0,
    nGain: -18.0,
    itdMs: 0.45,
    ildDb: 6.0,
    sines: [
      { f: 220, db: -6, phase: 0 },
      { f: 440, db: -12, phase: 1.57 }
    ]
  }
};

const hashUrlStn = serializeToHashQuery(stnPkg);
assert(hashUrlStn.startsWith('#/stn?p='), `Hash URL deve iniciar com #/stn?p=, obteve ${hashUrlStn.slice(0, 20)}`);
assert(hashUrlStn.length < 500, `Payload deve ser extremamente compacto (< 500 chars), tamanho = ${hashUrlStn.length}`);

const decodedStn = parseFromHashQuery(hashUrlStn);
assert(decodedStn, 'Desserialização do STN deve retornar pacote válido');
assert.strictEqual(decodedStn.tool, 'stn');
assert.strictEqual(decodedStn.title, 'Vocal Pluck Spatial 3D');
assert.strictEqual(decodedStn.params.sGain, -3.5);
assert.strictEqual(decodedStn.params.sines.length, 2);
console.log(`  ✓ Roundtrip STN validado (${hashUrlStn.length} caracteres)`);

// 2. Teste de Pacote Scalar Field 2D & Taylor Jets
const fieldPkg: SpectralPackage = {
  v: 1,
  tool: 'field',
  title: 'Sela de Macaco de 3ª Ordem',
  params: {
    centerX: 0.5,
    centerY: 0.5,
    scale: 1.8,
    order: 3,
    handles: [
      { order: 2, angle: 0.785, magnitude: 1.5 },
      { order: 3, angle: 2.356, magnitude: 0.9 }
    ]
  }
};

const hashUrlField = serializeToHashQuery(fieldPkg);
assert(hashUrlField.startsWith('#/field?p='));
const decodedField = parseFromHashQuery(hashUrlField);
assert(decodedField);
assert.strictEqual(decodedField.tool, 'field');
assert.strictEqual(decodedField.params.order, 3);
assert.strictEqual(decodedField.params.handles.length, 2);
console.log(`  ✓ Roundtrip Field IDE validado (${hashUrlField.length} caracteres)`);

// 3. Teste de Pacote Lean 4 Synth IDE
const leanPkg: SpectralPackage = {
  v: 1,
  tool: 'lean',
  title: 'Tubular Bells DX7 Patch',
  params: {
    algorithm: 5,
    feedback: 6,
    activeOp: 1,
    ops: [
      { id: 1, ratio: 1.0, level: 99 },
      { id: 2, ratio: 3.5, level: 82 }
    ]
  }
};

const hashUrlLean = serializeToHashQuery(leanPkg);
assert(hashUrlLean.startsWith('#/lean?p='));
const decodedLean = parseFromHashQuery(hashUrlLean);
assert(decodedLean);
assert.strictEqual(decodedLean.tool, 'lean');
assert.strictEqual(decodedLean.params.algorithm, 5);
console.log(`  ✓ Roundtrip Lean IDE validado (${hashUrlLean.length} caracteres)`);

// 4. Teste de Robustez e Tolerância a Erros
const invalidHash1 = '#/editor?invalid=123';
assert.strictEqual(parseFromHashQuery(invalidHash1), null, 'Query sem parâmetro "p" deve retornar null');

const corruptedHash = '#/stn?p=invalid_base64_json!!!';
assert.strictEqual(parseFromHashQuery(corruptedHash), null, 'Payload corrompido deve retornar null sem lançar exceção');
console.log('  ✓ Tratamento de erros e integridade validados');

console.log('\n🎉 TODOS OS TESTES DE SERIALIZAÇÃO DE SESSÃO PASSARAM COM SUCESSO!\n');

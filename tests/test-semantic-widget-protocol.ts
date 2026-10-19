import assert from 'assert';
import { 
  validateWidgetDescriptor, 
  SemanticSession, 
  type WidgetDescriptor, 
  type WidgetKind 
} from '../src/lib/widgets/types';
import { DX7_TUB_BELLS } from '../src/lib/synth/dx7_presets';
import { emitLeanCode } from '../src/lib/synth/lean_ast_codec';

console.log('🧪 Iniciando Teste TDD (Red Phase): Protocolo de Descritores Semânticos...');

// 1. Validação estrita do schema de WidgetDescriptor
const validDescriptor: WidgetDescriptor = {
  id: 'widget-algo-1',
  nodeId: 'fm.algorithm',
  revision: 1,
  from: 100,
  to: 115,
  kind: 'fm-algorithm',
  title: 'Algoritmo FM 1 (Cascata)',
  parameters: {
    algorithm: 1,
    feedback: 7
  },
  previewKey: 'preview-algo-1'
};

assert.strictEqual(validateWidgetDescriptor(validDescriptor), true, 'Descritor válido deve passar na validação de schema');

// 1.1 Violação de limites [from, to]
const invalidRangeDescriptor = {
  ...validDescriptor,
  from: 120,
  to: 110 // to < from deve ser rejeitado
};
assert.strictEqual(validateWidgetDescriptor(invalidRangeDescriptor as any), false, 'to < from deve ser inválido');

// 1.2 Tipo de widget não suportado
const invalidKindDescriptor = {
  ...validDescriptor,
  kind: 'unsupported-future-kind'
};
assert.strictEqual(validateWidgetDescriptor(invalidKindDescriptor as any), false, 'Kind não suportado deve falhar na validação');

// 2. Validação da Sessão Semântica e Descarte de Revisões Obsoletas (Race Condition Gate)
const session = new SemanticSession();

const codeV1 = emitLeanCode(DX7_TUB_BELLS);
const respV1 = session.processDocument(1, codeV1);
assert.strictEqual(respV1.revision, 1);
assert(respV1.descriptors.length >= 7, 'Deveria gerar descritores para algoritmo e operadores');

// Envia revisão 3 diretamente
const codeV3 = codeV1.replace('feedback := 7', 'feedback := 0');
const respV3 = session.processDocument(3, codeV3);
assert.strictEqual(respV3.revision, 3);
const algoDesc = respV3.descriptors.find(d => d.nodeId === 'fm.algorithm');
assert(algoDesc !== undefined);
assert.strictEqual(algoDesc.parameters.feedback, 0, 'Revisão 3 deve conter feedback atualizado para 0');

// Simula chegada atrasada da revisão 2 (que ficou presa na rede ou worker)
const codeV2 = codeV1.replace('feedback := 7', 'feedback := 4');
const respV2 = session.processDocument(2, codeV2);
assert.strictEqual(respV2.ignored, true, 'Resposta da revisão 2 obsoleta deve ser marcada como ignored');
assert.strictEqual(session.currentRevision, 3, 'Sessão deve manter a revisão ativa mais recente (3)');

// 3. Rastreamento estável de NodeId entre revisões
const op1V1 = respV1.descriptors.find(d => d.nodeId === 'op.1.eg');
const op1V3 = respV3.descriptors.find(d => d.nodeId === 'op.1.eg');
assert(op1V1 && op1V3);
assert.strictEqual(op1V1.nodeId, op1V3.nodeId, 'nodeId deve ser estável entre revisões');

console.log('✅ Testes do protocolo de descritores semânticos passaram com 100% de sucesso!');

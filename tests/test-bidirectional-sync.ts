import assert from 'assert';
import { EditorState } from '@codemirror/state';
import { 
  createParamUpdateTransaction 
} from '../src/lib/editor/SemanticBlockWidget';
import { 
  SemanticSession, 
  type WidgetDescriptor 
} from '../src/lib/widgets/types';
import { 
  getWidgetComponent, 
  widgetRegistry 
} from '../src/lib/widgets/registry';
import { DX7_E_PIANO_1 } from '../src/lib/synth/dx7_presets';
import { emitLeanCode, parseLeanCode } from '../src/lib/synth/lean_ast_codec';

console.log('🧪 Iniciando Teste TDD (Red Phase): Sincronização Bidirecional Texto <-> Widgets...');

// 1. Validação do Registro de Componentes (Registry)
assert(widgetRegistry['fm-algorithm'] !== undefined, 'Deve registrar componente para fm-algorithm');
assert(widgetRegistry['envelope-stage'] !== undefined, 'Deve registrar componente para envelope-stage');
assert(widgetRegistry['parameter-knob'] !== undefined, 'Deve registrar componente para parameter-knob');

const algoComponent = getWidgetComponent('fm-algorithm');
assert(algoComponent !== null && algoComponent !== undefined, 'getWidgetComponent deve retornar componente válido');
console.log('  ✓ Registro genérico de widgets Svelte 5 verificado com sucesso');

// 2. Fluxo Direção A: Texto -> Widget
const session = new SemanticSession();
let code = emitLeanCode(DX7_E_PIANO_1);
let state = EditorState.create({ doc: code });

let resp = session.processDocument(1, state.doc.toString());
let algoDesc = resp.descriptors.find(d => d.nodeId === 'fm.algorithm');
assert(algoDesc !== undefined);
assert.strictEqual(algoDesc.parameters.algorithm, 5, 'Algoritmo inicial deve ser 5 (E-Piano 1)');

// Simula edição direta no texto: usuário muda algorithm para 16 via teclado
code = code.replace('algorithm := 5', 'algorithm := 16');
state = EditorState.create({ doc: code });
resp = session.processDocument(2, state.doc.toString());
algoDesc = resp.descriptors.find(d => d.nodeId === 'fm.algorithm');
assert(algoDesc !== undefined);
assert.strictEqual(algoDesc.parameters.algorithm, 16, 'Widget deve receber o algoritmo 16 atualizado pelo texto');
console.log('  ✓ Direção A (Texto -> Widget) validada com sucesso');

// 3. Fluxo Direção B: Widget -> Texto (Transação Cirúrgica)
// Usuário clica no OperatorMatrixWidget e altera algorithm de 16 para 1
const trAlgo = createParamUpdateTransaction(state, 'algorithm', 1);
assert(trAlgo !== null, 'Deveria gerar transação para o algoritmo');
state = state.update(trAlgo).state;

// Verifica se o texto mudou no buffer CodeMirror
assert(state.doc.toString().includes('algorithm := 1'), 'Buffer de texto deve conter "algorithm := 1"');
assert(!state.doc.toString().includes('algorithm := 16'), 'Buffer de texto não deve mais conter "algorithm := 16"');

// 4. Sincronização Cirúrgica de Envelope (Arraste de nó gráfico -> Texto)
// Usuário arrasta o ponto de ataque R1 do Operador 1 de 88 para 99
const trEnv = createParamUpdateTransaction(state, 'op.1.eg.r1', 99);
assert(trEnv !== null, 'Deveria gerar transação para r1 do operador 1');
state = state.update(trEnv).state;

// Re-parseia a AST e confirma integridade matemática
let updatedPatch = parseLeanCode(state.doc.toString());
assert.strictEqual(updatedPatch.operators[0].rates[0], 99, 'Operador 1 deve ter taxa r1 = 99 na AST');
console.log('  ✓ Direção B (Widget de Envelope -> Transação Cirúrgica no Texto) validada');

// 5. Sincronização Cirúrgica de Knobs (Knob rotativo -> Texto)
// Usuário gira o knob de FreqCoarse do Operador 2 de 1 para 14
const trKnob = createParamUpdateTransaction(state, 'op.2.freqCoarse', 14);
assert(trKnob !== null, 'Deveria gerar transação para freqCoarse do operador 2');
state = state.update(trKnob).state;

updatedPatch = parseLeanCode(state.doc.toString());
assert.strictEqual(updatedPatch.operators[1].freqCoarse, 14, 'Operador 2 deve ter freqCoarse = 14');
console.log('  ✓ Direção B (Parameter Knob -> Transação Cirúrgica no Texto) validada');

// 6. Confirmação do Invariante de Texto Puro após múltiplas transações
const finalText = state.doc.toString();
assert(!finalText.includes('<div') && !finalText.includes('class='), 'O texto puro final não pode conter resíduos HTML');
assert(finalText.startsWith('import Synth.Fm'), 'Estrutura do código Lean 4 preservada integralmente');

console.log('✅ Todos os testes de sincronização bidirecional cirúrgica passaram com 100% de sucesso!');

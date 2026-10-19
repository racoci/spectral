import assert from 'assert';
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { 
  SemanticBlockWidget, 
  buildSemanticDecorations,
  createParamUpdateTransaction 
} from '../src/lib/editor/SemanticBlockWidget';
import { type WidgetDescriptor } from '../src/lib/widgets/types';
import { DX7_TUB_BELLS } from '../src/lib/synth/dx7_presets';
import { emitLeanCode } from '../src/lib/synth/lean_ast_codec';

console.log('🧪 Iniciando Teste TDD (Red Phase): Ciclo de Vida do CodeMirror 6 e Svelte 5...');

const descriptorA: WidgetDescriptor = {
  id: 'w-algo-1',
  nodeId: 'fm.algorithm',
  revision: 1,
  from: 50,
  to: 65,
  kind: 'fm-algorithm',
  title: 'Algoritmo FM 1',
  parameters: { algorithm: 1, feedback: 7 }
};

// 1. Testa estabilidade do método eq() do WidgetType
const widget1 = new SemanticBlockWidget(descriptorA);
const widgetSame = new SemanticBlockWidget({ ...descriptorA });
const widgetDiffRev = new SemanticBlockWidget({ ...descriptorA, revision: 2 });
const widgetDiffParam = new SemanticBlockWidget({ ...descriptorA, parameters: { algorithm: 1, feedback: 0 } });

assert.strictEqual(widget1.eq(widgetSame), true, 'Widgets com mesmo nodeId, revision e parameters devem ser eq() == true');
assert.strictEqual(widget1.eq(widgetDiffRev), false, 'Widgets com revisões distintas devem ser eq() == false');
assert.strictEqual(widget1.eq(widgetDiffParam), false, 'Widgets com parâmetros distintos devem ser eq() == false');
console.log('  ✓ Estabilidade de virtualização eq() verificada com sucesso');

// 2. Testa Invariante de Texto Puro (Zero contaminação no buffer de texto)
const leanText = emitLeanCode(DX7_TUB_BELLS);
const decorations = buildSemanticDecorations([descriptorA]);

const state = EditorState.create({
  doc: leanText,
  extensions: [decorations]
});

// Extração do buffer de texto puro
const extractedText = state.doc.toString();
assert.strictEqual(extractedText, leanText, 'O buffer do CodeMirror deve permanecer 100% texto puro original');
assert(!extractedText.includes('widget-algo'), 'Nenhum resíduo ou tag DOM do widget pode poluir o buffer de texto');
console.log('  ✓ Invariante de texto puro verificado (Zero contaminação no documento)');

// 3. Testa Transação Cirúrgica no Buffer (Widget -> CodeMirror)
const tr = createParamUpdateTransaction(state, 'feedback', '0');
assert(tr !== null, 'Deveria criar transação para o parâmetro feedback');
assert(tr.changes !== undefined && (tr.changes as any).insert === '0', 'Deve conter a substituição para "0"');

// Aplica a transação no estado
const nextState = state.update(tr).state;
assert(nextState.doc.toString().includes('feedback := 0'), 'Documento atualizado deve conter feedback := 0');
assert(!nextState.doc.toString().includes('feedback := 7'), 'Documento não deve mais conter feedback := 7');
console.log('  ✓ Transação cirúrgica no buffer CodeMirror testada com sucesso');

// 4. Testa Higiene de Montagem e Desmontagem DOM (Mock Svelte Lifecycle)
let mountedCount = 0;
let unmountedCount = 0;

// Configura hooks de teste no widget
widget1._testHooks = {
  onMount: () => { mountedCount++; },
  onUnmount: () => { unmountedCount++; }
};

const dom = widget1.toDOM({} as EditorView);
assert(dom instanceof Object, 'toDOM deve retornar elemento DOM');
assert.strictEqual(mountedCount, 1, 'toDOM deve disparar montagem do Svelte');

widget1.destroy(dom);
assert.strictEqual(unmountedCount, 1, 'destroy() deve chamar unmount() do Svelte');
console.log('  ✓ Higiene de montagem/desmontagem e prevenção de memory leaks verificadas');

console.log('✅ Testes de ciclo de vida CodeMirror 6 / Svelte 5 passaram com 100% de sucesso!');

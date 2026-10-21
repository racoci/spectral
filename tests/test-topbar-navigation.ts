import assert from 'assert';

// Módulo de navegação global a ser implementado na Green Phase
import {
  TOOL_ROUTES,
  resolveRouteFromShortcut,
  type NavigationRoute
} from '../src/lib/ui/navigation_helpers';

console.log('🧪 Iniciando Teste TDD (Red Phase): Navegação Global, Atalhos de Teclado e Rotas...');

// 1. Valida Especificação das 6 Ferramentas
assert.strictEqual(TOOL_ROUTES.length, 6, 'Devem existir exatamente 6 ferramentas registradas');

const expectedIds = ['converter', 'editor', 'synth', 'stn', 'lean', 'field'];
for (let i = 0; i < expectedIds.length; i++) {
  const r = TOOL_ROUTES[i];
  assert.strictEqual(r.id, expectedIds[i], `Ferramenta ${i + 1} deve ter id ${expectedIds[i]}`);
  assert.strictEqual(r.shortcut, `Alt+${i + 1}`, `Atalho deve ser Alt+${i + 1}`);
  assert(r.hash.startsWith('#/'), `Hash deve começar com #/`);
  assert(typeof r.iconSvg === 'string' && r.iconSvg.length > 0, `Ícone SVG deve estar presente`);
}
console.log('  ✓ Registro canônico das 6 ferramentas e rotas validado');

// 2. Validação da Resolução de Atalhos de Teclado (Alt+1..6)
// Caso Válido: Pressionar Alt+3 fora de campos de texto
const res3 = resolveRouteFromShortcut({ altKey: true, key: '3', targetTagName: 'BODY', targetClassName: '' });
assert.strictEqual(res3?.id, 'synth', 'Alt+3 deve resolver para a rota synth (DDSP Studio)');
assert.strictEqual(res3?.hash, '#/synth', 'Hash deve ser #/synth');

const res6 = resolveRouteFromShortcut({ altKey: true, key: '6', targetTagName: 'DIV', targetClassName: 'workspace' });
assert.strictEqual(res6?.id, 'field', 'Alt+6 deve resolver para o Scalar Field IDE');
assert.strictEqual(res6?.hash, '#/field', 'Hash deve ser #/field');

// Caso Protegido: Foco dentro do editor CodeMirror ou campo de texto
const resInsideCM = resolveRouteFromShortcut({ altKey: true, key: '2', targetTagName: 'DIV', targetClassName: 'cm-content' });
assert.strictEqual(resInsideCM, null, 'Atalhos não devem ser interceptados dentro do CodeMirror (.cm-content)');

const resInsideInput = resolveRouteFromShortcut({ altKey: true, key: '1', targetTagName: 'INPUT', targetClassName: 'param-field' });
assert.strictEqual(resInsideInput, null, 'Atalhos não devem ser interceptados dentro de elementos <input>');

// Teclas não mapeadas
const resInvalid = resolveRouteFromShortcut({ altKey: true, key: '9', targetTagName: 'BODY', targetClassName: '' });
assert.strictEqual(resInvalid, null, 'Teclas sem rota correspondente devem retornar null');

const resNoAlt = resolveRouteFromShortcut({ altKey: false, key: '3', targetTagName: 'BODY', targetClassName: '' });
assert.strictEqual(resNoAlt, null, 'Teclas sem Alt pressionado não devem ativar rotas');

console.log('  ✓ Mecânica de interceptação segura de atalhos de teclado (Alt+1..6) validada');

console.log('\n🎉 TODOS OS TESTES DE NAVEGAÇÃO E ATALHOS PASSARAM COM SUCESSO!\n');

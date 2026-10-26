import assert from 'assert';

// Módulo a ser implementado na Green Phase
import {
  translateTerm,
  getEducationalTooltip,
  type GlossaryKey,
  type UiMode
} from '../src/lib/ui/ui_glossary';

console.log('🧪 Iniciando Teste TDD (Red Phase): Dicionário Semântico Modo Criativo vs Expert...');

const testKeys: GlossaryKey[] = [
  'holomorphic_geometry',
  'reassignment',
  'cr_residual',
  'stn_decomposition',
  'rdo_dead_zone',
  'inharmonicity_b',
  'taylor_jets'
];

// 1. Validar traduções no Modo Criativo
for (const key of testKeys) {
  const creativeLabel = translateTerm(key, 'creative');
  const expertLabel = translateTerm(key, 'expert');
  const tooltip = getEducationalTooltip(key);

  assert(creativeLabel.length > 0, `Rótulo criativo para ${key} deve ser não-vazio`);
  assert(expertLabel.length > 0, `Rótulo expert para ${key} deve ser não-vazio`);
  assert.notStrictEqual(creativeLabel, expertLabel, `Rótulo criativo e expert devem diferir para ${key}`);
  assert(tooltip.length > 10, `Tooltip educativo para ${key} deve ser descritivo`);

  console.log(`  ✓ [${key}]:`);
  console.log(`     🎨 Criativo: "${creativeLabel}"`);
  console.log(`     🔬 Expert:   "${expertLabel}"`);
  console.log(`     💡 Dica:     "${tooltip}"`);
}

// 2. Testar fallback para chaves desconhecidas
const fallbackCreative = translateTerm('unknown_term' as any, 'creative');
assert.strictEqual(fallbackCreative, 'unknown_term', 'Chave desconhecida deve retornar a própria chave como fallback');

console.log('\n🎉 TODOS OS TESTES DO DICIONÁRIO CRIATIVO/EXPERT PASSARAM COM SUCESSO!\n');

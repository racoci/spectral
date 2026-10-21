import assert from 'assert';

// Módulo de lógica dos componentes de UI a ser implementado na Green Phase
import {
  clampValue,
  computeKnobAngle,
  computeKnobValueFromAngle,
  computeSplitterBounds,
  computeSmartTooltipPosition
} from '../src/lib/ui/ui_helpers';

console.log('🧪 Iniciando Teste TDD (Red Phase): Lógica e Geometria dos Componentes Atômicos de UI...');

// 1. Teste de Clamp e Limites de Valor
assert.strictEqual(clampValue(5, 0, 10), 5, 'Valor intermediário deve ser mantido');
assert.strictEqual(clampValue(-2, 0, 10), 0, 'Valor abaixo do mínimo deve ser clampado para 0');
assert.strictEqual(clampValue(15, 0, 10), 10, 'Valor acima do máximo deve ser clampado para 10');
console.log('  ✓ Função clampValue validada');

// 2. Teste de Mapeamento Angular do Knob [-135deg, +135deg]
const min = 0;
const max = 100;

const angleMin = computeKnobAngle(min, min, max);
assert.strictEqual(angleMin, -135, 'Valor mínimo deve mapear para -135 graus');

const angleMid = computeKnobAngle(50, min, max);
assert.strictEqual(angleMid, 0, 'Valor médio deve mapear para 0 graus');

const angleMax = computeKnobAngle(max, min, max);
assert.strictEqual(angleMax, 135, 'Valor máximo deve mapear para +135 graus');

// Bijeção e Reversibilidade
for (const testVal of [0, 25, 50, 75, 100]) {
  const ang = computeKnobAngle(testVal, min, max);
  const recovered = computeKnobValueFromAngle(ang, min, max);
  assert(Math.abs(recovered - testVal) < 1e-5, `Reversibilidade do Knob falhou para val=${testVal}`);
}
console.log('  ✓ Mapeamento angular bijetivo do Knob (270° de varredura) validado');

// 3. Teste de Limites do Splitter (Prevenção de Colapso de Painéis)
const totalWidth = 1000;
const minPane1 = 200;
const minPane2 = 250;

// Arraste muito para a esquerda (ex: 50px)
const clampedLeft = computeSplitterBounds(50, totalWidth, minPane1, minPane2);
assert.strictEqual(clampedLeft, 200, 'Splitter deve ser clampado no mínimo do primeiro painel (200px)');

// Arraste muito para a direita (ex: 950px -> deixaria o segundo painel com 50px)
const clampedRight = computeSplitterBounds(950, totalWidth, minPane1, minPane2);
assert.strictEqual(clampedRight, 750, 'Splitter deve ser clampado em 1000 - 250 = 750px');

// Arraste válido intermediário
const clampedMid = computeSplitterBounds(450, totalWidth, minPane1, minPane2);
assert.strictEqual(clampedMid, 450, 'Posição válida intermediária deve ser respeitada');
console.log('  ✓ Cálculo de limites e proteção contra colapso do Splitter validado');

// 4. Teste de Posicionamento Inteligente do Tooltip (Viewport Clamping e Flip)
const viewW = 1440;
const viewH = 900;
const tipW = 180;
const tipH = 60;
const offset = 12;

// Caso padrão: centro da tela
const posMid = computeSmartTooltipPosition(500, 300, tipW, tipH, viewW, viewH, offset);
assert.strictEqual(posMid.x, 500 + offset, 'Tooltip à direita do cursor no centro');
assert.strictEqual(posMid.y, 300 + offset, 'Tooltip abaixo do cursor no centro');

// Caso borda direita: deve inverter para a esquerda do cursor
const posRight = computeSmartTooltipPosition(1400, 300, tipW, tipH, viewW, viewH, offset);
assert(posRight.x + tipW <= viewW, 'Tooltip não pode vazar pela borda direita');
assert.strictEqual(posRight.x, 1400 - tipW - offset, 'Tooltip deve inverter para a esquerda quando próximo à borda direita');

// Caso borda inferior: deve inverter para cima do cursor
const posBottom = computeSmartTooltipPosition(500, 880, tipW, tipH, viewW, viewH, offset);
assert(posBottom.y + tipH <= viewH, 'Tooltip não pode vazar pela borda inferior');
assert.strictEqual(posBottom.y, 880 - tipH - offset, 'Tooltip deve inverter para cima quando próximo à borda inferior');

console.log('  ✓ Posicionamento inteligente e prevenção de oclusão do SmartTooltip validado');

console.log('\n🎉 TODOS OS TESTES DOS COMPONENTES ATÔMICOS DE UI PASSARAM COM SUCESSO!\n');

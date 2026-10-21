import assert from 'assert';

// Módulo do motor de brushes a ser implementado na Green Phase
import {
  InertialPointerFilter,
  applyBrushKernel,
  computeEquipotentialSnap,
  type GridField2D
} from '../src/lib/scalar_field/BrushEngine';
import type { BrushStroke } from '../src/lib/scalar_field/taylor_jet_math';

console.log('🧪 Iniciando Teste TDD (Red Phase): Operadores de Brush e Filtro Inercial...');

// 1. Teste do Filtro Inercial de Ponteiro
const filter = new InertialPointerFilter(0.0, 0.0);

// Com alpha=1.0, o filtro deve acompanhar imediatamente a posição bruta
const pDirect = filter.update(10.0, 20.0, 1.0);
assert.strictEqual(pDirect.x, 10.0, 'Com alpha=1.0, x deve ser instantâneo');
assert.strictEqual(pDirect.y, 20.0, 'Com alpha=1.0, y deve ser instantâneo');

// Com alpha=0.5, o filtro deve avançar metade da distância
const filterSmooth = new InertialPointerFilter(0.0, 0.0);
const pMid = filterSmooth.update(10.0, 20.0, 0.5);
assert.strictEqual(pMid.x, 5.0, 'Com alpha=0.5, x deve avançar metade do caminho (5.0)');
assert.strictEqual(pMid.y, 10.0, 'Com alpha=0.5, y deve avançar metade do caminho (10.0)');

console.log('  ✓ Filtro inercial de amortecimento p_{t+1} = p_t + α(m_t - p_t) validado');

// 2. Teste do Operador de Brush 'add' e 'sub' (Kernel Gaussiano)
const gridW = 50;
const gridH = 50;
const grid: GridField2D = {
  width: gridW,
  height: gridH,
  values: new Float32Array(gridW * gridH),
  domain: { xMin: -1.0, xMax: 1.0, yMin: -1.0, yMax: 1.0 }
};

const strokeAdd: BrushStroke = {
  id: 'brush-add-1',
  operator: 'add',
  radius: 0.3,
  intensity: 1.0,
  points: [{ x: 0.0, y: 0.0 }],
  profile1D: [1.0]
};

// Aplica elevação gaussiana centrada em (0, 0)
applyBrushKernel(grid, strokeAdd);

// O centro da grade deve ter valor ~1.0
const centerIdx = Math.floor(gridH / 2) * gridW + Math.floor(gridW / 2);
const centerVal = grid.values[centerIdx];
assert(centerVal > 0.9 && centerVal <= 1.05, `Centro do brush 'add' deve ser ~1.0, obteve ${centerVal}`);

// Pontos distantes devem decair para ~0.0
const cornerVal = grid.values[0];
assert(cornerVal < 0.05, 'Bordas longe do raio de influência do brush devem ser ~0.0');

console.log('  ✓ Operador de Brush Gaussiano "add" validado');

// 3. Teste do Operador 'smooth' (Difusão Laplaciana Local)
const noisyGrid: GridField2D = {
  width: 10,
  height: 10,
  values: new Float32Array(100),
  domain: { xMin: -1, xMax: 1, yMin: -1, yMax: 1 }
};
// Cria um pico isolado ruidoso no centro
noisyGrid.values[55] = 10.0;

const strokeSmooth: BrushStroke = {
  id: 'brush-smooth-1',
  operator: 'smooth',
  radius: 0.5,
  intensity: 0.8,
  points: [{ x: 0.1, y: 0.1 }],
  profile1D: [1.0]
};

applyBrushKernel(noisyGrid, strokeSmooth);
assert(noisyGrid.values[55] < 10.0, 'Operador smooth deve atenuar picos isolados via difusão');
assert(noisyGrid.values[56] > 0.0, 'Operador smooth deve espalhar energia para os vizinhos');

console.log('  ✓ Operador de Difusão Laplaciana "smooth" validado');

// 4. Teste de Equipotential Snap
// Em f(x,y) = x^2 + y^2, a curva de nível 1.0 tem raio r=1.0
// Um ponto em (1.2, 0.0) deve ser projetado de volta para (1.0, 0.0)
const snapTarget = 1.0;
const evalF = (x: number, y: number) => x * x + y * y;
const evalGrad = (x: number, y: number) => ({ fx: 2.0 * x, fy: 2.0 * y });

const snapped = computeEquipotentialSnap(1.2, 0.0, snapTarget, evalF, evalGrad);
assert(Math.abs(snapped.x - 1.0) < 1e-3, 'Snap equipotencial deve projetar ponto para o raio 1.0');
assert(Math.abs(snapped.y - 0.0) < 1e-3, 'Coordenada y não deve ser alterada desnecessariamente');

console.log('  ✓ Operador de Atração Equipotential Snap validado');

console.log('\n🎉 TODOS OS TESTES DE OPERADORES DE BRUSH E FILTRO INERCIAL PASSARAM!\n');

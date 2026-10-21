import assert from 'assert';

// Importa classes e utilitários a serem implementados na Green Phase
import {
  PolarHandlesManager,
  type PolarHandle,
  worldToScreen,
  screenToWorld
} from '../src/lib/scalar_field/PolarHandlesOverlay';
import { computeTaylorJetAt, type ScalarFieldModel } from '../src/lib/scalar_field/taylor_jet_math';

console.log('🧪 Iniciando Teste TDD (Red Phase): Interação com Alças Polares e Hit-Testing...');

// 1. Teste de Conversão de Coordenadas Mundo <-> Tela
const domain = { xMin: -2.0, xMax: 2.0, yMin: -2.0, yMax: 2.0 };
const width = 800;
const height = 600;

const screenPt = worldToScreen(0.0, 0.0, width, height, domain);
assert.strictEqual(screenPt.x, 400, 'Origem (0, 0) deve mapear exatamente para o centro horizontal (400px)');
assert.strictEqual(screenPt.y, 300, 'Origem (0, 0) deve mapear exatamente para o centro vertical (300px)');

const worldPt = screenToWorld(400, 300, width, height, domain);
assert(Math.abs(worldPt.x - 0.0) < 1e-6, 'Centro de tela deve reverter para x=0.0');
assert(Math.abs(worldPt.y - 0.0) < 1e-6, 'Centro de tela deve reverter para y=0.0');

// Roundtrip de coordenadas
const testCoords = [{ x: 1.25, y: -0.75 }, { x: -1.8, y: 1.4 }];
for (const tc of testCoords) {
  const scr = worldToScreen(tc.x, tc.y, width, height, domain);
  const wld = screenToWorld(scr.x, scr.y, width, height, domain);
  assert(Math.abs(wld.x - tc.x) < 1e-5 && Math.abs(wld.y - tc.y) < 1e-5, 'Roundtrip mundo-tela-mundo deve ser bijetivo');
}
console.log('  ✓ Conversão e bijeção geométrica Mundo <-> Tela verificadas');

// 2. Teste de Extração de Alças Polares das Ordens 0 a 8
const field: ScalarFieldModel = {
  domain,
  baseFunction: { type: 'saddle', gain: 1.0, bias: 0.0 },
  anchor: { x: 0.0, y: 0.0 },
  maxOrder: 4,
  brushLayers: []
};

const jet = computeTaylorJetAt(field, 0.0, 0.0, 4);
const manager = new PolarHandlesManager(width, height, domain);
const handles = manager.computeHandles(jet);

assert(handles.length > 0, 'Deveria gerar alças polares para as ordens 0 a 4');

// Valida existência de alça de Ordem 0 (centro)
const h0 = handles.find(h => h.order === 0);
assert(h0 !== undefined, 'Deve existir alça de ordem 0');
assert.strictEqual(h0.screenX, 400, 'Alça de ordem 0 deve coincidir com o ponto de ancoragem na tela');
assert.strictEqual(h0.screenY, 300, 'Alça de ordem 0 deve coincidir com o ponto de ancoragem na tela');

// Valida alças de ordens superiores (para ordem 3 deve haver 3+1 = 4 alças)
const h3 = handles.filter(h => h.order === 3);
assert.strictEqual(h3.length, 4, 'Ordem 3 deve possuir exatamente 4 alças direcionais');

console.log('  ✓ Geração de alças polares para todas as ordens validada');

// 3. Teste de Hit-Testing e Detecção de Clique / Toque
// Clique exato sobre a alça de ordem 0
const hit0 = manager.hitTest(400, 300, handles, 10);
assert(hit0 !== null, 'Hit test deve detectar clique sobre a alça central');
assert.strictEqual(hit0?.order, 0, 'Alça detectada deve ser de ordem 0');

// Clique fora de qualquer alça
const hitMiss = manager.hitTest(100, 100, handles, 10);
assert.strictEqual(hitMiss, null, 'Hit test longe de qualquer alça deve retornar null');

// Prioridade de seleção quando há sobreposição
const hitWithSelection = manager.hitTest(400, 300, handles, 10, h0.id);
assert.strictEqual(hitWithSelection?.id, h0.id, 'Alça previamente selecionada deve ter prioridade no hit test');

console.log('  ✓ Hit-testing de precisão com raio de tolerância e prioridade validado');

console.log('\n🎉 TODOS OS TESTES DE ALÇAS POLARES E HIT-TESTING PASSARAM COM SUCESSO!\n');

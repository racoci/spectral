import assert from 'assert';

// Módulo matemático a ser implementado na Green Phase
import { 
  evaluateBaseFunction, 
  computeTaylorJetAt, 
  evaluateAngularProfile,
  reconstructTaylorExpansion,
  computeHessianEigensystem,
  type ScalarFieldModel,
  type TaylorJet
} from '../src/lib/scalar_field/taylor_jet_math';

console.log('🧪 Iniciando Teste TDD (Red Phase): Modelo Matemático de Campos Escalares e Jatos de Taylor...');

// 1. Teste de Funções Base Analíticas
const x = 0.5;
const y = -0.3;

const valZero = evaluateBaseFunction('zero', x, y, 1.0, 0.42);
assert.strictEqual(valZero, 0.42, 'Função zero com bias deve retornar o próprio bias');

const valSaddle = evaluateBaseFunction('saddle', x, y, 1.0, 0.0);
const expectedSaddle = 0.5 * 0.5 - (-0.3) * (-0.3); // 0.25 - 0.09 = 0.16
assert(Math.abs(valSaddle - expectedSaddle) < 1e-6, 'Função sela deve ser exatamente x^2 - y^2');

const valBasin = evaluateBaseFunction('basin', x, y, 1.0, 0.0);
const expectedBasin = 0.5 * 0.5 + (-0.3) * (-0.3); // 0.25 + 0.09 = 0.34
assert(Math.abs(valBasin - expectedBasin) < 1e-6, 'Função bacia deve ser exatamente x^2 + y^2');

console.log('  ✓ Funções analíticas base verificadas com sucesso');

// 2. Teste do Jato de Taylor de Ordens 0, 1 e 2 (Gradiente e Hessiana)
const field: ScalarFieldModel = {
  domain: { xMin: -2, xMax: 2, yMin: -2, yMax: 2 },
  baseFunction: { type: 'saddle', gain: 1.0, bias: 0.0 },
  anchor: { x: 0.5, y: 0.5 },
  maxOrder: 8,
  brushLayers: []
};

const jet = computeTaylorJetAt(field, 0.5, 0.5, 8);

// Em f(x,y) = x^2 - y^2:
// f(0.5, 0.5) = 0.25 - 0.25 = 0.0
// f_x = 2x = 1.0
// f_y = -2y = -1.0
// f_xx = 2.0, f_yy = -2.0, f_xy = 0.0
assert(Math.abs(jet.order0.value - 0.0) < 1e-4, 'Ordem 0 (valor central) deve ser ~0.0');
assert(Math.abs(jet.order1.fx - 1.0) < 1e-3, 'Gradiente fx deve ser ~1.0');
assert(Math.abs(jet.order1.fy - (-1.0)) < 1e-3, 'Gradiente fy deve ser ~-1.0');
assert(Math.abs(jet.order2.fxx - 2.0) < 1e-3, 'Hessiana fxx deve ser ~2.0');
assert(Math.abs(jet.order2.fyy - (-2.0)) < 1e-3, 'Hessiana fyy deve ser ~-2.0');
assert(Math.abs(jet.order2.fxy - 0.0) < 1e-3, 'Hessiana fxy deve ser ~0.0');

console.log('  ✓ Jato de Taylor Ordens 0, 1 e 2 verificado analiticamente');

// 3. Teste do Espectro da Hessiana (Autovalores e Autovetores)
const eigen = computeHessianEigensystem(jet.order2.fxx, jet.order2.fxy, jet.order2.fyy);
assert(Math.abs(eigen.lambda1 - 2.0) < 1e-3, 'Primeiro autovalor deve ser 2.0');
assert(Math.abs(eigen.lambda2 - (-2.0)) < 1e-3, 'Segundo autovalor deve ser -2.0');

// Invariantes de Traço e Determinante
const trace = jet.order2.fxx + jet.order2.fyy;
const traceEigen = eigen.lambda1 + eigen.lambda2;
assert(Math.abs(trace - traceEigen) < 1e-5, 'Invariante de traço da Hessiana deve ser exato');

const det = jet.order2.fxx * jet.order2.fyy - jet.order2.fxy * jet.order2.fxy;
const detEigen = eigen.lambda1 * eigen.lambda2;
assert(Math.abs(det - detEigen) < 1e-5, 'Invariante de determinante da Hessiana deve ser exato');

console.log('  ✓ Autovalores e invariantes de traço/determinante validados');

// 4. Teste de Perfil Angular q_n(θ) e Ordens Superiores (3 a 8)
for (let n = 1; n <= 8; n++) {
  const samples = jet.orders[n]?.samples;
  assert(Array.isArray(samples), `Ordem ${n} deve possuir array de amostras`);
  assert.strictEqual(samples.length, n + 1, `Ordem ${n} deve ter exatamente n+1 = ${n+1} graus de liberdade independentes`);
}

// 5. Teste de Convergência da Expansão de Taylor
const deltaX = 0.05;
const deltaY = 0.05;
const approxVal = reconstructTaylorExpansion(jet, deltaX, deltaY, 2);
const realVal = evaluateBaseFunction('saddle', 0.5 + deltaX, 0.5 + deltaY, 1.0, 0.0);
const error = Math.abs(approxVal - realVal);
console.log(`  ✓ Erro da aproximação de Taylor para delta=(0.05, 0.05): ${error.toExponential(3)}`);
assert(error < 1e-4, 'Aproximação de Taylor deve convergir com alta precisão');

console.log('\n🎉 TODOS OS TESTES MATEMÁTICOS DE JATOS E CAMPOS PASSARAM COM SUCESSO!\n');

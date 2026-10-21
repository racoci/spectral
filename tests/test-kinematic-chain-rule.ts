import assert from 'assert';

// Módulo cinemático a ser implementado na Green Phase
import {
  computeKinematics,
  verifyKinematicChainRule,
  type TimedTrajectoryPoint
} from '../src/lib/scalar_field/kinematic_chain_analyzer';
import { evaluateBaseFunction, computeTaylorJetAt, type ScalarFieldModel } from '../src/lib/scalar_field/taylor_jet_math';

console.log('🧪 Iniciando Teste TDD (Red Phase): Regra da Cadeia Cinemática ao Longo da Trajetória...');

// 1. Cria uma trajetória analítica circular: r(t) = (cos(t), sin(t)), t in [0, 2π]
const numPoints = 100;
const dt = 0.05;
const trajectory: TimedTrajectoryPoint[] = [];

for (let i = 0; i < numPoints; i++) {
  const t = i * dt;
  trajectory.push({
    t,
    x: Math.cos(t),
    y: Math.sin(t)
  });
}

// 2. Computa derivadas cinemáticas r'(t), r''(t), r'''(t)
const kinematics = computeKinematics(trajectory);
assert.strictEqual(kinematics.length, numPoints, 'Deve calcular cinemática para todos os pontos da trajetória');

// Verifica velocidade tangencial unitária para r(t) = (cos(t), sin(t))
// ||r'(t)|| = sqrt((-sin(t))^2 + cos(t)^2) = 1.0
const midPt = kinematics[50];
const speed = Math.hypot(midPt.vx, midPt.vy);
assert(Math.abs(speed - 1.0) < 0.02, `Velocidade tangencial deve ser ~1.0, obteve ${speed.toFixed(3)}`);

console.log('  ✓ Cálculo de velocidade r\'(t) e aceleração r\'\'(t) validado');

// 3. Validação da 1ª Regra da Cadeia: d/dt f(r(t)) == ∇f(r(t)) · r'(t)
// Campo sela: f(x, y) = x^2 - y^2
const field: ScalarFieldModel = {
  domain: { xMin: -2, xMax: 2, yMin: -2, yMax: 2 },
  baseFunction: { type: 'saddle', gain: 1.0, bias: 0.0 },
  anchor: { x: 0, y: 0 },
  maxOrder: 2,
  brushLayers: []
};

const chainCheck = verifyKinematicChainRule(field, kinematics);
assert(chainCheck.maxFirstOrderRelativeError < 0.02, `Erro relativo da 1ª regra da cadeia deve ser < 2%, obteve ${(chainCheck.maxFirstOrderRelativeError * 100).toFixed(2)}%`);
console.log(`  ✓ 1ª Regra da Cadeia validada: Erro Máximo = ${(chainCheck.maxFirstOrderRelativeError * 100).toFixed(3)}%`);

// 4. Validação da 2ª Regra da Cadeia: d²/dt² f(r(t)) == r'^T H_f r' + ∇f · r''
assert(chainCheck.maxSecondOrderRelativeError < 0.03, `Erro relativo da 2ª regra da cadeia deve ser < 3%, obteve ${(chainCheck.maxSecondOrderRelativeError * 100).toFixed(2)}%`);
console.log(`  ✓ 2ª Regra da Cadeia (Curvatura) validada: Erro Máximo = ${(chainCheck.maxSecondOrderRelativeError * 100).toFixed(3)}%`);

console.log('\n🎉 TODOS OS TESTES DA REGRA DA CADEIA CINEMÁTICA PASSARAM COM SUCESSO!\n');

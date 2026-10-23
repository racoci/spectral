import assert from 'assert';

// Módulo TypeScript do rastreador a ser implementado na Green Phase
import {
  RidgeState4D,
  KalmanRidgeFilter,
  MahalanobisAssociator,
  type ObservedPeak
} from '../src/lib/tracking/probabilistic_tracker';

console.log('🧪 Iniciando Teste TDD (Red Phase): Rastreador Probabilístico EKF e Associação Mahalanobis (TS)...');

// 1. Teste de Inicialização e Transição de Estado 4D
const dt = 0.005; // 5 ms
const ekf = new KalmanRidgeFilter({
  f: 440.0,
  fDot: 50.0,
  lnA: -0.22,
  aDotRel: -1.0,
  dt
});

assert.strictEqual(ekf.state.f, 440.0, 'Frequência inicial deve ser 440 Hz');
assert.strictEqual(ekf.state.fDot, 50.0, 'Taxa de chirp inicial deve ser 50 Hz/s');

// Predição temporal: f_{t+1} = f_t + f_dot * dt = 440 + 50 * 0.005 = 440.25 Hz
ekf.predict();
assert(Math.abs(ekf.state.f - 440.25) < 1e-4, 'Predição de frequência deve incorporar taxa de chirp');

// Atualização de observação
const obs: ObservedPeak = { f: 441.0, lnA: -0.25 };
ekf.update(obs);

assert(ekf.state.f > 440.25 && ekf.state.f <= 441.0, 'Filtro deve convergir em direção à observação');
console.log(`  ✓ Predição e atualização EKF 4D validadas: f_pred=440.25 -> f_upd=${ekf.state.f.toFixed(2)} Hz`);

// 2. Teste de Distância de Mahalanobis
const associator = new MahalanobisAssociator(9.21);
const peakClose: ObservedPeak = { f: ekf.state.f + 0.5, lnA: ekf.state.lnA + 0.02 };
const dClose = associator.computeMahalanobisDistance(ekf, peakClose);
assert(dClose < 9.21, `Pico próximo deve estar dentro do gate de Mahalanobis, obteve d=${dClose.toFixed(2)}`);

const peakFar: ObservedPeak = { f: 880.0, lnA: -2.0 };
const dFar = associator.computeMahalanobisDistance(ekf, peakFar);
assert(dFar > 9.21, `Pico distante deve ser rejeitado pelo gate de Mahalanobis, obteve d=${dFar.toFixed(2)}`);
console.log('  ✓ Gating elíptico e distância de Mahalanobis validados com sucesso');

console.log('\n🎉 TODOS OS TESTES DO RASTREADOR PROBABILÍSTICO EM TYPESCRIPT PASSARAM!\n');

import type { ScalarFieldModel } from './taylor_jet_math';
import { evaluateBaseFunction } from './taylor_jet_math';

export interface TimedTrajectoryPoint {
  t: number;
  x: number;
  y: number;
}

export interface KinematicPoint {
  t: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
  ax: number;
  ay: number;
  jx: number;
  jy: number;
}

export interface ChainRuleVerificationResult {
  maxFirstOrderRelativeError: number;
  maxSecondOrderRelativeError: number;
  meanFirstOrderRelativeError: number;
  meanSecondOrderRelativeError: number;
}

/**
 * Calcula velocidade r'(t), aceleração r''(t) e jerk r'''(t) usando diferenças finitas centradas.
 */
export function computeKinematics(trajectory: TimedTrajectoryPoint[]): KinematicPoint[] {
  const n = trajectory.length;
  if (n === 0) return [];

  const result: KinematicPoint[] = [];

  for (let i = 0; i < n; i++) {
    const cur = trajectory[i];
    const prev = trajectory[Math.max(0, i - 1)];
    const next = trajectory[Math.min(n - 1, i + 1)];
    const prev2 = trajectory[Math.max(0, i - 2)];
    const next2 = trajectory[Math.min(n - 1, i + 2)];

    const dt = Math.max(1e-4, (next.t - prev.t) * 0.5);

    // Velocidade r'(t)
    const vx = (next.x - prev.x) / (2.0 * dt);
    const vy = (next.y - prev.y) / (2.0 * dt);

    // Aceleração r''(t)
    const ax = (next.x - 2.0 * cur.x + prev.x) / (dt * dt);
    const ay = (next.y - 2.0 * cur.y + prev.y) / (dt * dt);

    // Jerk r'''(t)
    const jx = (next2.x - 2.0 * next.x + 2.0 * prev.x - prev2.x) / (2.0 * dt * dt * dt);
    const jy = (next2.y - 2.0 * next.y + 2.0 * prev.y - prev2.y) / (2.0 * dt * dt * dt);

    result.push({
      t: cur.t,
      x: cur.x,
      y: cur.y,
      vx,
      vy,
      ax,
      ay,
      jx,
      jy
    });
  }

  return result;
}

/**
 * Verifica as identidades matemáticas da regra da cadeia ao longo da trajetória sobre o campo.
 */
export function verifyKinematicChainRule(
  field: ScalarFieldModel,
  kinematics: KinematicPoint[]
): ChainRuleVerificationResult {
  const n = kinematics.length;
  if (n < 4) {
    return {
      maxFirstOrderRelativeError: 0,
      maxSecondOrderRelativeError: 0,
      meanFirstOrderRelativeError: 0,
      meanSecondOrderRelativeError: 0
    };
  }

  const h = 0.0001;
  const evalF = (x: number, y: number) =>
    evaluateBaseFunction(field.baseFunction.type, x, y, field.baseFunction.gain, field.baseFunction.bias);

  // Amostras de f(r(t)) ao longo do tempo
  const g = kinematics.map((k) => evalF(k.x, k.y));

  let maxErr1 = 0;
  let maxErr2 = 0;
  let sumErr1 = 0;
  let sumErr2 = 0;
  let count = 0;

  // Itera pontos internos (ignorando bordas sujeitas a assimetria de diferenças finitas)
  for (let i = 2; i < n - 2; i++) {
    const k = kinematics[i];
    const dt = (kinematics[i + 1].t - kinematics[i - 1].t) * 0.5;

    // 1. Derivada temporal numérica empírica g'(t)
    const gPrimeEmpirical = (g[i + 1] - g[i - 1]) / (2.0 * dt);

    // Gradiente analítico espacial ∇f = (fx, fy)
    const fx = (evalF(k.x + h, k.y) - evalF(k.x - h, k.y)) / (2.0 * h);
    const fy = (evalF(k.x, k.y + h) - evalF(k.x, k.y - h)) / (2.0 * h);

    // 1ª Regra da Cadeia analítica: ∇f · r'(t)
    const gPrimeAnalytical = fx * k.vx + fy * k.vy;

    const denom1 = Math.max(0.1, Math.abs(gPrimeAnalytical));
    const relErr1 = Math.abs(gPrimeEmpirical - gPrimeAnalytical) / denom1;

    // 2. Derivada temporal segunda empírica g''(t)
    const gDoublePrimeEmpirical = (g[i + 1] - 2.0 * g[i] + g[i - 1]) / (dt * dt);

    // Hessiana espacial analítica H_f
    const val0 = g[i];
    const fxx = (evalF(k.x + h, k.y) - 2.0 * val0 + evalF(k.x - h, k.y)) / (h * h);
    const fyy = (evalF(k.x, k.y + h) - 2.0 * val0 + evalF(k.x, k.y - h)) / (h * h);
    const fxy =
      (evalF(k.x + h, k.y + h) - evalF(k.x + h, k.y - h) - evalF(k.x - h, k.y + h) + evalF(k.x - h, k.y - h)) /
      (4.0 * h * h);

    // 2ª Regra da Cadeia analítica: r'^T H_f r' + ∇f · r''
    const quadForm = k.vx * (fxx * k.vx + fxy * k.vy) + k.vy * (fxy * k.vx + fyy * k.vy);
    const accelTerm = fx * k.ax + fy * k.ay;
    const gDoublePrimeAnalytical = quadForm + accelTerm;

    const denom2 = Math.max(0.1, Math.abs(gDoublePrimeAnalytical));
    const relErr2 = Math.abs(gDoublePrimeEmpirical - gDoublePrimeAnalytical) / denom2;

    maxErr1 = Math.max(maxErr1, relErr1);
    maxErr2 = Math.max(maxErr2, relErr2);
    sumErr1 += relErr1;
    sumErr2 += relErr2;
    count++;
  }

  return {
    maxFirstOrderRelativeError: maxErr1,
    maxSecondOrderRelativeError: maxErr2,
    meanFirstOrderRelativeError: count > 0 ? sumErr1 / count : 0,
    meanSecondOrderRelativeError: count > 0 ? sumErr2 / count : 0
  };
}

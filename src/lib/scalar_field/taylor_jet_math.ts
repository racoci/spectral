/**
 * Núcleo Matemático de Campos Escalares 2D e Jatos de Taylor (Ordens 0 a 8).
 * Suporta expansões locais, decomposição espectral de curvatura e perfis angulares.
 */

export type BaseFunctionType = 'zero' | 'saddle' | 'basin' | 'waves' | 'gaussian' | 'procedural';

export interface BaseFunctionConfig {
  type: BaseFunctionType;
  gain: number;
  bias: number;
  sigma?: number;
}

export interface Domain2D {
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

export interface Point2D {
  x: number;
  y: number;
}

export interface Order0Jet {
  value: number;
}

export interface Order1Jet {
  fx: number;
  fy: number;
  magnitude: number;
  angleRad: number;
}

export interface Order2Jet {
  fxx: number;
  fxy: number;
  fyy: number;
  lambda1: number;
  lambda2: number;
  principalAngleRad: number;
}

export interface OrderNJet {
  order: number;
  samples: number[]; // Array de tamanho n + 1
  anglesRad: number[]; // Array de tamanho n + 1
}

export type OrderNJets = Record<number, OrderNJet>;

export interface TaylorJet {
  anchor: Point2D;
  order0: Order0Jet;
  order1: Order1Jet;
  order2: Order2Jet;
  orders: OrderNJets;
}

export interface BrushStroke {
  id: string;
  operator: 'add' | 'sub' | 'displace' | 'smooth' | 'snap';
  radius: number;
  intensity: number;
  points: Point2D[];
  profile1D: number[];
}

export interface ScalarFieldModel {
  domain: Domain2D;
  baseFunction: BaseFunctionConfig;
  anchor: Point2D;
  maxOrder: number;
  brushLayers: BrushStroke[];
}

/**
 * Avalia o valor da função analítica base em um ponto (x, y).
 */
export function evaluateBaseFunction(
  type: BaseFunctionType,
  x: number,
  y: number,
  gain = 1.0,
  bias = 0.0,
  sigma = 1.0
): number {
  switch (type) {
    case 'zero':
      return bias;
    case 'saddle':
      return gain * (x * x - y * y) + bias;
    case 'basin':
      return gain * (x * x + y * y) + bias;
    case 'waves':
      return gain * (Math.sin(3.0 * x) * Math.cos(3.0 * y)) + bias;
    case 'gaussian': {
      const s2 = 2.0 * sigma * sigma;
      return gain * Math.exp(-(x * x + y * y) / s2) + bias;
    }
    case 'procedural': {
      // Combinação mista suave não-separável
      const r = Math.sqrt(x * x + y * y) + 1e-6;
      return gain * (Math.sin(4.0 * r) / r + 0.5 * Math.sin(2.0 * x + y)) + bias;
    }
    default:
      return bias;
  }
}

/**
 * Calcula o sistema de autovalores e autovetores da matriz Hessiana 2x2.
 */
export function computeHessianEigensystem(
  fxx: number,
  fxy: number,
  fyy: number
): { lambda1: number; lambda2: number; principalAngleRad: number } {
  const tr = fxx + fyy;
  const det = fxx * fyy - fxy * fxy;
  const disc = Math.sqrt(Math.max(0.0, (fxx - fyy) * (fxx - fyy) + 4.0 * fxy * fxy));

  const lambda1 = (tr + disc) * 0.5;
  const lambda2 = (tr - disc) * 0.5;
  const principalAngleRad = 0.5 * Math.atan2(2.0 * fxy, fxx - fyy);

  return { lambda1, lambda2, principalAngleRad };
}

/**
 * Avalia derivadas direcionais numéricas de ordem n ao longo de um ângulo theta.
 * Usa diferenças finitas centradas de alta ordem para precisão numérica estrita.
 */
function computeDirectionalDerivative(
  field: ScalarFieldModel,
  x0: number,
  y0: number,
  thetaRad: number,
  order: number
): number {
  const ux = Math.cos(thetaRad);
  const uy = Math.sin(thetaRad);
  const h = 0.005;

  const evalAt = (t: number) => {
    return evaluateBaseFunction(
      field.baseFunction.type,
      x0 + t * ux,
      y0 + t * uy,
      field.baseFunction.gain,
      field.baseFunction.bias,
      field.baseFunction.sigma
    );
  };

  if (order === 0) return evalAt(0);

  if (order === 1) {
    return (evalAt(h) - evalAt(-h)) / (2.0 * h);
  }

  if (order === 2) {
    return (evalAt(h) - 2.0 * evalAt(0) + evalAt(-h)) / (h * h);
  }

  if (order === 3) {
    return (evalAt(2 * h) - 2 * evalAt(h) + 2 * evalAt(-h) - evalAt(-2 * h)) / (2.0 * h * h * h);
  }

  if (order === 4) {
    return (
      (evalAt(2 * h) - 4 * evalAt(h) + 6 * evalAt(0) - 4 * evalAt(-h) + evalAt(-2 * h)) /
      (h * h * h * h)
    );
  }

  // Ordens 5 a 8: aproximação por diferenças finitas centrais de passo escalonado
  const hN = 0.02;
  const stepCount = order + 1;
  let sum = 0;
  // Coeficientes binomiais com sinal alternado para derivada de ordem n
  for (let k = 0; k <= order; k++) {
    const coeff = (k % 2 === 0 ? 1 : -1) * binomialCoeff(order, k);
    const offset = (order / 2.0 - k) * hN;
    sum += coeff * evalAt(offset);
  }
  return sum / Math.pow(hN, order);
}

function binomialCoeff(n: number, k: number): number {
  if (k < 0 || k > n) return 0;
  if (k === 0 || k === n) return 1;
  let res = 1;
  for (let i = 1; i <= k; i++) {
    res = (res * (n - i + 1)) / i;
  }
  return res;
}

/**
 * Computa o Jato de Taylor completo de ordens 0 a maxOrder em torno do ponto (x0, y0).
 */
export function computeTaylorJetAt(
  field: ScalarFieldModel,
  x0: number,
  y0: number,
  maxOrder = 8
): TaylorJet {
  // Ordem 0: valor no ponto
  const val0 = evaluateBaseFunction(
    field.baseFunction.type,
    x0,
    y0,
    field.baseFunction.gain,
    field.baseFunction.bias,
    field.baseFunction.sigma
  );

  // Ordem 1: Gradiente analítico ou por diferenças
  const h = 0.0001;
  const fx =
    (evaluateBaseFunction(field.baseFunction.type, x0 + h, y0, field.baseFunction.gain, field.baseFunction.bias) -
      evaluateBaseFunction(field.baseFunction.type, x0 - h, y0, field.baseFunction.gain, field.baseFunction.bias)) /
    (2.0 * h);
  const fy =
    (evaluateBaseFunction(field.baseFunction.type, x0, y0 + h, field.baseFunction.gain, field.baseFunction.bias) -
      evaluateBaseFunction(field.baseFunction.type, x0, y0 - h, field.baseFunction.gain, field.baseFunction.bias)) /
    (2.0 * h);

  const magnitude = Math.sqrt(fx * fx + fy * fy);
  const angleRad = Math.atan2(fy, fx);

  // Ordem 2: Hessiana
  const fxx =
    (evaluateBaseFunction(field.baseFunction.type, x0 + h, y0, field.baseFunction.gain, field.baseFunction.bias) -
      2.0 * val0 +
      evaluateBaseFunction(field.baseFunction.type, x0 - h, y0, field.baseFunction.gain, field.baseFunction.bias)) /
    (h * h);

  const fyy =
    (evaluateBaseFunction(field.baseFunction.type, x0, y0 + h, field.baseFunction.gain, field.baseFunction.bias) -
      2.0 * val0 +
      evaluateBaseFunction(field.baseFunction.type, x0, y0 - h, field.baseFunction.gain, field.baseFunction.bias)) /
    (h * h);

  const fxy =
    (evaluateBaseFunction(field.baseFunction.type, x0 + h, y0 + h, field.baseFunction.gain, field.baseFunction.bias) -
      evaluateBaseFunction(field.baseFunction.type, x0 + h, y0 - h, field.baseFunction.gain, field.baseFunction.bias) -
      evaluateBaseFunction(field.baseFunction.type, x0 - h, y0 + h, field.baseFunction.gain, field.baseFunction.bias) +
      evaluateBaseFunction(field.baseFunction.type, x0 - h, y0 - h, field.baseFunction.gain, field.baseFunction.bias)) /
    (4.0 * h * h);

  const eigen = computeHessianEigensystem(fxx, fxy, fyy);

  // Amostragem polar para todas as ordens n de 1 a maxOrder
  const orders: OrderNJets = {};

  for (let n = 1; n <= maxOrder; n++) {
    const sampleCount = n + 1; // n + 1 amostras independentes em 2D
    const samples: number[] = [];
    const anglesRad: number[] = [];

    for (let k = 0; k < sampleCount; k++) {
      const theta = (2.0 * Math.PI * k) / sampleCount;
      anglesRad.push(theta);

      if (n === 1) {
        samples.push(fx * Math.cos(theta) + fy * Math.sin(theta));
      } else if (n === 2) {
        const c = Math.cos(theta);
        const s = Math.sin(theta);
        samples.push(fxx * c * c + 2.0 * fxy * c * s + fyy * s * s);
      } else {
        samples.push(computeDirectionalDerivative(field, x0, y0, theta, n));
      }
    }

    orders[n] = { order: n, samples, anglesRad };
  }

  return {
    anchor: { x: x0, y: y0 },
    order0: { value: val0 },
    order1: { fx, fy, magnitude, angleRad },
    order2: { fxx, fxy, fyy, lambda1: eigen.lambda1, lambda2: eigen.lambda2, principalAngleRad: eigen.principalAngleRad },
    orders
  };
}

/**
 * Avalia o perfil angular q_n(theta) para qualquer ângulo theta por interpolação de Fourier.
 */
export function evaluateAngularProfile(jet: TaylorJet, order: number, thetaRad: number): number {
  if (order === 0) return jet.order0.value;
  if (order === 1) {
    return jet.order1.fx * Math.cos(thetaRad) + jet.order1.fy * Math.sin(thetaRad);
  }
  if (order === 2) {
    const c = Math.cos(thetaRad);
    const s = Math.sin(thetaRad);
    return jet.order2.fxx * c * c + 2.0 * jet.order2.fxy * c * s + jet.order2.fyy * s * s;
  }

  const nJet = jet.orders[order];
  if (!nJet) return 0;

  // Interpolação trigonométrica sobre as n+1 amostras
  const M = nJet.samples.length;
  let val = 0;
  for (let k = 0; k < M; k++) {
    const th_k = nJet.anglesRad[k];
    const diff = thetaRad - th_k;
    // Kernel de Dirichlet normalizado
    const sinc = Math.abs(diff) < 1e-5 ? 1.0 : Math.sin((M * diff) / 2.0) / (M * Math.sin(diff / 2.0));
    val += nJet.samples[k] * sinc;
  }
  return val;
}

/**
 * Reconstrói a aproximação de Taylor de f(p + delta) até maxOrder.
 */
export function reconstructTaylorExpansion(
  jet: TaylorJet,
  deltaX: number,
  deltaY: number,
  maxOrder = 8
): number {
  let result = jet.order0.value;

  if (maxOrder >= 1) {
    result += jet.order1.fx * deltaX + jet.order1.fy * deltaY;
  }

  if (maxOrder >= 2) {
    result += 0.5 * (jet.order2.fxx * deltaX * deltaX + 2.0 * jet.order2.fxy * deltaX * deltaY + jet.order2.fyy * deltaY * deltaY);
  }

  const r = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
  if (r < 1e-9) return result;

  const theta = Math.atan2(deltaY, deltaX);

  let factorial = 2.0; // Começa em 2!
  for (let n = 3; n <= maxOrder; n++) {
    factorial *= n;
    const qN = evaluateAngularProfile(jet, n, theta);
    result += (Math.pow(r, n) / factorial) * qN;
  }

  return result;
}

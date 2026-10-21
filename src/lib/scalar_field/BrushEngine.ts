import type { Domain2D, Point2D, BrushStroke } from './taylor_jet_math';

export interface GridField2D {
  width: number;
  height: number;
  values: Float32Array;
  domain: Domain2D;
}

/**
 * Filtro inercial com amortecimento para suavização de trajetórias de mouse e caneta.
 * Fórmula: p_{t+1} = p_t + α(m_t - p_t)
 */
export class InertialPointerFilter {
  public x: number;
  public y: number;

  constructor(initX = 0.0, initY = 0.0) {
    this.x = initX;
    this.y = initY;
  }

  public reset(x: number, y: number) {
    this.x = x;
    this.y = y;
  }

  public update(rawX: number, rawY: number, alpha = 0.5): Point2D {
    const a = Math.max(0.0, Math.min(1.0, alpha));
    this.x = this.x + a * (rawX - this.x);
    this.y = this.y + a * (rawY - this.y);
    return { x: this.x, y: this.y };
  }
}

/**
 * Aplica o operador de deformação espacial do brush em uma grade 2D.
 */
export function applyBrushKernel(grid: GridField2D, stroke: BrushStroke) {
  const { width, height, values, domain } = grid;
  const radius = Math.max(0.005, stroke.radius);
  const intensity = stroke.intensity;
  const s2 = 2.0 * radius * radius;

  // Itera sobre todos os pontos da trajetória do traço
  for (let ptIdx = 0; ptIdx < stroke.points.length; ptIdx++) {
    const pt = stroke.points[ptIdx];
    const ptWeight = stroke.profile1D[ptIdx] !== undefined ? stroke.profile1D[ptIdx] : 1.0;

    // Converte centro do ponto do brush para coordenadas de grade
    const uCenter = (pt.x - domain.xMin) / (domain.xMax - domain.xMin);
    const vCenter = (pt.y - domain.yMin) / (domain.yMax - domain.yMin);

    const gx = Math.floor(uCenter * width);
    const gy = Math.floor(vCenter * height);

    const rCellsX = Math.ceil((radius / (domain.xMax - domain.xMin)) * width * 2.5);
    const rCellsY = Math.ceil((radius / (domain.yMax - domain.yMin)) * height * 2.5);

    const minX = Math.max(0, gx - rCellsX);
    const maxX = Math.min(width - 1, gx + rCellsX);
    const minY = Math.max(0, gy - rCellsY);
    const maxY = Math.min(height - 1, gy + rCellsY);

    if (stroke.operator === 'smooth') {
      // Difusão Laplaciana Local
      const temp = new Float32Array(values);
      const gamma = 0.5 * intensity * ptWeight;

      for (let y = minY; y <= maxY; y++) {
        for (let x = minX; x <= maxX; x++) {
          const idx = y * width + x;
          const wx = domain.xMin + (x / width) * (domain.xMax - domain.xMin);
          const wy = domain.yMin + (y / height) * (domain.yMax - domain.yMin);
          const d2 = (wx - pt.x) * (wx - pt.x) + (wy - pt.y) * (wy - pt.y);

          if (d2 <= radius * radius) {
            const left = x > 0 ? temp[idx - 1] : temp[idx];
            const right = x < width - 1 ? temp[idx + 1] : temp[idx];
            const up = y > 0 ? temp[idx - width] : temp[idx];
            const down = y < height - 1 ? temp[idx + width] : temp[idx];
            const laplacian = (left + right + up + down) * 0.25 - temp[idx];

            const weight = Math.exp(-d2 / s2);
            values[idx] += laplacian * gamma * weight;
          }
        }
      }
    } else {
      // Adição ou Subtração Gaussiana
      const sign = stroke.operator === 'sub' ? -1.0 : 1.0;
      const deltaAmp = sign * intensity * ptWeight;

      for (let y = minY; y <= maxY; y++) {
        for (let x = minX; x <= maxX; x++) {
          const idx = y * width + x;
          const wx = domain.xMin + (x / width) * (domain.xMax - domain.xMin);
          const wy = domain.yMin + (y / height) * (domain.yMax - domain.yMin);
          const d2 = (wx - pt.x) * (wx - pt.x) + (wy - pt.y) * (wy - pt.y);

          if (d2 <= radius * radius * 3.0) {
            const weight = Math.exp(-d2 / s2);
            values[idx] += deltaAmp * weight;
          }
        }
      }
    }
  }
}

/**
 * Projeta ortogonalmente um ponto (px, py) sobre a curva de nível de valor targetLevel mais próxima.
 * Fórmula: p_snap = p - (f(p) - C) / ||∇f||² * ∇f
 */
export function computeEquipotentialSnap(
  px: number,
  py: number,
  targetLevel: number,
  evalF: (x: number, y: number) => number,
  evalGrad: (x: number, y: number) => { fx: number; fy: number }
): Point2D {
  let curX = px;
  let curY = py;

  // Realiza 2 passos de projeção de Newton-Raphson para convergência estrita
  for (let iter = 0; iter < 2; iter++) {
    const val = evalF(curX, curY);
    const grad = evalGrad(curX, curY);
    const norm2 = grad.fx * grad.fx + grad.fy * grad.fy + 1e-6;

    const diff = val - targetLevel;
    const stepX = -(diff / norm2) * grad.fx;
    const stepY = -(diff / norm2) * grad.fy;

    curX += stepX;
    curY += stepY;
  }

  return { x: curX, y: curY };
}

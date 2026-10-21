import type { Domain2D, TaylorJet, Point2D } from './taylor_jet_math';
import { evaluateAngularProfile } from './taylor_jet_math';

export interface PolarHandle {
  id: string;
  order: number;
  index: number;
  worldX: number;
  worldY: number;
  screenX: number;
  screenY: number;
  color: string;
  label: string;
}

/**
 * Converte coordenadas do mundo cartesiano (x, y) em coordenadas de píxel na tela.
 */
export function worldToScreen(
  x: number,
  y: number,
  width: number,
  height: number,
  domain: Domain2D
): Point2D {
  const u = (x - domain.xMin) / (domain.xMax - domain.xMin);
  const v = (y - domain.yMin) / (domain.yMax - domain.yMin);
  return {
    x: u * width,
    y: (1.0 - v) * height
  };
}

/**
 * Converte coordenadas de píxel na tela (px, py) em coordenadas do mundo cartesiano.
 */
export function screenToWorld(
  px: number,
  py: number,
  width: number,
  height: number,
  domain: Domain2D
): Point2D {
  const u = px / width;
  const v = 1.0 - py / height;
  return {
    x: domain.xMin + u * (domain.xMax - domain.xMin),
    y: domain.yMin + v * (domain.yMax - domain.yMin)
  };
}

const ORDER_COLORS: Record<number, string> = {
  0: '#ffffff',
  1: '#2dd4bf', // teal
  2: '#fbbf24', // amber
  3: '#38bdf8', // sky blue
  4: '#c084fc', // purple
  5: '#4ade80', // green
  6: '#f87171', // red/coral
  7: '#fb923c', // orange
  8: '#f472b6'  // pink
};

/**
 * Gerenciador de geometria, hit-testing e renderização das alças polares de Taylor.
 */
export class PolarHandlesManager {
  constructor(
    private width: number,
    private height: number,
    private domain: Domain2D
  ) {}

  public updateDimensions(width: number, height: number, domain: Domain2D) {
    this.width = width;
    this.height = height;
    this.domain = { ...domain };
  }

  /**
   * Computa as posições e propriedades das alças polares para o jato atual.
   */
  public computeHandles(jet: TaylorJet): PolarHandle[] {
    const handles: PolarHandle[] = [];
    const anc = jet.anchor;
    const centerScreen = worldToScreen(anc.x, anc.y, this.width, this.height, this.domain);

    // Ordem 0: Alça central no ponto de ancoragem
    handles.push({
      id: 'handle-ord-0',
      order: 0,
      index: 0,
      worldX: anc.x,
      worldY: anc.y,
      screenX: centerScreen.x,
      screenY: centerScreen.y,
      color: ORDER_COLORS[0],
      label: `f(p) = ${jet.order0.value.toFixed(2)}`
    });

    const scaleRadiusScreen = 60.0; // Raio visual base na tela em pixels

    // Ordem 1: Vetor gradiente orientado
    const gradMag = jet.order1.magnitude;
    const gradAngle = jet.order1.angleRad;
    const gradLenScreen = Math.min(120, Math.max(20, gradMag * 30.0));
    const gradScreenX = centerScreen.x + Math.cos(gradAngle) * gradLenScreen;
    const gradScreenY = centerScreen.y - Math.sin(gradAngle) * gradLenScreen;
    const gradWorld = screenToWorld(gradScreenX, gradScreenY, this.width, this.height, this.domain);

    handles.push({
      id: 'handle-ord-1-grad',
      order: 1,
      index: 0,
      worldX: gradWorld.x,
      worldY: gradWorld.y,
      screenX: gradScreenX,
      screenY: gradScreenY,
      color: ORDER_COLORS[1],
      label: `∇f = (${jet.order1.fx.toFixed(2)}, ${jet.order1.fy.toFixed(2)})`
    });

    // Ordem 2: Curvatura principal da Hessiana
    const thPrin = jet.order2.principalAngleRad;
    const l1 = Math.abs(jet.order2.lambda1);
    const rL1Screen = Math.min(100, Math.max(15, l1 * 15.0));
    const h2x = centerScreen.x + Math.cos(thPrin) * rL1Screen;
    const h2y = centerScreen.y - Math.sin(thPrin) * rL1Screen;
    const h2World = screenToWorld(h2x, h2y, this.width, this.height, this.domain);

    handles.push({
      id: 'handle-ord-2-curv',
      order: 2,
      index: 0,
      worldX: h2World.x,
      worldY: h2World.y,
      screenX: h2x,
      screenY: h2y,
      color: ORDER_COLORS[2],
      label: `H_f (λ₁=${jet.order2.lambda1.toFixed(2)})`
    });

    // Ordens 3 a 8: n + 1 alças angulares direcionais
    const maxOrder = Math.max(2, ...Object.keys(jet.orders).map(Number));

    for (let n = 3; n <= maxOrder; n++) {
      const nJet = jet.orders[n];
      if (!nJet) continue;

      const count = nJet.samples.length;
      for (let k = 0; k < count; k++) {
        const theta = nJet.anglesRad[k];
        const val = nJet.samples[k];
        const rScreen = scaleRadiusScreen + (val / (Math.abs(val) + 2.0)) * 30.0;
        const sx = centerScreen.x + Math.cos(theta) * rScreen;
        const sy = centerScreen.y - Math.sin(theta) * rScreen;
        const wld = screenToWorld(sx, sy, this.width, this.height, this.domain);

        handles.push({
          id: `handle-ord-${n}-${k}`,
          order: n,
          index: k,
          worldX: wld.x,
          worldY: wld.y,
          screenX: sx,
          screenY: sy,
          color: ORDER_COLORS[n] || '#e2e8f0',
          label: `D^${n} (${k}): ${val.toFixed(2)}`
        });
      }
    }

    return handles;
  }

  /**
   * Realiza hit-testing com priorização da alça ativa em caso de sobreposição.
   */
  public hitTest(
    px: number,
    py: number,
    handles: PolarHandle[],
    hitRadius = 12,
    selectedHandleId?: string
  ): PolarHandle | null {
    // Se a alça atualmente selecionada estiver sob o cursor, retorne-a prioritariamente
    if (selectedHandleId) {
      const selected = handles.find((h) => h.id === selectedHandleId);
      if (selected) {
        const d = Math.hypot(px - selected.screenX, py - selected.screenY);
        if (d <= hitRadius) return selected;
      }
    }

    // Caso contrário, busca a alça mais próxima dentro do raio de tolerância
    let closest: PolarHandle | null = null;
    let minDist = hitRadius;

    for (const h of handles) {
      const d = Math.hypot(px - h.screenX, py - h.screenY);
      if (d <= minDist) {
        minDist = d;
        closest = h;
      }
    }

    return closest;
  }

  /**
   * Desenha os perfis angulares polares coloridos e as alças no contexto 2D.
   */
  public draw(
    ctx: CanvasRenderingContext2D,
    handles: PolarHandle[],
    jet: TaylorJet,
    selectedHandleId: string | null,
    activeOrder = 1
  ) {
    const center = worldToScreen(jet.anchor.x, jet.anchor.y, this.width, this.height, this.domain);

    // 1. Desenha anéis polares de referência discretos
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
    ctx.lineWidth = 1;
    for (const r of [30, 60, 90]) {
      ctx.beginPath();
      ctx.arc(center.x, center.y, r, 0, Math.PI * 2);
      ctx.stroke();
    }

    // 2. Desenha perfis angulares polares q_n(θ)
    const maxOrder = Math.max(2, ...Object.keys(jet.orders).map(Number));
    for (let n = 1; n <= maxOrder; n++) {
      const isSelected = n === activeOrder;
      ctx.strokeStyle = ORDER_COLORS[n] || '#ffffff';
      ctx.lineWidth = isSelected ? 2.5 : 1.0;
      ctx.beginPath();

      for (let s = 0; s <= 100; s++) {
        const theta = (s / 100.0) * Math.PI * 2;
        const qN = evaluateAngularProfile(jet, n, theta);
        const rPix = 60.0 + (qN / (Math.abs(qN) + 2.0)) * 25.0;
        const px = center.x + Math.cos(theta) * rPix;
        const py = center.y - Math.sin(theta) * rPix;
        if (s === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      }
      ctx.closePath();
      ctx.stroke();
    }

    // 3. Desenha vetor de gradiente (Ordem 1)
    const hGrad = handles.find((h) => h.id === 'handle-ord-1-grad');
    if (hGrad) {
      ctx.strokeStyle = ORDER_COLORS[1];
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      ctx.moveTo(center.x, center.y);
      ctx.lineTo(hGrad.screenX, hGrad.screenY);
      ctx.stroke();
    }

    // 4. Desenha as alças interativas
    for (const h of handles) {
      const isAct = h.id === selectedHandleId;
      ctx.fillStyle = h.color;
      ctx.beginPath();
      ctx.arc(h.screenX, h.screenY, isAct ? 6 : 4, 0, Math.PI * 2);
      ctx.fill();

      ctx.strokeStyle = isAct ? '#ffffff' : '#03080e';
      ctx.lineWidth = isAct ? 2.0 : 1.2;
      ctx.stroke();
    }
  }
}

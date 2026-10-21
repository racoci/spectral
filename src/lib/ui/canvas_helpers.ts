export interface CanvasDimensions {
  width: number;
  height: number;
}

export interface HudCoordinates {
  x: number;
  y: number;
}

/**
 * Calcula a resolução física do buffer do canvas a partir das dimensões do contêiner e do DPR do dispositivo.
 */
export function computeCanvasDimensions(
  containerWidth: number,
  containerHeight: number,
  dpr = 1.0,
  maxDim = 2048
): CanvasDimensions {
  const safeDpr = Math.max(1.0, dpr);
  const w = Math.max(1, Math.min(maxDim, Math.round(containerWidth * safeDpr)));
  const h = Math.max(1, Math.min(maxDim, Math.round(containerHeight * safeDpr)));
  return { width: w, height: h };
}

/**
 * Clampa e inverte as coordenadas do HUD flutuante no cursor para nunca ocluir o ponteiro ou vazar do canvas.
 */
export function computeClampedHud(
  cursorX: number,
  cursorY: number,
  hudWidth: number,
  hudHeight: number,
  containerWidth: number,
  containerHeight: number,
  margin = 10
): HudCoordinates {
  const offset = 14;

  let x = cursorX + offset;
  let y = cursorY + offset;

  // Inverte para a esquerda se ultrapassar a margem direita
  if (x + hudWidth > containerWidth - margin) {
    x = cursorX - hudWidth - offset;
  }

  // Inverte para cima se ultrapassar a margem inferior
  if (y + hudHeight > containerHeight - margin) {
    y = cursorY - hudHeight - offset;
  }

  // Clampa estritamente dentro dos limites do contêiner
  x = Math.max(margin, Math.min(containerWidth - hudWidth - margin, x));
  y = Math.max(margin, Math.min(containerHeight - hudHeight - margin, y));

  return { x, y };
}

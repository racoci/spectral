/**
 * Funções auxiliares geométricas e de cálculo para componentes atômicos de UI.
 */

export function clampValue(val: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, val));
}

/**
 * Converte um valor no intervalo [min, max] para um ângulo em graus de [-135, +135] (varredura de 270°).
 */
export function computeKnobAngle(val: number, min: number, max: number): number {
  const norm = (clampValue(val, min, max) - min) / Math.max(1e-6, max - min);
  return -135 + norm * 270;
}

/**
 * Converte um ângulo em graus de [-135, +135] de volta para o valor no intervalo [min, max].
 */
export function computeKnobValueFromAngle(angleDeg: number, min: number, max: number): number {
  const clampedAngle = clampValue(angleDeg, -135, 135);
  const norm = (clampedAngle + 135) / 270.0;
  return min + norm * (max - min);
}

/**
 * Calcula a posição segura do divisor Splitter impedindo o colapso dos painéis adjacentes.
 */
export function computeSplitterBounds(
  currentPx: number,
  totalPx: number,
  minFirstPx: number,
  minSecondPx: number
): number {
  const minAllowed = minFirstPx;
  const maxAllowed = totalPx - minSecondPx;
  return clampValue(currentPx, minAllowed, maxAllowed);
}

/**
 * Calcula a posição (x, y) do tooltip flutuante com detecção de bordas da viewport (clamping e flip).
 */
export function computeSmartTooltipPosition(
  mouseX: number,
  mouseY: number,
  tipWidth: number,
  tipHeight: number,
  viewportWidth: number,
  viewportHeight: number,
  offset = 12
): { x: number; y: number } {
  let x = mouseX + offset;
  let y = mouseY + offset;

  // Inverte horizontalmente para a esquerda caso ultrapasse a borda direita
  if (x + tipWidth > viewportWidth) {
    x = mouseX - tipWidth - offset;
  }

  // Inverte verticalmente para cima caso ultrapasse a borda inferior
  if (y + tipHeight > viewportHeight) {
    y = mouseY - tipHeight - offset;
  }

  // Clampa estritamente dentro da viewport visível
  x = clampValue(x, 4, Math.max(4, viewportWidth - tipWidth - 4));
  y = clampValue(y, 4, Math.max(4, viewportHeight - tipHeight - 4));

  return { x, y };
}

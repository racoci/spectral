/**
 * Módulo de Exportação Vetorial SVG de Cristas Espectrais e Curvas de Nível.
 */

export interface RidgePoint {
  t: number;
  f: number;
  amplitude?: number;
}

export interface RidgeTrajectoryExport {
  id: number;
  color?: string;
  points: RidgePoint[];
}

export interface SvgExportOptions {
  width?: number;
  height?: number;
  title?: string;
  theme?: 'dark' | 'light';
  fMin?: number;
  fMax?: number;
  duration?: number;
}

/**
 * Gera um documento SVG vetorial escalável e autocontido com as cristas espectrais.
 */
export function exportRidgeTrajectoriesSvg(
  trajectories: RidgeTrajectoryExport[],
  options: SvgExportOptions = {}
): string {
  const width = options.width || 1200;
  const height = options.height || 600;
  const title = options.title || 'Spectral Ridge Trajectories';
  const isDark = options.theme !== 'light';

  const bgColor = isDark ? '#020617' : '#ffffff';
  const gridColor = isDark ? '#1e293b' : '#e2e8f0';
  const textColor = isDark ? '#94a3b8' : '#64748b';
  const defaultStroke = isDark ? '#38bdf8' : '#0284c7';

  // Margens de plotagem
  const padLeft = 60;
  const padRight = 30;
  const padTop = 40;
  const padBottom = 50;

  const plotW = width - padLeft - padRight;
  const plotH = height - padTop - padBottom;

  const fMin = options.fMin || 50;
  const fMax = options.fMax || 4000;
  const duration = options.duration || 2.0;

  let svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}">
  <style>
    .bg { fill: ${bgColor}; }
    .grid { stroke: ${gridColor}; stroke-width: 1; stroke-dasharray: 4,4; }
    .axis { stroke: ${gridColor}; stroke-width: 1.5; }
    .label { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 11px; fill: ${textColor}; }
    .title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; font-size: 16px; font-weight: 700; fill: ${textColor}; }
    .ridge-path { fill: none; stroke-linecap: round; stroke-linejoin: round; }
  </style>
  
  <rect width="${width}" height="${height}" class="bg" />
  
  <text x="${padLeft}" y="25" class="title">${title}</text>
`;

  // Linhas de grade de tempo e frequência
  for (let s = 0; s <= 4; s++) {
    const frac = s / 4;
    const x = padLeft + frac * plotW;
    const tVal = (frac * duration).toFixed(2);
    svg += `  <line x1="${x}" y1="${padTop}" x2="${x}" y2="${padTop + plotH}" class="grid" />\n`;
    svg += `  <text x="${x}" y="${height - 20}" text-anchor="middle" class="label">${tVal}s</text>\n`;
  }

  for (let f = 0; f <= 4; f++) {
    const frac = f / 4;
    const y = padTop + plotH - frac * plotH;
    const fVal = Math.round(fMin + frac * (fMax - fMin));
    svg += `  <line x1="${padLeft}" y1="${y}" x2="${padLeft + plotW}" y2="${y}" class="grid" />\n`;
    svg += `  <text x="${padLeft - 10}" y="${y + 4}" text-anchor="end" class="label">${fVal}Hz</text>\n`;
  }

  // Trajetórias de cristas
  for (const trk of trajectories) {
    if (!trk.points || trk.points.length === 0) continue;

    let pathD = '';
    for (let i = 0; i < trk.points.length; i++) {
      const pt = trk.points[i];
      const normT = Math.max(0, Math.min(1, pt.t / duration));
      const normF = Math.max(0, Math.min(1, (pt.f - fMin) / (fMax - fMin)));

      const x = padLeft + normT * plotW;
      const y = padTop + plotH - normF * plotH;

      if (i === 0) {
        pathD += `M ${x.toFixed(1)} ${y.toFixed(1)}`;
      } else {
        pathD += ` L ${x.toFixed(1)} ${y.toFixed(1)}`;
      }
    }

    const strokeColor = trk.color || defaultStroke;
    svg += `  <path d="${pathD}" stroke="${strokeColor}" stroke-width="2.5" class="ridge-path" />\n`;
  }

  svg += `</svg>\n`;
  return svg;
}

/**
 * Dispara o download de um arquivo SVG no navegador.
 */
export function triggerSvgDownload(svgContent: string, filename = 'spectral-export.svg'): void {
  if (typeof window === 'undefined') return;
  const blob = new Blob([svgContent], { type: 'image/svg+xml;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

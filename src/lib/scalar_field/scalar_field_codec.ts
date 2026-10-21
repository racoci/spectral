import type { ScalarFieldModel, BaseFunctionType, BrushStroke, Point2D } from './taylor_jet_math';

/**
 * Emite código Lean 4 formal e fortemente tipado para um modelo de campo escalar 2D.
 */
export function emitLeanScalarField(field: ScalarFieldModel): string {
  const dom = field.domain;
  const base = field.baseFunction;
  const anc = field.anchor;

  const brushLines = field.brushLayers.map((b) => {
    const ptStrings = b.points.map((p) => `{ x := ${p.x.toFixed(2)}, y := ${p.y.toFixed(2)} }`);
    const profStrings = b.profile1D.map((v) => v.toFixed(2));
    return `    { id := "${b.id}", op := "${b.operator}", radius := ${b.radius}, intensity := ${b.intensity},
      points := #[ ${ptStrings.join(', ')} ],
      profile := #[ ${profStrings.join(', ')} ] }`;
  });

  const brushesBlock =
    brushLines.length > 0 ? `\n${brushLines.join(',\n')}\n  ` : '';

  return `import Geometry.ScalarField

open Geometry

/-- Definição canônica do campo escalar bidimensional em Lean 4 -/
def myScalarField : ScalarField2D := {
  domain := { xMin := ${dom.xMin}, xMax := ${dom.xMax}, yMin := ${dom.yMin}, yMax := ${dom.yMax} },
  base := { baseType := "${base.type}", gain := ${base.gain}, bias := ${base.bias}, sigma := ${base.sigma || 1.0} },
  anchor := { x := ${anc.x}, y := ${anc.y} },
  maxOrder := ${field.maxOrder},
  brushes := #[${brushesBlock}]
}
`;
}

/**
 * Analisa e extrai um ScalarFieldModel a partir do código-fonte Lean 4.
 */
export function parseLeanScalarField(code: string): ScalarFieldModel {
  // 1. Domain
  const xMinMatch = code.match(/xMin\s*:=\s*(-?[0-9.]+)/);
  const xMaxMatch = code.match(/xMax\s*:=\s*(-?[0-9.]+)/);
  const yMinMatch = code.match(/yMin\s*:=\s*(-?[0-9.]+)/);
  const yMaxMatch = code.match(/yMax\s*:=\s*(-?[0-9.]+)/);

  const xMin = xMinMatch ? parseFloat(xMinMatch[1]) : -2.0;
  const xMax = xMaxMatch ? parseFloat(xMaxMatch[1]) : 2.0;
  const yMin = yMinMatch ? parseFloat(yMinMatch[1]) : -2.0;
  const yMax = yMaxMatch ? parseFloat(yMaxMatch[1]) : 2.0;

  // 2. Base Function
  const typeMatch = code.match(/baseType\s*:=\s*"([^"]+)"/);
  const gainMatch = code.match(/gain\s*:=\s*(-?[0-9.]+)/);
  const biasMatch = code.match(/bias\s*:=\s*(-?[0-9.]+)/);
  const sigmaMatch = code.match(/sigma\s*:=\s*(-?[0-9.]+)/);

  const baseType = (typeMatch ? typeMatch[1] : 'saddle') as BaseFunctionType;
  const gain = gainMatch ? parseFloat(gainMatch[1]) : 1.0;
  const bias = biasMatch ? parseFloat(biasMatch[1]) : 0.0;
  const sigma = sigmaMatch ? parseFloat(sigmaMatch[1]) : 1.0;

  // 3. Anchor Point
  const ancXMatch = code.match(/anchor\s*:=\s*\{\s*x\s*:=\s*(-?[0-9.]+)/);
  const ancYMatch = code.match(/y\s*:=\s*(-?[0-9.]+)\s*\}/);

  const anchorX = ancXMatch ? parseFloat(ancXMatch[1]) : 0.0;
  const anchorY = ancYMatch ? parseFloat(ancYMatch[1]) : 0.0;

  // 4. Max Order
  const orderMatch = code.match(/maxOrder\s*:=\s*(\d+)/);
  const maxOrder = orderMatch ? parseInt(orderMatch[1], 10) : 8;

  // 5. Brushes
  const brushLayers: BrushStroke[] = [];
  const brushMatches = code.matchAll(/\{\s*id\s*:=\s*"([^"]+)"\s*,\s*op\s*:=\s*"([^"]+)"\s*,\s*radius\s*:=\s*([0-9.]+)\s*,\s*intensity\s*:=\s*([0-9.]+)/g);

  for (const m of brushMatches) {
    const id = m[1];
    const op = m[2] as 'add' | 'sub' | 'displace' | 'smooth' | 'snap';
    const radius = parseFloat(m[3]);
    const intensity = parseFloat(m[4]);

    brushLayers.push({
      id,
      operator: op,
      radius,
      intensity,
      points: [],
      profile1D: []
    });
  }

  return {
    domain: { xMin, xMax, yMin, yMax },
    baseFunction: { type: baseType, gain, bias, sigma },
    anchor: { x: anchorX, y: anchorY },
    maxOrder,
    brushLayers
  };
}

/**
 * Localiza os limites [from, to] exatos de um parâmetro numérico no texto Lean 4.
 */
export function locateScalarFieldParamRange(
  code: string,
  paramPath: string
): { from: number; to: number } | null {
  // Parâmetros da função base: 'base.gain', 'base.bias', 'base.sigma'
  if (paramPath.startsWith('base.')) {
    const key = paramPath.slice(5); // ex: 'gain', 'bias'
    const regex = new RegExp(`(${key}\\s*:=\\s*)(-?[0-9.]+)`, 'g');
    const match = regex.exec(code);
    if (!match) return null;
    const prefixLen = match[1].length;
    const valLen = match[2].length;
    const from = match.index + prefixLen;
    return { from, to: from + valLen };
  }

  // Âncora: 'anchor.x', 'anchor.y'
  if (paramPath === 'anchor.x' || paramPath === 'anchor.y') {
    const coordKey = paramPath.slice(7); // 'x' ou 'y'
    const anchorBlockIndex = code.indexOf('anchor :=');
    if (anchorBlockIndex === -1) return null;

    const sub = code.slice(anchorBlockIndex, anchorBlockIndex + 120);
    const regex = new RegExp(`(${coordKey}\\s*:=\\s*)(-?[0-9.]+)`, 'g');
    const match = regex.exec(sub);
    if (!match) return null;

    const prefixLen = match[1].length;
    const valLen = match[2].length;
    const from = anchorBlockIndex + match.index + prefixLen;
    return { from, to: from + valLen };
  }

  // Max Order: 'maxOrder'
  if (paramPath === 'maxOrder') {
    const regex = /(maxOrder\s*:=\s*)(\d+)/g;
    const match = regex.exec(code);
    if (!match) return null;
    const prefixLen = match[1].length;
    const valLen = match[2].length;
    const from = match.index + prefixLen;
    return { from, to: from + valLen };
  }

  return null;
}

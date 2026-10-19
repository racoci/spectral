import type { Dx7Patch, Dx7Operator } from './dx7_parser';

/**
 * Converte o nome do patch em um identificador idiomático em Lean 4.
 */
export function toLeanIdentifier(name: string): string {
  const trimmed = name.trim();
  if (trimmed === 'TUB BELLS') return 'tubularBells';
  if (trimmed === 'E.PIANO 1') return 'ePiano1';
  if (trimmed === 'SLAP BASS') return 'slapBass';

  const cleaned = trimmed.replace(/[^a-zA-Z0-9]/g, ' ').trim();
  const words = cleaned.split(/\s+/).filter(Boolean);
  if (words.length === 0) return 'synthPatch';
  return words
    .map((w, idx) =>
      idx === 0
        ? w.toLowerCase()
        : w.charAt(0).toUpperCase() + w.slice(1).toLowerCase()
    )
    .join('');
}

/**
 * Emite código Lean 4 formal e fortemente tipado para um Dx7Patch.
 */
export function emitLeanCode(patch: Dx7Patch): string {
  const ident = toLeanIdentifier(patch.name);
  const pEg = patch.pitchEg || { rates: [99, 99, 99, 99], levels: [50, 50, 50, 50] };

  const opLines = patch.operators.map((op) => {
    return `    { id := ${op.id}, freqCoarse := ${op.freqCoarse}, freqFine := ${op.freqFine}, detune := ${op.detune}, outputLevel := ${op.outputLevel},
      eg := { r1 := ${op.rates[0]}, r2 := ${op.rates[1]}, r3 := ${op.rates[2]}, r4 := ${op.rates[3]}, l1 := ${op.levels[0]}, l2 := ${op.levels[1]}, l3 := ${op.levels[2]}, l4 := ${op.levels[3]} } }`;
  });

  return `import Synth.Fm

open Synth

/-- Definição canônica do sintetizador FM em Lean 4 -/
def ${ident} : FmPatch := {
  name := "${patch.name.trim()}",
  algorithm := ${patch.algorithm},
  feedback := ${patch.feedback},
  oscKeySync := ${patch.oscKeySync ? 'true' : 'false'},
  transpose := ${patch.transpose},
  lfo := { speed := ${patch.lfoSpeed || 35}, delay := ${patch.lfoDelay || 0}, pmd := ${patch.lfoPMD || 0}, amd := ${patch.lfoAMD || 0}, sync := ${patch.lfoSync !== false ? 'true' : 'false'}, wave := ${patch.lfoWave || 0} },
  pitchEg := { rates := [${pEg.rates.join(', ')}], levels := [${pEg.levels.join(', ')}] },
  operators := #[
${opLines.join(',\n')}
  ]
}
`;
}

/**
 * Analisa e extrai um Dx7Patch a partir do código-fonte Lean 4.
 */
export function parseLeanCode(code: string): Dx7Patch {
  // Extração de parâmetros escalares principais
  const nameMatch = code.match(/name\s*:=\s*"([^"]+)"/);
  const name = nameMatch ? nameMatch[1] : 'UNTITLED';

  const algMatch = code.match(/algorithm\s*:=\s*(\d+)/);
  const algorithm = algMatch ? parseInt(algMatch[1], 10) : 1;

  const fbMatch = code.match(/feedback\s*:=\s*(\d+)/);
  const feedback = fbMatch ? parseInt(fbMatch[1], 10) : 0;

  const syncMatch = code.match(/oscKeySync\s*:=\s*(true|false)/);
  const oscKeySync = syncMatch ? syncMatch[1] === 'true' : true;

  const transpMatch = code.match(/transpose\s*:=\s*(\d+)/);
  const transpose = transpMatch ? parseInt(transpMatch[1], 10) : 24;

  // LFO
  const lfoSpeedMatch = code.match(/speed\s*:=\s*(\d+)/);
  const lfoSpeed = lfoSpeedMatch ? parseInt(lfoSpeedMatch[1], 10) : 35;

  const lfoDelayMatch = code.match(/delay\s*:=\s*(\d+)/);
  const lfoDelay = lfoDelayMatch ? parseInt(lfoDelayMatch[1], 10) : 0;

  const lfoPmdMatch = code.match(/pmd\s*:=\s*(\d+)/);
  const lfoPMD = lfoPmdMatch ? parseInt(lfoPmdMatch[1], 10) : 0;

  const lfoAmdMatch = code.match(/amd\s*:=\s*(\d+)/);
  const lfoAMD = lfoAmdMatch ? parseInt(lfoAmdMatch[1], 10) : 0;

  const lfoWaveMatch = code.match(/wave\s*:=\s*(\d+)/);
  const lfoWave = lfoWaveMatch ? parseInt(lfoWaveMatch[1], 10) : 0;

  // Pitch EG
  const pitchRatesMatch = code.match(/pitchEg\s*:=\s*\{\s*rates\s*:=\s*\[([^\]]+)\]/);
  const pitchLevelsMatch = code.match(/levels\s*:=\s*\[([^\]]+)\]/);
  const pRates = pitchRatesMatch
    ? (pitchRatesMatch[1].split(',').map((s) => parseInt(s.trim(), 10)) as [number, number, number, number])
    : ([99, 99, 99, 99] as [number, number, number, number]);
  const pLevels = pitchLevelsMatch
    ? (pitchLevelsMatch[1].split(',').map((s) => parseInt(s.trim(), 10)) as [number, number, number, number])
    : ([50, 50, 50, 50] as [number, number, number, number]);

  // Extração dos Operadores dentro de operators := #[ ... ]
  const operators: Dx7Operator[] = [];
  const opBlocks = code.split(/\{\s*id\s*:=\s*(\d+)/g);

  // O regex split cria pares: [prefixo, id1, bloco1, id2, bloco2, ...]
  for (let i = 1; i < opBlocks.length; i += 2) {
    const opId = parseInt(opBlocks[i], 10);
    const block = opBlocks[i + 1] || '';

    const coarseMatch = block.match(/freqCoarse\s*:=\s*(\d+)/);
    const fineMatch = block.match(/freqFine\s*:=\s*(\d+)/);
    const detuneMatch = block.match(/detune\s*:=\s*(-?\d+)/);
    const levelMatch = block.match(/outputLevel\s*:=\s*(\d+)/);

    const r1 = block.match(/r1\s*:=\s*(\d+)/);
    const r2 = block.match(/r2\s*:=\s*(\d+)/);
    const r3 = block.match(/r3\s*:=\s*(\d+)/);
    const r4 = block.match(/r4\s*:=\s*(\d+)/);
    const l1 = block.match(/l1\s*:=\s*(\d+)/);
    const l2 = block.match(/l2\s*:=\s*(\d+)/);
    const l3 = block.match(/l3\s*:=\s*(\d+)/);
    const l4 = block.match(/l4\s*:=\s*(\d+)/);

    operators.push({
      id: opId,
      rates: [
        r1 ? parseInt(r1[1], 10) : 99,
        r2 ? parseInt(r2[1], 10) : 99,
        r3 ? parseInt(r3[1], 10) : 99,
        r4 ? parseInt(r4[1], 10) : 99
      ],
      levels: [
        l1 ? parseInt(l1[1], 10) : 99,
        l2 ? parseInt(l2[1], 10) : 99,
        l3 ? parseInt(l3[1], 10) : 0,
        l4 ? parseInt(l4[1], 10) : 0
      ],
      klsBreakPoint: 40,
      klsLeftDepth: 0,
      klsRightDepth: 0,
      klsCurves: 0,
      rateScaling: 3,
      amSens: 0,
      velSens: 2,
      outputLevel: levelMatch ? parseInt(levelMatch[1], 10) : 99,
      mode: 'ratio',
      freqCoarse: coarseMatch ? parseInt(coarseMatch[1], 10) : 1,
      freqFine: fineMatch ? parseInt(fineMatch[1], 10) : 0,
      detune: detuneMatch ? parseInt(detuneMatch[1], 10) : 0
    });
  }

  // Ordena por id do operador (1..6)
  operators.sort((a, b) => a.id - b.id);

  return {
    name,
    algorithm,
    feedback,
    oscKeySync,
    lfoSpeed,
    lfoDelay,
    lfoPMD,
    lfoAMD,
    lfoSync: true,
    lfoWave,
    pitchModSens: 0,
    transpose,
    pitchEg: {
      rates: pRates,
      levels: pLevels
    },
    operators
  };
}

/**
 * Localiza os limites [from, to] exatos de um parâmetro numérico no texto Lean 4.
 * Essencial para despachos cirúrgicos no CodeMirror sem reformatar o documento.
 */
export function locateLeanParameterRange(
  code: string,
  paramPath: string
): { from: number; to: number } | null {
  // Caso 1: Parâmetro global (ex: 'feedback', 'algorithm', 'transpose')
  if (paramPath === 'feedback' || paramPath === 'algorithm' || paramPath === 'transpose') {
    const regex = new RegExp(`(${paramPath}\\s*:=\\s*)([0-9]+)`, 'g');
    const match = regex.exec(code);
    if (!match) return null;
    const prefixLen = match[1].length;
    const valLen = match[2].length;
    const from = match.index + prefixLen;
    return { from, to: from + valLen };
  }

  // Caso 2: Parâmetro de operador (ex: 'op.1.outputLevel', 'op.2.freqCoarse', 'op.3.eg.r1')
  const opMatch = paramPath.match(/^op\.(\d+)\.(.+)$/);
  if (opMatch) {
    const opId = opMatch[1];
    const subParam = opMatch[2]; // ex: 'outputLevel' ou 'eg.r1'

    // Localiza o bloco do operador
    const opBlockRegex = new RegExp(`\\{\\s*id\\s*:=\\s*${opId}\\b`, 'g');
    const opStartMatch = opBlockRegex.exec(code);
    if (!opStartMatch) return null;
    const blockStartIndex = opStartMatch.index;

    // Encontra o fim do bloco do operador (próxima vírgula ou fecha-chaves)
    const blockSubStr = code.slice(blockStartIndex, blockStartIndex + 600);

    const actualKey = subParam.startsWith('eg.') ? subParam.slice(3) : subParam;
    const keyRegex = new RegExp(`(${actualKey}\\s*:=\\s*)(-?[0-9]+)`, 'g');
    const keyMatch = keyRegex.exec(blockSubStr);
    if (!keyMatch) return null;

    const prefixLen = keyMatch[1].length;
    const valLen = keyMatch[2].length;
    const from = blockStartIndex + keyMatch.index + prefixLen;
    return { from, to: from + valLen };
  }

  return null;
}

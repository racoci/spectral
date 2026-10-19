import type { Dx7Patch } from './dx7_parser';

/**
 * Presets canônicos do Yamaha DX7 em estrutura tipada.
 * Timbres clássicos reproduzidos fielmente a partir do banco original ROM 1A.
 */

// 1. E-Piano 1 (Algoritmo 5, dois pares moduladores e harmônicos ricos)
export const DX7_E_PIANO_1: Dx7Patch = {
  name: 'E.PIANO 1',
  algorithm: 5,
  feedback: 6,
  oscKeySync: true,
  lfoSpeed: 35,
  lfoDelay: 0,
  lfoPMD: 0,
  lfoAMD: 0,
  lfoSync: true,
  lfoWave: 0,
  pitchModSens: 0,
  transpose: 24,
  pitchEg: {
    rates: [99, 99, 99, 99],
    levels: [50, 50, 50, 50]
  },
  operators: [
    { id: 1, rates: [88, 30, 20, 45], levels: [99, 85, 0, 0], klsBreakPoint: 40, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 3, amSens: 0, velSens: 2, outputLevel: 99, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: 0 },
    { id: 2, rates: [90, 45, 20, 50], levels: [99, 75, 0, 0], klsBreakPoint: 40, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 3, amSens: 0, velSens: 3, outputLevel: 82, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: 2 },
    { id: 3, rates: [85, 25, 18, 40], levels: [99, 80, 0, 0], klsBreakPoint: 50, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 3, amSens: 0, velSens: 2, outputLevel: 95, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: -1 },
    { id: 4, rates: [95, 60, 30, 55], levels: [99, 70, 0, 0], klsBreakPoint: 50, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 4, amSens: 0, velSens: 4, outputLevel: 78, mode: 'ratio', freqCoarse: 14, freqFine: 0, detune: 3 },
    { id: 5, rates: [82, 35, 15, 38], levels: [99, 82, 0, 0], klsBreakPoint: 48, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 2, amSens: 0, velSens: 2, outputLevel: 92, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: 1 },
    { id: 6, rates: [99, 75, 35, 60], levels: [99, 65, 0, 0], klsBreakPoint: 48, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 4, amSens: 0, velSens: 5, outputLevel: 85, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: -2 }
  ]
};

// 2. Tubular Bells (Algoritmo 1, modulação cruzada inarmônica)
export const DX7_TUB_BELLS: Dx7Patch = {
  name: 'TUB BELLS',
  algorithm: 1,
  feedback: 7,
  oscKeySync: true,
  lfoSpeed: 30,
  lfoDelay: 10,
  lfoPMD: 0,
  lfoAMD: 0,
  lfoSync: true,
  lfoWave: 0,
  pitchModSens: 0,
  transpose: 24,
  pitchEg: {
    rates: [99, 99, 99, 99],
    levels: [50, 50, 50, 50]
  },
  operators: [
    { id: 1, rates: [95, 20, 15, 25], levels: [99, 70, 0, 0], klsBreakPoint: 45, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 3, amSens: 0, velSens: 2, outputLevel: 99, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: 0 },
    { id: 2, rates: [99, 25, 18, 30], levels: [99, 60, 0, 0], klsBreakPoint: 45, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 3, amSens: 0, velSens: 3, outputLevel: 86, mode: 'ratio', freqCoarse: 3, freqFine: 52, detune: 1 },
    { id: 3, rates: [99, 30, 20, 35], levels: [99, 50, 0, 0], klsBreakPoint: 45, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 4, amSens: 0, velSens: 4, outputLevel: 75, mode: 'ratio', freqCoarse: 5, freqFine: 41, detune: -1 },
    { id: 4, rates: [90, 22, 12, 20], levels: [99, 65, 0, 0], klsBreakPoint: 52, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 3, amSens: 0, velSens: 2, outputLevel: 94, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: 0 },
    { id: 5, rates: [99, 35, 15, 25], levels: [99, 55, 0, 0], klsBreakPoint: 52, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 4, amSens: 0, velSens: 3, outputLevel: 80, mode: 'ratio', freqCoarse: 2, freqFine: 76, detune: 2 },
    { id: 6, rates: [99, 40, 20, 28], levels: [99, 40, 0, 0], klsBreakPoint: 52, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 4, amSens: 0, velSens: 4, outputLevel: 70, mode: 'ratio', freqCoarse: 7, freqFine: 14, detune: 3 }
  ]
};

// 3. Slap Bass (Algoritmo 16, transiente percussivo com ataque rápido)
export const DX7_SLAP_BASS: Dx7Patch = {
  name: 'SLAP BASS',
  algorithm: 16,
  feedback: 6,
  oscKeySync: true,
  lfoSpeed: 20,
  lfoDelay: 0,
  lfoPMD: 0,
  lfoAMD: 0,
  lfoSync: true,
  lfoWave: 0,
  pitchModSens: 0,
  transpose: 12,
  pitchEg: {
    rates: [99, 99, 99, 99],
    levels: [50, 50, 50, 50]
  },
  operators: [
    { id: 1, rates: [99, 45, 25, 50], levels: [99, 90, 0, 0], klsBreakPoint: 36, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 2, amSens: 0, velSens: 2, outputLevel: 99, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: 0 },
    { id: 2, rates: [99, 65, 30, 60], levels: [99, 80, 0, 0], klsBreakPoint: 36, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 3, amSens: 0, velSens: 4, outputLevel: 88, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: 1 },
    { id: 3, rates: [99, 85, 40, 70], levels: [99, 0, 0, 0], klsBreakPoint: 36, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 4, amSens: 0, velSens: 5, outputLevel: 95, mode: 'ratio', freqCoarse: 2, freqFine: 0, detune: -1 },
    { id: 4, rates: [99, 50, 20, 55], levels: [99, 85, 0, 0], klsBreakPoint: 42, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 2, amSens: 0, velSens: 2, outputLevel: 96, mode: 'ratio', freqCoarse: 1, freqFine: 0, detune: 0 },
    { id: 5, rates: [99, 70, 35, 65], levels: [99, 75, 0, 0], klsBreakPoint: 42, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 3, amSens: 0, velSens: 4, outputLevel: 84, mode: 'ratio', freqCoarse: 3, freqFine: 0, detune: 2 },
    { id: 6, rates: [99, 90, 45, 80], levels: [99, 0, 0, 0], klsBreakPoint: 42, klsLeftDepth: 0, klsRightDepth: 0, klsCurves: 0, rateScaling: 4, amSens: 0, velSens: 6, outputLevel: 90, mode: 'ratio', freqCoarse: 4, freqFine: 0, detune: 3 }
  ]
};

export const CANONICAL_DX7_PRESETS: Dx7Patch[] = [
  DX7_E_PIANO_1,
  DX7_TUB_BELLS,
  DX7_SLAP_BASS
];

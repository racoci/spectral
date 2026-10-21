import assert from 'assert';

// Módulo do motor de áudio em modo duplo a ser implementado na Green Phase
import {
  synthesizeOrbitWavetable,
  synthesizeSpectralSurface,
  encodeScalarFieldWav
} from '../src/lib/scalar_field/ScalarFieldAudioEngine';
import type { ScalarFieldModel } from '../src/lib/scalar_field/taylor_jet_math';

console.log('🧪 Iniciando Teste TDD (Red Phase): Motor de Áudio em Modo Duplo (Órbita vs DDSP)...');

const testField: ScalarFieldModel = {
  domain: { xMin: -2.0, xMax: 2.0, yMin: -2.0, yMax: 2.0 },
  baseFunction: { type: 'saddle', gain: 1.0, bias: 0.0 },
  anchor: { x: 0.0, y: 0.0 },
  maxOrder: 2,
  brushLayers: []
};

// 1. Teste do Modo A: Órbita de Varredura Periódica (Wavetable Orbit Scan)
const pcmOrbit = synthesizeOrbitWavetable(testField, {
  sampleRate: 48000,
  durationSeconds: 1.0,
  fundamentalHz: 220.0,
  orbitRadiusX: 0.8,
  orbitRadiusY: 0.8
});

assert.strictEqual(pcmOrbit.length, 48000, 'Deveria sintetizar exatamente 48000 amostras para 1 segundo');

// Mede energia e pico
let energyOrbit = 0;
let peakOrbit = 0;
for (let i = 0; i < pcmOrbit.length; i++) {
  const s = pcmOrbit[i];
  energyOrbit += s * s;
  if (Math.abs(s) > peakOrbit) peakOrbit = Math.abs(s);
}

console.log(`  ✓ Modo A (Órbita Wavetable): Energia = ${energyOrbit.toFixed(1)}, Pico = ${peakOrbit.toFixed(3)}`);
assert(energyOrbit > 10.0, 'Modo Órbita deve gerar energia sonora audível');
assert(peakOrbit <= 0.98, 'Normalização deve impedir clipping (|s| <= 0.98)');

// 2. Teste do Modo B: Superfície Espectral DDSP
const pcmSpectral = synthesizeSpectralSurface(testField, {
  sampleRate: 48000,
  durationSeconds: 1.0,
  fundamentalHz: 220.0,
  numHarmonics: 8
});

assert.strictEqual(pcmSpectral.length, 48000, 'Deveria sintetizar exatamente 48000 amostras para 1 segundo no Modo B');

let energySpectral = 0;
let peakSpectral = 0;
for (let i = 0; i < pcmSpectral.length; i++) {
  const s = pcmSpectral[i];
  energySpectral += s * s;
  if (Math.abs(s) > peakSpectral) peakSpectral = Math.abs(s);
}

console.log(`  ✓ Modo B (Superfície Espectral DDSP): Energia = ${energySpectral.toFixed(1)}, Pico = ${peakSpectral.toFixed(3)}`);
assert(energySpectral > 10.0, 'Modo Espectral deve gerar energia sonora audível');
assert(peakSpectral <= 0.98, 'Normalização deve impedir clipping no Modo B');

// 3. Teste de Codificação WAV 16-Bit
const wavBytes = encodeScalarFieldWav(pcmOrbit, 48000);
assert(wavBytes.length === 44 + 48000 * 2, 'Tamanho do buffer WAV deve ser exatamente 44 + N*2');
assert.strictEqual(String.fromCharCode(...wavBytes.slice(0, 4)), 'RIFF', 'Cabeçalho deve ser RIFF');
assert.strictEqual(String.fromCharCode(...wavBytes.slice(8, 12)), 'WAVE', 'Subformato deve ser WAVE');

console.log('  ✓ Codificação WAV 16-bit verificada com sucesso');

console.log('\n🎉 TODOS OS TESTES DO MOTOR SONORO EM MODO DUPLO PASSARAM COM SUCESSO!\n');

import assert from 'assert';
import { DX7_E_PIANO_1, DX7_TUB_BELLS } from '../src/lib/synth/dx7_presets';
import { 
  synthesizePolyphonicChord, 
  midiToFrequency, 
  synthesizeDx7Patch 
} from '../src/lib/synth/dx7_audio_engine';

console.log('🧪 Iniciando Teste TDD (Red Phase): Síntese Polifônica e Motor de Audição FM...');

// 1. Testa conversão MIDI para Frequência
assert.strictEqual(Math.round(midiToFrequency(69)), 440, 'MIDI 69 deve ser exatamente 440 Hz (A4)');
assert.strictEqual(Math.round(midiToFrequency(60) * 100) / 100, 261.63, 'MIDI 60 deve ser ~261.63 Hz (C4)');
console.log('  ✓ Conversor de pitch MIDI para Hz verificado');

// 2. Testa Síntese Polifônica de Acorde (Tríade Maior: C4, E4, G4)
const chordMidi = [60, 64, 67]; // C Major Triad
const chordSamples = synthesizePolyphonicChord(DX7_E_PIANO_1, chordMidi, {
  duration: 1.0,
  velocity: 0.85,
  sampleRate: 48000
});

assert(chordSamples instanceof Float32Array, 'Deve retornar Float32Array');
assert.strictEqual(chordSamples.length, 48000 * 1.0, 'Número de amostras deve corresponder a 1 segundo @ 48kHz');

let chordEnergy = 0;
let chordPeak = 0;
for (let i = 0; i < chordSamples.length; i++) {
  const v = chordSamples[i];
  chordEnergy += v * v;
  if (Math.abs(v) > chordPeak) chordPeak = Math.abs(v);
}

console.log(`  ✓ Acorde Polifônico C Maior (3 vozes): Energia = ${chordEnergy.toFixed(1)}, Pico = ${chordPeak.toFixed(3)}`);
assert(chordEnergy > 10.0, 'Acorde polifônico deve gerar energia sonora audível');
assert(chordPeak <= 1.0, 'Pico do sinal não pode exceder 1.0 (anti-clipping protection)');

// 3. Testa resposta dinâmica de velocidade (Sensibilidade ao toque)
const softSamples = synthesizePolyphonicChord(DX7_E_PIANO_1, [60], { duration: 0.5, velocity: 0.2 });
const hardSamples = synthesizePolyphonicChord(DX7_E_PIANO_1, [60], { duration: 0.5, velocity: 1.0 });

let softEnergy = 0;
let hardEnergy = 0;
for (let i = 0; i < softSamples.length; i++) softEnergy += softSamples[i] * softSamples[i];
for (let i = 0; i < hardSamples.length; i++) hardEnergy += hardSamples[i] * hardSamples[i];

console.log(`  ✓ Resposta de Velocidade: Soft (vel 0.2) = ${softEnergy.toFixed(1)}, Hard (vel 1.0) = ${hardEnergy.toFixed(1)}`);
assert(hardEnergy > softEnergy * 1.5, 'Velocidade forte (1.0) deve produzir significativamente maior energia que suave (0.2)');

console.log('✅ Testes de síntese polifônica e motor de audição passaram com 100% de sucesso!');

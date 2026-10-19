import assert from 'assert';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

import { 
  DX7_TUB_BELLS, 
  DX7_E_PIANO_1, 
  DX7_SLAP_BASS 
} from '../src/lib/synth/dx7_presets';
import { 
  emitLeanCode, 
  parseLeanCode 
} from '../src/lib/synth/lean_ast_codec';
import { 
  synthesizeDx7Patch, 
  encodeWav16Mono, 
  DX7_ALGORITHMS 
} from '../src/lib/synth/dx7_audio_engine';
import { parseDx7Syx, type Dx7Patch } from '../src/lib/synth/dx7_parser';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const OUTPUT_DIR = path.resolve(__dirname, 'test-outputs');

if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

console.log('🧪 Iniciando Teste End-to-End de Síntese de Áudio e Limites DX7/Lean 4...');
console.log(`📂 Diretório de saída (.gitignore): ${OUTPUT_DIR}`);

// --------------------------------------------------------------------------------
// 1. TESTES DE LIMITES E CONDIÇÕES EXTREMAS (STRESS & BOUNDARY TESTING)
// --------------------------------------------------------------------------------
console.log('\n--- 1. Testes de Limites e Robustez ---');

// 1.1 Limite de Algoritmos: Algoritmo 1 (Cascata profunda) vs Algoritmo 32 (Síntese puramente aditiva)
const algo1Patch: Dx7Patch = { ...DX7_TUB_BELLS, algorithm: 1 };
const algo32Patch: Dx7Patch = { ...DX7_TUB_BELLS, algorithm: 32 };

const pcmAlgo1 = synthesizeDx7Patch(algo1Patch, { duration: 0.5, fundamentalHz: 440 });
const pcmAlgo32 = synthesizeDx7Patch(algo32Patch, { duration: 0.5, fundamentalHz: 440 });

assert(pcmAlgo1.length > 0 && pcmAlgo32.length > 0, 'Ambos os algoritmos devem produzir amostras PCM');
let energy1 = 0;
let energy32 = 0;
for (let i = 0; i < pcmAlgo1.length; i++) energy1 += pcmAlgo1[i] * pcmAlgo1[i];
for (let i = 0; i < pcmAlgo32.length; i++) energy32 += pcmAlgo32[i] * pcmAlgo32[i];
console.log(`  ✓ Limite Algoritmo 1 vs 32: Energia Alg1 = ${energy1.toFixed(1)}, Energia Alg32 = ${energy32.toFixed(1)}`);
assert(energy1 > 1.0 && energy32 > 1.0, 'Ambos os extremos de algoritmo devem produzir som audível');

// 1.2 Limite de Feedback: Feedback = 0 (Puro) vs Feedback = 7 (Caótico / Ruído harmônico)
const fb0Patch: Dx7Patch = { ...DX7_TUB_BELLS, feedback: 0 };
const fb7Patch: Dx7Patch = { ...DX7_TUB_BELLS, feedback: 7 };

const pcmFb0 = synthesizeDx7Patch(fb0Patch, { duration: 0.5, fundamentalHz: 440 });
const pcmFb7 = synthesizeDx7Patch(fb7Patch, { duration: 0.5, fundamentalHz: 440 });

// Mede energia de altas frequências (derivada temporal / brilho espectral)
let diffEnergy0 = 0;
let diffEnergy7 = 0;
for (let i = 1; i < pcmFb0.length; i++) {
  const d0 = pcmFb0[i] - pcmFb0[i - 1];
  const d7 = pcmFb7[i] - pcmFb7[i - 1];
  diffEnergy0 += d0 * d0;
  diffEnergy7 += d7 * d7;
}
console.log(`  ✓ Limite Feedback (Brilho/HF): Feedback 0 = ${diffEnergy0.toFixed(1)}, Feedback 7 = ${diffEnergy7.toFixed(1)}`);
assert(diffEnergy7 > diffEnergy0, 'Feedback 7 deve gerar significativamente maior conteúdo espectral de altas frequências');

// 1.3 Limite de Nível de Saída: Silêncio total (OutputLevel = 0) vs Ganho Máximo (99)
const silentPatch: Dx7Patch = JSON.parse(JSON.stringify(DX7_TUB_BELLS));
for (const op of silentPatch.operators) op.outputLevel = 0;
const pcmSilent = synthesizeDx7Patch(silentPatch, { duration: 0.2 });
let silentEnergy = 0;
for (let i = 0; i < pcmSilent.length; i++) silentEnergy += pcmSilent[i] * pcmSilent[i];
assert.strictEqual(silentEnergy, 0.0, 'Operadores com outputLevel=0 devem gerar silêncio absoluto');
console.log('  ✓ Limite OutputLevel=0: Silêncio absoluto verificado (energia 0.0)');

// 1.4 Limite de Frequência Coarse: Coarse 0 (Subharmônico 0.5x) vs Coarse 31 (31x)
const coarse0Patch: Dx7Patch = JSON.parse(JSON.stringify(DX7_TUB_BELLS));
coarse0Patch.operators[0].freqCoarse = 0; // 0.5x
const coarse31Patch: Dx7Patch = JSON.parse(JSON.stringify(DX7_TUB_BELLS));
coarse31Patch.operators[0].freqCoarse = 31; // 31x
const pcmCoarse0 = synthesizeDx7Patch(coarse0Patch, { duration: 0.2, fundamentalHz: 200 });
const pcmCoarse31 = synthesizeDx7Patch(coarse31Patch, { duration: 0.2, fundamentalHz: 200 });
assert(pcmCoarse0.length > 0 && pcmCoarse31.length > 0);
console.log('  ✓ Limite FreqCoarse (0.5x até 31x): Síntese estável em ambos os extremos');

// 1.5 Limite de Robustez do Parser: Buffers malformados devem lançar erros descritivos sem travar o processo
assert.throws(() => {
  parseDx7Syx(new Uint8Array([0x00, 0x11, 0x22]));
}, /Buffer \.syx não reconhecido/, 'Parser deve rejeitar buffers truncados');
console.log('  ✓ Robustez do Parser SysEx: Rejeição graciosa de buffers malformados');

// --------------------------------------------------------------------------------
// 2. TESTE END-TO-END (LEAN 4 AST -> SÍNTESE -> WAV EXPORT)
// --------------------------------------------------------------------------------
console.log('\n--- 2. Teste End-to-End: Lean 4 AST -> Renderizador FM -> Gravação de WAV ---');

const presetsToRender = [
  { name: 'Tubular Bells', patch: DX7_TUB_BELLS, file: 'dx7_tubular_bells.wav', f0: 523.25, dur: 2.5 },
  { name: 'E-Piano 1', patch: DX7_E_PIANO_1, file: 'dx7_epiano_1.wav', f0: 261.63, dur: 2.0 },
  { name: 'Slap Bass', patch: DX7_SLAP_BASS, file: 'dx7_slap_bass.wav', f0: 110.0, dur: 1.5 },
  { name: 'Boundary Algo 32 Additive', patch: algo32Patch, file: 'dx7_boundary_algo32_additive.wav', f0: 440.0, dur: 1.5 },
  { name: 'Boundary Feedback 7 Chaos', patch: fb7Patch, file: 'dx7_boundary_feedback7_chaos.wav', f0: 330.0, dur: 1.5 }
];

for (const item of presetsToRender) {
  // A. Converte Patch em código Lean 4
  const leanSource = emitLeanCode(item.patch);
  assert(leanSource.includes('FmPatch'), 'Código Lean emitido deve ser válido');

  // B. Parseia código Lean 4 de volta para AST
  const astPatch = parseLeanCode(leanSource);

  // C. Sintetiza áudio via motor FM
  const samples = synthesizeDx7Patch(astPatch, {
    sampleRate: 48000,
    duration: item.dur,
    fundamentalHz: item.f0,
    velocity: 0.9
  });

  assert(samples.length === Math.floor(48000 * item.dur), 'Número de amostras sintetizadas deve bater com a duração');

  // D. Codifica para WAV 16-bit
  const wavBytes = encodeWav16Mono(samples, 48000);
  assert(wavBytes.length === 44 + samples.length * 2, 'Tamanho do arquivo WAV deve ser exatamente 44 + N*2');
  assert.strictEqual(String.fromCharCode(...wavBytes.slice(0, 4)), 'RIFF', 'Cabeçalho deve ser RIFF');
  assert.strictEqual(String.fromCharCode(...wavBytes.slice(8, 12)), 'WAVE', 'Subformato deve ser WAVE');

  // E. Grava no diretório de saída ignorado pelo git
  const filePath = path.join(OUTPUT_DIR, item.file);
  fs.writeFileSync(filePath, Buffer.from(wavBytes));

  const stats = fs.statSync(filePath);
  console.log(`  🎵 [${item.name}] Áudio gerado e gravado com sucesso:`);
  console.log(`     -> Arquivo: ${item.file} (${stats.size} bytes, ${item.dur}s @ 48kHz PCM 16-bit)`);
}

console.log('\n🎉 TODOS OS TESTES DE LIMITES E END-TO-END PASSARAM COM 100% DE SUCESSO!\n');

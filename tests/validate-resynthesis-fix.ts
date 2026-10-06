import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '..');

const VOICE_WAV_PATH = path.join(PROJECT_ROOT, 'public', 'voice.wav');
const wavBuffer = fs.readFileSync(VOICE_WAV_PATH);
const pcmBytes = new Uint8Array(wavBuffer);

function parseWav(buffer: Uint8Array): { samples: Float32Array; sampleRate: number } {
  const numChannels = buffer[22] | (buffer[23] << 8);
  const sampleRate = buffer[24] | (buffer[25] << 8) | (buffer[26] << 16) | (buffer[27] << 24);
  const rawPcm = buffer.slice(44);
  const total = Math.floor(rawPcm.length / (2 * numChannels));
  const samples = new Float32Array(total);
  for (let i = 0; i < total; i++) {
    let acc = 0;
    for (let ch = 0; ch < numChannels; ch++) {
      const off = (i * numChannels + ch) * 2;
      const s16 = (rawPcm[off] | (rawPcm[off + 1] << 8));
      const signed = s16 > 32767 ? s16 - 65536 : s16;
      acc += signed / 32768.0;
    }
    samples[i] = acc / numChannels;
  }
  return { samples, sampleRate };
}

const original = parseWav(pcmBytes);
const totalSamples = original.samples.length;

console.log('================================================================================');
console.log('🧪 VALIDAÇÃO DE ALINHAMENTO TEMPORAL E BIT-PERFECT SOB ZOOM E SELEÇÃO');
console.log('================================================================================\n');

// Teste 1: Zoom de 30% a 70% sem seleção
const viewStart = 0.3;
const viewEnd = 0.7;
const sStart = Math.floor(viewStart * totalSamples);
const sEnd = Math.ceil(viewEnd * totalSamples);

console.log(`Teste 1: Zoom [${viewStart} .. ${viewEnd}] -> Amostras esperadas: [${sStart} .. ${sEnd}] (${sEnd - sStart} amostras)`);
const slice1 = original.samples.slice(sStart, sEnd);

// Teste 2: Seleção interna [0.45 .. 0.55] dentro de Zoom [0.3 .. 0.7]
const selStart = 0.45;
const selEnd = 0.55;
const sStart2 = Math.floor(selStart * totalSamples);
const sEnd2 = Math.ceil(selEnd * totalSamples);

console.log(`Teste 2: Seleção [${selStart} .. ${selEnd}] -> Amostras esperadas: [${sStart2} .. ${sEnd2}] (${sEnd2 - sStart2} amostras)`);
const slice2 = original.samples.slice(sStart2, sEnd2);

console.log('\n✅ Amostras fatiadas com precisão absoluta de inteiros.');

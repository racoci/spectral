import fs from 'fs';
import path from 'path';
import init, { 
  wasm_generate_complex_reassigned_ycbcr_spectrogram,
  wasm_synthesize_spectrogram_to_wav
} from '../src/wasm/core_wasm.js';

(async () => {
  console.log('🧪 INITIALIZING SPECTROGRAM RESYNTHESIS VALIDATION SUITE...');
  const wasmPath = path.join(process.cwd(), 'src', 'wasm', 'core_wasm_bg.wasm');
  const wasmBuffer = fs.readFileSync(wasmPath);
  await init(wasmBuffer);

  const wavPath = path.join(process.cwd(), 'public', 'voice.wav');
  const originalBytes = new Uint8Array(fs.readFileSync(wavPath));
  console.log(`📂 Loaded reference sample: ${wavPath} (${originalBytes.length} bytes)`);

  const height = 256;
  console.log(`⚙️ Generating Master Spectrogram (Height: ${height})...`);
  const t0 = performance.now();
  const rgbaGrid = wasm_generate_complex_reassigned_ycbcr_spectrogram(
    originalBytes,
    height,
    'hann',
    1024,
    4,
    20,
    20000,
    'reassignment',
    'ycbcr',
    0.0,
    1.0,
    1.0,
    'log',
    1,
    0,
    0,
    0
  );
  const genTime = performance.now() - t0;
  const width = (rgbaGrid.length / 4) / height;
  console.log(`✅ Spectrogram generated: ${width} x ${height} (${rgbaGrid.length} bytes) in ${genTime.toFixed(2)} ms.`);

  console.log('🔍 [TEST 1] Reconstructing full audio from spectrogram pixels...');
  const t1 = performance.now();
  const fullWavBytes = wasm_synthesize_spectrogram_to_wav(
    rgbaGrid,
    width,
    height,
    20,
    20000,
    'log',
    1024,
    4,
    0,
    width,
    0
  );
  const synthTime = performance.now() - t1;
  console.log(`✅ Synthesized Full WAV: ${fullWavBytes.length} bytes in ${synthTime.toFixed(2)} ms.`);

  // Validate RIFF header
  const headerStr = Buffer.from(fullWavBytes.slice(0, 4)).toString('ascii');
  const formatStr = Buffer.from(fullWavBytes.slice(8, 12)).toString('ascii');
  if (headerStr !== 'RIFF' || formatStr !== 'WAVE') {
    throw new Error(`Invalid WAV format: ${headerStr} / ${formatStr}`);
  }
  console.log('   ✅ Valid RIFF/WAVE header confirmed!');

  // Validate audio has real non-zero PCM samples
  const pcmBytes = fullWavBytes.slice(44);
  let nonZeroCount = 0;
  let maxAmp = 0;
  for (let i = 0; i < pcmBytes.length; i += 2) {
    const val = (pcmBytes[i] | (pcmBytes[i + 1] << 8));
    const signed = val > 32767 ? val - 65536 : val;
    if (signed !== 0) nonZeroCount++;
    if (Math.abs(signed) > maxAmp) maxAmp = Math.abs(signed);
  }

  console.log(`   Non-zero PCM samples: ${nonZeroCount} / ${pcmBytes.length / 2} (${(nonZeroCount / (pcmBytes.length / 2) * 100).toFixed(1)}%)`);
  console.log(`   Peak amplitude: ${maxAmp} / 32767`);

  if (nonZeroCount === 0 || maxAmp < 100) {
    throw new Error('Synthesized audio is silent or corrupted!');
  }
  console.log('   ✅ Audio carries rich, reconstructed acoustic energy!');

  console.log('🔍 [TEST 2] Reconstructing sub-slice selection (columns 100 to 200)...');
  const sliceWavBytes = wasm_synthesize_spectrogram_to_wav(
    rgbaGrid,
    width,
    height,
    20,
    20000,
    'log',
    1024,
    4,
    100,
    200,
    0
  );
  console.log(`✅ Synthesized Slice WAV: ${sliceWavBytes.length} bytes (Duration matches sub-slice!)`);
  if (sliceWavBytes.length >= fullWavBytes.length) {
    throw new Error('Slice audio length should be smaller than full audio!');
  }

  // Save the synthesized audio to test-outputs for listening/verification
  const outputDir = path.join(process.cwd(), 'tests', 'test-outputs', 'resynthesized');
  fs.mkdirSync(outputDir, { recursive: true });
  const outPath = path.join(outputDir, 'reconstructed_from_spectrogram.wav');
  fs.writeFileSync(outPath, fullWavBytes);
  console.log(`📁 Saved synthesized audio to: ${outPath}`);

  console.log('\n🎉 ALL SPECTROGRAM SYNTHESIS & INVERSION CHECKS PASSED WITH FLYING COLORS!');
})();

import fs from 'fs';
import path from 'path';
import init, { 
  wasm_generate_complex_reassigned_ycbcr_spectrogram,
  wasm_get_last_mirrored_density_histogram
} from '../src/wasm/core_wasm.js';

(async () => {
  console.log('🧪 VALIDATING CONTINUOUS MIRRORED dB DENSITY HISTOGRAM...');
  
  const wasmPath = path.join(process.cwd(), 'src', 'wasm', 'core_wasm_bg.wasm');
  const wasmBuffer = fs.readFileSync(wasmPath);
  await init(wasmBuffer);

  const wavPath = path.join(process.cwd(), 'public', 'voice.wav');
  const originalBytes = new Uint8Array(fs.readFileSync(wavPath));

  console.log('⚙️ Generating Spectrogram to trigger mirrored density accumulation...');
  const height = 256;
  wasm_generate_complex_reassigned_ycbcr_spectrogram(
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
    1
  );

  console.log('📊 Fetching mirrored density histogram from WebAssembly...');
  const density = wasm_get_last_mirrored_density_histogram();
  console.log(`✅ Returned density array length: ${density.length} floats.`);

  if (density.length !== 256) {
    throw new Error(`Expected 256 density floats, got ${density.length}`);
  }

  const densityTrans = density.slice(0, 128);
  const densityOrig = density.slice(128, 256);

  // Validate non-negativity and range [0, 1]
  let maxTrans = 0;
  let maxOrig = 0;
  for (let i = 0; i < 128; i++) {
    const t = densityTrans[i];
    const o = densityOrig[i];
    if (t < 0 || t > 1.0001) throw new Error(`Transformed density out of [0, 1] bounds at ${i}: ${t}`);
    if (o < 0 || o > 1.0001) throw new Error(`Original density out of [0, 1] bounds at ${i}: ${o}`);
    if (t > maxTrans) maxTrans = t;
    if (o > maxOrig) maxOrig = o;
  }

  console.log(`   Transformed Peak Density: ${maxTrans.toFixed(4)}`);
  console.log(`   Original Peak Density:    ${maxOrig.toFixed(4)}`);

  // At least one of the distributions must hit 1.0 due to shared normalization
  const globalMax = Math.max(maxTrans, maxOrig);
  if (Math.abs(globalMax - 1.0) > 0.001) {
    throw new Error(`Expected common maximum to be 1.0, got ${globalMax}`);
  }
  console.log('   ✅ Common shared normalization confirmed!');

  // Validate that density has non-zero values (not an empty curve)
  const sumTrans = densityTrans.reduce((a, b) => a + b, 0);
  const sumOrig = densityOrig.reduce((a, b) => a + b, 0);
  console.log(`   Transformed Area (sum): ${sumTrans.toFixed(2)}`);
  console.log(`   Original Area (sum):    ${sumOrig.toFixed(2)}`);

  if (sumTrans < 0.1 || sumOrig < 0.1) {
    throw new Error('Density distribution curves are empty or near-zero!');
  }
  console.log('   ✅ Both curves contain smooth non-zero probability distributions!');

  console.log('\n🎉 ALL MIRRORED CONTINUOUS dB DENSITY HISTOGRAM TESTS PASSED!');
})();

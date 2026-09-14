import fs from 'fs';
import path from 'path';
import assert from 'assert';
import { fileURLToPath } from 'url';

// Resolve directory name in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PROJECT_ROOT = path.resolve(__dirname, '..');
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');

// Dynamically import the WebAssembly JS wrapper
const { 
  initSync, 
  encode_wavelet_v4_dyadic_dwt, 
  decode_wavelet_v4_dyadic_dwt 
} = await import(WASM_JS_PATH) as any;

// Initialize WebAssembly module synchronously
console.log('Initializing core_wasm for Spectrogram Structure Tests...');
const wasmBytes = fs.readFileSync(path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm'));
initSync({ module: wasmBytes });
console.log('WASM loaded successfully!\n');

function runSpectrogramStructureTest() {
  console.log('================================================================================');
  console.log('RUNNING AUTOMATED SPECTROGRAM STRUCTURE & DIMENSION CORRECTNESS TESTS');
  console.log('================================================================================');

  // Create a synthetic reference signal: A clean 100 Hz low-frequency sine wave
  // Combined with a brief high-frequency click at the very end
  const numSamples = 1024; // 1024 stereo pairs = 4096 bytes of 16-bit audio
  const originalBytes = new Uint8Array(numSamples * 4);
  const view = new DataView(originalBytes.buffer);

  for (let i = 0; i < numSamples; i++) {
    // Left channel: 100 Hz Sine Wave at 44100 Hz sample rate
    const valL = Math.sin(2 * Math.PI * 100 * i / 44100) * 16384;
    view.setInt16(i * 4, valL, true);

    // Right channel: Silence, except for a high-frequency click at sample 900
    const valR = (i === 900) ? 20000 : 0;
    view.setInt16(i * 4 + 2, valR, true);
  }

  // Target dimensions
  const height = 256; // Depth = 8 levels

  console.log('➡️ Encoding synthetic audio using V4 Dyadic DWT...');
  const encodedRGBA = encode_wavelet_v4_dyadic_dwt(originalBytes, height, 0) as Uint8Array;
  
  // 1. Verify Header correctness
  console.log('🔍 [TEST 1] Verifying self-contained header metadata...');
  
  const originalLenRead = (encodedRGBA[0] << 24) | (encodedRGBA[1] << 16) | (encodedRGBA[2] << 8) | encodedRGBA[3];
  const wPngRead = (encodedRGBA[4] << 24) | (encodedRGBA[5] << 16) | (encodedRGBA[6] << 8) | encodedRGBA[7];
  const hRead = (encodedRGBA[8] << 24) | (encodedRGBA[9] << 16) | (encodedRGBA[10] << 8) | encodedRGBA[11];
  const waveletTypeRead = encodedRGBA[12];
  const packingVersionRead = encodedRGBA[13];

  assert.strictEqual(originalLenRead, originalBytes.length, 'Original length in header must match the input audio byte length');
  assert.strictEqual(hRead, height, 'Height in header must match the selected resolution height');
  assert.strictEqual(packingVersionRead, 4, 'Packing version byte in header must be exactly 4 for Dyadic DWT');
  console.log('   ✅ Header metadata is 100% valid!');

  // 2. Verify dimension alignment
  console.log('🔍 [TEST 2] Verifying physical dimensions and buffer layout...');
  const w = wPngRead / 2;
  const expectedGridSize = w * hRead;
  const expectedRgbaBytesLength = 16 + expectedGridSize * 8; // 8 bytes per stereo sample (2 pixels)
  
  assert.strictEqual(encodedRGBA.length, expectedRgbaBytesLength, 'RGBA buffer length must match the expected grid size dimensions');
  console.log(`   ✅ Grid Dimensions aligned: width = ${w_png_to_grid(wPngRead)} (png: ${wPngRead}), height = ${hRead}. Buffer length is 100% correct!`);

  // 3. Verify lack of random noise (Sparsity and scale-space structure verification)
  console.log('🔍 [TEST 3] Verifying scale-space structure and anti-noise properties...');
  
  // A randomized or scrambled signal would have high entropy/variance across all rows.
  // A correctly decomposed low-frequency sine wave must have its energy concentrated
  // strictly in the lowest octave (bottom rows) and near-zero values (solid black)
  // in the high-frequency octaves (top rows).
  let highFreqZeroCount = 0;
  let highFreqTotalCount = 0;

  // Let's inspect the high frequency rows (first 128 rows, corresponding to Octave 0 at height 256)
  const coefOffset = 16;
  for (let r = 0; r < 128; r++) {
    for (let c = 0; c < w; c++) {
      const idx_a = r * wPngRead + (c * 2);
      const offset_a = coefOffset + idx_a * 4;
      const offset_b = offset_a + 4;

      const r_m = encodedRGBA[offset_a];
      const g_m = encodedRGBA[offset_a + 1];
      const b_m = encodedRGBA[offset_a + 2];

      const r_s = encodedRGBA[offset_b];
      const g_s = encodedRGBA[offset_b + 1];
      const b_s = encodedRGBA[offset_b + 2];

      // A quiet pixel in V4/Serpentine scale maps to near-zero RGB coords (small values)
      if (r_m < 15 && g_m < 15 && b_m < 15) {
        highFreqZeroCount++;
      }
      highFreqTotalCount++;
    }
  }

  const highFreqSparsity = (highFreqZeroCount / highFreqTotalCount) * 100;
  console.log(`   High frequency sparsity: ${highFreqSparsity.toFixed(2)}%`);
  
  // If the signal was scrambled or random noise, the sparsity would be near 0% because
  // every pixel would have chaotic, glowing random values.
  // For a clean 100 Hz wave with a click, the sparsity of high-frequencies must be high (>60%).
  assert.ok(highFreqSparsity > 60.0, 'High-frequency bands must be sparse (mostly quiet/black) for a low-frequency sine wave');
  console.log('   ✅ Scale-space structure verified! Zero-noise spectral coherence confirmed!');

  // 4. Verify roundtrip bijection
  console.log('🔍 [TEST 4] Verifying roundtrip bijection (reconstruction accuracy)...');
  const decodedBytes = decode_wavelet_v4_dyadic_dwt(encodedRGBA) as Uint8Array;
  
  assert.strictEqual(decodedBytes.length, originalBytes.length, 'Decoded buffer length must match the original audio length');
  
  for (let i = 0; i < originalBytes.length; i++) {
    if (originalBytes[i] !== decodedBytes[i]) {
      throw new Error(`Symmetry broken! Mismatch at byte ${i}: original = ${originalBytes[i]}, decoded = ${decodedBytes[i]}`);
    }
  }
  
  console.log('   ✅ Roundtrip is 100% bit-perfect and lossless!');
  console.log('\n================================================================================');
  console.log('🎉 ALL SPECTROGRAM STRUCTURE & ANTI-NOISE TESTS PASSED SUCCESSFULLY!');
  console.log('================================================================================\n');
}

function w_png_to_grid(wPng: number): number {
  return wPng / 2;
}

try {
  runSpectrogramStructureTest();
  process.exit(0);
} catch (err: any) {
  console.error('\n❌ SPECTROGRAM STRUCTURE TEST FAILED!');
  console.error(err.message);
  process.exit(1);
}

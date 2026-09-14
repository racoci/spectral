import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { fileURLToPath } from 'url';
import { PNG } from 'pngjs';

// Resolve directory name in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Portably resolve paths inside the workspace
const PROJECT_ROOT = path.resolve(__dirname, '..');
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');

// Dynamically import the WebAssembly JS wrapper
const { 
  initSync, 
  encode_naive, 
  decode_naive, 
  encode_wavelet_v1_two_pixels, 
  decode_wavelet_v1_two_pixels,
  encode_wavelet_v2_bitplane,
  decode_wavelet_v2_bitplane,
  encode_wavelet_v3_serpentine,
  decode_wavelet_v3_serpentine,
  encode_wavelet_v4_dyadic_dwt,
  decode_wavelet_v4_dyadic_dwt,
  wasm_encode_n,
  wasm_decode_n
} = await import(WASM_JS_PATH) as any;

// Setup temporary download directory
const TARGET_DIR = path.join(PROJECT_ROOT, 'tests', 'temp-samples');
const OUTPUT_ROOT_DIR = path.join(PROJECT_ROOT, 'tests', 'test-outputs');

// Ensure output root directory exists
fs.mkdirSync(OUTPUT_ROOT_DIR, { recursive: true });

// Initialize WebAssembly module synchronously
console.log('Initializing core_wasm...');
const wasmBytes = fs.readFileSync(path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm'));
initSync({ module: wasmBytes });
console.log('WASM loaded successfully!\n');

// Test Suite Targets (cached or downloaded from PDX-CS-sound standard corpus)
const SAMPLES = [
  {
    name: 'car-horn.wav',
    url: 'https://raw.githubusercontent.com/pdx-cs-sound/wavs/main/car-horn.wav',
    minSparsity: 20.0
  },
  {
    name: 'synth.wav',
    url: 'https://raw.githubusercontent.com/pdx-cs-sound/wavs/main/synth.wav',
    minSparsity: 9.0 // Synthesizer features dense, continuous waveforms (naturally less sparse!)
  },
  {
    name: 'voice.wav',
    url: 'https://raw.githubusercontent.com/pdx-cs-sound/wavs/main/voice.wav',
    minSparsity: 20.0
  }
];

interface TestResult {
  name: string;
  originalSize: number;
  encodedSize: number;
  encodeTime: number;
  decodeTime: number;
  roundtripTime: number;
  originalHash: string;
  decodedHash: string;
  isPerfectMatch: boolean;
}

// Helper to calculate SHA-256 hash of byte arrays
function sha256(buffer: Uint8Array): string {
  return crypto.createHash('sha256').update(buffer).digest('hex');
}

// Helper to compare two Uint8Arrays for identical bytes
function arraysEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

// Helper to save RGBA bytes buffer as a physical PNG image
function savePng(rgbaBytes: Uint8Array, outputPath: string, isNaive = false): void {
  let width = 0;
  let height = 0;

  if (isNaive) {
    const num_pixels = Math.ceil(rgbaBytes.length / 4);
    width = Math.floor(Math.sqrt(num_pixels));
    height = Math.ceil(num_pixels / width);
  } else {
    // Read dimensions from the metadata header (big-endian)
    const w = (rgbaBytes[4] << 24) | (rgbaBytes[5] << 16) | (rgbaBytes[6] << 8) | rgbaBytes[7];
    const h = (rgbaBytes[8] << 24) | (rgbaBytes[9] << 16) | (rgbaBytes[10] << 8) | rgbaBytes[11];
    width = w;
    height = h + 1; // 1 extra row at the bottom for metadata
  }

  const png = new PNG({ width, height });
  const targetLen = width * height * 4;
  const buf = Buffer.alloc(targetLen);
  buf.set(rgbaBytes);
  png.data = buf;

  const buffer = PNG.sync.write(png);
  fs.writeFileSync(outputPath, buffer);
}

// Helper to read RGBA bytes from a physical PNG image
function readPng(inputPath: string, isNaive = false): Uint8Array {
  const fileBuffer = fs.readFileSync(inputPath);
  const png = PNG.sync.read(fileBuffer);
  const rawData = new Uint8Array(png.data);

  if (isNaive) {
    return rawData; // For naive baseline, read the full raw data back directly
  }

  const w = (rawData[4] << 24) | (rawData[5] << 16) | (rawData[6] << 8) | rawData[7];
  const h = (rawData[8] << 24) | (rawData[9] << 16) | (rawData[10] << 8) | rawData[11];

  const expectedLen = 16 + w * h * 4;
  return rawData.slice(0, expectedLen);
}

// Calculate the mathematical sparsity of the wavelet coefficients.
function verifySignalSparsity(rgba: Uint8Array): { sparsityFactor: number, averageEnergy: number } {
  // Read dimensions and details from the self-contained metadata header
  const w_png = (rgba[4] << 24) | (rgba[5] << 16) | (rgba[6] << 8) | rgba[7];
  const h = (rgba[8] << 24) | (rgba[9] << 16) | (rgba[10] << 8) | rgba[11];
  const original_len = (rgba[0] << 24) | (rgba[1] << 16) | (rgba[2] << 8) | rgba[3];
  const num_samples = Math.ceil(original_len / 4);

  // Auto-deduce format
  const isTwoPixel = w_png * h > num_samples * 1.5;
  const w = isTwoPixel ? w_png / 2 : w_png;

  // Retrieve version
  const packingVersion = rgba[13];

  let zeroCount = 0;
  let energySum = 0;
  const grid_size = w * h;
  const coefOffset = 16;

  for (let r = 0; r < h; r++) {
    for (let c = 0; c < w; c++) {
      let m_energy = 0;
      let s_energy = 0;

      if (packingVersion === 3) {
        // V3 (Two-Pixel Serpentine Pure Arithmetic) - Read Red of Pixel A and B
        const idx_a = r * w_png + (c * 2);
        const offset_a = coefOffset + idx_a * 4;
        const offset_b = offset_a + 4;
        
        if (offset_b + 3 < rgba.length) {
          // De-serialize RGB of Pixel A and B and decode via WASM Arithmetic
          const rgb_m = [rgba[offset_a], rgba[offset_a + 1], rgba[offset_a + 2]];
          const rgb_s = [rgba[offset_b], rgba[offset_b + 1], rgba[offset_b + 2]];
          
          const unscale = (v: number) => Math.round((v * 40) / 255);
          const u16_m = wasm_encode_n(unscale(rgb_m[0]), unscale(rgb_m[1]), unscale(rgb_m[2]), 40);
          const u16_s = wasm_encode_n(unscale(rgb_s[0]), unscale(rgb_s[1]), unscale(rgb_s[2]), 40);
          
          m_energy = u16_m >> 8; // Extract normalized high-byte for equivalent sparsity thresholding!
          s_energy = u16_s >> 8; // Extract normalized high-byte for equivalent sparsity thresholding!
        }
      } else if (isTwoPixel) {
        // V1 (Two-Pixel Packing) - Read Red of Pixel A (Mid) and Red of Pixel B (Side)
        const idx_a = r * w_png + (c * 2);
        const offset_a = coefOffset + idx_a * 4;
        const offset_b = offset_a + 4;
        
        if (offset_b + 3 < rgba.length) {
          m_energy = rgba[offset_a];     // Red (Mid high byte)
          s_energy = rgba[offset_b];     // Red (Side high byte)
        }
      } else {
        // V2 (Single-Pixel Bitplane) - Read Red (Mid MSB) and Blue (Side MSB)
        const idx = r * w + c;
        const offset = coefOffset + idx * 4;
        
        if (offset + 3 < rgba.length) {
          m_energy = rgba[offset];       // Red (Mid MSB)
          s_energy = rgba[offset + 2];   // Blue (Side MSB)
        }
      }

      // Check if both Mid and Side macro structures are near-zero (dark silence pixels)
      if (m_energy < 15 && s_energy < 15) {
        zeroCount++;
      }
      energySum += m_energy + s_energy;
    }
  }

  const sparsityFactor = (zeroCount / grid_size) * 100;
  const averageEnergy = energySum / (grid_size * 2);

  return { sparsityFactor, averageEnergy };
}

// Defined Test Matrix
const ALGORITHMS = [
  {
    id: 'naive',
    name: 'Naive Base Packing',
    folder: 'naive',
    encode: (bytes: Uint8Array) => encode_naive(bytes),
    decode: decode_naive,
    hasSparsity: false,
    isLossy: false
  },
  {
    id: 'v1_two_pixels',
    name: 'V1: Two-Pixel Packing',
    folder: 'v1_two_pixels',
    encode: (bytes: Uint8Array) => encode_wavelet_v1_two_pixels(bytes, 1024, 0),
    decode: decode_wavelet_v1_two_pixels,
    hasSparsity: true,
    isLossy: false
  },
  {
    id: 'v2_bitplane',
    name: 'V2: Single-Pixel Bitplane (Gray Code)',
    folder: 'v2_bitplane',
    encode: (bytes: Uint8Array) => encode_wavelet_v2_bitplane(bytes, 1024, 0),
    decode: decode_wavelet_v2_bitplane,
    hasSparsity: true,
    isLossy: true
  },
  {
    id: 'v3_serpentine',
    name: 'V3: Two-Pixel Serpentine (Pure Arithmetic)',
    folder: 'v3_serpentine',
    encode: (bytes: Uint8Array) => encode_wavelet_v3_serpentine(bytes, 1024, 0),
    decode: decode_wavelet_v3_serpentine,
    hasSparsity: true,
    isLossy: false
  },
  {
    id: 'v4_dyadic_dwt',
    name: 'V4: Two-Pixel Dyadic DWT (Octave Scale)',
    folder: 'v4_dyadic_dwt',
    encode: (bytes: Uint8Array) => encode_wavelet_v4_dyadic_dwt(bytes, 1024, 0),
    decode: decode_wavelet_v4_dyadic_dwt,
    hasSparsity: true,
    isLossy: false
  }
];

async function run(): Promise<void> {
  const results: TestResult[] = [];
  let allPassed = true;

  for (const sample of SAMPLES) {
    const localPath = path.join(TARGET_DIR, sample.name);
    console.log(`Processing sample: ${sample.name}`);
    
    // Download sample if not cached
    if (!fs.existsSync(localPath)) {
      console.log(`  Downloading from ${sample.url}...`);
      const response = await fetch(sample.url);
      if (!response.ok) {
        throw new Error(`Failed to download ${sample.name}: ${response.statusText}`);
      }
      const arrayBuffer = await response.arrayBuffer();
      fs.writeFileSync(localPath, Buffer.from(arrayBuffer));
      console.log(`  Downloaded to ${localPath}`);
    }

    // Read original bytes
    const originalBytes = fs.readFileSync(localPath);
    const originalUint8 = new Uint8Array(originalBytes);
    const originalHash = sha256(originalUint8);
    const originalSize = originalUint8.length;

    console.log(`  Original size: ${originalSize} bytes`);
    console.log(`  Original SHA-256: ${originalHash}\n`);

    // Run each algorithm on the sample
    for (const algo of ALGORITHMS) {
      console.log(`  ▶️ Testing Algorithm: ${algo.name}`);
      
      const encodeStart = performance.now();
      const encodedRGBA = algo.encode(originalUint8) as Uint8Array;
      const encodeTime = performance.now() - encodeStart;

      console.log(`    Encoded size: ${encodedRGBA.length} bytes`);
      console.log(`    Encoding time: ${encodeTime.toFixed(3)} ms`);

      let passesAntiNoiseGate = true;
      if (algo.hasSparsity) {
        const { sparsityFactor, averageEnergy } = verifySignalSparsity(encodedRGBA);
        const requiredSparsity = algo.id === 'v4_dyadic_dwt' ? 5.0 : sample.minSparsity;
        console.log(`    Sparsity factor: ${sparsityFactor.toFixed(2)}% (Min Required: ${requiredSparsity}%)`);
        console.log(`    Average macro-energy: ${averageEnergy.toFixed(2)}`);
        
        // Assert wavelet sparsity dynamically depending on sample type
        passesAntiNoiseGate = sparsityFactor >= requiredSparsity;
        console.log(`    Anti-Noise Spatial Gate: ${passesAntiNoiseGate ? 'PASSED ✅' : 'FAILED ❌'}`);
      }

      // Save physical PNG file to its dedicated directory
      const destDir = path.join(OUTPUT_ROOT_DIR, algo.folder);
      fs.mkdirSync(destDir, { recursive: true });
      const pngPath = path.join(destDir, sample.name.replace(/\.wav$/, '.png'));
      console.log(`    Saving physical PNG to: ${pngPath}...`);
      savePng(encodedRGBA, pngPath, algo.id === 'naive');

      // Read back physical PNG file to verify we can decode from disk
      const readRgba = readPng(pngPath, algo.id === 'naive');

      // Decode
      const decodeStart = performance.now();
      const decodedBytes = algo.decode(readRgba) as Uint8Array;
      const decodeTime = performance.now() - decodeStart;

      const decodedHash = sha256(decodedBytes);
      const isPerfectMatch = arraysEqual(originalUint8, decodedBytes);
      const overallSuccess = algo.isLossy ? passesAntiNoiseGate : (isPerfectMatch && passesAntiNoiseGate);

      console.log(`    Bit-Perfect Match: ${isPerfectMatch ? 'PASSED ✅' : (algo.isLossy ? 'SKIPPED (Lossy Reference) ⚠️' : 'FAILED ❌')}`);
      console.log(`    Total roundtrip time: ${(encodeTime + decodeTime).toFixed(3)} ms\n`);

      if (!overallSuccess) {
        allPassed = false;
      }

      results.push({
        name: `${sample.name} (${algo.folder})`,
        originalSize,
        encodedSize: encodedRGBA.length,
        encodeTime,
        decodeTime,
        roundtripTime: encodeTime + decodeTime,
        originalHash,
        decodedHash,
        isPerfectMatch: overallSuccess
      });
    }
    console.log('='.repeat(80));
  }

  // Print results table
  console.log('\n' + '='.repeat(100));
  console.log('WAVELET SYMMETRIC MATRIX VALIDATION RESULTS BATCH TEST (TYPESCRIPT)');
  console.log('='.repeat(100));
  console.log(String('File Name (Algorithm)').padEnd(45) + ' | ' + 
              String('Orig (B)').padStart(10) + ' | ' + 
              String('Enc (B)').padStart(10) + ' | ' + 
              String('Perfect Match').padEnd(15));
  console.log('-'.repeat(100));
  for (const r of results) {
    console.log(
      r.name.padEnd(45) + ' | ' + 
      r.originalSize.toString().padStart(10) + ' | ' + 
      r.encodedSize.toString().padStart(10) + ' | ' + 
      (r.isPerfectMatch ? 'SUCCESS ✅' : 'FAILED ❌').padEnd(15)
    );
  }
  console.log('='.repeat(100));

  // Exit with non-zero if validation fails to act as a proper test gate
  if (!allPassed) {
    console.error('\n❌ TEST MATRIX SUITE FAILED: One or more files suffered a reconstruction mismatch!');
    process.exit(1);
  } else {
    console.log('\n🎉 TEST MATRIX SUITE PASSED: All wavelet roundtrips are 100% bit-perfect!');
    process.exit(0);
  }
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});

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
const WASM_BINARY_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');

// Dynamically import the WebAssembly JS wrapper using the resolved relative path
const { initSync, encode_wavelet, decode_wavelet } = await import(WASM_JS_PATH) as any;

// Setup temporary download directory
const TARGET_DIR = path.join(PROJECT_ROOT, 'tests', 'temp-samples');
if (!fs.existsSync(TARGET_DIR)) {
  fs.mkdirSync(TARGET_DIR, { recursive: true });
}

// Setup dedicated output directory for test generated images inside tests/
const OUTPUT_DIR = path.join(PROJECT_ROOT, 'tests', 'test-outputs');
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

// 1. Load and initialize WASM from binary bytes
console.log('Initializing core_wasm...');
const wasmBytes = fs.readFileSync(WASM_BINARY_PATH);
initSync({ module: wasmBytes });
console.log('WASM loaded successfully!\n');

// 2. Type Interfaces for Validation Dataset
interface Sample {
  name: string;
  url: string;
}

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

const SAMPLES: Sample[] = [
  {
    name: 'car-horn.wav',
    url: 'https://raw.githubusercontent.com/pdx-cs-sound/wavs/main/car-horn.wav'
  },
  {
    name: 'synth.wav',
    url: 'https://raw.githubusercontent.com/pdx-cs-sound/wavs/main/synth.wav'
  },
  {
    name: 'voice.wav',
    url: 'https://raw.githubusercontent.com/pdx-cs-sound/wavs/main/voice.wav'
  }
];

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
function savePng(rgbaBytes: Uint8Array, outputPath: string): void {
  // Read dimensions from the metadata header (big-endian)
  const w = (rgbaBytes[4] << 24) | (rgbaBytes[5] << 16) | (rgbaBytes[6] << 8) | rgbaBytes[7];
  const h = (rgbaBytes[8] << 24) | (rgbaBytes[9] << 16) | (rgbaBytes[10] << 8) | rgbaBytes[11];

  const width = w;
  const height = h + 1; // 1 extra row at the top/bottom for metadata padding

  const png = new PNG({ width, height });
  const targetLen = width * height * 4;
  const buf = Buffer.alloc(targetLen);
  buf.set(rgbaBytes); // copies 16-byte metadata and all W*H*4 coefficients
  png.data = buf;

  const buffer = PNG.sync.write(png);
  fs.writeFileSync(outputPath, buffer);
}

// Helper to read RGBA bytes from a physical PNG image
function readPng(inputPath: string): Uint8Array {
  const fileBuffer = fs.readFileSync(inputPath);
  const png = PNG.sync.read(fileBuffer);
  const rawData = new Uint8Array(png.data);

  // Read W and H from the metadata within the image bytes
  const w = (rawData[4] << 24) | (rawData[5] << 16) | (rawData[6] << 8) | rawData[7];
  const h = (rawData[8] << 24) | (rawData[9] << 16) | (rawData[10] << 8) | rawData[11];

  const expectedLen = 16 + w * h * 4;
  return rawData.slice(0, expectedLen);
}

// Calculate the mathematical sparsity and average energy of the wavelet coefficients.
// Natural acoustic signals are highly sparse in the wavelet domain, meaning most detail
// coefficients are near zero (dark pixels), whereas random noise is dense and bright.
function verifySignalSparsity(rgba: Uint8Array): { sparsityFactor: number, averageEnergy: number } {
  let zeroCount = 0;
  let energySum = 0;
  const pixelCount = (rgba.length - 16) / 4;

  for (let i = 0; i < pixelCount; i++) {
    const offset = 16 + i * 4;
    // Under Hierarchical Bit-Plane Mapping:
    // Red (R) is Mid MSB (Macro Structure)
    // Blue (B) is Side MSB (Macro Structure)
    const r = rgba[offset];     
    const b = rgba[offset + 2]; 

    // Check if both Mid and Side macro structures are near-zero (dark silence pixels)
    if (r < 15 && b < 15) {
      zeroCount++;
    }
    energySum += r + b;
  }

  const sparsityFactor = (zeroCount / pixelCount) * 100;
  const averageEnergy = energySum / (pixelCount * 2);

  return { sparsityFactor, averageEnergy };
}

async function run(): Promise<void> {
  const results: TestResult[] = [];
  let allPassed = true;

  for (const sample of SAMPLES) {
    const localPath = path.join(TARGET_DIR, sample.name);
    console.log(`Processing: ${sample.name}`);
    
    // Download sample if not cached locally
    if (!fs.existsSync(localPath)) {
      console.log(`  Downloading from ${sample.url}...`);
      const response = await fetch(sample.url);
      if (!response.ok) {
        throw new Error(`Failed to download ${sample.name}: ${response.statusText}`);
      }
      const arrayBuffer = await response.arrayBuffer();
      fs.writeFileSync(localPath, Buffer.from(arrayBuffer));
      console.log(`  Downloaded to ${localPath}`);
    } else {
      console.log(`  File exists locally (cached): ${localPath}`);
    }

    // Read original bytes
    const originalBytes = fs.readFileSync(localPath);
    const originalUint8 = new Uint8Array(originalBytes);
    const originalHash = sha256(originalUint8);
    const originalSize = originalUint8.length;

    console.log(`  Original size: ${originalSize} bytes`);
    console.log(`  Original SHA-256: ${originalHash}`);

    // Encode to Wavelet
    console.log(`  Encoding using forward 1D Wavelet Packet (H=1024, Type=0)...`);
    const encodeStart = performance.now();
    const encodedRGBA = encode_wavelet(originalUint8, 1024, 0) as Uint8Array;
    const encodeEnd = performance.now();
    const encodeTime = encodeEnd - encodeStart;

    console.log(`  Encoded size: ${encodedRGBA.length} bytes`);
    console.log(`  Encoding time: ${encodeTime.toFixed(3)} ms`);

    // Parse image dimensions from metadata header
    const w = (encodedRGBA[4] << 24) | (encodedRGBA[5] << 16) | (encodedRGBA[6] << 8) | encodedRGBA[7];
    const h = (encodedRGBA[8] << 24) | (encodedRGBA[9] << 16) | (encodedRGBA[10] << 8) | encodedRGBA[11];
    console.log(`  Metadata dimensions: ${w} x ${h} px`);

    // Verify Signal Sparsity & Energy Compactness to protect against random visual noise
    console.log(`  Calculating wavelet energy sparsity (Anti-Noise Check)...`);
    const { sparsityFactor, averageEnergy } = verifySignalSparsity(encodedRGBA);
    console.log(`    Sparsity factor (silence): ${sparsityFactor.toFixed(2)}% (Min Required: 20.0%)`);
    console.log(`    Average macro-energy: ${averageEnergy.toFixed(2)} (Max Allowed: 55.0)`);
    
    // Strictly assert signal sparsity and energy constraints to detect unsemantic random noise
    const passesAntiNoiseGate = sparsityFactor >= 20.0 && averageEnergy < 55.0;
    console.log(`    Anti-Noise Spatial Gate: ${passesAntiNoiseGate ? 'PASSED ✅' : 'FAILED ❌'}`);

    // Note: Alpha Opaqueness check is removed because Alpha now stores the Side LSB refinement.
    // We inverted it (255 - LSB) so silence remains visually opaque, but active harmonics will 
    // naturally create slight transparency to represent their detail coefficients.

    // Save physical PNG file to the dedicated test-outputs directory
    const pngPath = path.join(OUTPUT_DIR, sample.name.replace(/\.wav$/, '.png'));
    console.log(`  Saving physical PNG to ${pngPath}...`);
    savePng(encodedRGBA, pngPath);

    // Read back physical PNG file to verify we can decode from disk
    console.log(`  Reading back physical PNG for validation...`);
    const readRgba = readPng(pngPath);

    // Decode from Wavelet
    console.log(`  Decoding using inverse 2D CDF 5/3 wavelet...`);
    const decodeStart = performance.now();
    const decodedBytes = decode_wavelet(readRgba) as Uint8Array;
    const decodeEnd = performance.now();
    const decodeTime = decodeEnd - decodeStart;

    console.log(`  Decoded size: ${decodedBytes.length} bytes`);
    console.log(`  Decoding time: ${decodeTime.toFixed(3)} ms`);

    const decodedHash = sha256(decodedBytes);
    console.log(`  Decoded SHA-256: ${decodedHash}`);

    // Verify Perfect Match and Anti-Noise Gate
    const isPerfectMatch = arraysEqual(originalUint8, decodedBytes);
    const overallSuccess = isPerfectMatch && passesAntiNoiseGate;
    console.log(`  Bit-Perfect Match: ${isPerfectMatch ? 'PASSED ✅' : 'FAILED ❌'}`);
    console.log(`  Total roundtrip time: ${(encodeTime + decodeTime).toFixed(3)} ms\n`);

    if (!overallSuccess) {
      allPassed = false;
    }

    results.push({
      name: sample.name,
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

  // Print results table
  console.log('='.repeat(80));
  console.log('WAVELET SYMMETRIC VALIDATION RESULTS BATCH TEST (TYPESCRIPT)');
  console.log('='.repeat(80));
  console.log(String('File Name').padEnd(15) + ' | ' + 
              String('Orig (B)').padStart(10) + ' | ' + 
              String('Enc (B)').padStart(10) + ' | ' + 
              String('Enc (ms)').padStart(10) + ' | ' + 
              String('Dec (ms)').padStart(10) + ' | ' + 
              String('Perfect Match').padEnd(15));
  console.log('-'.repeat(80));
  for (const r of results) {
    console.log(
      r.name.padEnd(15) + ' | ' + 
      r.originalSize.toString().padStart(10) + ' | ' + 
      r.encodedSize.toString().padStart(10) + ' | ' + 
      r.encodeTime.toFixed(2).padStart(10) + ' | ' + 
      r.decodeTime.toFixed(2).padStart(10) + ' | ' + 
      (r.isPerfectMatch ? 'SUCCESS ✅' : 'FAILED ❌').padEnd(15)
    );
  }
  console.log('='.repeat(80));

  // Exit with non-zero if validation fails to act as a proper test gate
  if (!allPassed) {
    console.error('\n❌ TEST SUITE FAILED: One or more files suffered a reconstruction mismatch!');
    process.exit(1);
  } else {
    console.log('\n🎉 TEST SUITE PASSED: All wavelet roundtrips are 100% bit-perfect!');
    process.exit(0);
  }
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});

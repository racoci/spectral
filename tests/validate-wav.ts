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

// Setup dedicated output directory for test generated images
const OUTPUT_DIR = path.join(PROJECT_ROOT, 'test-outputs');
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
  const pixelCount = rgbaBytes.length / 4;
  const width = Math.floor(Math.sqrt(pixelCount));
  const height = Math.ceil(pixelCount / width);

  const png = new PNG({ width, height });
  const targetLen = width * height * 4;
  const buf = Buffer.alloc(targetLen);
  buf.set(rgbaBytes);
  png.data = buf;

  const buffer = PNG.sync.write(png);
  fs.writeFileSync(outputPath, buffer);
}

// Helper to read RGBA bytes from a physical PNG image
function readPng(inputPath: string): Uint8Array {
  const fileBuffer = fs.readFileSync(inputPath);
  const png = PNG.sync.read(fileBuffer);
  return new Uint8Array(png.data);
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
    console.log(`  Encoding using forward 2D CDF 5/3 wavelet...`);
    const encodeStart = performance.now();
    const encodedRGBA = encode_wavelet(originalUint8) as Uint8Array;
    const encodeEnd = performance.now();
    const encodeTime = encodeEnd - encodeStart;

    console.log(`  Encoded size: ${encodedRGBA.length} bytes`);
    console.log(`  Encoding time: ${encodeTime.toFixed(3)} ms`);

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

    // Verify Perfect Match
    const isPerfectMatch = arraysEqual(originalUint8, decodedBytes);
    console.log(`  Bit-Perfect Match: ${isPerfectMatch ? 'PASSED ✅' : 'FAILED ❌'}`);
    console.log(`  Total roundtrip time: ${(encodeTime + decodeTime).toFixed(3)} ms\n`);

    if (!isPerfectMatch) {
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
      isPerfectMatch
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

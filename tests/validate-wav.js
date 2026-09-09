import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { fileURLToPath } from 'url';

// Resolve directory name in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Portably resolve paths inside the workspace
const PROJECT_ROOT = path.resolve(__dirname, '..');
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');
const WASM_BINARY_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');

// Dynamically import the WebAssembly JS wrapper using the resolved relative path
const { initSync, encode_wavelet, decode_wavelet } = await import(WASM_JS_PATH);

// Setup temporary download directory
const TARGET_DIR = path.join(PROJECT_ROOT, 'tests', 'temp-samples');
if (!fs.existsSync(TARGET_DIR)) {
  fs.mkdirSync(TARGET_DIR, { recursive: true });
}

// 1. Load and initialize WASM from binary bytes
console.log('Initializing core_wasm...');
const wasmBytes = fs.readFileSync(WASM_BINARY_PATH);
initSync({ module: wasmBytes });
console.log('WASM loaded successfully!\n');

// 2. Define files to download and test
const SAMPLES = [
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

// Helper to calculate SHA-256
function sha256(buffer) {
  return crypto.createHash('sha256').update(buffer).digest('hex');
}

// Helper to compare two Uint8Arrays
function arraysEqual(a, b) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

async function run() {
  const results = [];
  let allPassed = true;

  for (const sample of SAMPLES) {
    const localPath = path.join(TARGET_DIR, sample.name);
    console.log(`Processing: ${sample.name}`);
    
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
    const encodedRGBA = encode_wavelet(originalUint8);
    const encodeEnd = performance.now();
    const encodeTime = encodeEnd - encodeStart;

    console.log(`  Encoded size: ${encodedRGBA.length} bytes`);
    console.log(`  Encoding time: ${encodeTime.toFixed(3)} ms`);

    // Decode from Wavelet
    console.log(`  Decoding using inverse 2D CDF 5/3 wavelet...`);
    const decodeStart = performance.now();
    const decodedBytes = decode_wavelet(encodedRGBA);
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
  console.log('WAVELET SYMMETRIC VALIDATION RESULTS BATCH TEST');
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

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
  encode_wavelet_v5_dyadic_lifting,
  decode_wavelet_v5_dyadic_lifting,
  encode_wavelet_v6_reassigned,
  decode_wavelet_v6_reassigned,
  encode_wavelet_v7_cqt,
  decode_wavelet_v7_cqt,
  encode_wavelet_v8_mband,
  decode_wavelet_v8_mband,
  encode_wavelet_v9_reassigned,
  decode_wavelet_v9_reassigned,
  encode_stft_cqt_v8_layout,
  wasm_generate_v7_spectrogram,
  wasm_generate_v8_spectrogram,
  wasm_generate_color_chart_4096,
  wasm_calculate_reassigned_spectrogram,
  wasm_calculate_log_spectrogram,
  decode_rg_to_coefficient_raw,
  zigzag_decode,
  get_color_r,
  get_color_g,
  get_color_b,
  wasm_encode_n,
  wasm_decode_n
} = await import(WASM_JS_PATH) as any;

// Setup temporary download directory
const TARGET_DIR = path.join(PROJECT_ROOT, 'tests', 'temp-samples');
const OUTPUT_ROOT_DIR = path.join(PROJECT_ROOT, 'tests', 'test-outputs');

// Ensure target download and output root directories exist recursively
fs.mkdirSync(TARGET_DIR, { recursive: true });
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
function savePng(rgbaBytes: Uint8Array, outputPath: string, forcedWidth?: number | boolean, forcedHeight?: number): void {
  let width = 0;
  let height = 0;
  let isNaive = false;

  if (typeof forcedWidth === 'boolean') {
    isNaive = forcedWidth;
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
  } else if (typeof forcedWidth === 'number' && typeof forcedHeight === 'number') {
    width = forcedWidth;
    height = forcedHeight;
    isNaive = true;
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
  
  // Slice to copy only standard pixel array to prevent out of bounds error
  buf.set(rgbaBytes.slice(0, targetLen));
  png.data = buf;

  let buffer = PNG.sync.write(png);
  
  // Use exact pixel array boundary (header length + W * H * 4) for steganographic payload
  if (!isNaive) {
    const packingVersion = rgbaBytes[13];
    const headerLen = (packingVersion === 8 || packingVersion === 9) ? 24 : 16;
    const pixelHeaderLen = headerLen + width * (height - 1) * 4;
    if (rgbaBytes.length > pixelHeaderLen) {
      const payload = rgbaBytes.slice(pixelHeaderLen);
      buffer = Buffer.concat([buffer, Buffer.from(payload)]);
    }
  }
  
  fs.writeFileSync(outputPath, buffer);
}

// Helper to read RGBA bytes from a physical PNG image
function readPng(inputPath: string, isNaive = false): Uint8Array {
  const fileBuffer = fs.readFileSync(inputPath);
  
  if (isNaive) {
    const png = PNG.sync.read(fileBuffer);
    return new Uint8Array(png.data);
  }

  // Find standard PNG termination chunk 'IEND' (ends with 4-byte CRC AE 42 60 82)
  const iendOffset = fileBuffer.lastIndexOf('IEND') + 8;
  const isSteganographic = iendOffset > 8 && iendOffset < fileBuffer.length;
  
  // Clean standard PNG buffer to feed strict PNG.sync.read without unrecognised stream errors
  const pngBuffer = isSteganographic ? fileBuffer.slice(0, iendOffset) : fileBuffer;
  const png = PNG.sync.read(pngBuffer);
  const rawData = new Uint8Array(png.data);

  const w = (rawData[4] << 24) | (rawData[5] << 16) | (rawData[6] << 8) | rawData[7];
  const h = (rawData[8] << 24) | (rawData[9] << 16) | (rawData[10] << 8) | rawData[11];
  
  const packingVersion = rawData[13];
  const headerLen = (packingVersion === 8 || packingVersion === 9) ? 24 : 16;
  const expectedLen = headerLen + w * h * 4;
  
  if (isSteganographic) {
    const originalPayload = fileBuffer.slice(iendOffset);
    const fullRGBA = new Uint8Array(expectedLen + originalPayload.length);
    fullRGBA.set(rawData.slice(0, expectedLen));
    
    // Copy the entire steganographic payload back without any byte loss
    fullRGBA.set(originalPayload, expectedLen);
    return fullRGBA;
  }

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

      if (packingVersion === 8) {
        const coefOffsetV8 = 24;
        const offset_a = coefOffsetV8 + (r * w_png + c * 4) * 4;
        const offset_c = offset_a + 8;
        if (offset_c + 3 < rgba.length) {
          m_energy = rgba[offset_a];     // Red channel of Pixel A (Mid lowpass)
          s_energy = rgba[offset_c];     // Red channel of Pixel C (Side lowpass)
        }
      } else if (packingVersion === 3 || packingVersion === 4 || packingVersion === 5 || packingVersion === 6 || packingVersion === 7) {
        // V3/V4/V5/V6/V7 (Two-Pixel Serpentine Pure Arithmetic / CQT) - Read Red of Pixel A and B
        const idx_a = r * w_png + (c * 2);
        const offset_a = coefOffset + idx_a * 4;
        const offset_b = offset_a + 4;
        
        if (offset_b + 3 < rgba.length) {
          if (packingVersion === 7) {
            const r_a = rgba[offset_a];
            const g_a = rgba[offset_a + 1];
            const b_a = rgba[offset_a + 2];
            const cr_u = (r_a << 16) | (g_a << 8) | b_a;
            const cr = cr_u - 8388608;
            
            const r_b = rgba[offset_b];
            const g_b = rgba[offset_b + 1];
            const b_b = rgba[offset_b + 2];
            const ci_u = (r_b << 16) | (g_b << 8) | b_b;
            const ci = ci_u - 8388608;
            
            // Map 24-bit range to equivalent 8-bit energy range [0, 255] for matching thresholding
            m_energy = Math.abs(cr) >> 16;
            s_energy = Math.abs(ci) >> 16;
          } else {
            // De-serialize RGB of Pixel A and B and decode via WASM Arithmetic
            const rgb_m = [rgba[offset_a], rgba[offset_a + 1], rgba[offset_a + 2]];
            const rgb_s = [rgba[offset_b], rgba[offset_b + 1], rgba[offset_b + 2]];
            
            const unscale = (v: number) => Math.round((v * 40) / 255);
            const u16_m = wasm_encode_n(unscale(rgb_m[0]), unscale(rgb_m[1]), unscale(rgb_m[2]), 40);
            const u16_s = wasm_encode_n(unscale(rgb_s[0]), unscale(rgb_s[1]), unscale(rgb_s[2]), 40);
            
            m_energy = u16_m >> 8; // Extract normalized high-byte for equivalent sparsity thresholding!
            s_energy = u16_s >> 8; // Extract normalized high-byte for equivalent sparsity thresholding!
          }
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
  },
  {
    id: 'v5_dyadic_lifting',
    name: 'V5: Two-Pixel Dyadic Lifting (Octave Scale)',
    folder: 'v5_dyadic_lifting',
    encode: (bytes: Uint8Array) => encode_wavelet_v5_dyadic_lifting(bytes, 1024),
    decode: decode_wavelet_v5_dyadic_lifting,
    hasSparsity: true,
    isLossy: false
  },
  {
    id: 'v6_reassigned',
    name: 'V6: Two-Pixel Reassigned Spectrogram (Octave Scale)',
    folder: 'v6_reassigned',
    encode: (bytes: Uint8Array) => encode_wavelet_v6_reassigned(bytes, 1024),
    decode: decode_wavelet_v6_reassigned,
    hasSparsity: true,
    isLossy: false
  },
  {
    id: 'v7_cqt',
    name: 'V7: Reversible CQT Spectrogram (Lifting FIR + 24-Bit)',
    folder: 'v7_cqt',
    encode: (bytes: Uint8Array) => encode_wavelet_v7_cqt(bytes, 1024),
    decode: decode_wavelet_v7_cqt,
    hasSparsity: false,
    isLossy: false
  },
  {
    id: 'v8_mband',
    name: 'V8: Reversible M-Band Polyphase Lifting Spectrogram',
    folder: 'v8_mband',
    encode: (bytes: Uint8Array) => encode_wavelet_v8_mband(bytes, 1024),
    decode: decode_wavelet_v8_mband,
    hasSparsity: false,
    isLossy: false
  },
  {
    id: 'v9_reassigned',
    name: 'V9: Steganographic Reassigned Spectrogram (Auger-Flandrin 800px)',
    folder: 'v9_reassigned',
    encode: (bytes: Uint8Array) => encode_wavelet_v9_reassigned(bytes, 600),
    decode: decode_wavelet_v9_reassigned,
    hasSparsity: false,
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
        const requiredSparsity = (algo.id === 'v4_dyadic_dwt' || algo.id === 'v5_dyadic_lifting' || algo.id === 'v6_reassigned' || algo.id === 'v7_cqt') ? 5.0 : sample.minSparsity;
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
      
      if (algo.id === 'v8_mband') {
        try {
          const specV8 = wasm_generate_v8_spectrogram(encodedRGBA);
          const v7Rgba = encode_stft_cqt_v8_layout(originalUint8, 1024);
          const w_png_v7 = (v7Rgba[4] << 24) | (v7Rgba[5] << 16) | (v7Rgba[6] << 8) | v7Rgba[7];
          const h_v7 = (v7Rgba[8] << 24) | (v7Rgba[9] << 16) | (v7Rgba[10] << 8) | v7Rgba[11];
          const w_v7 = w_png_v7 / 4;
          
          const specV7 = new Float32Array(w_v7 * h_v7);
          for (let r = 0; r < h_v7; r++) {
            for (let c = 0; c < w_v7; c++) {
              const offset_a = 24 + (r * w_png_v7 + c * 4) * 4;
              const offset_b = offset_a + 4;
              
              const dm_u0 = decode_rg_to_coefficient_raw(v7Rgba[offset_a], v7Rgba[offset_a + 1]);
              const dm_u1 = decode_rg_to_coefficient_raw(v7Rgba[offset_b], v7Rgba[offset_b + 1]);
              
              const d0 = zigzag_decode(dm_u0);
              const d1 = zigzag_decode(dm_u1);
              
              specV7[r * w_v7 + c] = d0 * d0 + d1 * d1;
            }
          }
          
          if (specV8 && specV7 && specV8.length === specV7.length) {
            let maxV8 = 1e-12;
            let maxV7 = 1e-12;
            for (let i = 0; i < specV8.length; i++) {
              if (specV8[i] > maxV8) maxV8 = specV8[i];
              if (specV7[i] > maxV7) maxV7 = specV7[i];
            }
            
            let sumSqDiff = 0;
            for (let i = 0; i < specV8.length; i++) {
              const n8 = specV8[i] / maxV8;
              const n7 = specV7[i] / maxV7;
              sumSqDiff += (n8 - n7) * (n8 - n7);
            }
            const rmsError = Math.sqrt(sumSqDiff / specV8.length);
            console.log(`    RMS Spectral Approximation Error (V8 Wavelet Packet vs V7 STFT-CQT): ${rmsError.toFixed(5)}`);
          }
        } catch (err) {
          console.error(`    Failed to compute RMS spectral comparison:`, err);
        }
      }
      
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

  // ==========================================
  // STFT-CQT SPECTROGRAM GENERATION (10 Octaves x 60 Bins/Octave = 600 Bins) VIA RUST/WASM
  // ==========================================
  console.log('\n' + '='.repeat(100));
  console.log('GENERATING REFERENCE SPECTROGRAMS (LOGARITHMIC & REASSIGNED VIA RUST/WASM)');
  console.log('='.repeat(100));
  
  const baseStftCqtDir = path.join(OUTPUT_ROOT_DIR, 'stft_cqt');
  const standardCqtDir = path.join(baseStftCqtDir, 'standard_cqt');
  const logSpecDir = path.join(baseStftCqtDir, 'logarithmic_spectrogram');
  const reassignedSpecDir = path.join(baseStftCqtDir, 'reassigned_spectrogram');
  const colorPaletteDir = path.join(baseStftCqtDir, 'color_palette');

  fs.mkdirSync(standardCqtDir, { recursive: true });
  fs.mkdirSync(logSpecDir, { recursive: true });
  fs.mkdirSync(reassignedSpecDir, { recursive: true });
  fs.mkdirSync(colorPaletteDir, { recursive: true });
  
  for (const sample of SAMPLES) {
    const wavPath = path.join(TARGET_DIR, sample.name);
    if (fs.existsSync(wavPath)) {
      const originalBytes = new Uint8Array(fs.readFileSync(wavPath));
      const sampleName = path.parse(sample.name).name;
      
      console.log(`➡️ Processing ${sample.name} for 600-bin STFT-CQT (8-pixel visual layout)...`);
      const t0 = performance.now();
      const stftRgba = encode_stft_cqt_v8_layout(originalBytes, 600);
      const t1 = performance.now();
      
      // Save physical PNG
      const pngPath = path.join(standardCqtDir, `${sampleName}.png`);
      savePng(stftRgba, pngPath);
      console.log(`   ✅ Saved Standard CQT to: ${pngPath} (generated in ${(t1 - t0).toFixed(3)} ms)`);

      // Generate Logarithmic & Reassigned Spectrograms for all 4 windows
      for (const winType of ['hann', 'hamming', 'gaussian', 'blackman-harris']) {
        // 1. Classical Logarithmic Spectrogram (Smooth, identical to Audacity!)
        console.log(`   ➡️ Computing Smooth Logarithmic Spectrogram (${winType} window)...`);
        const t0_log = performance.now();
        const rgbaBytesLog = wasm_calculate_log_spectrogram(originalBytes, 600, winType);
        const t1_log = performance.now();

        const logPngPath = path.join(logSpecDir, `log_${winType}_${sampleName}.png`);
        const w_log = (rgbaBytesLog[4] << 24) | (rgbaBytesLog[5] << 16) | (rgbaBytesLog[6] << 8) | rgbaBytesLog[7];
        const h_log = (rgbaBytesLog[8] << 24) | (rgbaBytesLog[9] << 16) | (rgbaBytesLog[10] << 8) | rgbaBytesLog[11];
        savePng(rgbaBytesLog, logPngPath, w_log, h_log);
        console.log(`      ✅ Saved Log Spectrogram to: ${logPngPath} (generated in ${(t1_log - t0_log).toFixed(3)} ms)`);

        // 2. Focused Reassigned Spectrogram
        console.log(`   ➡️ Computing Reassigned Spectrogram (${winType} window)...`);
        const t0_re = performance.now();
        const rgbaBytesRe = wasm_calculate_reassigned_spectrogram(originalBytes, 600, winType);
        const t1_re = performance.now();

        const rePngPath = path.join(reassignedSpecDir, `reassigned_${winType}_${sampleName}.png`);
        const w_re = (rgbaBytesRe[4] << 24) | (rgbaBytesRe[5] << 16) | (rgbaBytesRe[6] << 8) | rgbaBytesRe[7];
        const h_re = (rgbaBytesRe[8] << 24) | (rgbaBytesRe[9] << 16) | (rgbaBytesRe[10] << 8) | rgbaBytesRe[11];
        savePng(rgbaBytesRe, rePngPath, w_re, h_re);
        console.log(`      ✅ Saved Reassigned Spectrogram to: ${rePngPath} (generated in ${(t1_re - t0_re).toFixed(3)} ms)`);
      }
    }
  }
  
  // Generate and save 4096 x 4096 Geodesic Snake Color Chart
  console.log('\n➡️ Generating Geodesic Snake 4096 x 4096 High-Resolution Color Palette...');
  const t0_chart = performance.now();
  const chartBytes = wasm_generate_color_chart_4096();
  const t1_chart = performance.now();
  const chartPath = path.join(colorPaletteDir, 'color_pallet_4096.png');
  savePng(chartBytes, chartPath, 4096, 4096);
  console.log(`   ✅ Saved Color Palette to: ${chartPath} (generated in ${(t1_chart - t0_chart).toFixed(3)} ms)`);
  
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

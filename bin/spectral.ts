#!/usr/bin/env npx tsx
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { PNG } from 'pngjs';

// Resolve directory name in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Portably resolve paths inside the workspace
const PROJECT_ROOT = path.resolve(__dirname, '..');
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');
const WASM_BINARY_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm');

// Help Menu template
function printHelp(): void {
  console.log(`
Spectral CLI - 2D Wavelet CDF 5/3 Lossless Converter
====================================================
Convert audio files to lossless PNG images and vice versa using the Cohen-Daubechies-Feauveau 5/3 integer wavelet transform.

Usage:
  npx spectral <input_file> [output_file]
  npm run spectral <input_file> [output_file]

Arguments:
  <input_file>   Path to the input file. Can be an audio file (e.g., .wav) or a PNG image.
  [output_file]  Path to save the converted output. (Optional)
                 If omitted, saves to the same folder with the corresponding extension (.png or .wav).

Options:
  -h, --help     Show this help menu.

Automatic Detection:
  - Any file starting with PNG magic bytes (or with .png extension) is treated as an image and converted to a reconstructed .wav audio.
  - Other files (audio formats like .wav) are treated as audio and converted to a lossless PNG.
`);
}

// Helper to check if file is PNG based on signature or extension
function isPngFile(filePath: string, fileBytes: Uint8Array): boolean {
  // Check PNG Magic Bytes: 89 50 4E 47 0D 0A 1A 0A
  if (fileBytes.length >= 8) {
    if (
      fileBytes[0] === 0x89 &&
      fileBytes[1] === 0x50 &&
      fileBytes[2] === 0x4e &&
      fileBytes[3] === 0x47 &&
      fileBytes[4] === 0x0d &&
      fileBytes[5] === 0x0a &&
      fileBytes[6] === 0x1a &&
      fileBytes[7] === 0x0a
    ) {
      return true;
    }
  }
  return filePath.toLowerCase().endsWith('.png');
}

// Helper to save RGBA bytes buffer as a physical PNG image
function savePng(rgbaBytes: Uint8Array, outputPath: string): { width: number; height: number } {
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
  return { width, height };
}

// Helper to read RGBA bytes from a physical PNG image
function readPng(inputPath: string): { rgbaBytes: Uint8Array; width: number; height: number } {
  const fileBuffer = fs.readFileSync(inputPath);
  const png = PNG.sync.read(fileBuffer);
  return {
    rgbaBytes: new Uint8Array(png.data),
    width: png.width,
    height: png.height
  };
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);

  // Help argument checks
  if (args.length === 0 || args.includes('-h') || args.includes('--help')) {
    printHelp();
    process.exit(0);
  }

  const inputPath = path.resolve(args[0]);
  if (!fs.existsSync(inputPath)) {
    console.error(`❌ Error: Input file not found: ${inputPath}`);
    process.exit(1);
  }

  // Load and initialize WASM
  let encode_wavelet: any;
  let decode_wavelet: any;
  try {
    const { initSync, encode_wavelet: enc, decode_wavelet: dec } = await import(WASM_JS_PATH) as any;
    const wasmBytes = fs.readFileSync(WASM_BINARY_PATH);
    initSync({ module: wasmBytes });
    encode_wavelet = enc;
    decode_wavelet = dec;
  } catch (err: any) {
    console.error(`❌ Error: Failed to initialize WebAssembly engine: ${err.message || err}`);
    process.exit(1);
  }

  // Read file bytes
  let fileBytes: Buffer;
  try {
    fileBytes = fs.readFileSync(inputPath);
  } catch (err: any) {
    console.error(`❌ Error: Failed to read input file: ${err.message || err}`);
    process.exit(1);
  }
  const fileUint8 = new Uint8Array(fileBytes);

  const isImg = isPngFile(inputPath, fileUint8);

  // Determine output path if not specified
  let outputPath = args[1] ? path.resolve(args[1]) : '';
  if (!outputPath) {
    const dir = path.dirname(inputPath);
    const ext = path.extname(inputPath);
    const baseName = path.basename(inputPath, ext);
    if (isImg) {
      outputPath = path.join(dir, `${baseName}.wav`);
    } else {
      outputPath = path.join(dir, `${baseName}.png`);
    }
  }

  console.log(`🔷 Spectral CLI - Running CDF 5/3 Wavelet...`);
  console.log(`   Input File  : ${inputPath}`);
  console.log(`   Input Size  : ${fileUint8.length.toLocaleString()} bytes`);
  console.log(`   Detected    : ${isImg ? 'PNG Lossless Image' : 'Audio File'}`);
  console.log(`   Output File : ${outputPath}`);

  if (isImg) {
    // IMAGE -> AUDIO (Inverse Wavelet)
    try {
      console.log(`   Extracting wavelet coefficients from PNG...`);
      const { rgbaBytes, width, height } = readPng(inputPath);
      console.log(`   PNG Dimensions: ${width} x ${height} px (${(width * height).toLocaleString()} pixels)`);

      console.log(`   Performing inverse 2D Wavelet CDF 5/3...`);
      const start = performance.now();
      const decodedAudioBytes = decode_wavelet(rgbaBytes) as Uint8Array;
      const duration = performance.now() - start;

      fs.writeFileSync(outputPath, Buffer.from(decodedAudioBytes));
      console.log(`\n🎉 Success! Reconstructed audio saved to: ${outputPath}`);
      console.log(`   Reconstructed size : ${decodedAudioBytes.length.toLocaleString()} bytes`);
      console.log(`   Inverse transform  : ${duration.toFixed(2)} ms`);
    } catch (err: any) {
      console.error(`\n❌ Error during inverse transformation: ${err.message || err}`);
      process.exit(1);
    }
  } else {
    // AUDIO -> IMAGE (Forward Wavelet)
    try {
      console.log(`   Performing forward 2D Wavelet CDF 5/3...`);
      const start = performance.now();
      const encodedRGBA = encode_wavelet(fileUint8) as Uint8Array;
      const duration = performance.now() - start;

      console.log(`   Saving wavelet image to PNG...`);
      const { width, height } = savePng(encodedRGBA, outputPath);
      
      console.log(`\n🎉 Success! Wavelet image saved to: ${outputPath}`);
      console.log(`   Resolution         : ${width} x ${height} px (${(width * height).toLocaleString()} pixels)`);
      console.log(`   Forward transform  : ${duration.toFixed(2)} ms`);
    } catch (err: any) {
      console.error(`\n❌ Error during forward transformation: ${err.message || err}`);
      process.exit(1);
    }
  }
}

main().catch((err) => {
  console.error(`❌ Unexpected CLI error:`, err);
  process.exit(1);
});

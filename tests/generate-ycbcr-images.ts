import * as fs from 'fs';
import * as path from 'path';
import { PNG } from 'pngjs';

const PROJECT_ROOT = '/home/racoci/Projects/audio2image';
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');

// Dynamically import WASM
const { 
  initSync, 
  wasm_generate_complex_spectrogram 
} = await import(WASM_JS_PATH) as any;

// Initialize WASM
const wasmBytes = fs.readFileSync(path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm'));
initSync({ module: wasmBytes });

const logBaseFactor = 15.0 / 254.0;
const b = Math.pow(2.0, logBaseFactor);

// Mathematical decoder mapping complex coefficient z = re + i * im to YCbCr Magnitude-Phase space (254-step log base 2^(15/254))
function decibelLuminance(re: number, im: number): { r_u: number, g_u: number, b_u: number } {
  const abs_z = Math.sqrt(re * re + im * im);
  if (abs_z < 1e-12) {
    return { r_u: 0, g_u: 0, b_u: 0 }; // Reserved Y = 0 for absolute silence!
  }
  
  // Normalize to [2^-15, 1.0] range based on maximum 16-bit signed PCM amplitude
  const abs_z_norm = abs_z / 32768.0;
  
  // Y = floor(log_b(|z_norm|)) + 255
  let Y = Math.floor(Math.log2(abs_z_norm) / logBaseFactor) + 255;
  Y = Math.max(1, Math.min(255, Y));
  
  // Recalculated estimate of Abs(z): A_z = b^(Y - 255)
  const A_z = Math.pow(2.0, (Y - 255) * logBaseFactor);
  
  const re_norm = re / 32768.0;
  const im_norm = im / 32768.0;
  
  // Denominator to scale the residual vector w to [-1.0, 1.0] perfectly
  const denom = A_z * (b - 1.0);
  const w_re = (re_norm - A_z * (re_norm / abs_z_norm)) / (denom > 1e-15 ? denom : 1e-15);
  const w_im = (im_norm - A_z * (im_norm / abs_z_norm)) / (denom > 1e-15 ? denom : 1e-15);
  
  // Cr = -Re(w), Cb = Im(w)
  const Cr = -w_re;
  const Cb = w_im;
  
  // Map chrominance offsets Cb, Cr to [16, 240] centered around 128
  const cb_byte = Math.max(16.0, Math.min(240.0, Cb * 112.0 + 128.0));
  const cr_byte = Math.max(16.0, Math.min(240.0, Cr * 112.0 + 128.0));
  
  // Convert YCbCr BT.601 to RGB color space, using the actual Y in [1, 255] as luminance
  const r_val = Y + 1.402 * (cr_byte - 128.0);
  const g_val = Y - 0.344136 * (cb_byte - 128.0) - 0.714136 * (cr_byte - 128.0);
  const b_val = Y + 1.772 * (cb_byte - 128.0);
  
  return {
    r_u: Math.max(0, Math.min(255, Math.round(r_val))),
    g_u: Math.max(0, Math.min(255, Math.round(g_val))),
    b_u: Math.max(0, Math.min(255, Math.round(b_val)))
  };
}

function generateYCbCrPng(wavPath: string, destPath: string, title: string) {
  if (!fs.existsSync(wavPath)) {
    console.error(`WAV file not found: ${wavPath}`);
    return;
  }
  
  const originalBytes = new Uint8Array(fs.readFileSync(wavPath));
  console.log(`➡️  Generating YCbCr Complex Spectrogram for ${path.basename(wavPath)}...`);
  const t0 = performance.now();
  
  const height = 600;
  const complex_grid = wasm_generate_complex_spectrogram(originalBytes, height, 'hann') as Float32Array;
  const width = 800; // Fixed high-fidelity landscape width
  
  const png = new PNG({ width, height });
  const buf = Buffer.alloc(width * height * 4);
  
  for (let r = 0; r < height; r++) {
    for (let c = 0; c < width; c++) {
      // Invert vertically so low frequencies are at the bottom
      const grid_y = height - 1 - r;
      const complex_idx = (grid_y * width + c) * 2;
      const re = complex_grid[complex_idx];
      const im = complex_grid[complex_idx + 1];
      
      const { r_u, g_u, b_u } = decibelLuminance(re, im);
      
      const out_idx = (r * width + c) * 4;
      buf[out_idx]     = r_u;
      buf[out_idx + 1] = g_u;
      buf[out_idx + 2] = b_u;
      buf[out_idx + 3] = 255;
    }
  }
  
  png.data = buf;
  fs.writeFileSync(destPath, PNG.sync.write(png));
  const t1 = performance.now();
  console.log(`   ✅ Saved YCbCr PNG to: ${destPath} (generated in ${(t1 - t0).toFixed(3)} ms)\n`);
}

const destDir = path.join(PROJECT_ROOT, 'tests', 'test-outputs', 'stft_cqt', 'logarithmic_spectrogram');
fs.mkdirSync(destDir, { recursive: true });

const voiceWav = path.join(PROJECT_ROOT, 'public', 'voice.wav');
const voiceDest = path.join(destDir, 'complex_ycbcr_voice.png');
generateYCbCrPng(voiceWav, voiceDest, 'YCbCr Magnitude-Phase Spectrogram (voice.wav)');

const synthWav = path.join(PROJECT_ROOT, 'tests', 'temp-samples', 'synth.wav');
const synthDest = path.join(destDir, 'complex_ycbcr_synth.png');
generateYCbCrPng(synthWav, synthDest, 'YCbCr Magnitude-Phase Spectrogram (synth.wav)');

import * as fs from 'fs';
import * as path from 'path';

const PROJECT_ROOT = '/home/racoci/Projects/audio2image';
const WASM_JS_PATH = path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm.js');

try {
  const { initSync, wasm_calculate_complex_reassigned_spectrogram } = await import(WASM_JS_PATH) as any;
  const wasmBytes = fs.readFileSync(path.join(PROJECT_ROOT, 'src', 'wasm', 'core_wasm_bg.wasm'));
  initSync({ module: wasmBytes });
  console.log('✅ WebAssembly initialized successfully inside test!');

  const wavPath = path.join(PROJECT_ROOT, 'public', 'voice.wav');
  const bytes = new Uint8Array(fs.readFileSync(wavPath));
  console.log(`➡️ Processing ${bytes.length} bytes from voice.wav...`);

  const complexGrid = wasm_calculate_complex_reassigned_spectrogram(bytes, 1024, 'hann');
  console.log(`✅ Success! Generated complex grid of size: ${complexGrid.length} floats.`);
  console.log(`   Dimensions: width = ${(complexGrid.length / 2) / 1024}, height = 1024`);
} catch (e: any) {
  console.error('❌ Failed to run complex reassigned spectrogram calculation:', e);
}

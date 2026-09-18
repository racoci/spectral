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
  
  // Find ranges and non-zero counts
  let maxAbs = 0;
  let minAbs = Infinity;
  let nonZeroCount = 0;
  let sampleVals: number[] = [];

  for (let i = 0; i < complexGrid.length / 2; i++) {
    const re = complexGrid[i * 2];
    const im = complexGrid[i * 2 + 1];
    const abs = Math.sqrt(re * re + im * im);
    if (abs > 1e-5) {
      nonZeroCount++;
      if (abs > maxAbs) maxAbs = abs;
      if (abs < minAbs) minAbs = abs;
      if (sampleVals.length < 10) {
        sampleVals.push(abs);
      }
    }
  }

  console.log('--------------------------------------------------');
  console.log(`📊 Active (non-zero) bins:  ${nonZeroCount} / ${complexGrid.length / 2} (${(nonZeroCount / (complexGrid.length / 2) * 100).toFixed(2)}%)`);
  console.log(`📈 Maximum amplitude (|z|): ${maxAbs}`);
  console.log(`📉 Minimum active amp (|z|): ${minAbs}`);
  console.log(`🧪 Sample active values:    [${sampleVals.map(v => v.toFixed(3)).join(', ')}]`);
  console.log('--------------------------------------------------');
} catch (e: any) {
  console.error('❌ Failed to run complex reassigned spectrogram calculation:', e);
}

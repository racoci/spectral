import * as fs from 'fs';
import * as path from 'path';
import { PNG } from 'pngjs';

const PROJECT_ROOT = '/home/racoci/Projects/audio2image';

// Close match terminal colors
function renderAnsiSpectrogram(pngPath: string, title: string) {
  if (!fs.existsSync(pngPath)) {
    console.error(`File not found: ${pngPath}`);
    return;
  }

  const fileBuffer = fs.readFileSync(pngPath);
  const png = PNG.sync.read(fileBuffer);
  
  const srcW = png.width;
  const srcH = png.height;
  
  // Terminal size target: 80 columns, 24 rows
  const termW = 80;
  const termH = 30;
  
  console.log('\n' + '='.repeat(84));
  console.log(` 🌌 SPECTRAL TERMINAL HIGH-RESOLUTION SPECTROGRAM: ${title}`);
  console.log(`    File: ${path.basename(pngPath)} (${srcW}x${srcH} pixels)`);
  console.log('='.repeat(84) + '\n');
  
  let nonBlackCount = 0;
  let maxR = 0;
  let maxG = 0;
  let maxB = 0;
  let sumR = 0;
  
  for (let i = 0; i < png.data.length / 4; i++) {
    const idx = i * 4;
    const r = png.data[idx];
    const g = png.data[idx + 1];
    const b = png.data[idx + 2];
    sumR += r;
    if (r > 0 || g > 0 || b > 0) {
      nonBlackCount++;
    }
    if (r > maxR) maxR = r;
    if (g > maxG) maxG = g;
    if (b > maxB) maxB = b;
  }
  
  console.log(`📊 Image Stats:`);
  console.log(`   Total pixels: ${png.data.length / 4}`);
  console.log(`   Non-black pixels: ${nonBlackCount} (${(nonBlackCount / (png.data.length / 4) * 100).toFixed(2)}%)`);
  console.log(`   Max RGB: (${maxR}, ${maxG}, ${maxB})`);
  console.log(`   Average Red brightness: ${(sumR / (png.data.length / 4)).toFixed(4)}\n`);

  console.log('   20 kHz ┼' + '─'.repeat(termW));

  // Loop through rows from top (high frequency) to bottom (low frequency)
  const shadingChars = ' .:-=+*#%@';
  
  for (let ty = 0; ty < termH; ty++) {
    // Map terminal row to PNG row (top of image is high frequency, but we print top-to-bottom)
    const pngY = Math.floor((ty / termH) * srcH);
    
    let line = '          │'; // Axis padding
    
    for (let tx = 0; tx < termW; tx++) {
      const pngX = Math.floor((tx / termW) * srcW);
      
      const idx = (pngY * srcW + pngX) * 4;
      const r = png.data[idx];
      const g = png.data[idx + 1];
      const b = png.data[idx + 2];
      
      // Compute perceived brightness (0.0 to 1.0)
      const brightness = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
      const charIdx = Math.floor(brightness * (shadingChars.length - 1));
      const char = shadingChars[charIdx];
      
      // Colorize foreground based on energy levels:
      // - High (Vibrant Yellow/Red): Red (> 0.7) or Yellow (> 0.4)
      // - Medium (Green): Green (> 0.15)
      // - Low: Blue/Dim
      let ansiColor = '\x1b[90m'; // Dim Gray
      if (brightness > 0.6) {
        ansiColor = '\x1b[1;31m'; // Bold Red
      } else if (brightness > 0.35) {
        ansiColor = '\x1b[1;33m'; // Bold Yellow
      } else if (brightness > 0.12) {
        ansiColor = '\x1b[32m'; // Green
      } else if (brightness > 0.03) {
        ansiColor = '\x1b[34m'; // Blue
      }
      
      line += `${ansiColor}${char}\x1b[0m`;
    }
    
    // Add frequency axis labels at specific intervals
    if (ty === Math.floor(termH * 0.25)) {
      line += ' ┼ 10 kHz';
    } else if (ty === Math.floor(termH * 0.5)) {
      line += ' ┼  1 kHz';
    } else if (ty === Math.floor(termH * 0.75)) {
      line += ' ┼ 100 Hz';
    } else {
      line += ' │';
    }
    
    console.log(line);
  }
  
  console.log('    20 Hz ┼' + '─'.repeat(termW));
  console.log('          │' + ' '.repeat(termW / 2 - 3) + 'TIME (seconds) ──▶' + ' '.repeat(termW / 2 - 12) + '│');
  console.log('\x1b[1;33m   [LEGEND]: \x1b[0;41m  Low Energy (Black)  \x1b[0;42m  Medium Energy (Green)  \x1b[0;43m  High Energy (Vibrant Amber/Yellow)  \x1b[0m\n');
}

// Render both the smooth Logarithmic and the ultra-sharp Reassigned Spectrograms side-by-side!
const logPng = path.join(PROJECT_ROOT, 'tests', 'test-outputs', 'stft_cqt', 'logarithmic_spectrogram', 'log_hann_voice.png');
const rePng = path.join(PROJECT_ROOT, 'tests', 'test-outputs', 'stft_cqt', 'reassigned_spectrogram', 'reassigned_hann_voice.png');

renderAnsiSpectrogram(logPng, 'SMOOTH LOGARITHMIC SPECTROGRAM (AUDACITY REFERENCE)');
renderAnsiSpectrogram(rePng, 'AUGER-FLANDRIN REASSIGNED SPECTROGRAM (INFINITE SHARPNESS)');

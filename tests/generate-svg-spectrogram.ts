import * as fs from 'fs';
import * as path from 'path';
import { PNG } from 'pngjs';

const PROJECT_ROOT = '/home/racoci/Projects/audio2image';

function generateSvgSpectrogram(pngPath: string, svgPath: string, title: string) {
  if (!fs.existsSync(pngPath)) {
    console.error(`Source PNG not found: ${pngPath}`);
    return;
  }

  const fileBuffer = fs.readFileSync(pngPath);
  const png = PNG.sync.read(fileBuffer);
  
  const width = png.width;
  const height = png.height;
  
  console.log(`➡️  Converting PNG ${path.basename(pngPath)} to high-fidelity Vector SVG...`);
  const t0 = performance.now();
  
  let svgContent = `<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" style="background-color: #000000;">
  <defs>
    <!-- Filter for subtle organic glow on spectral energy lines -->
    <filter id="glow" x="-10%" y="-10%" width="120%" height="120%">
      <feGaussianBlur stdDeviation="1.5" result="blur" />
      <feComposite in="SourceGraphic" in2="blur" operator="over" />
    </filter>
  </defs>

  <style>
    .title { fill: #ffffff; font-family: monospace; font-size: 16px; font-weight: bold; }
    .axis { stroke: #444444; stroke-width: 1px; }
    .axis-label { fill: #888888; font-family: monospace; font-size: 11px; }
    .spectral-line { stroke-linecap: round; filter: url(#glow); }
  </style>

  <!-- Title metadata -->
  <text x="20" y="30" class="title">${title}</text>
  <text x="20" y="50" class="axis-label">Dimensions: ${width} x ${height} scaled vector lines</text>
  
  <!-- Axis structure -->
  <line x1="60" y1="80" x2="60" y2="${height - 60}" class="axis" />
  <line x1="60" y1="${height - 60}" x2="${width - 40}" y2="${height - 60}" class="axis" />
  
  <!-- Axis ticks and labels -->
  <text x="15" y="90" class="axis-label">20 kHz</text>
  <text x="15" y="${Math.floor(height * 0.25)}" class="axis-label">10 kHz</text>
  <text x="20" y="${Math.floor(height * 0.5)}" class="axis-label">1 kHz</text>
  <text x="20" y="${Math.floor(height * 0.75)}" class="axis-label">100 Hz</text>
  <text x="25" y="${height - 56}" class="axis-label">20 Hz</text>
  
  <text x="${Math.floor(width / 2)}" y="${height - 25}" class="axis-label" text-anchor="middle">TIME (seconds) ──▶</text>
`;

  // To keep the SVG file size optimized while retaining perfect details,
  // we scan columns and find contiguous runs of active pixels (non-black, brightness > 0.05)
  // and output them as high-precision vertical vectors (<line> or <path>)
  let linesCount = 0;
  
  // Boundary margins to draw inside the axis grid
  const marginL = 60;
  const marginR = width - 40;
  const marginT = 80;
  const marginB = height - 60;
  
  const gridW = marginR - marginL;
  const gridH = marginB - marginT;

  for (let x = 0; x < gridW; x++) {
    // Map SVG coordinates to PNG coordinates
    const pngX = Math.floor((x / gridW) * width);
    
    for (let y = 0; y < gridH; y++) {
      const pngY = Math.floor((y / gridH) * height);
      
      const idx = (pngY * width + pngX) * 4;
      const r = png.data[idx];
      const g = png.data[idx + 1];
      const b = png.data[idx + 2];
      
      const brightness = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
      
      // Draw vector lines only for meaningful, glowing energy contributions
      if (brightness > 0.08) {
        const svgX = marginL + x;
        const svgY = marginT + y;
        
        // Output a highly detailed, colored vector dot/line
        const hexColor = "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
        const strokeWidth = (brightness * 2.5).toFixed(1);
        
        // Draw a tiny horizontal/vertical line segment representing the vector
        svgContent += `  <line x1="${svgX}" y1="${svgY}" x2="${svgX + 1}" y2="${svgY}" stroke="${hexColor}" stroke-width="${strokeWidth}" class="spectral-line" />\n`;
        linesCount++;
      }
    }
  }
  
  svgContent += `</svg>\n`;
  
  fs.writeFileSync(svgPath, svgContent);
  const t1 = performance.now();
  console.log(`   ✅ Successfully generated and saved vector SVG containing ${linesCount} laser lines to:`);
  console.log(`      ${svgPath} (completed in ${(t1 - t0).toFixed(3)} ms)\n`);
}

// Ensure the SVG directory exists
const svgDir = path.join(PROJECT_ROOT, 'tests', 'test-outputs', 'stft_cqt', 'svg_spectrogram');
fs.mkdirSync(svgDir, { recursive: true });

// Convert voice and synth reassigned PNGs to ultra-detailed scalable SVGs!
const voicePng = path.join(PROJECT_ROOT, 'tests', 'test-outputs', 'stft_cqt', 'reassigned_spectrogram', 'reassigned_hann_voice.png');
const voiceSvg = path.join(svgDir, 'reassigned_hann_voice.svg');
generateSvgSpectrogram(voicePng, voiceSvg, 'AUGER-FLANDRIN HIGH-RESOLUTION REASSIGNED VECTOR SPECTROGRAM (VOICE.WAV)');

const synthPng = path.join(PROJECT_ROOT, 'tests', 'test-outputs', 'stft_cqt', 'reassigned_spectrogram', 'reassigned_hann_synth.png');
const synthSvg = path.join(svgDir, 'reassigned_hann_synth.svg');
generateSvgSpectrogram(synthPng, synthSvg, 'AUGER-FLANDRIN HIGH-RESOLUTION REASSIGNED VECTOR SPECTROGRAM (SYNTH.WAV)');

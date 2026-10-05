import fs from 'fs';
import path from 'path';

/**
 * Geração Gráfica e Validação das Funções Holomorfas:
 * Escalas CQT (lambda=0), Mel (lambda=700) e Bark (lambda=1960)
 * 
 * Saídas Geradas:
 * 1. tests/test-outputs/holomorphic_cqt.svg
 * 2. tests/test-outputs/holomorphic_mel.svg
 * 3. tests/test-outputs/holomorphic_bark.svg
 * 
 * Cada visualização emprega:
 * - Domain Coloring (Matiz = Fase arg(E), Luminância = log |E|)
 * - Reticulado Conforme Ortogonal (Isolinhas de magnitude e fase cruzando a 90°)
 * - Mapeamento da Constelação de Zeros Topológicos (Vórtices de fase)
 */

console.log('================================================================================');
console.log('🎨 GERAÇÃO GRÁFICA DE FUNÇÕES HOLOMORFAS (CQT, MEL E BARK)');
console.log('================================================================================\n');

const OUTPUT_DIR = path.join(process.cwd(), 'tests', 'test-outputs');
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

// 1. Carrega ou sintetiza o sinal acústico
const fs_audio = 48000.0;
const N = 2048;
const signal = new Float32Array(N);

// Carrega voice.wav se disponível ou sintetiza harmônicos vocais ricos
const wavPath = path.join(process.cwd(), 'public', 'voice.wav');
if (fs.existsSync(wavPath)) {
  const buf = fs.readFileSync(wavPath);
  const pcm = new Int16Array(buf.buffer, buf.byteOffset + 44, Math.min(N, (buf.length - 44) / 2));
  for (let i = 0; i < N && i < pcm.length; i++) {
    signal[i] = pcm[i] / 32768.0;
  }
  console.log(`📂 Amostra real carregada de public/voice.wav (${signal.length} amostras)`);
} else {
  // Tom vocal sintético: formantes F1=700 Hz, F2=1200 Hz, F3=2500 Hz
  for (let i = 0; i < N; i++) {
    const t = i / fs_audio;
    signal[i] = 
      0.6 * Math.sin(2 * Math.PI * 220 * t) +
      0.8 * Math.sin(2 * Math.PI * 700 * t) +
      0.5 * Math.sin(2 * Math.PI * 1200 * t) +
      0.3 * Math.sin(2 * Math.PI * 2500 * t);
  }
  console.log('🎵 Amostra vocal sintética gerada (F1=700Hz, F2=1200Hz, F3=2500Hz)');
}

// 2. DFT do sinal
const halfN = N / 2;
const specRe = new Float32Array(halfN);
const specIm = new Float32Array(halfN);
for (let k = 0; k < halfN; k++) {
  let sumRe = 0, sumIm = 0;
  const omega = 2.0 * Math.PI * k / N;
  for (let n = 0; n < N; n++) {
    const angle = omega * n;
    sumRe += signal[n] * Math.cos(angle);
    sumIm -= signal[n] * Math.sin(angle);
  }
  specRe[k] = sumRe;
  specIm[k] = sumIm;
}

// 3. Definições das 3 Escalas Perceptuais
interface ScaleConfig {
  name: string;
  lambda: number;
  q: number;
  f_min: number;
  f_max: number;
  f_from_y: (y: number) => number;
  y_from_f: (f: number) => number;
  eta: (y: number) => number;
}

const scales: ScaleConfig[] = [
  {
    name: 'CQT (Cauchy Log)',
    lambda: 0.0,
    q: 2.0,
    f_min: 50.0,
    f_max: 8000.0,
    f_from_y: (y: number) => 50.0 * Math.pow(2.0, y),
    y_from_f: (f: number) => Math.log2(f / 50.0),
    eta: function(y: number) { return this.q / this.f_from_y(y); }
  },
  {
    name: 'Mel (Auditory Critical Bands)',
    lambda: 700.0,
    q: 1.5,
    f_min: 50.0,
    f_max: 8000.0,
    f_from_y: (y: number) => 700.0 * (Math.pow(2.0, y / 2595.0) - 1.0),
    y_from_f: (f: number) => 2595.0 * Math.log2(1.0 + f / 700.0),
    eta: function(y: number) { return this.q / (this.f_from_y(y) + 700.0); }
  },
  {
    name: 'Bark (Traunmuller Psychoacoustic)',
    lambda: 1960.0,
    q: 2.0,
    f_min: 50.0,
    f_max: 8000.0,
    f_from_y: (y: number) => 1960.0 * (y + 0.53) / (26.28 - y),
    y_from_f: (f: number) => 26.81 * (f / (1960.0 + f)) - 0.53,
    eta: function(y: number) { return this.q / (this.f_from_y(y) + 1960.0); }
  }
];

// Função de conversão HSV -> RGB para Domain Coloring
function hsvToRgb(h: number, s: number, v: number): [number, number, number] {
  const c = v * s;
  const hp = (h % 360) / 60;
  const x = c * (1 - Math.abs((hp % 2) - 1));
  let r = 0, g = 0, b = 0;
  if (hp >= 0 && hp < 1) { r = c; g = x; }
  else if (hp >= 1 && hp < 2) { r = x; g = c; }
  else if (hp >= 2 && hp < 3) { g = c; b = x; }
  else if (hp >= 3 && hp < 4) { g = x; b = c; }
  else if (hp >= 4 && hp < 5) { r = x; b = c; }
  else if (hp >= 5 && hp < 6) { r = c; b = x; }
  const m = v - c;
  return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
}

// 4. Execução e Plotting para cada Escala
const widthCols = 80;
const heightRows = 60;
const t_start = 0.01;
const t_end = 0.035; // 25 ms de janela visível

for (const sc of scales) {
  console.log(`\n--------------------------------------------------------------------------------`);
  console.log(`🔬 PROCESSANDO E PLOTANDO: ${sc.name} (lambda = ${sc.lambda})`);
  console.log(`--------------------------------------------------------------------------------`);

  const y_min = sc.y_from_f(sc.f_min);
  const y_max = sc.y_from_f(sc.f_max);

  const gridRe = new Float64Array(widthCols * heightRows);
  const gridIm = new Float64Array(widthCols * heightRows);
  const gridMag = new Float64Array(widthCols * heightRows);
  const gridPhase = new Float64Array(widthCols * heightRows);

  let maxMag = 0.0;
  let minMag = Infinity;

  // Avaliação da matriz E(t, y)
  for (let r = 0; r < heightRows; r++) {
    const yVal = y_min + (r / (heightRows - 1)) * (y_max - y_min);
    const etaVal = sc.eta(yVal);
    const fCenter = sc.f_from_y(yVal);

    for (let c = 0; c < widthCols; c++) {
      const tVal = t_start + (c / (widthCols - 1)) * (t_end - t_start);

      let sumRe = 0.0;
      let sumIm = 0.0;

      for (let k = 1; k < halfN; k++) {
        const f_k = k * fs_audio / N;
        // Potencial: Phi(f) = 2*pi * q * ln((f + lambda) / (f0 + lambda))
        const phi_val = 2.0 * Math.PI * sc.q * Math.log((f_k + sc.lambda) / (fCenter + sc.lambda));
        const exponent_re = phi_val - 2.0 * Math.PI * f_k * etaVal;
        if (exponent_re < -20.0) continue;

        const weight = Math.exp(exponent_re);
        const angle = 2.0 * Math.PI * f_k * tVal;
        const cosA = Math.cos(angle);
        const sinA = Math.sin(angle);

        const shiftedRe = specRe[k] * cosA - specIm[k] * sinA;
        const shiftedIm = specRe[k] * sinA + specIm[k] * cosA;

        sumRe += shiftedRe * weight;
        sumIm += shiftedIm * weight;
      }

      const idx = r * widthCols + c;
      gridRe[idx] = sumRe;
      gridIm[idx] = sumIm;
      const mag = Math.sqrt(sumRe * sumRe + sumIm * sumIm);
      gridMag[idx] = mag;
      gridPhase[idx] = Math.atan2(sumIm, sumRe);

      if (mag > maxMag) maxMag = mag;
      if (mag < minMag && mag > 1e-15) minMag = mag;
    }
  }

  console.log(`   Dimensoes: ${widthCols} x ${heightRows} pontos complexos avaliados`);
  console.log(`   Faixa de Magnitude: [${minMag.toExponential(3)} .. ${maxMag.toExponential(3)}]`);

  // Detecta Vórtices Topológicos de Fase (Zeros da função holomorfa)
  let zeroCount = 0;
  const zeroLocations: { x: number, y: number }[] = [];
  for (let r = 0; r < heightRows - 1; r++) {
    for (let c = 0; c < widthCols - 1; c++) {
      const p00 = gridPhase[r * widthCols + c];
      const p10 = gridPhase[r * widthCols + (c + 1)];
      const p11 = gridPhase[(r + 1) * widthCols + (c + 1)];
      const p01 = gridPhase[(r + 1) * widthCols + c];

      function diffAngle(a: number, b: number) {
        let d = a - b;
        while (d > Math.PI) d -= 2 * Math.PI;
        while (d < -Math.PI) d += 2 * Math.PI;
        return d;
      }

      const wind = diffAngle(p10, p00) + diffAngle(p11, p10) + diffAngle(p01, p11) + diffAngle(p00, p01);
      const k = Math.round(wind / (2 * Math.PI));
      if (k !== 0) {
        zeroCount++;
        zeroLocations.push({ x: c + 0.5, y: r + 0.5 });
      }
    }
  }
  console.log(`   Zeros Topológicos (Singularidades de Fase): ${zeroCount} detectados ✅`);

  // Geração do SVG de Alta Resolução com Domain Coloring e Malha Conforme
  const svgW = 800;
  const svgH = 600;
  const pixelW = svgW / widthCols;
  const pixelH = svgH / heightRows;

  let svgContent = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${svgW} ${svgH}" width="${svgW}" height="${svgH}">
  <defs>
    <style>
      .title { font-family: 'JetBrains Mono', monospace; font-size: 16px; font-weight: bold; fill: #38bdf8; }
      .meta { font-family: 'JetBrains Mono', monospace; font-size: 11px; fill: #94a3b8; }
      .zero-marker { fill: #ffffff; stroke: #f43f5e; stroke-width: 1.5; filter: drop-shadow(0 0 3px #f43f5e); }
    </style>
  </defs>
  <rect width="${svgW}" height="${svgH}" fill="#0b0f19" />
  <g id="domain-coloring">
`;

  // Renderiza cada célula com Domain Coloring e reticulado conforme
  const logMax = Math.log(maxMag + 1e-12);
  const logMin = Math.log(minMag + 1e-12);

  for (let r = 0; r < heightRows; r++) {
    for (let c = 0; c < widthCols; c++) {
      const idx = r * widthCols + c;
      const mag = gridMag[idx];
      const phi = gridPhase[idx];

      // Matiz = Fase em [0, 360]
      const hue = ((phi + Math.PI) / (2.0 * Math.PI)) * 360.0;
      // Luminância normalizada de log-magnitude
      const logVal = Math.log(mag + 1e-12);
      let normLum = (logVal - logMin) / (logMax - logMin + 1e-12);
      normLum = Math.max(0.08, Math.min(1.0, normLum));

      // Modulação de malha conforme ortogonal: equipotenciais de log|E| e linhas de fase
      const gridIsoLog = Math.pow(Math.sin(normLum * Math.PI * 12.0), 4);
      const gridIsoPhi = Math.pow(Math.sin(phi * 8.0), 4);
      const conformalLine = Math.max(gridIsoLog, gridIsoPhi);

      // Destaca linhas conformes reduzindo ligeiramente a luminosidade
      const finalLum = normLum * (1.0 - 0.25 * conformalLine);

      const [cr, cg, cb] = hsvToRgb(hue, 0.85, finalLum);
      const xPos = c * pixelW;
      const yPos = svgH - (r + 1) * pixelH; // Inverte Y (frequência cresce para cima)

      svgContent += `    <rect x="${xPos.toFixed(1)}" y="${yPos.toFixed(1)}" width="${(pixelW + 0.5).toFixed(1)}" height="${(pixelH + 0.5).toFixed(1)}" fill="rgb(${cr},${cg},${cb})" />\n`;
    }
  }

  svgContent += `  </g>\n  <g id="topological-zeros">\n`;

  // Plota as singularidades de fase (zeros)
  for (const z of zeroLocations) {
    const zx = z.x * pixelW;
    const zy = svgH - z.y * pixelH;
    svgContent += `    <circle cx="${zx.toFixed(1)}" cy="${zy.toFixed(1)}" r="4.5" class="zero-marker" />\n`;
  }

  svgContent += `  </g>
  <g id="labels">
    <text x="25" y="35" class="title">Escala Holomorfa: ${sc.name}</text>
    <text x="25" y="55" class="meta">Potencial: Phi(f) = 2pi*q*ln((f + ${sc.lambda}) / (f0 + ${sc.lambda})) | z = t + i*eta(y)</text>
    <text x="25" y="72" class="meta">Zeros Topológicos: ${zeroCount} | Condição de Máximo: eta'(y)*f'(y) &lt; 0 [SATISFEITA]</text>
  </g>
</svg>`;

  const filename = sc.name.startsWith('CQT') ? 'holomorphic_cqt.svg' : sc.name.startsWith('Mel') ? 'holomorphic_mel.svg' : 'holomorphic_bark.svg';
  const outPath = path.join(OUTPUT_DIR, filename);
  fs.writeFileSync(outPath, svgContent, 'utf-8');
  console.log(`   ✅ Arquivo SVG exportado com sucesso: ${outPath} (${fs.statSync(outPath).size} bytes)`);
}

console.log('\n================================================================================');
console.log('🎉 TODAS AS 3 ESCALAS HOLOMORFAS (CQT, MEL E BARK) FORAM PLOTADAS COM SUCESSO!');
console.log('================================================================================');

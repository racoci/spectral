import fs from 'fs';
import path from 'path';
import { PNG } from 'pngjs';

/**
 * Validador Independente de Alta Resolução das Funções Holomorfas
 * Escalas: CQT (lambda=0), Mel (lambda=700), Bark (lambda=1960)
 * Esquemas de Cores: YCbCr Magnitude-Fase e Geodesic Snake (24-bit)
 * Resolução: 1024 x 600 px (Alta Resolução no Padrão do Sistema)
 */

console.log('================================================================================');
console.log('📸 VALIDAÇÃO INDEPENDENTE DE ALTA RESOLUÇÃO: CQT, MEL E BARK (1024x600 PNG)');
console.log('================================================================================\n');

const OUTPUT_DIR = path.join(process.cwd(), 'documentation', 'holomorphic_geometry', 'plots');
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

// 1. Carrega o áudio real de voice.wav
const wavPath = path.join(process.cwd(), 'public', 'voice.wav');
if (!fs.existsSync(wavPath)) {
  throw new Error(`Arquivo não encontrado: ${wavPath}`);
}

const wavBuf = fs.readFileSync(wavPath);
const N_SAMPLES = 4096;
const pcm = new Int16Array(wavBuf.buffer, wavBuf.byteOffset + 44, Math.min(N_SAMPLES, (wavBuf.length - 44) / 2));
const signal = new Float32Array(N_SAMPLES);
for (let i = 0; i < N_SAMPLES && i < pcm.length; i++) {
  signal[i] = pcm[i] / 32768.0;
}
console.log(`📂 Amostra carregada: ${pcm.length} amostras de áudio real (voice.wav)\n`);

// 2. DFT precisa para o espectro de frequências positivas
const fs_audio = 48000.0;
const halfN = N_SAMPLES / 2;
const specRe = new Float32Array(halfN);
const specIm = new Float32Array(halfN);
for (let k = 0; k < halfN; k++) {
  let sumRe = 0, sumIm = 0;
  const omega = (2.0 * Math.PI * k) / N_SAMPLES;
  for (let n = 0; n < N_SAMPLES; n++) {
    const angle = omega * n;
    sumRe += signal[n] * Math.cos(angle);
    sumIm -= signal[n] * Math.sin(angle);
  }
  specRe[k] = sumRe;
  specIm[k] = sumIm;
}

// 3. Fórmulas dos Esquemas de Cores do Spectral

// Esquema 1: YCbCr Magnitude-Fase (Log base 2^(15/254) + BT.601)
const logBaseFactor = 15.0 / 254.0;
const b_base = Math.pow(2.0, logBaseFactor);

function decibelLuminanceYCbCr(re: number, im: number): [number, number, number] {
  const abs_z = Math.sqrt(re * re + im * im);
  if (abs_z < 1e-12) {
    return [0, 0, 0];
  }
  const abs_norm = Math.min(1.0, abs_z);
  let Y = Math.floor(Math.log2(abs_norm + 1e-15) / logBaseFactor) + 255;
  Y = Math.max(1, Math.min(255, Y));

  const A_z = Math.pow(2.0, (Y - 255) * logBaseFactor);
  const denom = Math.max(1e-15, A_z * (b_base - 1.0));
  const w_re = (re - A_z * (re / abs_z)) / denom;
  const w_im = (im - A_z * (im / abs_z)) / denom;

  const Cr = -Math.max(-1.0, Math.min(1.0, w_re));
  const Cb = Math.max(-1.0, Math.min(1.0, w_im));

  const cb_byte = Math.max(16.0, Math.min(240.0, Cb * 112.0 + 128.0));
  const cr_byte = Math.max(16.0, Math.min(240.0, Cr * 112.0 + 128.0));

  const r_val = Y + 1.402 * (cr_byte - 128.0);
  const g_val = Y - 0.344136 * (cb_byte - 128.0) - 0.714136 * (cr_byte - 128.0);
  const b_val = Y + 1.772 * (cb_byte - 128.0);

  return [
    Math.max(0, Math.min(255, Math.round(r_val))),
    Math.max(0, Math.min(255, Math.round(g_val))),
    Math.max(0, Math.min(255, Math.round(b_val))),
  ];
}

// Esquema 2: Geodesic Snake (Mapeamento contínuo em cascas 24-bit de Chebyshev)
function geodesicSnakeRGB(magNorm: number): [number, number, number] {
  if (magNorm <= 1e-6) {
    return [0, 0, 0]; // Silêncio absoluto
  }
  const level16 = Math.max(0, Math.min(65535, Math.round(magNorm * 65535.0)));
  const R = (level16 >> 8) & 0xff;
  const G = level16 & 0xff;
  const B = Math.abs(R - G);
  return [R, G, B];
}

// 4. Configuração das 3 Escalas Holomorfas
interface HoloScale {
  id: string;
  name: string;
  lambda: number;
  q: number;
  f_from_y: (y: number) => number;
  y_from_f: (f: number) => number;
  eta: (y: number) => number;
}

const fmin = 80.0;
const fmax = 12000.0;

const scales: HoloScale[] = [
  {
    id: 'cqt',
    name: 'CQT Cauchy Holomorfa (lambda = 0)',
    lambda: 0.0,
    q: 2.0,
    f_from_y: (y: number) => fmin * Math.pow(2.0, y),
    y_from_f: (f: number) => Math.log2(f / fmin),
    eta: function(y: number) { return this.q / this.f_from_y(y); }
  },
  {
    id: 'mel',
    name: 'Mel Holomorfa (lambda = 700)',
    lambda: 700.0,
    q: 1.5,
    f_from_y: (y: number) => 700.0 * (Math.pow(2.0, y / 2595.0) - 1.0),
    y_from_f: (f: number) => 2595.0 * Math.log2(1.0 + f / 700.0),
    eta: function(y: number) { return this.q / (this.f_from_y(y) + 700.0); }
  },
  {
    id: 'bark',
    name: 'Bark Holomorfa (lambda = 1960)',
    lambda: 1960.0,
    q: 2.0,
    f_from_y: (y: number) => 1960.0 * (y + 0.53) / (26.28 - y),
    y_from_f: (f: number) => 26.81 * (f / (1960.0 + f)) - 0.53,
    eta: function(y: number) { return this.q / (this.f_from_y(y) + 1960.0); }
  }
];

// 5. Geração em Alta Resolução: 1024 colunas x 600 linhas
const width = 1024;
const height = 600;
const t_start = 0.01;
const t_end = 0.06; // 50 ms de duração de voz

for (const sc of scales) {
  console.log(`--------------------------------------------------------------------------------`);
  console.log(`🚀 GERANDO MATRIZ DE ALTA RESOLUÇÃO: ${sc.name} (1024 x 600)...`);
  console.log(`--------------------------------------------------------------------------------`);
  const t0 = performance.now();

  const y_min = sc.y_from_f(fmin);
  const y_max = sc.y_from_f(fmax);

  const gridRe = new Float32Array(width * height);
  const gridIm = new Float32Array(width * height);
  const gridMag = new Float32Array(width * height);

  let globalMax = 0.0;

  // Avaliação holomorfa de alta resolução
  for (let r = 0; r < height; r++) {
    const yVal = y_min + (r / (height - 1)) * (y_max - y_min);
    const etaVal = sc.eta(yVal);
    const fCenter = sc.f_from_y(yVal);

    for (let c = 0; c < width; c++) {
      const tVal = t_start + (c / (width - 1)) * (t_end - t_start);

      let sumRe = 0.0;
      let sumIm = 0.0;

      for (let k = 1; k < halfN; k++) {
        const f_k = (k * fs_audio) / N_SAMPLES;
        const phi_val = 2.0 * Math.PI * sc.q * Math.log((f_k + sc.lambda) / (fCenter + sc.lambda));
        const exponent_re = phi_val - 2.0 * Math.PI * f_k * etaVal;
        if (exponent_re < -22.0) continue;

        const weight = Math.exp(exponent_re);
        const angle = 2.0 * Math.PI * f_k * tVal;
        const cosA = Math.cos(angle);
        const sinA = Math.sin(angle);

        const shiftedRe = specRe[k] * cosA - specIm[k] * sinA;
        const shiftedIm = specRe[k] * sinA + specIm[k] * cosA;

        sumRe += shiftedRe * weight;
        sumIm += shiftedIm * weight;
      }

      const idx = r * width + c;
      gridRe[idx] = sumRe;
      gridIm[idx] = sumIm;
      const mag = Math.sqrt(sumRe * sumRe + sumIm * sumIm);
      gridMag[idx] = mag;
      if (mag > globalMax) globalMax = mag;
    }
  }

  const elapsedMs = performance.now() - t0;
  console.log(`   ⏱️  Matriz computada em ${elapsedMs.toFixed(1)} ms | Max Amplitude: ${globalMax.toExponential(3)}`);

  // GERAÇÃO PNG 1: YCbCr Magnitude-Fase
  const pngYCbCr = new PNG({ width, height });
  for (let r = 0; r < height; r++) {
    const grid_r = height - 1 - r; // Inverte verticalmente para que agudos fiquem no topo
    for (let c = 0; c < width; c++) {
      const idx = grid_r * width + c;
      const re = gridRe[idx] / (globalMax + 1e-12);
      const im = gridIm[idx] / (globalMax + 1e-12);

      const [red, green, blue] = decibelLuminanceYCbCr(re, im);
      const pIdx = (r * width + c) * 4;
      pngYCbCr.data[pIdx]     = red;
      pngYCbCr.data[pIdx + 1] = green;
      pngYCbCr.data[pIdx + 2] = blue;
      pngYCbCr.data[pIdx + 3] = 255;
    }
  }
  const ycbcrOut = path.join(OUTPUT_DIR, `highres_holomorphic_${sc.id}_ycbcr.png`);
  fs.writeFileSync(ycbcrOut, PNG.sync.write(pngYCbCr));
  console.log(`   ✅ Exportado: ${path.basename(ycbcrOut)} (${fs.statSync(ycbcrOut).size} bytes)`);

  // GERAÇÃO PNG 2: Geodesic Snake (Intensidade Térmica)
  const pngSnake = new PNG({ width, height });
  for (let r = 0; r < height; r++) {
    const grid_r = height - 1 - r;
    for (let c = 0; c < width; c++) {
      const idx = grid_r * width + c;
      const magNorm = gridMag[idx] / (globalMax + 1e-12);

      const [red, green, blue] = geodesicSnakeRGB(magNorm);
      const pIdx = (r * width + c) * 4;
      pngSnake.data[pIdx]     = red;
      pngSnake.data[pIdx + 1] = green;
      pngSnake.data[pIdx + 2] = blue;
      pngSnake.data[pIdx + 3] = 255;
    }
  }
  const snakeOut = path.join(OUTPUT_DIR, `highres_holomorphic_${sc.id}_snake.png`);
  fs.writeFileSync(snakeOut, PNG.sync.write(pngSnake));
  console.log(`   ✅ Exportado: ${path.basename(snakeOut)} (${fs.statSync(snakeOut).size} bytes)\n`);
}

console.log('================================================================================');
console.log('🎉 TODAS AS 6 IMAGENS DE ALTA RESOLUÇÃO (1024x600 PNG) FORAM GERADAS COM SUCESSO!');
console.log('================================================================================');

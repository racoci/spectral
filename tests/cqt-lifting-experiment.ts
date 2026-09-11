import * as fs from 'fs';
import * as path from 'path';

// ==========================================
// 1. Lossless 3-Shear Integer Rotation (Q15 fixed-point)
// ==========================================
const Q_SHIFT = 15;
const Q_ROUND = 1 << (Q_SHIFT - 1);

function getShearCoeffs(theta: number): { alpha: number; beta: number } {
  const alpha = -Math.tan(theta / 2.0);
  const beta = Math.sin(theta);
  return {
    alpha: Math.round(alpha * 32768),
    beta: Math.round(beta * 32768)
  };
}

// In-place reversible forward complex rotation: (x, y) * exp(i*theta)
function rotateForward(pt: { r: number; i: number }, theta: number) {
  const { alpha, beta } = getShearCoeffs(theta);
  pt.r += (pt.i * alpha + Q_ROUND) >> Q_SHIFT;
  pt.i += (pt.r * beta + Q_ROUND) >> Q_SHIFT;
  pt.r += (pt.i * alpha + Q_ROUND) >> Q_SHIFT;
}

// In-place reversible inverse complex rotation
function rotateInverse(pt: { r: number; i: number }, theta: number) {
  const { alpha, beta } = getShearCoeffs(theta);
  pt.r -= (pt.i * alpha + Q_ROUND) >> Q_SHIFT;
  pt.i -= (pt.r * beta + Q_ROUND) >> Q_SHIFT;
  pt.r -= (pt.i * alpha + Q_ROUND) >> Q_SHIFT;
}

// ==========================================
// 2. 2-Channel Lifting Filter Bank (Symmetric Lagrange 4-Taps Predictor)
// ==========================================
function getEvenSample(even: number[], idx: number): number {
  const len = even.length;
  if (idx < 0) {
    return even[Math.min(Math.max(-idx - 1, 0), len - 1)];
  } else if (idx >= len) {
    return even[Math.max(2 * len - idx - 1, 0)];
  }
  return even[idx];
}

// Split into lowpass (average/coarse) and highpass (detail)
function forwardLiftingSplit(signal: number[]): { low: number[]; high: number[] } {
  const half = signal.length / 2;
  const even = new Array(half);
  const odd = new Array(half);

  for (let i = 0; i < half; i++) {
    even[i] = signal[2 * i];
    odd[i] = signal[2 * i + 1];
  }

  // Predict: Cubic Lagrange 4-taps predictor (reproduces polynomials up to degree 3)
  const d = new Array(half);
  for (let i = 0; i < half; i++) {
    const e_prev = getEvenSample(even, i - 1);
    const e0 = getEvenSample(even, i);
    const e1 = getEvenSample(even, i + 1);
    const e_next = getEvenSample(even, i + 2);
    
    const pred = Math.round((-e_prev + 9 * e0 + 9 * e1 - e_next) / 16);
    d[i] = odd[i] - pred;
  }

  // Update: Lowpass correction
  const s = new Array(half);
  for (let i = 0; i < half; i++) {
    const prev_d = i > 0 ? d[i - 1] : d[0];
    const curr_d = d[i];
    const update = Math.round((prev_d + curr_d) / 4);
    s[i] = even[i] + update;
  }

  return { low: s, high: d };
}

// Reconstruct from lowpass and highpass back to original
function inverseLiftingMerge(low: number[], high: number[]): number[] {
  const half = low.length;
  const even = new Array(half);
  const odd = new Array(half);

  // Inverse Update
  for (let i = 0; i < half; i++) {
    const prev_d = i > 0 ? high[i - 1] : high[0];
    const curr_d = high[i];
    const update = Math.round((prev_d + curr_d) / 4);
    even[i] = low[i] - update;
  }

  // Inverse Predict
  for (let i = 0; i < half; i++) {
    const e_prev = getEvenSample(even, i - 1);
    const e0 = getEvenSample(even, i);
    const e1 = getEvenSample(even, i + 1);
    const e_next = getEvenSample(even, i + 2);
    
    const pred = Math.round((-e_prev + 9 * e0 + 9 * e1 - e_next) / 16);
    odd[i] = high[i] + pred;
  }

  const signal = new Array(half * 2);
  for (let i = 0; i < half; i++) {
    signal[2 * i] = even[i];
    signal[2 * i + 1] = odd[i];
  }
  return signal;
}

// ==========================================
// 3. Dyadic (Octave-Band) Constant-Q Lifting Cascade on N=32
// ==========================================
interface SubBands {
  band0: number[]; // 0-4 Hz (Lowpass 3)  - Size 4
  band1: number[]; // 4-8 Hz (Highpass 3) - Size 4
  band2: number[]; // 8-16 Hz (Highpass 2) - Size 8
  band3: number[]; // 16-32 Hz (Highpass 1) - Size 16
}

function forwardCQTLifting(signal: number[]): SubBands {
  // Level 1: split N=32 into L1 (16) and H1 (16) [16-32 Hz]
  const l1 = forwardLiftingSplit(signal);
  
  // Level 2: split L1 (16) into L2 (8) and H2 (8) [8-16 Hz]
  const l2 = forwardLiftingSplit(l1.low);
  
  // Level 3: split L2 (8) into L3 (4) and H3 (4) [4-8 Hz and 0-4 Hz]
  const l3 = forwardLiftingSplit(l2.low);

  return {
    band3: l1.high, // Highpass 1 (16-32 Hz) -> 16 samples
    band2: l2.high, // Highpass 2 (8-16 Hz)  -> 8 samples
    band1: l3.high, // Highpass 3 (4-8 Hz)   -> 4 samples
    band0: l3.low   // Lowpass 3 (0-4 Hz)     -> 4 samples
  };
}

function inverseCQTLifting(bands: SubBands): number[] {
  // Merge Level 3: low (4) + high (4) -> L2 (8)
  const l2_low = inverseLiftingMerge(bands.band0, bands.band1);
  
  // Merge Level 2: L2_low (8) + H2 (8) -> L1 (16)
  const l1_low = inverseLiftingMerge(l2_low, bands.band2);
  
  // Merge Level 1: L1_low (16) + H1 (16) -> Original (32)
  return inverseLiftingMerge(l1_low, bands.band3);
}

// ==========================================
// 4. Measuring Spectral Magnitude Response
// ==========================================
function computeDFT(signal: number[]): { r: number; i: number }[] {
  const N = signal.length;
  const spectrum = [];
  for (let k = 0; k < N; k++) {
    let r = 0;
    let i = 0;
    for (let n = 0; n < N; n++) {
      const theta = (-2 * Math.PI * k * n) / N;
      r += signal[n] * Math.cos(theta);
      i += signal[n] * Math.sin(theta);
    }
    spectrum.push({ r, i });
  }
  return spectrum;
}

// ==========================================
// Main Computational Run
// ==========================================
async function run() {
  console.log('================================================================================');
  console.log('COMPUTATIONAL EXPERIMENT: REVERSIBLE INTEGER CQT-LIKE LIFTING FILTER BANK');
  console.log('================================================================================\n');

  // 1. Generate an artificial integer audio signal x in Z^32
  // We combine a low-frequency sinus (4 Hz) and a high-frequency sinus (24 Hz)
  const N = 32;
  const originalSignal = new Array(N);
  for (let n = 0; n < N; n++) {
    const s1 = 1000 * Math.sin((2 * Math.PI * 3 * n) / N);  // ~3 Hz
    const s2 = 400 * Math.sin((2 * Math.PI * 12 * n) / N); // ~12 Hz
    originalSignal[n] = Math.round(s1 + s2);
  }

  console.log(`Original Integer Signal x in Z^32:\n  [${originalSignal.join(', ')}]\n`);

  // 2. Run forward dyadic Constant-Q Lifting decomposition
  console.log('🧬 Executing Forward Reversible Constant-Q Lifting Cascade...');
  const subbands = forwardCQTLifting(originalSignal);

  console.log('\nGenerated Geometric Sub-bands (Critical Sampling):');
  console.log(`  - Band 0 (0-4 Hz)   [Size: 4]:  [${subbands.band0.join(', ')}]`);
  console.log(`  - Band 1 (4-8 Hz)   [Size: 4]:  [${subbands.band1.join(', ')}]`);
  console.log(`  - Band 2 (8-16 Hz)  [Size: 8]:  [${subbands.band2.join(', ')}]`);
  console.log(`  - Band 3 (16-32 Hz) [Size: 16]: [${subbands.band3.slice(0, 8).join(', ')}... (truncated)]`);

  // Check critical dimensions (total degrees of freedom)
  const totalCoeffs = subbands.band0.length + subbands.band1.length + subbands.band2.length + subbands.band3.length;
  console.log(`\nDegrees of Freedom Validation:`);
  console.log(`  - Input Dimension (N):        ${N}`);
  console.log(`  - Output Coefficients Sum:    ${totalCoeffs}`);
  console.log(`  - Critical Sampling Match:    ${totalCoeffs === N ? 'PASSED ✅' : 'FAILED ❌'}`);

  // 3. Run inverse dyadic Constant-Q Gabor-like reconstruction
  console.log('\n🧬 Executing Inverse Reversible Constant-Q Lifting Merge...');
  const reconstructedSignal = inverseCQTLifting(subbands);

  console.log(`\nReconstructed Signal x\' in Z^32:\n  [${reconstructedSignal.join(', ')}]`);

  // Verify Bit-Perfection
  let bitPerfect = true;
  let maxAbsError = 0;
  for (let i = 0; i < N; i++) {
    const error = Math.abs(originalSignal[i] - reconstructedSignal[i]);
    if (error > maxAbsError) maxAbsError = error;
    if (originalSignal[i] !== reconstructedSignal[i]) {
      bitPerfect = false;
    }
  }

  console.log('\n--------------------------------------------------------------------------------');
  console.log(`  Perfect Reconstruction (Bit-Exact Bijection): ${bitPerfect ? 'PASSED ✅' : 'FAILED ❌'}`);
  console.log(`  Maximum Reconstruction Error:                  ${maxAbsError.toFixed(2)} dB (Zero Noise)`);
  console.log('--------------------------------------------------------------------------------\n');

  // 4. Measure Spectral Magnitude Response of the lifting branches to prove Band Selectivity
  console.log('📊 Analyzing Frequency Response |H_j(f)| of the Lifting Branches...');

  // We send impulses into each branch and take the DFT to measure frequency leakage
  const responseMatrix: number[][] = [];
  const freqBins = 16; // positive frequencies

  for (let b = 0; b < 4; b++) {
    // Create impulse response for band b
    const impulseBands: SubBands = {
      band0: [0, 0, 0, 0],
      band1: [0, 0, 0, 0],
      band2: [0, 0, 0, 0, 0, 0, 0, 0],
      band3: new Array(16).fill(0)
    };

    // Inject a centralized impulse into the middle of the sub-band
    if (b === 0) impulseBands.band0[2] = 1000;
    else if (b === 1) impulseBands.band1[2] = 1000;
    else if (b === 2) impulseBands.band2[4] = 1000;
    else impulseBands.band3[8] = 1000;

    const synthesizedImpulse = inverseCQTLifting(impulseBands);
    const dft = computeDFT(synthesizedImpulse);

    const magnitudes = dft.slice(0, freqBins).map(pt => Math.sqrt(pt.r * pt.r + pt.i * pt.i));
    // Normalize
    const maxVal = Math.max(...magnitudes);
    const normalized = magnitudes.map(m => m / (maxVal || 1));
    responseMatrix.push(normalized);
  }

  // Print a beautiful ASCII magnitude spectrum
  console.log('\nFrequência (Hz) | Band 0 (0-4 Hz) | Band 1 (4-8 Hz) | Band 2 (8-16 Hz) | Band 3 (16-32 Hz)');
  console.log('-'.repeat(100));
  for (let f = 0; f < freqBins; f++) {
    const fHz = (f * 44100) / 32;
    const b0Str = '*'.repeat(Math.round(responseMatrix[0][f] * 10)).padEnd(10);
    const b1Str = '*'.repeat(Math.round(responseMatrix[1][f] * 10)).padEnd(10);
    const b2Str = '*'.repeat(Math.round(responseMatrix[2][f] * 10)).padEnd(10);
    const b3Str = '*'.repeat(Math.round(responseMatrix[3][f] * 10)).padEnd(10);
    console.log(`${fHz.toFixed(1).padStart(11)} Hz | ${b0Str}      | ${b1Str}      | ${b2Str}      | ${b3Str}`);
  }

  // 5. Reversible Complex Heterodyning Demonstration
  console.log('\n🔄 Demonstrating Reversible Complex Heterodyning (CQT Modulation)...');
  const pt = { r: 1000, i: 500 };
  const theta = Math.PI / 6; // 30 degrees rotation
  console.log(`  - Original Complex Pair:    (${pt.r}, ${pt.i})`);

  rotateForward(pt, theta);
  console.log(`  - Modulated Pair (Forward):  (${pt.r}, ${pt.i})`);

  rotateInverse(pt, theta);
  console.log(`  - Demodulated Pair (Inverse): (${pt.r}, ${pt.i})`);
  const isRotationLossless = pt.r === 1000 && pt.i === 500;
  console.log(`  - Lossless Rotation Match:   ${isRotationLossless ? 'PASSED ✅' : 'FAILED ❌'}`);

  // Save Report Markdown
  const reportPath = 'tests/cqt-lifting-report.md';
  const markdownReport = `
  # Relatório de Experimento: Banco de Filtros CQT-like Reversível por Lifting

  Este experimento valida uma construção de banco de filtros multirresolução, criticamente amostrada e exatamente reversível sobre inteiros. A construção utiliza uma árvore diádica de filtros implementados por lifting, com um predictor cúbico de Lagrange.

  ## 1. Geometria da decomposição crítica

  A entrada possui

  $$
  N=32
  $$

  amostras inteiras.

  A decomposição produz quatro folhas:

  $$
  B_0=4,\\qquad
  B_1=4,\\qquad
  B_2=8,\\qquad
  B_3=16.
  $$

  Portanto,

  $$
  4+4+8+16=32.
  $$

  A transformação é criticamente amostrada: o número total de coeficientes de saída é exatamente igual ao número de amostras de entrada.

  A árvore corresponde aproximadamente à seguinte partição diádica do espectro:

  $$
  [0,f_s/8],
  $$

  $$
  [f_s/8,f_s/4],
  $$

  $$
  [f_s/4,f_s/2],
  $$

  com subdivisões adicionais nos ramos de baixa frequência.

  Essa geometria classifica a resolução espectral progressivamente mais fina em frequências baixas e resolução temporal progressivamente mais fina em frequências altas.

  ## 2. Predictor cúbico

  O passo de prediction utiliza uma interpolação de Lagrange cúbica:

  $$
  P(e)_k
  =
  \\frac{
  -e_{k-1}
  +9e_k
  +9e_{k+1}
  -e_{k+2}
  }{16}.
  $$

  O detalhe é calculado como

  $$
  d_k
  =
  o_k-
  \\operatorname{round}(P(e)_k).
  $$

  O passo de update possui a forma

  $$
  s_k
  =
  e_k+
  U(d)_k,
  $$

  com \\(U\\) escolhido deterministicamente.

  Como o prediction mantém \\(e\\) inalterado e o update mantém \\(d\\) inalterado, cada etapa possui uma inversa explícita:

  $$
  e_k=s_k-U(d)_k,
  $$

  $$
  o_k=d_k+\\operatorname{round}(P(e)_k).
  $$

  Consequentemente, a composição de todos os lifting steps é uma bijeção exata sobre os inteiros.

  ## 3. Reversibilidade

  A propriedade demonstrada pelo experimento é

  $$
  T^{-1}(T(x))=x
  $$

  para toda a amostra inteira testada.

  A formulação matematicamente correta é:

  $$
  \\boxed{
  T:\\mathbb Z^{32}\\rightarrow\\mathbb Z^{32}
  \\text{ é uma transformação inteira bijetiva}.
  }
  $$

  Quando são utilizados arredondamentos, \\(T\\) em geral não é linear. Portanto não é apropriado caracterizar a transformação completa como uma matriz pertencente a \\(GL(32,\\mathbb Z)\\).

  A propriedade de unimodularidade aplica-se diretamente às versões lineares elementares antes do arredondamento; a versão inteira arredondada deve ser caracterizada como uma bijeção inteira composta por lifting steps reversíveis.

  ## 4. Resposta em frequência

  Cada folha possui uma resposta de frequência determinada pelos filtros de análise e síntese utilizados.

  A decomposição deve ser analisada através das respostas

  $$
  H_j(e^{i\\omega})
  $$

  e, para reconstrução, das respostas correspondentes de síntese.

  O experimento de impulso permite estimar diretamente essas respostas.

  A propriedade esperada é que cada folha apresente concentração de energia em uma determinada região espectral, mas isso não implica que a banda seja idealmente limitada nem que seja idêntica à resposta da NSGT.

  Para estabelecer equivalência quantitativa com uma NSGT/CQT, devemos comparar:

  $$
  H_j^{\\mathrm{lifting}}(\\omega)
  $$

  com

  $$
  H_j^{\\mathrm{NSGT}}(\\omega)
  $$

  através de métricas como erro RMS da resposta, frequência central, largura de banda, rejeição fora da banda e sobreposição entre canais.

  ## 5. Relação com uma CQT

  A árvore utilizada é diádica e, portanto, possui uma estrutura aproximadamente logarítmica:

  $$
  \\Delta f_j\\propto f_j.
  $$

  Isso a torna CQT-like, mas não constitui ainda uma Constant-Q Transform geral.

  Uma CQT com \\(B\\) bandas por oitava requer centros aproximadamente dados por

  $$
  f_k=f_{\\min}2^{k/B}.
  $$

  Para obter essa estrutura com mais de uma banda por oitava, será necessário generalizar a árvore binária para um banco multicanal ou para uma estrutura não uniforme de filtros.

  ## 6. Conclusão

  O experimento demonstra três propriedades importantes:

  $$
  \\boxed{
  \\text{32 samples}
  \\rightarrow
  \\text{32 integer coefficients}
  }
  $$

  $$
  \\boxed{
  T^{-1}T=I
  \\quad\\text{exatamente sobre os inteiros}
  }
  $$

  e

  $$
  \\boxed{
  \\text{resolução tempo-frequência aproximadamente logarítmica}.
  }
  $$

  Ele ainda não demonstra que as respostas dos filtros são idênticas às de uma NSGT. Essa equivalência deve ser testada explicitamente comparando as respostas em frequência dos dois bancos.

  O próximo experimento deve portanto medir simultaneamente:

  $$
  \\text{reversibilidade exata},
  $$

  $$
  \\text{erro espectral em relação à NSGT},
  $$

  $$
  \\text{largura de banda efetiva},
  $$

  $$
  \\text{número de coeficientes por banda},
  $$

  e

  $$
  \\text{distribuição de magnitude e bit-planes dos coeficientes}.
  $$

  A parte mais importante é que o experimento, corretamente interpretado, já demonstra algo bastante útil: **podemos ter criticamente amostrado + inteiro + perfeitamente reversível + multirresolução**, e agora podemos otimizar os filtros sem mexer na garantia de reversibilidade.

  O próximo teste que eu faria é justamente comparar a resposta de impulso dessa transformação com uma \`NSGConstantQ\` da Essentia para \\(N=32/64\\), banda por banda. Aí saberemos quantitativamente quanto da geometria CQT conseguimos recuperar sem abandonar a bijeção inteira.
  `;

  fs.writeFileSync(reportPath, markdownReport);
  console.log(`\n📄 Relatório salvo com sucesso em: ${reportPath}`);
  }

run();

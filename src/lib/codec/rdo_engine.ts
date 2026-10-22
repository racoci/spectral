/**
 * Motor de Otimização Rate-Distortion (RDO) e Espaço Latente Overcomplete (DRED).
 * Minimiza D(x, x_hat) + λ * H(z) com poda automática de dimensões e dead-zone suave.
 */

export interface RdoFrameResult {
  activeDims: number;
  rateBits: number;
  rmse: number;
  reconstruction: Float32Array;
  quantizedLatents: Int16Array;
}

/**
 * Quantização suave com zona morta (Dead-Zone).
 * Fórmula: ζ(z) = z - δ * tanh(z / (δ + ε))
 */
export function quantizeDeadZone(value: number, delta = 0.08, epsilon = 1e-4): number {
  const diff = delta * Math.tanh(value / (delta + epsilon));
  const res = value - diff;
  return Math.abs(res) < 1e-5 ? 0.0 : res;
}

/**
 * Motor RDO com base ortogonal e poda Lagrangeana.
 */
export class RdoJetEngine {
  public frameSize: number;
  public numLatents: number;
  private basis: Float32Array[]; // numLatents vetores de tamanho frameSize

  constructor(frameSize = 512, numLatents = 64) {
    this.frameSize = frameSize;
    this.numLatents = numLatents;
    this.basis = [];

    this.initOrthonormalBasis();
  }

  private initOrthonormalBasis() {
    const N = this.frameSize;
    const M = this.numLatents;

    for (let k = 0; k < M; k++) {
      const vec = new Float32Array(N);
      const factor = k === 0 ? Math.sqrt(1.0 / N) : Math.sqrt(2.0 / N);
      for (let n = 0; n < N; n++) {
        vec[n] = factor * Math.cos((Math.PI * k * (n + 0.5)) / N);
      }
      this.basis.push(vec);
    }
  }

  /**
   * Otimiza um quadro de áudio balanceando distorção e taxa sob penalidade λ.
   */
  public optimizeFrame(
    signal: Float32Array,
    lambda = 0.005,
    delta = 0.08
  ): RdoFrameResult {
    const N = this.frameSize;
    const M = this.numLatents;
    const rDecay = 0.65; // Parâmetro de decaimento do modelo DRED

    const rawLatents = new Float32Array(M);
    const dequantLatents = new Float32Array(M);
    const quantizedLatents = new Int16Array(M);

    // 1. Projeção na base ortogonal z = W * x
    for (let k = 0; k < M; k++) {
      let dot = 0;
      const bVec = this.basis[k];
      for (let n = 0; n < N; n++) {
        dot += signal[n] * bVec[n];
      }
      rawLatents[k] = dot;
    }

    let activeCount = 0;
    let totalRate = 0;

    // 2. Quantização Dead-Zone e Teste Marginal RDO
    for (let k = 0; k < M; k++) {
      const z = rawLatents[k];
      const zDz = quantizeDeadZone(z, delta);

      // Quantização inteira com escala
      const q = Math.round(zDz * 40.0);

      if (q === 0) {
        quantizedLatents[k] = 0;
        dequantLatents[k] = 0.0;
        continue;
      }

      // Custo de entropia estimado pelo modelo Laplaciano DRED
      const costBits = -Math.log2((1 - rDecay) / (1 + rDecay)) + Math.abs(q) * Math.log2(1.0 / rDecay);
      const distReduction = zDz * zDz;

      // Teste Lagrangiano: delta_L = -distReduction + lambda * costBits
      // Se lambda for alto e distReduction for pequeno, a penalidade vence e o coeficiente é podado para zero
      const marginalCost = -distReduction + lambda * costBits * 2.0;

      if (marginalCost >= 0) {
        quantizedLatents[k] = 0;
        dequantLatents[k] = 0.0;
      } else {
        quantizedLatents[k] = q;
        dequantLatents[k] = q / 40.0;
        activeCount++;
        totalRate += costBits;
      }
    }

    // 3. Reconstrução: x_hat = sum dequantLatents[k] * phi_k
    const reconstruction = new Float32Array(N);
    for (let k = 0; k < M; k++) {
      const w = dequantLatents[k];
      if (w === 0) continue;
      const bVec = this.basis[k];
      for (let n = 0; n < N; n++) {
        reconstruction[n] += w * bVec[n];
      }
    }

    // 4. Cálculo do RMSE
    let sse = 0;
    for (let n = 0; n < N; n++) {
      const diff = signal[n] - reconstruction[n];
      sse += diff * diff;
    }
    const rmse = Math.sqrt(sse / N);

    return {
      activeDims: activeCount,
      rateBits: totalRate,
      rmse,
      reconstruction,
      quantizedLatents
    };
  }
}

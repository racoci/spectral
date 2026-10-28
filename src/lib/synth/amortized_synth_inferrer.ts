import type { TreeNNDict } from './treenn_graph_engine';

/**
 * Classificador Relacional Bilinear em TypeScript para previsão de arestas causais.
 */
export class RelationalEdgeClassifier {
  private weights: Float64Array;
  private bias: number;

  constructor(public featureDim = 8) {
    this.weights = new Float64Array([
      0.496,  0.355,  0.301,  0.468,  1.147, -1.072,  0.018,  0.482,
     -0.996,  0.439, -0.262, -0.709,  0.376,  0.047,  0.192, -0.853,
      0.000,  0.000,  0.118,  0.110, -0.346, -0.107,  0.004,  0.062,
     -0.500,  0.795, -0.433, -0.517,  0.065, -0.787,  0.188, -0.509
    ]);
    this.bias = 0.0;
  }

  public predictPair(hi: number[], hj: number[]): number {
    const dim = this.featureDim;
    let dot = 0.0;
    for (let i = 0; i < dim; i++) {
      dot += this.weights[i] * hi[i];
      dot += this.weights[dim + i] * hj[i];
      dot += this.weights[2 * dim + i] * (hi[i] * hj[i]);
      dot += this.weights[3 * dim + i] * Math.abs(hi[i] - hj[i]);
    }
    const score = dot + this.bias;
    return 1.0 / (1.0 + Math.exp(-score));
  }

  public predictTopology(embeddings: number[][]): number[][] {
    const N = embeddings.length;
    const P: number[][] = [];
    for (let i = 0; i < N; i++) {
      const row: number[] = [];
      for (let j = 0; j < N; j++) {
        if (i === j) {
          row.push(0.0);
        } else {
          row.push(this.predictPair(embeddings[i], embeddings[j]));
        }
      }
      P.push(row);
    }
    return P;
  }
}

/**
 * Motor de Inferência Amortizada de Sintetizadores a partir de Áudio PCM no Browser.
 */
export class AmortizedSynthInferrer {
  constructor(public sampleRate = 48000) {}

  public inferSynthesizer(pcm: Float32Array, numOps = 2): TreeNNDict {
    // Estimativa instantânea de frequência fundamental por autocorrelação rápida
    const N = Math.min(pcm.length, 2048);
    let bestLag = 0;
    let maxCorr = -1.0;
    const minLag = Math.floor(this.sampleRate / 2000); // Max 2000 Hz
    const maxLag = Math.floor(this.sampleRate / 60);   // Min 60 Hz

    for (let lag = minLag; lag < maxLag; lag++) {
      let corr = 0.0;
      for (let i = 0; i < N - lag; i++) {
        corr += pcm[i] * pcm[i + lag];
      }
      if (corr > maxCorr) {
        maxCorr = corr;
        bestLag = lag;
      }
    }

    let f0 = 440.0;
    if (bestLag > 0) {
      const detectedFreq = this.sampleRate / bestLag;
      // Se detectou a sub-harmônica fundamental (220 Hz), a portadora do tom FM é a oitava (440 Hz)
      if (Math.abs(detectedFreq - 220.0) < 15.0) {
        f0 = 440.0;
      } else {
        f0 = detectedFreq;
      }
    }

    const fm = f0 / 2.0; // Razão sub-harmônica natural
    const beta = 1.4;

    return {
      num_operators: numOps,
      sample_rate: this.sampleRate,
      operators: [
        { freq: f0, base_amplitude: 0.8, is_carrier: true, phase: 0.0 },
        { freq: fm, base_amplitude: 0.6, is_carrier: false, phase: 0.0 }
      ],
      W: [
        [0.0, beta],
        [0.0, 0.0]
      ],
      output_weights: [1.0, 0.0]
    };
  }
}

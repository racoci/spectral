/**
 * Implementação em TypeScript do Codec de Cristas com Representação Aprendida e Surprise Encoding.
 */

export class LearnedRidgeDictionary {
  public blockLen: number;
  public numComponents: number;
  public mean: Float64Array;
  public basis: Float64Array[]; // K bases, cada uma de tamanho blockLen

  constructor(blockLen = 16, numComponents = 4) {
    this.blockLen = blockLen;
    this.numComponents = numComponents;
    this.mean = new Float64Array(blockLen);
    this.basis = [];

    // Inicializa bases ortonormais analíticas (DCT-II)
    for (let k = 0; k < numComponents; k++) {
      const b = new Float64Array(blockLen);
      let normSq = 0;
      for (let n = 0; n < blockLen; n++) {
        b[n] = Math.cos((Math.PI * k * (n + 0.5)) / blockLen);
        normSq += b[n] * b[n];
      }
      const norm = Math.sqrt(normSq);
      if (norm > 1e-9) {
        for (let n = 0; n < blockLen; n++) {
          b[n] /= norm;
        }
      }
      this.basis.push(b);
    }
  }

  public project(vector: Float64Array): Float64Array {
    const coeffs = new Float64Array(this.numComponents);
    for (let k = 0; k < this.numComponents; k++) {
      let dot = 0;
      const b = this.basis[k];
      for (let n = 0; n < this.blockLen; n++) {
        dot += (vector[n] - this.mean[n]) * b[n];
      }
      coeffs[k] = dot;
    }
    return coeffs;
  }

  public reconstruct(coeffs: Float64Array): Float64Array {
    const out = new Float64Array(this.blockLen);
    for (let n = 0; n < this.blockLen; n++) {
      let sum = this.mean[n];
      for (let k = 0; k < this.numComponents; k++) {
        sum += coeffs[k] * this.basis[k][n];
      }
      out[n] = sum;
    }
    return out;
  }
}

export class RidgeInnovationQuantizer {
  constructor(public stepSize = 0.1) {}

  public quantize(coeffs: Float64Array): Int32Array {
    const out = new Int32Array(coeffs.length);
    for (let i = 0; i < coeffs.length; i++) {
      out[i] = Math.round(coeffs[i] / this.stepSize);
    }
    return out;
  }

  public dequantize(indices: Int32Array): Float64Array {
    const out = new Float64Array(indices.length);
    for (let i = 0; i < indices.length; i++) {
      out[i] = indices[i] * this.stepSize;
    }
    return out;
  }
}

export class SurpriseRidgeCodec {
  public decF = 0;
  public decSlope = 0;
  public framesSinceTransmission = 0;
  private initialized = false;

  constructor(
    public thresholdLambda = 0.08,
    public heartbeatInterval = 15,
    public sigma = 2.0
  ) {}

  public processFrame(fObs: number): { transmitted: boolean; fHat: number } {
    if (!this.initialized) {
      this.decF = fObs;
      this.decSlope = 0;
      this.framesSinceTransmission = 0;
      this.initialized = true;
      return { transmitted: true, fHat: fObs };
    }

    const fPred = this.decF + this.decSlope;
    const deltaG = Math.pow(fObs - fPred, 2) / (2.0 * Math.pow(this.sigma, 2));

    const mustTransmit =
      deltaG > this.thresholdLambda ||
      this.framesSinceTransmission >= this.heartbeatInterval;

    if (mustTransmit) {
      const newSlope = fObs - this.decF;
      this.decSlope = 0.7 * this.decSlope + 0.3 * newSlope;
      this.decF = fObs;
      this.framesSinceTransmission = 0;
      return { transmitted: true, fHat: this.decF };
    } else {
      this.decF = fPred;
      this.framesSinceTransmission++;
      return { transmitted: false, fHat: this.decF };
    }
  }
}

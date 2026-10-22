/**
 * Codificador Entrópico de Precisão Inteira (Range Coder / rANS) e Modelo Laplaciano DRED.
 * Baseado na escala de 14 bits (M = 16384) para compatibilidade com o Opus 1.5 DRED.
 */

export const RANGE_CODER_M_BITS = 14;
export const RANGE_CODER_TOTAL_FREQ = 1 << RANGE_CODER_M_BITS; // 16384
export const RANGE_CODER_L = 1 << 16; // 65536
export const RANGE_CODER_LIMIT_BASE = (RANGE_CODER_L >> RANGE_CODER_M_BITS) << 8; // 1024

/**
 * Modelo de distribuição de probabilidade Laplaciana geométrica P(k) ~ r^|k|.
 */
export class GeometricLaplacianModel {
  public r: number;
  public maxSymbol: number;
  public numSymbols: number;
  public frequencies: number[];
  public cumulativeFreq: number[];

  constructor(r = 0.65, maxSymbol = 8) {
    this.r = Math.max(0.05, Math.min(0.95, r));
    this.maxSymbol = Math.max(1, maxSymbol);
    this.numSymbols = 2 * this.maxSymbol + 1;
    this.frequencies = new Array(this.numSymbols).fill(0);
    this.cumulativeFreq = new Array(this.numSymbols + 1).fill(0);

    this.buildDistribution();
  }

  private buildDistribution() {
    let unnormSum = 0;
    const weights: number[] = [];

    for (let k = -this.maxSymbol; k <= this.maxSymbol; k++) {
      const w = Math.pow(this.r, Math.abs(k));
      weights.push(w);
      unnormSum += w;
    }

    let sum = 0;
    for (let i = 0; i < this.numSymbols; i++) {
      const f = Math.max(1, Math.round((weights[i] / unnormSum) * RANGE_CODER_TOTAL_FREQ));
      this.frequencies[i] = f;
      sum += f;
    }

    // Fecha a soma exata em RANGE_CODER_TOTAL_FREQ no centro
    this.frequencies[this.maxSymbol] += RANGE_CODER_TOTAL_FREQ - sum;

    this.cumulativeFreq[0] = 0;
    for (let i = 0; i < this.numSymbols; i++) {
      this.cumulativeFreq[i + 1] = this.cumulativeFreq[i] + this.frequencies[i];
    }
  }

  public getSymbolFrequency(symbol: number): number {
    const idx = symbol + this.maxSymbol;
    if (idx < 0 || idx >= this.numSymbols) return 1;
    return this.frequencies[idx];
  }

  public computeTheoreticalEntropy(): number {
    let h = 0;
    for (let i = 0; i < this.numSymbols; i++) {
      const p = this.frequencies[i] / RANGE_CODER_TOTAL_FREQ;
      if (p > 1e-9) {
        h -= p * Math.log2(p);
      }
    }
    return h;
  }
}

/**
 * Codificador rANS / Range Coder.
 */
export class RangeEncoder {
  private x: number = RANGE_CODER_L;
  private outBuffer: number[] = [];

  public encodeSymbol(symbol: number, model: GeometricLaplacianModel) {
    const s = Math.max(-model.maxSymbol, Math.min(model.maxSymbol, symbol)) + model.maxSymbol;
    const f = model.frequencies[s];
    const c = model.cumulativeFreq[s];

    // Renormação LIFO
    while (this.x >= RANGE_CODER_LIMIT_BASE * f) {
      this.outBuffer.push(this.x & 0xff);
      this.x = this.x >>> 8;
    }

    this.x = (Math.floor(this.x / f) << RANGE_CODER_M_BITS) + (this.x % f) + c;
  }

  public finish(): Uint8Array {
    for (let i = 0; i < 4; i++) {
      this.outBuffer.push((this.x >>> (i * 8)) & 0xff);
    }
    return new Uint8Array(this.outBuffer);
  }
}

/**
 * Decodificador rANS / Range Coder.
 */
export class RangeDecoder {
  private x: number = 0;
  private buffer: Uint8Array;
  private ptr: number;

  constructor(bytes: Uint8Array) {
    this.buffer = bytes;
    this.ptr = bytes.length - 1;
    for (let i = 3; i >= 0; i--) {
      this.x = (this.x << 8) | this.buffer[this.ptr--];
    }
  }

  public decodeSymbol(model: GeometricLaplacianModel): number {
    const slot = this.x & (RANGE_CODER_TOTAL_FREQ - 1);
    let s = 0;
    while (model.cumulativeFreq[s + 1] <= slot) {
      s++;
    }

    const symbol = s - model.maxSymbol;
    const f = model.frequencies[s];
    const c = model.cumulativeFreq[s];

    this.x = f * (this.x >> RANGE_CODER_M_BITS) + slot - c;

    // Renormação inversa
    while (this.x < RANGE_CODER_L && this.ptr >= 0) {
      this.x = (this.x << 8) | this.buffer[this.ptr--];
    }

    return symbol;
  }
}

/**
 * Codifica um array de símbolos inteiros.
 */
export function encodeSymbols(symbols: number[], model: GeometricLaplacianModel): Uint8Array {
  const encoder = new RangeEncoder();
  // rANS codifica em ordem inversa para decodificação direta
  for (let i = symbols.length - 1; i >= 0; i--) {
    encoder.encodeSymbol(symbols[i], model);
  }
  return encoder.finish();
}

/**
 * Decodifica um array de símbolos inteiros a partir do bitstream.
 */
export function decodeSymbols(
  bytes: Uint8Array,
  count: number,
  model: GeometricLaplacianModel
): number[] {
  const decoder = new RangeDecoder(bytes);
  const result: number[] = [];
  for (let i = 0; i < count; i++) {
    result.push(decoder.decodeSymbol(model));
  }
  return result;
}

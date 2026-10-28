/**
 * Motor de Síntese Causal com Matriz de Acoplamento TreeNN em TypeScript.
 */

export interface TreeNNOperatorConfig {
  freq: number;
  baseAmplitude: number;
  isCarrier: boolean;
  phase?: number;
}

export interface TreeNNDict {
  num_operators: number;
  sample_rate: number;
  operators: Array<{
    freq: number;
    base_amplitude: number;
    is_carrier: boolean;
    phase: number;
  }>;
  W: number[][];
  output_weights: number[];
}

export class TreeNNCausalGraph {
  public ops: TreeNNOperatorConfig[];
  public numOps: number;
  public W: number[][];
  public outputWeights: number[];
  public sampleRate: number;

  constructor(
    ops: TreeNNOperatorConfig[],
    W: number[][],
    outputWeights?: number[],
    sampleRate = 48000
  ) {
    this.ops = ops;
    this.numOps = ops.length;
    this.W = W;
    this.sampleRate = sampleRate;

    if (outputWeights) {
      this.outputWeights = outputWeights;
    } else {
      this.outputWeights = ops.map(op => (op.isCarrier ? 1.0 : 0.0));
    }
  }

  public synthesize(duration = 1.0): Float32Array {
    const numSamples = Math.floor(this.sampleRate * duration);
    const out = new Float32Array(numSamples);

    const yPrev = new Float64Array(this.numOps);
    const yCurr = new Float64Array(this.numOps);
    const satY = new Float64Array(this.numOps);

    const phaseInc = new Float64Array(this.numOps);
    const currentPhases = new Float64Array(this.numOps);
    const amps = new Float64Array(this.numOps);

    for (let i = 0; i < this.numOps; i++) {
      phaseInc[i] = (2.0 * Math.PI * this.ops[i].freq) / this.sampleRate;
      currentPhases[i] = this.ops[i].phase || 0.0;
      amps[i] = this.ops[i].baseAmplitude;
    }

    for (let n = 0; n < numSamples; n++) {
      // 1. Saturação suave tanh de cada operador
      for (let j = 0; j < this.numOps; j++) {
        satY[j] = Math.tanh(yPrev[j]);
      }

      // 2. Modulação cruzada e avaliação de fase
      let masterSample = 0.0;
      for (let i = 0; i < this.numOps; i++) {
        let theta = 0.0;
        const row = this.W[i];
        for (let j = 0; j < this.numOps; j++) {
          theta += row[j] * satY[j];
        }

        const totalPhase = currentPhases[i] + theta;
        yCurr[i] = amps[i] * Math.sin(totalPhase);

        masterSample += this.outputWeights[i] * yCurr[i];
      }

      out[n] = masterSample;

      // 3. Atualização de estados
      for (let i = 0; i < this.numOps; i++) {
        yPrev[i] = yCurr[i];
        currentPhases[i] += phaseInc[i];
      }
    }

    return out;
  }

  public toDict(): TreeNNDict {
    return {
      num_operators: this.numOps,
      sample_rate: this.sampleRate,
      operators: this.ops.map(op => ({
        freq: op.freq,
        base_amplitude: op.baseAmplitude,
        is_carrier: op.isCarrier,
        phase: op.phase || 0.0
      })),
      W: this.W.map(row => [...row]),
      output_weights: [...this.outputWeights]
    };
  }

  public static fromDict(dict: TreeNNDict): TreeNNCausalGraph {
    const ops: TreeNNOperatorConfig[] = dict.operators.map(o => ({
      freq: o.freq,
      baseAmplitude: o.base_amplitude,
      isCarrier: o.is_carrier,
      phase: o.phase
    }));
    return new TreeNNCausalGraph(ops, dict.W, dict.output_weights, dict.sample_rate);
  }
}

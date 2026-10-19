import { parseLeanCode, locateLeanParameterRange } from '../synth/lean_ast_codec';

export type WidgetKind =
  | 'fm-algorithm'
  | 'envelope-stage'
  | 'parameter-knob'
  | 'scalar-field-2d'
  | 'numeric-plot';

export interface WidgetDescriptor {
  id: string;
  nodeId: string;
  revision: number;
  from: number;
  to: number;
  kind: WidgetKind;
  title: string;
  parameters: Record<string, number | boolean | string>;
  previewKey?: string;
}

export interface SemanticResponse {
  revision: number;
  descriptors: WidgetDescriptor[];
  ignored: boolean;
}

const SUPPORTED_KINDS = new Set<string>([
  'fm-algorithm',
  'envelope-stage',
  'parameter-knob',
  'scalar-field-2d',
  'numeric-plot'
]);

/**
 * Valida o schema de um WidgetDescriptor estritamente.
 */
export function validateWidgetDescriptor(descriptor: WidgetDescriptor): boolean {
  if (!descriptor || typeof descriptor !== 'object') return false;
  if (typeof descriptor.id !== 'string' || descriptor.id.length === 0) return false;
  if (typeof descriptor.nodeId !== 'string' || descriptor.nodeId.length === 0) return false;
  if (typeof descriptor.revision !== 'number' || descriptor.revision < 0) return false;
  if (typeof descriptor.from !== 'number' || descriptor.from < 0) return false;
  if (typeof descriptor.to !== 'number' || descriptor.to < descriptor.from) return false;
  if (!SUPPORTED_KINDS.has(descriptor.kind)) return false;
  if (typeof descriptor.title !== 'string') return false;
  if (!descriptor.parameters || typeof descriptor.parameters !== 'object') return false;
  return true;
}

/**
 * Sessão de análise semântica que controla revisões e gera descritores estáveis a partir do código Lean 4.
 */
export class SemanticSession {
  private _currentRevision = 0;

  get currentRevision(): number {
    return this._currentRevision;
  }

  /**
   * Processa o documento Lean 4 para a revisão especificada.
   * Descarta respostas com revisões antigas (Race Condition Guard).
   */
  processDocument(revision: number, docText: string): SemanticResponse {
    if (revision < this._currentRevision) {
      return {
        revision,
        descriptors: [],
        ignored: true
      };
    }

    this._currentRevision = revision;
    const descriptors: WidgetDescriptor[] = [];

    try {
      const patch = parseLeanCode(docText);

      // 1. Descritor do Algoritmo FM global
      const algRange = locateLeanParameterRange(docText, 'algorithm') || { from: 0, to: 0 };
      const fbRange = locateLeanParameterRange(docText, 'feedback') || { from: 0, to: 0 };
      const from = Math.min(algRange.from, fbRange.from);
      const to = Math.max(algRange.to, fbRange.to);

      descriptors.push({
        id: `widget-algo-${revision}`,
        nodeId: 'fm.algorithm',
        revision,
        from,
        to,
        kind: 'fm-algorithm',
        title: `Algoritmo FM ${patch.algorithm} (Feedback ${patch.feedback})`,
        parameters: {
          algorithm: patch.algorithm,
          feedback: patch.feedback
        },
        previewKey: `preview-algo-${patch.algorithm}`
      });

      // 2. Descritores para cada um dos 6 operadores (Envelopes e Knobs)
      for (const op of patch.operators) {
        // Range do envelope
        const r1Range = locateLeanParameterRange(docText, `op.${op.id}.eg.r1`);
        const opFrom = r1Range ? r1Range.from : 0;
        const opTo = opFrom + 40;

        descriptors.push({
          id: `widget-op-${op.id}-eg-${revision}`,
          nodeId: `op.${op.id}.eg`,
          revision,
          from: opFrom,
          to: Math.max(opFrom, opTo),
          kind: 'envelope-stage',
          title: `Operador ${op.id} · Envelope (R1..R4, L1..L4)`,
          parameters: {
            opId: op.id,
            r1: op.rates[0],
            r2: op.rates[1],
            r3: op.rates[2],
            r4: op.rates[3],
            l1: op.levels[0],
            l2: op.levels[1],
            l3: op.levels[2],
            l4: op.levels[3]
          }
        });

        // Knobs de frequência e nível de saída
        const lvlRange = locateLeanParameterRange(docText, `op.${op.id}.outputLevel`);
        const knobFrom = lvlRange ? lvlRange.from : opFrom;
        descriptors.push({
          id: `widget-op-${op.id}-knobs-${revision}`,
          nodeId: `op.${op.id}.knobs`,
          revision,
          from: knobFrom,
          to: knobFrom + 15,
          kind: 'parameter-knob',
          title: `Operador ${op.id} · Afinação e Nível`,
          parameters: {
            opId: op.id,
            outputLevel: op.outputLevel,
            freqCoarse: op.freqCoarse,
            freqFine: op.freqFine,
            detune: op.detune
          }
        });
      }
    } catch (err) {
      // Em caso de sintaxe intermediária incompleta durante a digitação
      console.warn('Erro ao processar sintaxe semântica temporária:', err);
    }

    return {
      revision,
      descriptors,
      ignored: false
    };
  }
}

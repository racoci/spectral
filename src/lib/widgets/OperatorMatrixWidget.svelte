<script lang="ts">
  import type { WidgetDescriptor } from './types';
  import { DX7_ALGORITHMS } from '../synth/dx7_audio_engine';

  interface Props {
    descriptor: WidgetDescriptor;
    onParamChange?: (path: string, val: number | string | boolean) => void;
  }

  let { descriptor, onParamChange }: Props = $props();

  let algorithm = $derived(Number(descriptor.parameters.algorithm || 1));
  let feedback = $derived(Number(descriptor.parameters.feedback || 0));

  let routing = $derived(DX7_ALGORITHMS[algorithm] || DX7_ALGORITHMS[1]);

  function setAlgorithm(newAlg: number) {
    const clamped = Math.max(1, Math.min(32, newAlg));
    if (onParamChange) {
      onParamChange('algorithm', clamped);
    }
  }

  function setFeedback(newFb: number) {
    const clamped = Math.max(0, Math.min(7, newFb));
    if (onParamChange) {
      onParamChange('feedback', clamped);
    }
  }
</script>

<div class="op-matrix-widget">
  <div class="matrix-header">
    <div class="badge-title">
      <span class="icon">∿</span>
      <strong>{descriptor.title}</strong>
    </div>

    <div class="controls-row">
      <label class="control-item">
        <span>Algoritmo:</span>
        <select value={algorithm} onchange={(e) => setAlgorithm(Number((e.target as HTMLSelectElement).value))}>
          {#each Array.from({ length: 32 }, (_, i) => i + 1) as algNum}
            <option value={algNum}>{algNum}</option>
          {/each}
        </select>
      </label>

      <label class="control-item">
        <span>Feedback (0-7):</span>
        <input 
          type="number" 
          min="0" 
          max="7" 
          value={feedback} 
          onchange={(e) => setFeedback(Number((e.target as HTMLInputElement).value))}
        />
      </label>
    </div>
  </div>

  <div class="diagram-container">
    <div class="operators-grid">
      {#each [6, 5, 4, 3, 2, 1] as opId}
        {@const isCarrier = routing.carriers.includes(opId)}
        {@const isFeedback = routing.feedbackOp === opId}
        {@const modulators = routing.modulators[opId] || []}
        <div 
          class="op-node" 
          class:carrier={isCarrier}
          class:modulator={!isCarrier}
          class:feedback-loop={isFeedback}
        >
          <div class="op-id-tag">Op {opId}</div>
          <div class="op-role">{isCarrier ? 'Portadora' : 'Modulador'}</div>
          {#if isFeedback && feedback > 0}
            <div class="fb-badge">FB {feedback}</div>
          {/if}
          {#if modulators.length > 0}
            <div class="mod-sources">← Op {modulators.join(', ')}</div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .op-matrix-widget {
    background: #091724;
    border: 1px solid #1c3d5a;
    border-radius: 6px;
    padding: 8px 12px;
    margin: 4px 0;
    color: #e5f1fb;
    font-family: Inter, monospace, sans-serif;
    font-size: 11px;
    user-select: none;
  }

  .matrix-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid #162f46;
    padding-bottom: 6px;
    margin-bottom: 8px;
  }

  .badge-title {
    display: flex;
    align-items: center;
    gap: 6px;
    color: #38bdf8;
  }

  .icon {
    font-size: 14px;
    font-weight: bold;
  }

  .controls-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .control-item {
    display: flex;
    align-items: center;
    gap: 5px;
    color: #94a3b8;
  }

  select, input[type="number"] {
    background: #06111b;
    border: 1px solid #234566;
    color: #fff;
    padding: 3px 6px;
    border-radius: 4px;
    font-size: 11px;
    outline: none;
  }

  select:focus, input:focus {
    border-color: #38bdf8;
  }

  .diagram-container {
    padding: 4px 0;
  }

  .operators-grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 6px;
  }

  .op-node {
    background: #0d2133;
    border: 1px solid #1e405f;
    border-radius: 5px;
    padding: 6px 4px;
    text-align: center;
    position: relative;
    transition: all 0.15s;
  }

  .op-node.carrier {
    background: rgba(56, 189, 248, 0.12);
    border-color: #38bdf8;
    color: #e0f2fe;
  }

  .op-node.modulator {
    background: rgba(168, 85, 247, 0.12);
    border-color: #a855f7;
    color: #f3e8ff;
  }

  .op-node.feedback-loop {
    box-shadow: 0 0 6px rgba(251, 146, 60, 0.35);
    border-color: #fb923c;
  }

  .op-id-tag {
    font-weight: bold;
    font-size: 11px;
  }

  .op-role {
    font-size: 9px;
    color: #94a3b8;
    margin-top: 2px;
  }

  .fb-badge {
    display: inline-block;
    background: rgba(251, 146, 60, 0.25);
    color: #fdba74;
    border: 1px solid #ea580c;
    border-radius: 3px;
    font-size: 8px;
    font-weight: bold;
    padding: 1px 3px;
    margin-top: 3px;
  }

  .mod-sources {
    font-size: 8px;
    color: #cbd5e1;
    margin-top: 2px;
  }
</style>

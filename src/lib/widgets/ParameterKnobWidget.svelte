<script lang="ts">
  import type { WidgetDescriptor } from './types';

  interface Props {
    descriptor: WidgetDescriptor;
    onParamChange?: (path: string, val: number | string | boolean) => void;
  }

  let { descriptor, onParamChange }: Props = $props();

  let opId = $derived(Number(descriptor.parameters.opId || 1));
  let outputLevel = $derived(Number(descriptor.parameters.outputLevel ?? 99));
  let freqCoarse = $derived(Number(descriptor.parameters.freqCoarse ?? 1));
  let freqFine = $derived(Number(descriptor.parameters.freqFine ?? 0));
  let detune = $derived(Number(descriptor.parameters.detune ?? 0));

  function updateParam(field: string, val: number) {
    if (onParamChange) {
      onParamChange(`op.${opId}.${field}`, Math.round(val));
    }
  }
</script>

<div class="param-knob-widget">
  <div class="knob-header">
    <strong>{descriptor.title}</strong>
    <span class="badge">Afinação e Nível</span>
  </div>

  <div class="knob-grid">
    <div class="knob-cell">
      <span class="cell-label">Nível Saída</span>
      <input 
        type="range" 
        min="0" 
        max="99" 
        value={outputLevel} 
        oninput={(e) => updateParam('outputLevel', Number((e.target as HTMLInputElement).value))}
      />
      <span class="cell-val">{outputLevel}</span>
    </div>

    <div class="knob-cell">
      <span class="cell-label">Razão Coarse</span>
      <input 
        type="number" 
        min="0" 
        max="31" 
        value={freqCoarse} 
        onchange={(e) => updateParam('freqCoarse', Number((e.target as HTMLInputElement).value))}
      />
      <span class="cell-val">{freqCoarse === 0 ? '0.5x' : `${freqCoarse}x`}</span>
    </div>

    <div class="knob-cell">
      <span class="cell-label">Fine (Cents)</span>
      <input 
        type="number" 
        min="0" 
        max="99" 
        value={freqFine} 
        onchange={(e) => updateParam('freqFine', Number((e.target as HTMLInputElement).value))}
      />
      <span class="cell-val">{freqFine}</span>
    </div>

    <div class="knob-cell">
      <span class="cell-label">Detune</span>
      <input 
        type="range" 
        min="-7" 
        max="7" 
        value={detune} 
        oninput={(e) => updateParam('detune', Number((e.target as HTMLInputElement).value))}
      />
      <span class="cell-val">{detune > 0 ? `+${detune}` : detune}</span>
    </div>
  </div>
</div>

<style>
  .param-knob-widget {
    background: #081624;
    border: 1px solid #1c3d5a;
    border-radius: 6px;
    padding: 8px 12px;
    margin: 4px 0;
    color: #e5f1fb;
    font-family: Inter, monospace, sans-serif;
    font-size: 11px;
    user-select: none;
  }

  .knob-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 6px;
    border-bottom: 1px solid #162f46;
    margin-bottom: 8px;
  }

  .badge {
    background: #11283d;
    color: #94a3b8;
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .knob-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }

  .knob-cell {
    background: #040d16;
    border: 1px solid #152d42;
    border-radius: 4px;
    padding: 6px 4px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .cell-label {
    font-size: 9px;
    color: #94a3b8;
  }

  .cell-val {
    font-size: 10px;
    font-weight: bold;
    color: #38bdf8;
    font-family: monospace;
  }

  input[type="range"] {
    width: 90%;
    accent-color: #38bdf8;
    cursor: pointer;
  }

  input[type="number"] {
    width: 44px;
    background: #06111b;
    border: 1px solid #234566;
    color: #fff;
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 10px;
    text-align: center;
    outline: none;
  }

  input[type="number"]:focus {
    border-color: #38bdf8;
  }
</style>

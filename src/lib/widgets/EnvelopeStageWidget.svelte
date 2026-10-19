<script lang="ts">
  import type { WidgetDescriptor } from './types';

  interface Props {
    descriptor: WidgetDescriptor;
    onParamChange?: (path: string, val: number | string | boolean) => void;
  }

  let { descriptor, onParamChange }: Props = $props();

  let opId = $derived(Number(descriptor.parameters.opId || 1));
  let r1 = $derived(Number(descriptor.parameters.r1 ?? 95));
  let r2 = $derived(Number(descriptor.parameters.r2 ?? 30));
  let r3 = $derived(Number(descriptor.parameters.r3 ?? 20));
  let r4 = $derived(Number(descriptor.parameters.r4 ?? 45));

  let l1 = $derived(Number(descriptor.parameters.l1 ?? 99));
  let l2 = $derived(Number(descriptor.parameters.l2 ?? 80));
  let l3 = $derived(Number(descriptor.parameters.l3 ?? 0));
  let l4 = $derived(Number(descriptor.parameters.l4 ?? 0));

  function updateRate(idx: number, val: number) {
    const clamped = Math.max(0, Math.min(99, Math.round(val)));
    if (onParamChange) {
      onParamChange(`op.${opId}.eg.r${idx}`, clamped);
    }
  }

  function updateLevel(idx: number, val: number) {
    const clamped = Math.max(0, Math.min(99, Math.round(val)));
    if (onParamChange) {
      onParamChange(`op.${opId}.eg.l${idx}`, clamped);
    }
  }

  // Coordenadas normalizadas para desenho da curva SVG (Width 320, Height 70)
  const W = 320;
  const H = 64;

  let x1 = $derived(20 + (100 - r1) * 0.5);
  let y1 = $derived(H - 4 - (l1 / 99) * (H - 8));

  let x2 = $derived(x1 + Math.max(15, (100 - r2) * 0.6));
  let y2 = $derived(H - 4 - (l2 / 99) * (H - 8));

  let x3 = $derived(x2 + Math.max(20, (100 - r3) * 0.8));
  let y3 = $derived(H - 4 - (l3 / 99) * (H - 8));

  let x4 = $derived(Math.min(W - 10, x3 + Math.max(15, (100 - r4) * 0.6)));
  let y4 = $derived(H - 4 - (l4 / 99) * (H - 8));
</script>

<div class="env-stage-widget">
  <div class="widget-header">
    <strong>{descriptor.title}</strong>
    <span class="badge">DX7 4-Rate 4-Level</span>
  </div>

  <div class="chart-container">
    <svg width={W} height={H} class="envelope-svg">
      <!-- Linhas de grade -->
      <line x1="0" y1={H - 4} x2={W} y2={H - 4} stroke="#1b3952" stroke-width="1" />
      <line x1="0" y1="4" x2={W} y2="4" stroke="#1b3952" stroke-dasharray="3,3" stroke-width="1" />

      <!-- Caminho do envelope -->
      <path 
        d="M 10,{H - 4} L {x1},{y1} L {x2},{y2} L {x3},{y3} L {x4},{y4}" 
        fill="none" 
        stroke="#38bdf8" 
        stroke-width="2" 
      />

      <!-- Preenchimento suave sob a curva -->
      <path 
        d="M 10,{H - 4} L {x1},{y1} L {x2},{y2} L {x3},{y3} L {x4},{y4} L {x4},{H - 4} Z" 
        fill="rgba(56, 189, 248, 0.12)" 
      />

      <!-- Ponto 1 (Ataque) -->
      <circle cx={x1} cy={y1} r="4" fill="#38bdf8" />
      <!-- Ponto 2 (Decay 1) -->
      <circle cx={x2} cy={y2} r="4" fill="#38bdf8" />
      <!-- Ponto 3 (Sustain L3) -->
      <circle cx={x3} cy={y3} r="4" fill="#fbbf24" />
      <!-- Ponto 4 (Release L4) -->
      <circle cx={x4} cy={y4} r="4" fill="#f87171" />
    </svg>
  </div>

  <div class="params-table">
    <div class="param-row">
      <span class="param-lbl">Taxas (R1..R4):</span>
      <div class="inputs-group">
        <label>R1 <input type="number" min="0" max="99" value={r1} onchange={(e) => updateRate(1, Number((e.target as HTMLInputElement).value))} /></label>
        <label>R2 <input type="number" min="0" max="99" value={r2} onchange={(e) => updateRate(2, Number((e.target as HTMLInputElement).value))} /></label>
        <label>R3 <input type="number" min="0" max="99" value={r3} onchange={(e) => updateRate(3, Number((e.target as HTMLInputElement).value))} /></label>
        <label>R4 <input type="number" min="0" max="99" value={r4} onchange={(e) => updateRate(4, Number((e.target as HTMLInputElement).value))} /></label>
      </div>
    </div>

    <div class="param-row">
      <span class="param-lbl">Níveis (L1..L4):</span>
      <div class="inputs-group">
        <label>L1 <input type="number" min="0" max="99" value={l1} onchange={(e) => updateLevel(1, Number((e.target as HTMLInputElement).value))} /></label>
        <label>L2 <input type="number" min="0" max="99" value={l2} onchange={(e) => updateLevel(2, Number((e.target as HTMLInputElement).value))} /></label>
        <label>L3 <input type="number" min="0" max="99" value={l3} onchange={(e) => updateLevel(3, Number((e.target as HTMLInputElement).value))} /></label>
        <label>L4 <input type="number" min="0" max="99" value={l4} onchange={(e) => updateLevel(4, Number((e.target as HTMLInputElement).value))} /></label>
      </div>
    </div>
  </div>
</div>

<style>
  .env-stage-widget {
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

  .widget-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 6px;
    border-bottom: 1px solid #162f46;
  }

  .badge {
    background: #11283d;
    color: #94a3b8;
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .chart-container {
    margin: 8px 0;
    background: #040c14;
    border: 1px solid #152d42;
    border-radius: 4px;
    padding: 4px;
    display: flex;
    justify-content: center;
  }

  .envelope-svg {
    display: block;
    max-width: 100%;
  }

  .params-table {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .param-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .param-lbl {
    color: #94a3b8;
    font-size: 10px;
    min-width: 90px;
  }

  .inputs-group {
    display: flex;
    gap: 8px;
  }

  .inputs-group label {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 10px;
    color: #cbd5e1;
  }

  input[type="number"] {
    width: 36px;
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

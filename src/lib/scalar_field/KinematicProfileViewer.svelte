<script lang="ts">
  import { onMount } from 'svelte';
  import type { KinematicPoint, ChainRuleVerificationResult } from './kinematic_chain_analyzer';

  interface Props {
    kinematics: KinematicPoint[];
    fieldValues: number[];
    verification: ChainRuleVerificationResult | null;
  }

  let { kinematics, fieldValues, verification }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);

  function drawProfile() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;

    // Fundo
    ctx.fillStyle = '#040d16';
    ctx.fillRect(0, 0, w, h);

    // Linha de centro / zero
    ctx.strokeStyle = '#18334b';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();

    const n = fieldValues.length;
    if (n < 2) return;

    // Escala min/max
    let minV = -1.0;
    let maxV = 1.0;
    for (const v of fieldValues) {
      if (v < minV) minV = v;
      if (v > maxV) maxV = v;
    }
    const span = Math.max(0.1, maxV - minV);

    // Curva de f(r(t)) em ciano
    ctx.strokeStyle = '#38bdf8';
    ctx.lineWidth = 2.0;
    ctx.beginPath();

    for (let i = 0; i < n; i++) {
      const x = (i / (n - 1)) * w;
      const normY = (fieldValues[i] - minV) / span;
      const y = h - normY * (h - 16) - 8;

      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();

    // Curva de velocidade ||r'(t)|| em âmbar pontilhado
    if (kinematics.length === n) {
      ctx.strokeStyle = '#fbbf24';
      ctx.lineWidth = 1.2;
      ctx.setLineDash([3, 3]);
      ctx.beginPath();

      for (let i = 0; i < n; i++) {
        const k = kinematics[i];
        const spd = Math.hypot(k.vx, k.vy);
        const x = (i / (n - 1)) * w;
        const y = h - Math.min(1.0, spd / 3.0) * (h - 20) - 10;

        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
      ctx.setLineDash([]);
    }
  }

  $effect(() => {
    if (fieldValues || kinematics) {
      drawProfile();
    }
  });

  onMount(() => {
    drawProfile();
  });
</script>

<div class="profile-viewer">
  <div class="header">
    <div class="title-group">
      <strong>Perfil 1D q(t) ao longo do traço</strong>
      <span class="legend">
        <span class="dot val"></span> f(r(t))
        <span class="dot spd"></span> ||r'(t)||
      </span>
    </div>

    {#if verification}
      <div class="verification-stats">
        <span class="stat">Erro 1ª Regra: <strong>{(verification.meanFirstOrderRelativeError * 100).toFixed(1)}%</strong></span>
        <span class="stat">Erro 2ª Regra (H_f): <strong>{(verification.meanSecondOrderRelativeError * 100).toFixed(1)}%</strong></span>
      </div>
    {/if}
  </div>

  <canvas bind:this={canvas} width={500} height={90} class="profile-canvas"></canvas>
</div>

<style>
  .profile-viewer {
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: #06111d;
    border: 1px solid #162c40;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11px;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .title-group strong {
    color: #e5f1fb;
  }

  .legend {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    color: #8ca5ba;
  }

  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .dot.val { background: #38bdf8; }
  .dot.spd { background: #fbbf24; }

  .verification-stats {
    display: flex;
    gap: 10px;
    font-size: 10px;
    color: #8ca5ba;
  }

  .stat strong {
    color: #22d69a;
  }

  .profile-canvas {
    width: 100%;
    height: 90px;
    display: block;
    border-radius: 4px;
  }
</style>

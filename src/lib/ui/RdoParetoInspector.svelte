<script lang="ts">
  import { onMount } from 'svelte';
  import PanelCard from './PanelCard.svelte';
  import PillTabs from './PillTabs.svelte';
  import { 
    wasm_rdo_encode, 
    wasm_rdo_decode, 
    wasm_rdo_compute_pareto 
  } from '../../wasm/core_wasm.js';

  interface Props {
    rawAudioBytes?: Uint8Array;
    onDecompressedAudio?: (wavBytes: Uint8Array) => void;
  }

  let { rawAudioBytes, onDecompressedAudio }: Props = $props();

  let lambdaVal = $state<number>(0.005);
  let deadZoneVal = $state<number>(0.08);
  let isLossless = $state<boolean>(false);
  let compressionStats = $state<{
    originalBytes: number;
    compressedBytes: number;
    ratio: number;
    rateKbps: number;
  } | null>(null);

  let paretoPoints = $state<Array<{ lambda: number; rate_kbps: number; rmse: number; active_dims: number }>>([]);
  let paretoCanvas = $state<HTMLCanvasElement | null>(null);
  let statusMsg = $state<string>('Pronto para otimização RDO');

  function updateParetoCurve() {
    if (!rawAudioBytes || rawAudioBytes.length < 44) return;
    try {
      const slice = rawAudioBytes.slice(0, Math.min(rawAudioBytes.length, 44 + 4096 * 2));
      const jsonStr = wasm_rdo_compute_pareto(slice);
      paretoPoints = JSON.parse(jsonStr);
      drawParetoCurve();
    } catch (e) {
      console.warn('Erro ao calcular curva de Pareto:', e);
    }
  }

  function drawParetoCurve() {
    if (!paretoCanvas) return;
    const ctx = paretoCanvas.getContext('2d');
    if (!ctx) return;

    const w = paretoCanvas.width;
    const h = paretoCanvas.height;

    ctx.fillStyle = '#03080e';
    ctx.fillRect(0, 0, w, h);

    // Grid de referência
    ctx.strokeStyle = '#0b1c2e';
    ctx.lineWidth = 1;
    for (let x = 40; x < w; x += 40) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, h - 20);
      ctx.stroke();
    }
    for (let y = 20; y < h - 20; y += 20) {
      ctx.beginPath();
      ctx.moveTo(40, y);
      ctx.lineTo(w, y);
      ctx.stroke();
    }

    if (paretoPoints.length < 2) return;

    const maxRate = Math.max(1, ...paretoPoints.map(p => p.rate_kbps)) * 1.1;
    const maxRmse = Math.max(0.01, ...paretoPoints.map(p => p.rmse)) * 1.1;

    // Curva de Pareto em Cyan
    ctx.strokeStyle = '#38bdf8';
    ctx.lineWidth = 2.5;
    ctx.beginPath();

    for (let i = 0; i < paretoPoints.length; i++) {
      const pt = paretoPoints[i];
      const px = 40 + (pt.rate_kbps / maxRate) * (w - 60);
      const py = (h - 25) - (pt.rmse / maxRmse) * (h - 45);

      if (i === 0) ctx.moveTo(px, py);
      else ctx.lineTo(px, py);
    }
    ctx.stroke();

    // Plota pontos com círculos e marca ponto de operação atual
    for (const pt of paretoPoints) {
      const px = 40 + (pt.rate_kbps / maxRate) * (w - 60);
      const py = (h - 25) - (pt.rmse / maxRmse) * (h - 45);

      ctx.fillStyle = '#2dd4bf';
      ctx.beginPath();
      ctx.arc(px, py, 4, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  function handleCompress() {
    if (!rawAudioBytes || rawAudioBytes.length === 0) {
      statusMsg = 'Nenhum áudio carregado para compressão.';
      return;
    }

    try {
      statusMsg = 'Comprimindo via WASM RDO-Jet / DRED...';
      const t0 = performance.now();
      const compressed = wasm_rdo_encode(rawAudioBytes, lambdaVal, deadZoneVal, isLossless);
      const elapsed = performance.now() - t0;

      const origLen = rawAudioBytes.length;
      const compLen = compressed.length;
      const ratio = origLen / Math.max(1, compLen);
      const durationSec = (origLen - 44) / (48000 * 2);
      const rateKbps = ((compLen * 8) / Math.max(0.001, durationSec)) / 1000.0;

      compressionStats = {
        originalBytes: origLen,
        compressedBytes: compLen,
        ratio,
        rateKbps
      };

      statusMsg = `Concluído em ${elapsed.toFixed(1)}ms (${ratio.toFixed(1)}x)`;
    } catch (e: any) {
      statusMsg = `Erro na compressão: ${e?.message || e}`;
    }
  }

  function handleDecodeAndPlay() {
    if (!rawAudioBytes) return;
    try {
      const compressed = wasm_rdo_encode(rawAudioBytes, lambdaVal, deadZoneVal, isLossless);
      const decompressed = wasm_rdo_decode(compressed);
      onDecompressedAudio?.(decompressed);
      statusMsg = `Decodificado: ${decompressed.length} bytes (WAV Bit-Perfect: ${isLossless ? 'SIM' : 'Lossy DRED'})`;
    } catch (e: any) {
      statusMsg = `Erro na decodificação: ${e?.message || e}`;
    }
  }

  $effect(() => {
    if (rawAudioBytes) {
      updateParetoCurve();
    }
  });

  onMount(() => {
    updateParetoCurve();
  });
</script>

<PanelCard title="Rate-Distortion Optimization & Codec DRED" icon="📉" badge="Fase 7 MDL">
  <div class="rdo-inspector">
    <!-- Gráfico 2D da Curva de Pareto -->
    <div class="graph-container">
      <div class="graph-header">
        <strong>Fronteira de Pareto (Distorção vs Taxa de Bits)</strong>
        <span class="legend">Eixo X: Taxa (kbps) | Eixo Y: RMSE</span>
      </div>
      <canvas bind:this={paretoCanvas} width={420} height={120} class="pareto-canvas"></canvas>
    </div>

    <!-- Controles de Parâmetros -->
    <div class="controls-grid">
      <div class="ctrl-item">
        <label for="lambda-range">Penalidade Lagrangeana λ: <strong>{lambdaVal.toExponential(2)}</strong></label>
        <input 
          id="lambda-range"
          type="range" 
          min="0.0001" 
          max="0.05" 
          step="0.0005" 
          bind:value={lambdaVal} 
        />
      </div>

      <div class="ctrl-item">
        <label for="deadzone-range">Dead-Zone δ: <strong>{deadZoneVal.toFixed(2)}</strong></label>
        <input 
          id="deadzone-range"
          type="range" 
          min="0.01" 
          max="0.20" 
          step="0.01" 
          bind:value={deadZoneVal} 
        />
      </div>

      <div class="ctrl-item">
        <span class="mode-label">Modo de Compressão:</span>
        <div class="toggle-group">
          <button 
            class:active={!isLossless} 
            onclick={() => isLossless = false}
          >
            Lossy DRED (&lt;16 kbps)
          </button>
          <button 
            class:active={isLossless} 
            onclick={() => isLossless = true}
          >
            Bit-Exact Lossless
          </button>
        </div>
      </div>
    </div>

    <!-- Ações e Estatísticas -->
    <div class="actions-row">
      <button class="primary-btn" onclick={handleCompress}>Comprimir Bitstream</button>
      <button class="secondary-btn" onclick={handleDecodeAndPlay}>Decodificar & Ouvir</button>
      <span class="status font-mono">{statusMsg}</span>
    </div>

    {#if compressionStats}
      <div class="stats-card">
        <div class="stat">Original: <strong>{(compressionStats.originalBytes / 1024).toFixed(1)} KB</strong></div>
        <div class="stat">DRED: <strong>{(compressionStats.compressedBytes / 1024).toFixed(1)} KB</strong></div>
        <div class="stat">Compressão: <strong class="highlight">{compressionStats.ratio.toFixed(1)}x</strong></div>
        <div class="stat">Taxa: <strong>{compressionStats.rateKbps.toFixed(1)} kbps</strong></div>
      </div>
    {/if}
  </div>
</PanelCard>

<style>
  .rdo-inspector {
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 11px;
    user-select: none;
  }

  .graph-container {
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    border-radius: 5px;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .graph-header {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
  }

  .legend {
    color: var(--text-secondary);
  }

  .pareto-canvas {
    width: 100%;
    height: 120px;
    display: block;
    border-radius: 4px;
  }

  .controls-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 12px;
  }

  .ctrl-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .mode-label {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .ctrl-item label {
    color: var(--text-secondary);
  }

  .ctrl-item strong {
    color: var(--accent-cyan);
  }

  .toggle-group {
    display: flex;
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    overflow: hidden;
  }

  .toggle-group button {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 4px 6px;
    font-size: 10px;
    cursor: pointer;
  }

  .toggle-group button.active {
    background: var(--border-hover);
    color: var(--text-primary);
    font-weight: 600;
  }

  .actions-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .primary-btn {
    background: #183e60;
    border: 1px solid #2a5a84;
    color: #fff;
    padding: 5px 12px;
    border-radius: 4px;
    font-weight: 600;
    cursor: pointer;
  }

  .secondary-btn {
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    color: var(--text-primary);
    padding: 5px 12px;
    border-radius: 4px;
    cursor: pointer;
  }

  .status {
    color: var(--text-secondary);
    font-size: 10px;
  }

  .stats-card {
    display: flex;
    gap: 16px;
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 11px;
  }

  .stat strong {
    color: var(--text-primary);
  }

  .stat strong.highlight {
    color: var(--accent-green);
  }
</style>

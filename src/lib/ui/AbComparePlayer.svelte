<script lang="ts">
  import { computeCompressionMetrics, type CompressionMetrics } from './ab_player_logic';

  interface Props {
    originalLength?: number;
    processedLength?: number;
    activeMode?: 'A' | 'B';
    isPlaying?: boolean;
    onToggleMode?: (mode: 'A' | 'B') => void;
    onPlayPause?: () => void;
    onDownloadOriginal?: () => void;
    onDownloadProcessed?: () => void;
  }

  let {
    originalLength = 0,
    processedLength = 0,
    activeMode = 'A',
    isPlaying = false,
    onToggleMode,
    onPlayPause,
    onDownloadOriginal,
    onDownloadProcessed
  }: Props = $props();

  let metrics = $derived<CompressionMetrics>(
    computeCompressionMetrics(originalLength, processedLength)
  );
</script>

<div class="ab-compare-card" role="region" aria-label="Comparador de Áudio A/B">
  <div class="header-row">
    <div class="title-group">
      <span class="icon">⚖️</span>
      <strong>Comparador A/B</strong>
    </div>

    {#if originalLength > 0 && processedLength > 0}
      <span class="metrics-badge font-mono" class:savings={metrics.ratio > 1.05}>
        {metrics.formattedSavings}
      </span>
    {/if}
  </div>

  <div class="controls-row">
    <!-- Seletor A / B -->
    <div class="ab-toggle-group" role="radiogroup" aria-label="Canal de reprodução">
      <button
        type="button"
        class="ab-btn"
        class:active={activeMode === 'A'}
        onclick={() => onToggleMode?.('A')}
        aria-checked={activeMode === 'A'}
        role="radio"
      >
        <span class="badge-letter">A</span>
        <span>Original</span>
        {#if originalLength > 0}
          <small class="size-hint font-mono">{(originalLength / 1024).toFixed(0)} KB</small>
        {/if}
      </button>

      <button
        type="button"
        class="ab-btn"
        class:active={activeMode === 'B'}
        onclick={() => onToggleMode?.('B')}
        aria-checked={activeMode === 'B'}
        role="radio"
      >
        <span class="badge-letter">B</span>
        <span>Processado</span>
        {#if processedLength > 0}
          <small class="size-hint font-mono">{(processedLength / 1024).toFixed(0)} KB</small>
        {/if}
      </button>
    </div>

    <!-- Botão Play / Pause Global -->
    <button
      type="button"
      class="play-btn"
      class:playing={isPlaying}
      onclick={() => onPlayPause?.()}
      aria-label={isPlaying ? 'Pausar áudio' : 'Reproduzir áudio'}
    >
      <span class="play-icon">{isPlaying ? '⏸' : '▶'}</span>
      <span>{isPlaying ? 'Pausar' : 'Ouvir'}</span>
    </button>
  </div>

  <div class="actions-row">
    {#if onDownloadOriginal && originalLength > 0}
      <button type="button" class="action-link" onclick={() => onDownloadOriginal()}>
        💾 Baixar Original (A)
      </button>
    {/if}

    {#if onDownloadProcessed && processedLength > 0}
      <button type="button" class="action-link accent" onclick={() => onDownloadProcessed()}>
        ✨ Baixar Comprimido (B)
      </button>
    {/if}
  </div>
</div>

<style>
  .ab-compare-card {
    background: var(--bg-surface, #0f172a);
    border: 1px solid var(--border-subtle, #334155);
    border-radius: 8px;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    color: var(--text-main, #f8fafc);
  }

  .header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
  }

  .metrics-badge {
    font-size: 0.72rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    background: var(--bg-elevated, #1e293b);
    color: var(--text-muted, #94a3b8);
    border: 1px solid var(--border-subtle, #334155);
  }

  .metrics-badge.savings {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border-color: rgba(16, 185, 129, 0.3);
  }

  .controls-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .ab-toggle-group {
    display: flex;
    background: var(--bg-base, #020617);
    padding: 2px;
    border-radius: 6px;
    border: 1px solid var(--border-subtle, #334155);
  }

  .ab-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: transparent;
    border: none;
    color: var(--text-muted, #94a3b8);
    padding: 0.35rem 0.65rem;
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .ab-btn:hover {
    color: var(--text-main, #f8fafc);
  }

  .ab-btn.active {
    background: var(--bg-surface, #1e293b);
    color: var(--accent-cyan, #38bdf8);
    font-weight: 600;
  }

  .badge-letter {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--bg-elevated, #334155);
    font-size: 0.65rem;
    font-weight: 700;
  }

  .ab-btn.active .badge-letter {
    background: var(--accent-cyan, #38bdf8);
    color: #020617;
  }

  .size-hint {
    font-size: 0.65rem;
    opacity: 0.8;
  }

  .play-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: var(--accent-blue, #0284c7);
    color: white;
    border: none;
    padding: 0.35rem 0.85rem;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .play-btn:hover {
    background: #0369a1;
  }

  .play-btn.playing {
    background: #e11d48;
  }

  .play-icon {
    font-size: 0.75rem;
  }

  .actions-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.72rem;
  }

  .action-link {
    background: none;
    border: none;
    color: var(--text-muted, #94a3b8);
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    transition: color 0.15s ease;
  }

  .action-link:hover {
    color: var(--text-main, #f8fafc);
  }

  .action-link.accent {
    color: var(--accent-cyan, #38bdf8);
  }
</style>

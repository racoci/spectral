<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { TOOL_ROUTES, resolveRouteFromShortcut, type NavigationRoute } from './navigation_helpers';

  interface Props {
    currentView: 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field';
    isAudioPlaying?: boolean;
    hasAudioLoaded?: boolean;
    onNavigate: (view: 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field') => void;
  }

  let {
    currentView,
    isAudioPlaying = false,
    hasAudioLoaded = false,
    onNavigate
  }: Props = $props();

  function handleKeyDown(e: KeyboardEvent) {
    const activeEl = document.activeElement;
    const resolved = resolveRouteFromShortcut({
      altKey: e.altKey,
      key: e.key,
      targetTagName: activeEl?.tagName,
      targetClassName: activeEl?.className
    });

    if (resolved) {
      e.preventDefault();
      onNavigate(resolved.id);
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeyDown);
  });
</script>

<header class="global-topbar">
  <div class="brand">
    <div class="brand-logo">🎨🔊</div>
    <div class="brand-text">
      <span class="brand-title">Spectral</span>
      <span class="brand-badge">Lossless v0.1</span>
    </div>
  </div>

  <div class="divider"></div>

  <!-- Botoes das 6 Ferramentas -->
  <nav class="tool-nav" aria-label="Navegação entre Ferramentas">
    {#each TOOL_ROUTES as route}
      {@const isActive = route.id === currentView}
      {@const isDisabled = route.id === 'editor' && !hasAudioLoaded}
      <button
        type="button"
        class="nav-item"
        class:active={isActive}
        disabled={isDisabled}
        onclick={() => onNavigate(route.id)}
        title="{route.label} ({route.shortcut})"
      >
        <span class="icon">{@html route.iconSvg}</span>
        <span class="label">{route.label}</span>
        <kbd class="shortcut">{route.shortcut}</kbd>
      </button>
    {/each}
  </nav>

  <div class="grow"></div>

  <!-- Indicador de Áudio Ativo -->
  {#if isAudioPlaying}
    <div class="audio-active-badge">
      <span class="pulse-dot"></span>
      <span>ÁUDIO ATIVO</span>
    </div>
  {/if}
</header>

<style>
  .global-topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 48px;
    padding: 0 16px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-subtle);
    user-select: none;
    z-index: 50;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .brand-logo {
    font-size: 16px;
    line-height: 1;
  }

  .brand-text {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .brand-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .brand-badge {
    font-size: 9px;
    color: var(--accent-cyan);
    font-family: var(--font-mono);
    background: var(--bg-input);
    border: 1px solid var(--border-subtle);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .divider {
    width: 1px;
    height: 22px;
    background: var(--border-subtle);
  }

  .tool-nav {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-secondary);
    padding: 4px 8px;
    border-radius: 5px;
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease-in-out;
  }

  .nav-item:hover:not(:disabled) {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
  }

  .nav-item.active {
    background: var(--border-hover);
    color: var(--text-primary);
    border-color: var(--border-active);
    font-weight: 600;
  }

  .nav-item:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 14px;
    height: 14px;
  }

  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }

  .shortcut {
    font-size: 9px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    padding: 0 3px;
    border-radius: 2px;
  }

  .grow { flex: 1; }

  .audio-active-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(34, 214, 154, 0.1);
    border: 1px solid var(--accent-green);
    color: var(--accent-green);
    font-size: 9px;
    font-family: var(--font-mono);
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-green);
    box-shadow: 0 0 6px var(--accent-green);
    animation: pulse 1.2s infinite ease-in-out;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.4; transform: scale(0.8); }
  }
</style>

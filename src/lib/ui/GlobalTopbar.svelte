<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { TOOL_ROUTES, resolveRouteFromShortcut } from './navigation_helpers';
  import { DEMO_SAMPLES, type DemoSampleId } from './demo_samples';
  import type { UiMode } from './ui_glossary';

  interface Props {
    currentView: 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field';
    isAudioPlaying?: boolean;
    hasAudioLoaded?: boolean;
    uiMode?: UiMode;
    onNavigate: (view: 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field') => void;
    onToggleUiMode?: (mode: UiMode) => void;
    onSelectDemo?: (demoId: DemoSampleId) => void;
    onOpenWelcomeModal?: () => void;
    onShareSession?: () => void;
  }

  let {
    currentView,
    isAudioPlaying = false,
    hasAudioLoaded = false,
    uiMode = 'creative',
    onNavigate,
    onToggleUiMode,
    onSelectDemo,
    onOpenWelcomeModal,
    onShareSession
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

  <div class="divider"></div>

  <!-- Barra de Demonstrações Rápidas de 1 Clique -->
  <div class="quick-demos-wrap" aria-label="Demonstrações de Áudio">
    <span class="demos-label">Demos:</span>
    {#each DEMO_SAMPLES as demo}
      <button
        type="button"
        class="demo-chip"
        onclick={() => onSelectDemo?.(demo.id)}
        title="Carregar demonstração: {demo.description}"
      >
        <span>{demo.icon}</span>
        <span>{demo.label}</span>
      </button>
    {/each}
  </div>

  <div class="grow"></div>

  <!-- Alternador de Modo [Criativo 🎨 / Expert 🔬] -->
  <div class="mode-toggle-wrap" role="radiogroup" aria-label="Modo de Visualização">
    <button
      type="button"
      class="mode-btn"
      class:active={uiMode === 'creative'}
      onclick={() => onToggleUiMode?.('creative')}
      title="Modo Criativo: terminologia sonora intuitiva para músicos e designers"
    >
      🎨 Criativo
    </button>
    <button
      type="button"
      class="mode-btn"
      class:active={uiMode === 'expert'}
      onclick={() => onToggleUiMode?.('expert')}
      title="Modo Expert: terminologia matemática rigorosa e fórmulas"
    >
      🔬 Expert
    </button>
  </div>

  <!-- Botão de Ajuda / Onboarding Tour -->
  {#if onOpenWelcomeModal}
    <button
      type="button"
      class="help-tour-btn"
      onclick={() => onOpenWelcomeModal()}
      title="Abrir introdução rápida e atalhos de teclado"
      aria-label="Abrir introdução"
    >
      ❓ Guia
    </button>
  {/if}

  <!-- Botão de Compartilhar Sessão -->
  {#if onShareSession}
    <button
      type="button"
      class="share-btn"
      onclick={() => onShareSession()}
      title="Compartilhar sessão: copia link com a configuração atual"
      aria-label="Compartilhar sessão atual"
    >
      🔗 Compartilhar
    </button>
  {/if}

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
    gap: 10px;
    height: 48px;
    padding: 0 16px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-subtle);
    user-select: none;
    z-index: 50;
    flex-wrap: nowrap;
    overflow-x: auto;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
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
    flex-shrink: 0;
  }

  .tool-nav {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
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
    white-space: nowrap;
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

  .quick-demos-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .demos-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
  }

  .demo-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--bg-input, #1e293b);
    border: 1px solid var(--border-subtle);
    color: var(--text-primary);
    padding: 3px 7px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .demo-chip:hover {
    border-color: var(--accent-cyan);
    background: rgba(56, 189, 248, 0.15);
  }

  .grow { flex: 1; min-width: 8px; }

  .mode-toggle-wrap {
    display: flex;
    align-items: center;
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    padding: 2px;
    border-radius: 5px;
    flex-shrink: 0;
  }

  .mode-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 3px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.15s ease;
  }

  .mode-btn:hover {
    color: var(--text-primary);
  }

  .mode-btn.active {
    background: var(--bg-surface);
    color: var(--accent-cyan);
    font-weight: 600;
  }

  .help-tour-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    padding: 3px 8px;
    border-radius: 5px;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .help-tour-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }

  .share-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: rgba(56, 189, 248, 0.1);
    border: 1px solid var(--accent-cyan);
    color: var(--accent-cyan);
    padding: 3px 8px;
    border-radius: 5px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .share-btn:hover {
    background: rgba(56, 189, 248, 0.25);
  }

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
    flex-shrink: 0;
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

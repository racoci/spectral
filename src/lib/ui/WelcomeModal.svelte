<script lang="ts">
  import { DEMO_SAMPLES, type DemoSampleId } from './demo_samples';
  import { setOnboardingDismissed } from './ab_player_logic';

  interface Props {
    isOpen?: boolean;
    onClose?: () => void;
    onSelectDemo?: (demoId: DemoSampleId) => void;
  }

  let {
    isOpen = false,
    onClose,
    onSelectDemo
  }: Props = $props();

  let dontShowAgain = $state(false);

  function handleClose() {
    if (dontShowAgain) {
      setOnboardingDismissed(true);
    }
    onClose?.();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      handleClose();
    }
  }

  function handleDemoClick(demoId: DemoSampleId) {
    if (dontShowAgain) {
      setOnboardingDismissed(true);
    }
    onSelectDemo?.(demoId);
    onClose?.();
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if isOpen}
  <div class="modal-backdrop" onclick={handleClose} role="presentation">
    <div
      class="modal-card"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      onclick={(e) => e.stopPropagation()}
    >
      <header class="modal-header">
        <div class="title-wrap">
          <span class="hero-icon">🎨🔊</span>
          <div>
            <h2 id="modal-title">Bem-vindo ao Spectral</h2>
            <p class="subtitle">Laboratório sonoro de alta fidelidade e arte visual</p>
          </div>
        </div>

        <button type="button" class="close-btn" onclick={handleClose} aria-label="Fechar introdução">
          ✕
        </button>
      </header>

      <div class="modal-body">
        <div class="features-grid">
          <div class="feature-item">
            <span class="feat-icon">👁️</span>
            <div>
              <strong>Veja o Som com Foco Infinito</strong>
              <p>O espectrograma quântico transforma borrões sonoros em linhas nítidas e brilhantes.</p>
            </div>
          </div>

          <div class="feature-item">
            <span class="feat-icon">🗜️</span>
            <div>
              <strong>Super Compressão em Áudio</strong>
              <p>Reduza o tamanho em até 34x (&lt; 16 kbps) ou guarde o áudio como imagem PNG física 100% sem perdas.</p>
            </div>
          </div>

          <div class="feature-item">
            <span class="feat-icon">🎨</span>
            <div>
              <strong>Pinte e Esculpa Timbres</strong>
              <p>Desenhe relevos matemáticos de Taylor na tela e ouça o sintetizador tocar a paisagem.</p>
            </div>
          </div>
        </div>

        <div class="demos-section">
          <span class="section-label">Experimente agora com 1 clique (sem precisar enviar arquivos):</span>
          <div class="demo-buttons">
            {#each DEMO_SAMPLES as demo}
              <button
                type="button"
                class="demo-chip-btn"
                onclick={() => handleDemoClick(demo.id)}
              >
                <span class="chip-icon">{demo.icon}</span>
                <div class="chip-text">
                  <strong>{demo.label}</strong>
                  <small>{demo.category}</small>
                </div>
              </button>
            {/each}
          </div>
        </div>

        <div class="shortcut-tip">
          <span>⌨️ <strong>Dica de Navegação:</strong> Pressione <code>Alt + 1</code> a <code>Alt + 6</code> para alternar rapidamente entre as 6 ferramentas.</span>
        </div>
      </div>

      <footer class="modal-footer">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={dontShowAgain} />
          <span>Não exibir esta introdução novamente</span>
        </label>

        <button type="button" class="start-btn" onclick={handleClose}>
          🚀 Começar a Explorar
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(2, 6, 23, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 1rem;
    animation: fadeIn 0.2s ease-out;
  }

  .modal-card {
    background: var(--bg-surface, #0f172a);
    border: 1px solid var(--border-subtle, #334155);
    border-radius: 12px;
    max-width: 600px;
    width: 100%;
    color: var(--text-main, #f8fafc);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-subtle, #334155);
    background: var(--bg-elevated, #1e293b);
  }

  .title-wrap {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .hero-icon {
    font-size: 2rem;
  }

  h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
  }

  .subtitle {
    margin: 0.15rem 0 0 0;
    font-size: 0.8rem;
    color: var(--text-muted, #94a3b8);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #94a3b8);
    font-size: 1.25rem;
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 4px;
    line-height: 1;
    transition: color 0.15s ease;
  }

  .close-btn:hover {
    color: var(--text-main, #f8fafc);
  }

  .modal-body {
    padding: 1.25rem 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }

  .features-grid {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .feature-item {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    background: var(--bg-base, #020617);
    padding: 0.65rem 0.85rem;
    border-radius: 8px;
    border: 1px solid var(--border-subtle, #1e293b);
  }

  .feat-icon {
    font-size: 1.4rem;
    flex-shrink: 0;
  }

  .feature-item strong {
    font-size: 0.85rem;
    display: block;
    color: var(--accent-cyan, #38bdf8);
  }

  .feature-item p {
    margin: 0.2rem 0 0 0;
    font-size: 0.75rem;
    color: var(--text-muted, #94a3b8);
    line-height: 1.35;
  }

  .demos-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .section-label {
    font-size: 0.75rem;
    color: var(--text-muted, #94a3b8);
    font-weight: 500;
  }

  .demo-buttons {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.5rem;
  }

  .demo-chip-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-elevated, #1e293b);
    border: 1px solid var(--border-subtle, #334155);
    border-radius: 8px;
    padding: 0.5rem 0.65rem;
    cursor: pointer;
    color: var(--text-main, #f8fafc);
    text-align: left;
    transition: all 0.15s ease;
  }

  .demo-chip-btn:hover {
    border-color: var(--accent-cyan, #38bdf8);
    background: #253349;
    transform: translateY(-1px);
  }

  .chip-icon {
    font-size: 1.25rem;
  }

  .chip-text strong {
    display: block;
    font-size: 0.78rem;
  }

  .chip-text small {
    display: block;
    font-size: 0.65rem;
    color: var(--text-muted, #94a3b8);
  }

  .shortcut-tip {
    background: rgba(56, 189, 248, 0.1);
    border: 1px solid rgba(56, 189, 248, 0.2);
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    font-size: 0.72rem;
    color: var(--text-muted, #94a3b8);
  }

  .shortcut-tip code {
    background: rgba(0, 0, 0, 0.4);
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    font-size: 0.7rem;
    color: var(--accent-cyan, #38bdf8);
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--border-subtle, #334155);
    background: var(--bg-elevated, #1e293b);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.75rem;
    color: var(--text-muted, #94a3b8);
    cursor: pointer;
  }

  .start-btn {
    background: var(--accent-blue, #0284c7);
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.5rem 1.25rem;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .start-btn:hover {
    background: #0369a1;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes slideUp {
    from { transform: translateY(12px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
</style>

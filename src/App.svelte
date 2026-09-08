<script lang="ts">
  import AudioConverter from './lib/AudioConverter.svelte';
</script>

<main class="app-container">
  <header class="app-header">
    <div class="logo-area">
      <span class="icon-brand">🎨🔊</span>
      <h1>Spectral</h1>
      <span class="badge-tag">Lossless Sandbox v0.1</span>
    </div>
    <p class="tagline">
      Conversão bit-a-bit perfeitamente reversível de áudio em imagens
    </p>
  </header>

  <section class="main-content">
    <AudioConverter />
  </section>

  <footer class="app-footer">
    <p>
      Desenvolvido em Rust (WebAssembly) & Svelte 5. Implantado via GitHub Pages.
    </p>
  </footer>
</main>

<style>
  .app-container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 1.5rem;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .app-header {
    border-bottom: 1px solid #1e293b;
    padding-bottom: 1rem;
    margin-bottom: 1.5rem;
  }

  .logo-area {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .icon-brand {
    font-size: 2rem;
  }

  h1 {
    font-size: 2.25rem;
    margin: 0;
    font-weight: 800;
    letter-spacing: -0.025em;
    background: linear-gradient(135deg, #38bdf8 0%, #3b82f6 100%);
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .badge-tag {
    background-color: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.2rem 0.5rem;
    border-radius: 9999px;
  }

  .tagline {
    margin-top: 0.35rem;
    margin-bottom: 0;
    color: #94a3b8;
    font-size: 1rem;
  }

  .main-content {
    flex: 1;
  }

  .app-footer {
    margin-top: 3rem;
    border-top: 1px solid #1e293b;
    padding-top: 1rem;
    text-align: center;
    font-size: 0.8rem;
    color: #64748b;
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import AudioConverter from './lib/AudioConverter.svelte';
  import WebGlEditor from './lib/WebGlEditor.svelte';
  import init, { wasm_calculate_complex_reassigned_spectrogram } from './wasm/core_wasm.js';

  let currentView = $state<'converter' | 'editor'>('converter');
  let complexGrid = $state<Float32Array | null>(null);
  let gridW = $state(0);
  let gridH = $state(0);

  onMount(async () => {
    try {
      await init();
      console.log('📢 WebAssembly initialized successfully in App.svelte root!');
    } catch (e) {
      console.error('❌ Failed to initialize WebAssembly in App.svelte root:', e);
    }
  });

  // When a file is loaded and converted, we can trigger the editor view
  function handleAudioLoaded(data: Uint8Array, h: number) {
    console.log('📢 App.svelte handleAudioLoaded callback received data with length:', data?.length, 'height:', h);
    // Generate the full complex reassigned spectrogram in Rust
    try {
      const t0 = performance.now();
      complexGrid = wasm_calculate_complex_reassigned_spectrogram(data, h, 'hann') as Float32Array;
      gridH = h;
      gridW = (complexGrid.length / 2) / h;
      console.log(`✅ App.svelte generated complexGrid of size ${complexGrid.length} floats (dimensions: ${gridW} x ${gridH}) in ${(performance.now() - t0).toFixed(3)} ms.`);
      currentView = 'editor';
    } catch (e) {
      console.error("❌ App.svelte failed to generate WebGL Editor payload:", e);
    }
  }
</script>

<main class="app-container">
  <header class="app-header">
    <div class="logo-area">
      <span class="icon-brand">🎨🔊</span>
      <h1>Spectral</h1>
      <span class="badge-tag">Lossless Sandbox v0.1</span>
      
      <div class="view-toggles" style="margin-left: auto;">
        <button class:active={currentView === 'converter'} onclick={() => currentView = 'converter'}>1. Converter</button>
        <button class:active={currentView === 'editor'} onclick={() => currentView = 'editor'} disabled={!complexGrid}>2. WebGL Editor</button>
      </div>
    </div>
    <p class="tagline">
      Conversão bit-a-bit perfeitamente reversível de áudio em imagens
    </p>
  </header>

  <section class="main-content">
    {#if currentView === 'converter'}
      <AudioConverter onAudioLoaded={handleAudioLoaded} />
    {:else if currentView === 'editor'}
      <WebGlEditor complexGrid={complexGrid} width={gridW} height={gridH} />
    {/if}
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

  .view-toggles {
    display: flex;
    gap: 0.5rem;
  }

  .view-toggles button {
    background-color: #1e293b;
    border: 1px solid #334155;
    color: #cbd5e1;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.85rem;
    transition: all 0.2s;
  }

  .view-toggles button:hover:not(:disabled) {
    background-color: #334155;
  }

  .view-toggles button.active {
    background-color: #0284c7;
    border-color: #0284c7;
    color: white;
  }

  .view-toggles button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>

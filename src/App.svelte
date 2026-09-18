<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import AudioConverter from './lib/AudioConverter.svelte';
  import WebGlEditor from './lib/WebGlEditor.svelte';
  import init, { wasm_generate_complex_reassigned_ycbcr_spectrogram } from './wasm/core_wasm.js';

  let currentView = $state<'converter' | 'editor'>('converter');
  
  // Globally preserved state
  let originalBytes = $state<Uint8Array | null>(null);
  let selectedHeight = $state<number>(1024);
  
  let rgbaGrid = $state<Uint8Array | null>(null);
  let gridW = $state(0);
  let gridH = $state(0);

  // Global Audio Looping State
  let originalAudio = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let selectionStart = $state<number | null>(null); // normalized [0, 1]
  let selectionEnd = $state<number | null>(null);   // normalized [0, 1]
  let loopEnabled = $state(true);
  let playbackTimer: any = null;

  // Hash-based client router
  function updateRoute() {
    const hash = window.location.hash;
    if (hash === '#/editor' && rgbaGrid) {
      currentView = 'editor';
    } else {
      currentView = 'converter';
      if (window.location.hash !== '#/converter') {
        window.location.hash = '#/converter';
      }
    }
  }

  onMount(async () => {
    try {
      await init();
      console.log('📢 WebAssembly initialized successfully in App.svelte root!');
      
      // Preload default sample globally on startup so that BOTH the converter and editor are instantly active!
      if (!originalBytes) {
        await loadDefaultSample();
      }
      
      window.addEventListener('hashchange', updateRoute);
      updateRoute();
    } catch (e) {
      console.error('❌ Failed to initialize WebAssembly in App.svelte root:', e);
    }
  });

  onDestroy(() => {
    if (playbackTimer) clearInterval(playbackTimer);
    if (originalAudio) {
      originalAudio.pause();
    }
  });

  // Asynchronously fetch and preload the default voice sample globally
  async function loadDefaultSample() {
    try {
      const defaultUrl = './voice.wav';
      console.log('📢 Preloading default sample globally in App.svelte on startup:', defaultUrl);
      const response = await fetch(defaultUrl);
      if (response.ok) {
        const arrayBuffer = await response.arrayBuffer();
        const bytes = new Uint8Array(arrayBuffer);
        originalBytes = bytes;
        handleAudioLoaded(bytes, selectedHeight);
      } else {
        console.error("Failed to fetch default sample in App.svelte:", response.statusText);
      }
    } catch (e) {
      console.error("❌ Failed to load default sample globally in App.svelte:", e);
    }
  }

  // When a file is loaded and converted, we generate the complex grid and initialize global Audio element
  function handleAudioLoaded(data: Uint8Array, h: number) {
    console.log('📢 App.svelte handleAudioLoaded callback received data with length:', data?.length, 'height:', h);
    try {
      const t0 = performance.now();
      rgbaGrid = wasm_generate_complex_reassigned_ycbcr_spectrogram(data, h, 'hann') as Uint8Array;
      gridH = h;
      gridW = (rgbaGrid.length / 4) / h;
      console.log(`✅ App.svelte generated rgbaGrid of size ${rgbaGrid.length} bytes (dimensions: ${gridW} x ${gridH}) in ${(performance.now() - t0).toFixed(3)} ms.`);
      
      // Initialize or rebuild global Audio element
      if (originalAudio) {
        originalAudio.pause();
      }
      const blob = new Blob([data as any], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      originalAudio = new Audio(url);
      originalAudio.onended = () => {
        isPlaying = false;
        if (playbackTimer) clearInterval(playbackTimer);
      };
      
      // Clear selection bounds upon loading new audio
      selectionStart = null;
      selectionEnd = null;
    } catch (e) {
      console.error("❌ App.svelte failed to generate WebGL Editor payload:", e);
    }
  }

  // High-Resolution 15ms DAW-Loop Controller
  function triggerAudioPlayback() {
    if (!originalAudio) return;
    
    if (isPlaying) {
      // Pause
      originalAudio.pause();
      isPlaying = false;
      if (playbackTimer) clearInterval(playbackTimer);
    } else {
      // Play
      isPlaying = true;
      if (playbackTimer) clearInterval(playbackTimer);
      
      const duration = originalAudio.duration || 1.0;
      const startSec = selectionStart !== null ? selectionStart * duration : 0.0;
      const endSec = selectionEnd !== null ? selectionEnd * duration : duration;
      
      // Set start point
      originalAudio.currentTime = startSec;
      originalAudio.play();
      
      playbackTimer = setInterval(() => {
        if (!originalAudio) return;
        
        const current = originalAudio.currentTime;
        // Seamless loop boundaries check
        if (current >= endSec || current >= duration) {
          if (loopEnabled) {
            originalAudio.currentTime = startSec;
          } else {
            originalAudio.pause();
            isPlaying = false;
            if (playbackTimer) clearInterval(playbackTimer);
          }
        }
      }, 15); // Tight 15ms interval for seamless gaps!
    }
  }

  // Handle direct file uploads inside the WebGL Editor itself
  function handleDirectAudioUpload(bytes: Uint8Array) {
    console.log('📢 App.svelte received direct audio upload from WebGL Editor:', bytes.length, 'bytes.');
    originalBytes = bytes;
    handleAudioLoaded(bytes, selectedHeight);
  }

  // Switch to route helpers
  function navigateTo(view: 'converter' | 'editor') {
    if (view === 'editor' && !rgbaGrid) return;
    window.location.hash = view === 'editor' ? '#/editor' : '#/converter';
  }
</script>

<main class="app-container" class:full-screen-layout={currentView === 'editor'}>
  
  {#if currentView === 'converter'}
    <header class="app-header">
      <div class="logo-area">
        <span class="icon-brand">🎨🔊</span>
        <h1>Spectral</h1>
        <span class="badge-tag">Lossless Sandbox v0.1</span>
        
        <div class="view-toggles" style="margin-left: auto;">
          <button class="active" onclick={() => navigateTo('converter')}>1. Converter</button>
          <button onclick={() => navigateTo('editor')} disabled={!rgbaGrid}>2. WebGL Editor</button>
        </div>
      </div>
      <p class="tagline">
        Conversão bit-a-bit perfeitamente reversível de áudio em imagens
      </p>
    </header>

    <section class="main-content">
      <AudioConverter 
        bind:originalBytes={originalBytes} 
        bind:selectedHeight={selectedHeight}
        onAudioLoaded={handleAudioLoaded} 
      />
    </section>

    <footer class="app-footer">
      <p>
        Desenvolvido em Rust (WebAssembly) & Svelte 5. Implantado via GitHub Pages.
      </p>
    </footer>
  {:else if currentView === 'editor'}
    <!-- In editor view, the WebGlEditor fills 100% of the screen as a transparent overlay background -->
    <WebGlEditor 
      rgbaGrid={rgbaGrid} 
      width={gridW} 
      height={gridH} 
      bind:selectionStart={selectionStart}
      bind:selectionEnd={selectionEnd}
      bind:loopEnabled={loopEnabled}
      originalAudio={originalAudio}
      isPlaying={isPlaying}
      onPlayToggle={triggerAudioPlayback}
      onAudioUploaded={handleDirectAudioUpload}
      onBackToConverter={() => navigateTo('converter')}
    />
  {/if}

</main>

<style>
  .app-container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 1.5rem;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    transition: all 0.3s ease-in-out;
  }

  /* Full Screen Overlay styles for Editor mode */
  .app-container.full-screen-layout {
    max-width: 100% !important;
    padding: 0 !important;
    margin: 0 !important;
    width: 100vw !important;
    height: 100vh !important;
    overflow: hidden !important;
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
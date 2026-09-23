<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import AudioConverter from './lib/AudioConverter.svelte';
  import WebGlEditor from './lib/WebGlEditor.svelte';
  import init, { 
    wasm_generate_complex_reassigned_ycbcr_spectrogram,
    wasm_synthesize_spectrogram_to_wav,
    wasm_get_last_mirrored_density_histogram
  } from './wasm/core_wasm.js';

  let currentHash = $state<string>(typeof window !== 'undefined' && window.location.hash ? window.location.hash : '#/converter');
  let wasmLoaded = $state(false);
  
  // Globally preserved state
  let originalBytes = $state<Uint8Array | null>(null);
  let selectedHeight = $state<number>(512);
  
  let rgbaGrid = $state<Uint8Array | null>(null);
  let gridW = $state(0);
  let gridH = $state(0);
  let mirroredDensity = $state<Float32Array | null>(null);
  let currentView = $derived<'converter' | 'editor'>((currentHash === '#/editor' && rgbaGrid) ? 'editor' : 'converter');

  // Advanced DSP Configurations
  let windowType = $state<'hann' | 'hamming' | 'gaussian' | 'blackman-harris'>('hann');
  let windowSize = $state<number>(1024);
  let zeroPadding = $state<number>(2);
  let fmin = $state<number>(20);
  let fmax = $state<number>(20000);
  let algorithmType = $state<'reassignment' | 'log'>('reassignment');
  let paletteType = $state<'ycbcr' | 'snake'>('ycbcr');

  // Adaptive Zoom View Bounds (Immediate UI bounds vs Background WASM bounds)
  let viewStart = $state<number>(0.0);
  let viewEnd = $state<number>(1.0);
  let wasmViewStart = $state<number>(0.0);
  let wasmViewEnd = $state<number>(1.0);
  let viewDebounceTimer: any = null;
  
  // Point/Gaussian Spread Customization Size
  let pointRadius = $state<number>(1.0);
  
  // Frequency Scale Type
  let frequencyScale = $state<'log' | 'linear'>('log');

  // Zoom Strategy & Horizontal Precomputation Factor (2^k)
  let zoomMode = $state<'gpu_debounced' | 'continuous_resample'>('gpu_debounced');
  let horizontalResolutionK = $state<number>(0);

  // Global Audio Transport & Selection Looping State
  let originalAudio = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let selectionStart = $state<number | null>(null); // normalized [0, 1]
  let selectionEnd = $state<number | null>(null);   // normalized [0, 1]
  let loopMode = $state<'normal' | 'mirrored' | 'none'>('normal');
  let playbackTimer: any = null;

  // Hash-based client router synchronized with Svelte 5 state
  function updateRoute() {
    if (typeof window !== 'undefined') {
      currentHash = window.location.hash || '#/converter';
    }
  }

  onMount(async () => {
    try {
      await init();
      wasmLoaded = true;
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
        regenerateSpectrogram();
      } else {
        console.error("Failed to fetch default sample in App.svelte:", response.statusText);
      }
    } catch (e) {
      console.error("❌ Failed to load default sample globally in App.svelte:", e);
    }
  }

  // Master Spectrogram Regeneration function using all dynamic DSP parameters
  function regenerateSpectrogram() {
    if (!originalBytes || !wasmLoaded) return;
    try {
      const t0 = performance.now();
      rgbaGrid = wasm_generate_complex_reassigned_ycbcr_spectrogram(
        originalBytes,
        selectedHeight,
        windowType,
        windowSize,
        zeroPadding,
        fmin,
        fmax,
        algorithmType,
        paletteType,
        wasmViewStart,
        wasmViewEnd,
        pointRadius,
        frequencyScale,
        horizontalResolutionK
      ) as Uint8Array;
      gridH = selectedHeight;
      gridW = (rgbaGrid.length / 4) / selectedHeight;
      
      const rawDensity = wasm_get_last_mirrored_density_histogram();
      if (rawDensity && rawDensity.length === 256) {
        mirroredDensity = new Float32Array(rawDensity);
      }
      
      console.log(`✅ Regenerated Master Spectrogram [Scale: ${frequencyScale}, 2^k: ${horizontalResolutionK} (${gridW} cols)]: size ${rgbaGrid.length} bytes in ${(performance.now() - t0).toFixed(3)} ms.`);
      
      // Rebuild global audio playback if not already created
      if (!originalAudio) {
        const blob = new Blob([originalBytes as any], { type: 'audio/wav' });
        const url = URL.createObjectURL(blob);
        originalAudio = new Audio(url);
        originalAudio.onended = () => {
          isPlaying = false;
          if (playbackTimer) clearInterval(playbackTimer);
        };
      }
    } catch (e) {
      console.error("❌ App.svelte failed to generate WebGL Editor payload:", e);
    }
  }

  // React to immediate wheel/pan gestures: debounce WASM in gpu_debounced mode or sync immediately in continuous mode
  $effect(() => {
    const _vStart = viewStart;
    const _vEnd = viewEnd;
    const _mode = zoomMode;
    
    if (_mode === 'continuous_resample') {
      wasmViewStart = _vStart;
      wasmViewEnd = _vEnd;
    } else {
      if (viewDebounceTimer) clearTimeout(viewDebounceTimer);
      viewDebounceTimer = setTimeout(() => {
        wasmViewStart = _vStart;
        wasmViewEnd = _vEnd;
      }, 150);
    }
  });

  // Trigger regeneration dynamically whenever any DSP parameter changes, utilizing Svelte 5 untrack to break dependency loops
  $effect(() => {
    // Register active triggers explicitly including adaptive zoom bounds!
    const _winType = windowType;
    const _winSize = windowSize;
    const _zeroPadding = zeroPadding;
    const _fmin = fmin;
    const _fmax = fmax;
    const _algo = algorithmType;
    const _pal = paletteType;
    const _height = selectedHeight;
    const _bytes = originalBytes;
    const _loaded = wasmLoaded;
    const _viewStart = wasmViewStart;
    const _viewEnd = wasmViewEnd;
    const _pointRadius = pointRadius;
    const _scale = frequencyScale;
    const _resK = horizontalResolutionK;
    
    if (_bytes && _loaded && _winType && _winSize && _zeroPadding && _fmin && _fmax && _algo && _pal && _height && _scale && _resK !== undefined) {
      untrack(() => {
        regenerateSpectrogram();
      });
    }
  });

  // When a file is loaded and converted, we clear selection and rebuild audio
  function handleAudioLoaded(data: Uint8Array, h: number) {
    console.log('📢 App.svelte handleAudioLoaded callback received data with length:', data?.length, 'height:', h);
    originalBytes = data;
    selectedHeight = h;
    
    // Reset global Audio element and selection bounds
    if (originalAudio) {
      originalAudio.pause();
    }
    originalAudio = null;
    selectionStart = null;
    selectionEnd = null;
    
    regenerateSpectrogram();
  }

  // High-Resolution DAW Controller synthesizing audio directly from the on-screen spectrogram pixels!
  function triggerAudioPlayback() {
    if (!rgbaGrid || gridW === 0 || gridH === 0) return;
    
    if (isPlaying) {
      if (originalAudio) originalAudio.pause();
      isPlaying = false;
      if (playbackTimer) clearInterval(playbackTimer);
    } else {
      isPlaying = true;
      if (playbackTimer) clearInterval(playbackTimer);
      
      // Map normalized selection bounds [0.0, 1.0] to spectrogram texture columns
      const sStart = selectionStart !== null ? Math.min(selectionStart, selectionEnd ?? selectionStart) : 0.0;
      const sEnd = selectionEnd !== null ? Math.max(selectionStart ?? selectionEnd, selectionEnd) : 1.0;
      
      const startCol = Math.floor(sStart * gridW);
      const endCol = Math.ceil(sEnd * gridW);
      
      console.log(`🔊 Resynthesizing audio from spectrogram cols [${startCol}..${endCol}]...`);
      const t0 = performance.now();
      
      try {
        const wavBytes = wasm_synthesize_spectrogram_to_wav(
          rgbaGrid,
          gridW,
          gridH,
          fmin,
          fmax,
          frequencyScale,
          windowSize,
          zeroPadding,
          startCol,
          endCol
        );
        console.log(`🔊 Resynthesized ${wavBytes.length} bytes in ${(performance.now() - t0).toFixed(2)}ms!`);
        
        const blob = new Blob([wavBytes as any], { type: 'audio/wav' });
        const url = URL.createObjectURL(blob);
        
        if (originalAudio) {
          originalAudio.pause();
        }
        originalAudio = new Audio(url);
        
        const duration = originalAudio.duration || ((endCol - startCol) * 64 / 44100.0);
        
        originalAudio.play();
        
        playbackTimer = setInterval(() => {
          if (!originalAudio) return;
          
          const current = originalAudio.currentTime;
          
          if (current >= duration || originalAudio.ended) {
            if (loopMode === 'normal') {
              originalAudio.currentTime = 0;
              originalAudio.play();
            } else {
              originalAudio.pause();
              isPlaying = false;
              if (playbackTimer) clearInterval(playbackTimer);
            }
          }
        }, 15);
      } catch (err) {
        console.error("Failed to synthesize audio from spectrogram:", err);
        isPlaying = false;
      }
    }
  }

  // Handle direct file uploads inside the WebGL Editor itself
  function handleDirectAudioUpload(bytes: Uint8Array) {
    console.log('📢 App.svelte received direct audio upload from WebGL Editor:', bytes.length, 'bytes.');
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
      mirroredDensity={mirroredDensity}
      width={gridW} 
      height={gridH} 
      
      bind:windowType={windowType}
      bind:windowSize={windowSize}
      bind:zeroPadding={zeroPadding}
      bind:fmin={fmin}
      bind:fmax={fmax}
      bind:algorithmType={algorithmType}
      bind:paletteType={paletteType}
      bind:selectedHeight={selectedHeight}
      
      bind:selectionStart={selectionStart}
      bind:selectionEnd={selectionEnd}
      bind:loopMode={loopMode}
      
      bind:viewStart={viewStart}
      bind:viewEnd={viewEnd}
      
      bind:pointRadius={pointRadius}
      bind:frequencyScale={frequencyScale}
      bind:zoomMode={zoomMode}
      bind:horizontalResolutionK={horizontalResolutionK}
      
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
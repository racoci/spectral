<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import AudioConverter from './lib/AudioConverter.svelte';
  import WebGlEditor from './lib/WebGlEditor.svelte';
  import init, { 
    wasm_generate_complex_reassigned_ycbcr_spectrogram,
    wasm_get_spectrogram_dimensions,
    WasmSpectrogramStreamer,
    wasm_cache_base_quadruplets,
    wasm_has_cached_quadruplets,
    wasm_render_from_cached_quadruplets,
    wasm_bicubic_resample_spectrogram,
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
  let progressiveSessionId = 0;
  let refinementProgress = $state<number | null>(null);
  let mirroredDensity = $state<Float32Array | null>(null);
  let currentView = $derived<'converter' | 'editor'>((currentHash === '#/editor' && (rgbaGrid || originalBytes)) ? 'editor' : 'converter');

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
        wasm_cache_base_quadruplets(
          bytes,
          selectedHeight,
          windowType,
          windowSize,
          zeroPadding,
          fmin,
          fmax,
          algorithmType,
          frequencyScale,
          horizontalResolutionK
        );
        regenerateSpectrogram();
      } else {
        console.error("Failed to fetch default sample in App.svelte:", response.statusText);
      }
    } catch (e) {
      console.error("❌ Failed to load default sample globally in App.svelte:", e);
    }
  }

  // Master Spectrogram Regeneration function using all dynamic DSP parameters
  let currentQualityLod = $state<number>(0); // 0 = Refined High-Res, 1 = Fast Draft Preview
  let adaptiveRefineTimer: any = null;
  let isInteracting = $state<boolean>(false);

  // Progressive Two-Stage LOD Trigger:
  // 1. Immediately renders Fast Draft LOD 1 (<5ms) while the user is actively sliding or zooming
  // 2. Automatically refines into Full Resolution LOD 0 (80ms) when the user stops moving!
  function triggerAdaptiveInteraction() {
    isInteracting = true;
    regenerateSpectrogram(1); // Fast 3ms draft!
    
    if (adaptiveRefineTimer) clearTimeout(adaptiveRefineTimer);
    adaptiveRefineTimer = setTimeout(() => {
      isInteracting = false;
      regenerateSpectrogram(0); // Full quality refinement!
    }, 200);
  }

  // Persistent texture cache for Pyramidal Spatial Resampling & Zoom Refinement
  let lastTextureGrid: Uint8Array | null = null;
  let lastTextureW = 0;
  let lastTextureH = 0;
  let lastTextureViewStart = 0.0;
  let lastTextureViewEnd = 1.0;
  let lastTextureFmin = 20;
  let lastTextureFmax = 20000;

  function resamplePreviousGrid(
    prevGrid: Uint8Array, prevW: number, prevH: number,
    prevTStart: number, prevTEnd: number, prevFmin: number, prevFmax: number,
    newW: number, newH: number,
    newTStart: number, newTEnd: number, newFmin: number, newFmax: number
  ): Uint8Array {
    const newGrid = new Uint8Array(newW * newH * 4);
    const logPrevFmin = Math.log2(Math.max(1, prevFmin));
    const logPrevSpan = Math.log2(prevFmax / Math.max(1, prevFmin));
    const logNewFmin = Math.log2(Math.max(1, newFmin));
    const logNewSpan = Math.log2(newFmax / Math.max(1, newFmin));
    const prevTSpan = Math.max(1e-6, prevTEnd - prevTStart);
    const newTSpan = Math.max(1e-6, newTEnd - newTStart);

    for (let r = 0; r < newH; r++) {
      const freqRatio = r / Math.max(1, newH - 1);
      const logF = logNewFmin + freqRatio * logNewSpan;
      const prevFreqRatio = (logF - logPrevFmin) / logPrevSpan;
      const srcRow = Math.round(prevFreqRatio * (prevH - 1));
      if (srcRow < 0 || srcRow >= prevH) continue;

      for (let c = 0; c < newW; c++) {
        const timeRatio = c / Math.max(1, newW - 1);
        const t = newTStart + timeRatio * newTSpan;
        const prevTimeRatio = (t - prevTStart) / prevTSpan;
        const srcCol = Math.round(prevTimeRatio * (prevW - 1));
        if (srcCol < 0 || srcCol >= prevW) continue;

        const srcIdx = (srcRow * prevW + srcCol) * 4;
        const dstIdx = (r * newW + c) * 4;
        newGrid[dstIdx] = prevGrid[srcIdx];
        newGrid[dstIdx + 1] = prevGrid[srcIdx + 1];
        newGrid[dstIdx + 2] = prevGrid[srcIdx + 2];
        newGrid[dstIdx + 3] = prevGrid[srcIdx + 3];
      }
    }
    return newGrid;
  }

  function regenerateSpectrogram(lod = 0) {
    if (!originalBytes || !wasmLoaded) return;
    try {
      const t0 = performance.now();
      currentQualityLod = lod;
      
      // If LOD 1 (Draft Mode): instant single-shot 1.5ms computation!
      if (lod === 1) {
        progressiveSessionId++; // cancel any running progressive sweep
        refinementProgress = null;
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
          horizontalResolutionK,
          1,
          0,
          0
        ) as Uint8Array;
        gridH = 256;
        gridW = (rgbaGrid.length / 4) / gridH;
        return;
      }

      // If LOD 0 (Refined Ultra-Foco):
      // 1. Initialize persistent state streamer in WebAssembly
      const streamer = new WasmSpectrogramStreamer(
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
      );
      const targetW = streamer.get_width();
      const targetH = streamer.get_height();
      const currentSession = ++progressiveSessionId;

      // 2. VECTOR-AUDIO CONTINUOUS PROJECTION: Render instant continuous zoom from cached quadruplets (t, f, log(A), phi)!
      let fullGrid = new Uint8Array(targetW * targetH * 4);
      if (wasm_has_cached_quadruplets()) {
        const vectorBg = wasm_render_from_cached_quadruplets(
          targetW,
          targetH,
          wasmViewStart,
          wasmViewEnd,
          fmin,
          fmax,
          pointRadius,
          paletteType
        );
        if (vectorBg && vectorBg.length === targetW * targetH * 4) {
          fullGrid.set(vectorBg);
        }
      } else if (lastTextureGrid && lastTextureW > 0 && lastTextureH > 0) {
        // Fallback to high-speed WebAssembly bicubic resampling
        const bicubicBg = wasm_bicubic_resample_spectrogram(
          lastTextureGrid,
          lastTextureW,
          lastTextureH,
          lastTextureViewStart,
          lastTextureViewEnd,
          lastTextureFmin,
          lastTextureFmax,
          targetW,
          targetH,
          wasmViewStart,
          wasmViewEnd,
          fmin,
          fmax
        );
        if (bicubicBg && bicubicBg.length === targetW * targetH * 4) {
          fullGrid.set(bicubicBg);
        }
      }

      // Display the vector-projected zoom image immediately in <1.5ms!
      rgbaGrid = fullGrid;
      gridW = targetW;
      gridH = targetH;

      // 3. Ultra-light gradual streaming: 24 columns per chunk (~0.6 ms per chunk!)
      const CHUNK_SIZE = 24;

      function streamNextChunk() {
        if (currentSession !== progressiveSessionId) {
          streamer.free();
          return;
        }

        const startCol = streamer.get_current_col();
        const chunkBytes = streamer.process_chunk(CHUNK_SIZE);
        const chunkW = streamer.get_current_col() - startCol;

        if (chunkW > 0) {
          // Overwrite the resampled coarse pixels with razor-sharp Auger-Flandrin lines!
          for (let r = 0; r < targetH; r++) {
            const srcOffset = r * chunkW * 4;
            const dstOffset = (r * targetW + startCol) * 4;
            fullGrid.set(chunkBytes.subarray(srcOffset, srcOffset + chunkW * 4), dstOffset);
          }

          rgbaGrid = fullGrid;
          refinementProgress = Math.round(streamer.get_progress_pct());
        }

        if (!streamer.is_complete()) {
          requestAnimationFrame(streamNextChunk);
        } else {
          // Completed full sweep!
          refinementProgress = null;
          lastTextureGrid = new Uint8Array(fullGrid);
          lastTextureW = targetW;
          lastTextureH = targetH;
          lastTextureViewStart = wasmViewStart;
          lastTextureViewEnd = wasmViewEnd;
          lastTextureFmin = fmin;
          lastTextureFmax = fmax;

          streamer.free();

          const rawDensity = wasm_get_last_mirrored_density_histogram();
          if (rawDensity && rawDensity.length === 256) {
            mirroredDensity = new Float32Array(rawDensity);
          }
          console.log(`✅ [LOD 0 - REFINED COMPLETED] (${targetW}x${targetH}) in ${(performance.now() - t0).toFixed(2)} ms.`);
        }
      }

      requestAnimationFrame(streamNextChunk);

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
    
    // Cache the original base quadruplets (t, f, log(A), phi) in continuous vector space!
    wasm_cache_base_quadruplets(
      data,
      h,
      windowType,
      windowSize,
      zeroPadding,
      fmin,
      fmax,
      algorithmType,
      frequencyScale,
      horizontalResolutionK
    );
    
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
      currentQualityLod={currentQualityLod}
      refinementProgress={refinementProgress}
      onAdaptiveInteract={triggerAdaptiveInteraction}
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
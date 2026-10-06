<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import AudioConverter from './lib/AudioConverter.svelte';
  import WebGlEditor from './lib/WebGlEditor.svelte';
  import init, { 
    wasm_generate_complex_reassigned_ycbcr_spectrogram,
    wasm_generate_holomorphic_exploration_spectrogram,
    wasm_get_spectrogram_dimensions,
    WasmSpectrogramStreamer,
    wasm_cache_base_quadruplets,
    wasm_has_cached_quadruplets,
    wasm_render_from_cached_quadruplets,
    wasm_bicubic_resample_spectrogram,
    wasm_synthesize_spectrogram_to_wav,
    wasm_synthesize_hybrid_spectrogram_to_wav,
    wasm_get_last_mirrored_density_histogram
  } from './wasm/core_wasm.js';

  let currentHash = $state<string>(
    typeof window !== 'undefined' && window.location.hash && window.location.hash !== '#' && window.location.hash !== '#/'
      ? window.location.hash
      : '#/editor'
  );
  let wasmLoaded = $state(false);
  
  // Globally preserved state
  let originalBytes = $state<Uint8Array | null>(null);
  let selectedHeight = $state<number>(1024);

  let rgbaGrid = $state<Uint8Array | null>(null);
  let gridW = $state(0);
  let gridH = $state(0);
  let progressiveSessionId = 0;
  let refinementProgress = $state<number | null>(null);
  let mirroredDensity = $state<Float32Array | null>(null);
  let currentView = $derived<'converter' | 'editor'>(currentHash === '#/converter' ? 'converter' : 'editor');

  // Advanced DSP Configurations (Pure Gaussian-Hermite Pipeline)
  const windowType = 'gaussian';
  let windowSize = $state<number>(256);
  let zeroPadding = $state<number>(2);
  let fmin = $state<number>(20);
  let fmax = $state<number>(20000);
  let algorithmType = $state<'reassignment' | 'log' | 'cqt' | 'holomorphic' | 'higher_order' | 'sliding_jet'>('reassignment');
  let higherOrderO = $state<number>(2);
  let higherOrderVisualMode = $state<'ridge' | 'anisotropy' | 'curvature' | 'vector_reassign'>('ridge');
  let paletteType = $state<'ycbcr' | 'snake'>('ycbcr');

  // Hermite-Gaussian STFT Jet & Independent Reassignment Controls
  let enableTimeReassignment = $state(true);
  let enableFreqReassignment = $state(true);
  let maxDerivativeOrder = $state<number>(2);

  // Adaptive Zoom View Bounds (Immediate UI bounds vs Background WASM bounds)
  let viewStart = $state<number>(0.0);
  let viewEnd = $state<number>(1.0);
  let wasmViewStart = $state<number>(0.0);
  let wasmViewEnd = $state<number>(1.0);
  let viewDebounceTimer: any = null;
  
  // Point/Gaussian Spread Customization Size
  let pointRadius = $state<number>(1.0);
  
  // Frequency Scale Type
  let frequencyScale = $state<'log' | 'mel' | 'bark' | 'linear'>('log');

  // Zoom Strategy & Horizontal Precomputation Factor (2^k)
  let zoomMode = $state<'gpu_debounced' | 'continuous_resample'>('gpu_debounced');
  let horizontalResolutionK = $state<number>(0);

  // Global Audio Transport & Selection Looping State
  let originalAudio = $state<HTMLAudioElement | null>(null);
  let synthAudio = $state<HTMLAudioElement | null>(null);
  let activeAudio = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let selectionStart = $state<number | null>(null); // normalized [0, 1]
  let selectionEnd = $state<number | null>(null);   // normalized [0, 1]
  let loopMode = $state<'normal' | 'mirrored' | 'none'>('normal');
  let playbackTimer: any = null;

  function setupOriginalAudio(bytes: Uint8Array) {
    if (originalAudio) {
      originalAudio.pause();
    }
    const blob = new Blob([bytes as any], { type: 'audio/wav' });
    const url = URL.createObjectURL(blob);
    originalAudio = new Audio(url);
    activeAudio = originalAudio;
    originalAudio.onended = () => {
      isPlaying = false;
      if (playbackTimer) clearInterval(playbackTimer);
    };
  }

  // Hash-based client router synchronized with Svelte 5 state
  function updateRoute() {
    if (typeof window !== 'undefined') {
      const hash = window.location.hash;
      if (!hash || hash === '#' || hash === '#/') {
        currentHash = '#/editor';
        window.location.hash = '#/editor';
      } else {
        currentHash = hash;
      }
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
        setupOriginalAudio(bytes);
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
  let texStart = $state<number>(0.0);
  let texEnd = $state<number>(1.0);

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
      
      const effectiveAlgo = (algorithmType === 'higher_order' || algorithmType === 'sliding_jet')
        ? `${algorithmType}:${higherOrderO}:${higherOrderVisualMode}`
        : algorithmType;

      progressiveSessionId++; // cancel any running progressive sweep
      refinementProgress = null;

      const targetViewStart = wasmViewStart;
      const targetViewEnd = wasmViewEnd;

      // Executa nova versão holomorfa isolada ou mantém 100% intacta a versão anterior:
      let rawGrid: Uint8Array;
      if (algorithmType === 'holomorphic') {
        const approxW = gridW > 0 ? gridW : 1000;
        const c_start = Math.floor(targetViewStart * approxW);
        const c_end = Math.ceil(targetViewEnd * approxW);
        rawGrid = wasm_generate_holomorphic_exploration_spectrogram(
          originalBytes,
          selectedHeight,
          fmin,
          fmax,
          frequencyScale,
          paletteType,
          c_start,
          c_end,
          2.0,
          enableTimeReassignment || enableFreqReassignment,
          true
        );
      } else {
        rawGrid = wasm_generate_complex_reassigned_ycbcr_spectrogram(
          originalBytes,
          selectedHeight,
          windowType,
          windowSize,
          zeroPadding,
          fmin,
          fmax,
          effectiveAlgo,
          paletteType,
          targetViewStart,
          targetViewEnd,
          pointRadius,
          frequencyScale,
          horizontalResolutionK,
          lod,
          0,
          0,
          enableTimeReassignment,
          enableFreqReassignment,
          maxDerivativeOrder
        );
      }

      const h = lod === 1 ? 256 : selectedHeight;
      const w = (rawGrid.length / 4) / h;

      // Assign a fresh Uint8Array so Svelte 5 reactivity immediately triggers WebGL texture upload!
      rgbaGrid = new Uint8Array(rawGrid);
      gridW = w;
      gridH = h;
      texStart = targetViewStart;
      texEnd = targetViewEnd;

      const rawDensity = wasm_get_last_mirrored_density_histogram();
      if (rawDensity && rawDensity.length === 256) {
        mirroredDensity = new Float32Array(rawDensity);
      }
      console.log(`✅ [LOD ${lod} COMPLETED] (${w}x${h}) in ${(performance.now() - t0).toFixed(2)} ms.`);

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
    const _ho_o = higherOrderO;
    const _ho_m = higherOrderVisualMode;
    const _pal = paletteType;
    const _height = selectedHeight;
    const _bytes = originalBytes;
    const _loaded = wasmLoaded;
    const _viewStart = wasmViewStart;
    const _viewEnd = wasmViewEnd;
    const _pointRadius = pointRadius;
    const _scale = frequencyScale;
    const _resK = horizontalResolutionK;
    
    if (_bytes && _loaded && _winType && _winSize && _zeroPadding && _fmin && _fmax && _algo && _pal && _height && _scale && _resK !== undefined && _ho_o !== undefined && _ho_m !== undefined) {
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
    setupOriginalAudio(data);
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

  // Master DAW Audio Controller: Plays authentic original audio OR resynthesized spectrogram audio
  function triggerAudioPlayback(mode: 'original' | 'resynthesized' = 'original') {
    if (!originalBytes) return;
    
    if (isPlaying) {
      if (activeAudio) activeAudio.pause();
      isPlaying = false;
      if (playbackTimer) clearInterval(playbackTimer);
      return;
    }

    if (playbackTimer) clearInterval(playbackTimer);

    if (!originalAudio) {
      setupOriginalAudio(originalBytes);
    }

    // Determina o intervalo de tempo normalizado [0.0, 1.0] correspondente à seleção ou à visão visível
    const sStart = selectionStart !== null ? Math.min(selectionStart, selectionEnd ?? selectionStart) : viewStart;
    const sEnd = selectionEnd !== null ? Math.max(selectionStart ?? selectionEnd, selectionEnd) : viewEnd;

    if (mode === 'original' || !rgbaGrid || gridW === 0 || gridH === 0) {
      activeAudio = originalAudio;
      playAudioRange(originalAudio!, false, sStart, sEnd);
    } else {
      // Síntese espectral de alta resolução diretamente dos pixels do espectrograma
      try {
        const dims = wasm_get_spectrogram_dimensions(
          originalBytes,
          selectedHeight,
          horizontalResolutionK,
          currentQualityLod,
          viewStart,
          viewEnd
        );
        const calculatedHop = Number(dims[2]);
        const sampleRate = Number(dims[3]);
        const totalAudioSamples = Number(dims[4]) || 10000;

        // Fatiamento com precisão absoluta de amostra individual (Bit-Perfect)
        const startSample = Math.round(sStart * totalAudioSamples);
        const endSample = Math.round(sEnd * totalAudioSamples);

        console.log(`🔊 Resynthesizing audio from spectrogram time [${sStart.toFixed(3)}..${sEnd.toFixed(3)}] (samples ${startSample}..${endSample}) at ${sampleRate} Hz...`);
        const t0 = performance.now();

        const wavBytes = wasm_synthesize_hybrid_spectrogram_to_wav(
          originalBytes ?? new Uint8Array(),
          rgbaGrid,
          totalAudioSamples,
          selectedHeight,
          fmin,
          fmax,
          frequencyScale,
          windowSize,
          zeroPadding,
          startSample,
          endSample,
          calculatedHop,
          sampleRate,
          false // Bit-Perfect fidelity: exact match with original audio!
        );
        console.log(`🔊 Resynthesized ${wavBytes.length} bytes in ${(performance.now() - t0).toFixed(2)}ms!`);

        const blob = new Blob([wavBytes as any], { type: 'audio/wav' });
        const url = URL.createObjectURL(blob);
        if (synthAudio) {
          synthAudio.pause();
        }
        synthAudio = new Audio(url);
        activeAudio = synthAudio;
        playAudioRange(synthAudio, true, sStart, sEnd);
      } catch (err) {
        console.error("Failed to synthesize audio from spectrogram, falling back to original:", err);
        activeAudio = originalAudio;
        playAudioRange(originalAudio!, false, sStart, sEnd);
      }
    }
  }

  function playAudioRange(audioEl: HTMLAudioElement, isSynthesizedSlice: boolean, sStart: number, sEnd: number) {
    if (playbackTimer) clearInterval(playbackTimer);
    isPlaying = true;

    const startPlayback = () => {
      const origDuration = originalAudio?.duration && !isNaN(originalAudio.duration) && originalAudio.duration > 0
        ? originalAudio.duration
        : 1.0;

      let startTime = 0.0;
      let endTime = audioEl.duration && !isNaN(audioEl.duration) ? audioEl.duration : 1.0;

      if (!isSynthesizedSlice) {
        startTime = sStart * origDuration;
        endTime = sEnd * origDuration;
      }

      audioEl.currentTime = startTime;
      audioEl.play().catch(err => {
        console.warn("Audio play prevented:", err);
        isPlaying = false;
      });

      playbackTimer = setInterval(() => {
        if (!audioEl || !isPlaying) {
          clearInterval(playbackTimer);
          return;
        }

        if (audioEl.currentTime >= endTime || audioEl.ended) {
          if (loopMode === 'normal' || loopMode === 'mirrored') {
            audioEl.currentTime = startTime;
            audioEl.play().catch(() => {});
          } else {
            audioEl.pause();
            isPlaying = false;
            clearInterval(playbackTimer);
          }
        }
      }, 15);
    };

    if (audioEl.readyState >= 1) {
      startPlayback();
    } else {
      audioEl.onloadedmetadata = () => startPlayback();
      setTimeout(() => {
        if (isPlaying && isNaN(audioEl.currentTime)) {
          startPlayback();
        }
      }, 150);
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
      originalBytes={originalBytes}
      mirroredDensity={mirroredDensity}
      width={gridW} 
      height={gridH} 
      texStart={texStart}
      texEnd={texEnd} 
      
      bind:windowSize={windowSize}
      bind:zeroPadding={zeroPadding}
      bind:fmin={fmin}
      bind:fmax={fmax}
      bind:algorithmType={algorithmType}
      bind:higherOrderO={higherOrderO}
      bind:higherOrderVisualMode={higherOrderVisualMode}
      bind:paletteType={paletteType}
      bind:selectedHeight={selectedHeight}
      bind:enableTimeReassignment={enableTimeReassignment}
      bind:enableFreqReassignment={enableFreqReassignment}
      bind:maxDerivativeOrder={maxDerivativeOrder}
      
      bind:selectionStart={selectionStart}
      bind:selectionEnd={selectionEnd}
      bind:loopMode={loopMode}
      
      bind:viewStart={viewStart}
      bind:viewEnd={viewEnd}
      
      bind:pointRadius={pointRadius}
      bind:frequencyScale={frequencyScale}
      bind:zoomMode={zoomMode}
      bind:horizontalResolutionK={horizontalResolutionK}
      
      originalAudio={activeAudio ?? originalAudio}
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
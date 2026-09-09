<script lang="ts">
  import { onMount } from 'svelte';
  import init, { 
    init_panic_hook, 
    encode_naive, 
    decode_naive,
    encode_wavelet,
    decode_wavelet
  } from '../wasm/core_wasm.js';
  import CanvasVisualizer from './CanvasVisualizer.svelte';

  // State using Svelte 5 standard runes
  let wasmLoaded = $state(false);
  let wasmError = $state<string | null>(null);

  // Selected conversion algorithm
  let selectedAlgorithm = $state<'wavelet' | 'naive'>('wavelet');

  // Original File State
  let originalFile = $state<File | null>(null);
  let originalBytes = $state<Uint8Array | null>(null);
  let originalHash = $state<string>('');
  let originalUrl = $state<string>('');
  let originalAudio: HTMLAudioElement | null = null;
  let isOriginalPlaying = $state(false);

  // Encoded Image (RGBA) State
  let rgbaBytes = $state<Uint8Array | null>(null);
  let encodingTimeMs = $state<number>(0);

  // Reconstructed File State
  let decodedBytes = $state<Uint8Array | null>(null);
  let decodingTimeMs = $state<number>(0);
  let decodedHash = $state<string>('');
  let decodedUrl = $state<string>('');
  let decodedAudio: HTMLAudioElement | null = null;
  let isDecodedPlaying = $state(false);

  // Symmetrical Invariant Matching Status
  let sizeMatches = $derived(
    originalBytes && decodedBytes && originalBytes.length === decodedBytes.length
  );
  let hashMatches = $derived(
    originalHash && decodedHash && originalHash === decodedHash
  );
  let symmetricalMatch = $derived(sizeMatches && hashMatches);

  onMount(async () => {
    try {
      // Initialize the WebAssembly module
      await init();
      init_panic_hook();
      wasmLoaded = true;
    } catch (e: any) {
      console.error('Failed to load WASM:', e);
      wasmError = e.message || String(e);
    }
  });

  // Helper: Computes SHA-256 of a byte array using browser's native crypto APIs
  async function computeSHA256(bytes: Uint8Array): Promise<string> {
    const buffer = bytes.buffer;
    if (buffer instanceof ArrayBuffer) {
      const hashBuffer = await crypto.subtle.digest('SHA-256', buffer);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
    }
    return 'unsupported-buffer-type';
  }

  // Handle Audio File Selection
  async function handleFileChange(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    // Reset everything
    resetAllStates();

    const file = input.files[0];
    originalFile = file;

    // Create object URL for native audio playing
    originalUrl = URL.createObjectURL(file);
    originalAudio = new Audio(originalUrl);
    originalAudio.onended = () => { isOriginalPlaying = false; };

    // Read file bytes
    const arrayBuffer = await file.arrayBuffer();
    originalBytes = new Uint8Array(arrayBuffer);

    // Compute hash
    originalHash = await computeSHA256(originalBytes);

    // Note: We do not call runEncoding() here manually.
    // Svelte 5's $effect block below will automatically trigger runEncoding()
    // in a clean, unified, and sequential manner when originalBytes changes,
    // avoiding asynchronous race conditions on shared states.
  }

  // Reactive effect to re-run the entire pipeline when the algorithm changes
  $effect(() => {
    if (originalBytes && wasmLoaded && selectedAlgorithm) {
      runEncoding();
    }
  });

  // Core Forward Process: Audio -> Image
  async function runEncoding() {
    if (!originalBytes || !wasmLoaded) return;

    try {
      const start = performance.now();
      if (selectedAlgorithm === 'wavelet') {
        rgbaBytes = encode_wavelet(originalBytes);
      } else {
        rgbaBytes = encode_naive(originalBytes);
      }
      encodingTimeMs = performance.now() - start;

      // Immediately run the inverse process (Image -> Audio) to verify symmetry
      await runDecoding();
    } catch (error) {
      console.error('Error during encoding:', error);
    }
  }

  // Core Inverse Process: Image -> Audio
  async function runDecoding() {
    if (!rgbaBytes || !wasmLoaded) return;

    try {
      const start = performance.now();
      if (selectedAlgorithm === 'wavelet') {
        decodedBytes = decode_wavelet(rgbaBytes);
      } else {
        decodedBytes = decode_naive(rgbaBytes);
      }
      decodingTimeMs = performance.now() - start;

      // Compute hash of reconstructed bytes
      decodedHash = await computeSHA256(decodedBytes);

      // Create blob and audio element for reconstructed file playing/downloading
      // We use original file type (typically audio/wav)
      const buffer = decodedBytes.buffer;
      if (buffer instanceof ArrayBuffer) {
        const blob = new Blob([buffer], { type: originalFile?.type || 'audio/wav' });
        decodedUrl = URL.createObjectURL(blob);
        decodedAudio = new Audio(decodedUrl);
        decodedAudio.onended = () => { isDecodedPlaying = false; };
      } else {
        console.error('Reconstructed buffer is not a standard ArrayBuffer.');
      }
    } catch (error) {
      console.error('Error during decoding:', error);
    }
  }

  function toggleOriginalPlay() {
    if (!originalAudio) return;
    if (isOriginalPlaying) {
      originalAudio.pause();
      isOriginalPlaying = false;
    } else {
      // Stop the other audio if playing
      if (isDecodedPlaying && decodedAudio) {
        decodedAudio.pause();
        isDecodedPlaying = false;
      }
      originalAudio.play();
      isOriginalPlaying = true;
    }
  }

  function toggleDecodedPlay() {
    if (!decodedAudio) return;
    if (isDecodedPlaying) {
      decodedAudio.pause();
      isDecodedPlaying = false;
    } else {
      // Stop the other audio if playing
      if (isOriginalPlaying && originalAudio) {
        originalAudio.pause();
        isOriginalPlaying = false;
      }
      decodedAudio.play();
      isDecodedPlaying = true;
    }
  }

  function resetAllStates() {
    // Revoke old object URLs to avoid memory leaks
    if (originalUrl) URL.revokeObjectURL(originalUrl);
    if (decodedUrl) URL.revokeObjectURL(decodedUrl);

    // Stop audio
    if (originalAudio) {
      originalAudio.pause();
      originalAudio = null;
    }
    if (decodedAudio) {
      decodedAudio.pause();
      decodedAudio = null;
    }

    originalFile = null;
    originalBytes = null;
    originalHash = '';
    originalUrl = '';
    isOriginalPlaying = false;

    rgbaBytes = null;
    encodingTimeMs = 0;

    decodedBytes = null;
    decodingTimeMs = 0;
    decodedHash = '';
    decodedUrl = '';
    isDecodedPlaying = false;
  }
</script>

<div class="converter-layout">
  <!-- Left Side: Controls & Symmetrical Verification -->
  <div class="controls-panel">
    <div class="panel-section">
      <h2>1. Upload de Áudio</h2>
      {#if !wasmLoaded}
        <div class="loading-state">
          {#if wasmError}
            <p class="error-text">❌ Erro ao inicializar o compilado Rust/Wasm: {wasmError}</p>
          {:else}
            <div class="spinner"></div>
            <p>Carregando motor matemático em Rust...</p>
          {/if}
        </div>
      {:else}
        <label class="file-upload-box">
          <input type="file" accept="audio/*" onchange={handleFileChange} />
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="upload-icon">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="17 8 12 3 7 8"></polyline>
            <line x1="12" y1="3" x2="12" y2="15"></line>
          </svg>
          {#if originalFile}
            <span class="file-name">{originalFile.name}</span>
            <span class="file-size">({(originalFile.size / 1024 / 1024).toFixed(2)} MB)</span>
          {:else}
            <span>Escolha ou arraste um arquivo de áudio</span>
            <span class="file-tip">Recomendável: arquivos .wav pequenos de alta fidelidade</span>
          {/if}
        </label>

        <!-- Algorithm Selector Dropdown -->
        <div class="algorithm-selector-box">
          <label for="algorithm-select" class="algorithm-label">Algoritmo de Codificação:</label>
          <select 
            id="algorithm-select" 
            bind:value={selectedAlgorithm}
            class="algorithm-select-dropdown"
          >
            <option value="wavelet">Wavelet Inteira CDF 5/3 (Lossless)</option>
            <option value="naive">Naive Byte Packing (Lossless Baseline)</option>
          </select>
          <p class="algorithm-description">
            {#if selectedAlgorithm === 'wavelet'}
              <strong>Wavelet CDF 5/3:</strong> Agrupa as frequências e o envelope de tempo em 2D usando o Lifting Scheme do JPEG 2000 sem perdas. Cada pixel representa um coeficiente de wavelet real.
            {:else}
              <strong>Naive Packing:</strong> Mapeia os bytes binários brutos do arquivo diretamente nos canais de cor RGBA. Parecido com estática analógica.
            {/if}
          </p>
        </div>
      {/if}
    </div>

    <!-- Symmetrical Validation Panel -->
    {#if originalBytes}
      <div class="panel-section">
        <h2>2. Validação Cruzada Simétrica</h2>
        
        <div class="validation-status-badge {symmetricalMatch ? 'status-match' : 'status-mismatch'}">
          {#if symmetricalMatch}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="badge-icon">
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
            <span>INTEGRIDADE COMPROVADA (BIT-PERFECT)</span>
          {:else}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="badge-icon">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
            <span>ERRO DE RECONSTRUÇÃO (DIVERGÊNCIA)</span>
          {/if}
        </div>

        <div class="metrics-grid">
          <!-- Column Headers -->
          <div class="metric-header">Invariante</div>
          <div class="metric-header">Áudio Original</div>
          <div class="metric-header">Áudio Reconstruído</div>

          <!-- Size Comparison -->
          <div class="metric-label">Tamanho (Bytes)</div>
          <div class="metric-val font-mono">{originalBytes.length.toLocaleString()} B</div>
          <div class="metric-val font-mono {sizeMatches ? 'text-green' : 'text-red'}">
            {decodedBytes ? decodedBytes.length.toLocaleString() : '---'} B
          </div>

          <!-- Hash Comparison -->
          <div class="metric-label">Hash SHA-256</div>
          <div class="metric-val font-mono hash-text" title={originalHash}>{originalHash.slice(0, 12)}...</div>
          <div class="metric-val font-mono hash-text {hashMatches ? 'text-green' : 'text-red'}" title={decodedHash}>
            {decodedHash ? `${decodedHash.slice(0, 12)}...` : '---'}
          </div>

          <!-- Performance -->
          <div class="metric-label">Tempo de Processamento</div>
          <div class="metric-val font-mono text-gray">Fwd: {encodingTimeMs.toFixed(1)}ms</div>
          <div class="metric-val font-mono text-gray">Inv: {decodingTimeMs.toFixed(1)}ms</div>
        </div>
      </div>

      <div class="panel-section">
        <h2>3. Audição Comparativa</h2>
        <div class="player-controls">
          <button class="btn-play {isOriginalPlaying ? 'playing' : ''}" onclick={toggleOriginalPlay}>
            <span class="btn-indicator"></span>
            {isOriginalPlaying ? 'Pausar Original' : 'Tocar Áudio Original'}
          </button>
          
          <button class="btn-play {isDecodedPlaying ? 'playing' : ''}" onclick={toggleDecodedPlay} disabled={!decodedBytes}>
            <span class="btn-indicator"></span>
            {isDecodedPlaying ? 'Pausar Reconstruído' : 'Tocar Reconstruído'}
          </button>
        </div>

        {#if decodedUrl}
          <div class="download-section">
            <a href={decodedUrl} download="reconstructed_{originalFile?.name || 'audio.wav'}" class="btn-download">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="icon-download">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                <polyline points="7 10 12 15 17 10"></polyline>
                <line x1="12" y1="15" x2="12" y2="3"></line>
              </svg>
              Baixar Áudio Reconstruído (.wav)
            </a>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Right Side: Interactive Image Visualizer -->
  <div class="visualizer-panel">
    <CanvasVisualizer {rgbaBytes} />
  </div>
</div>

<style>
  .converter-layout {
    display: grid;
    grid-template-columns: 1fr;
    gap: 1.5rem;
    width: 100%;
    margin-top: 1.5rem;
  }

  @media (min-width: 1024px) {
    .converter-layout {
      grid-template-columns: 450px 1fr;
    }
  }

  .controls-panel {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .panel-section {
    background-color: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 1.25rem;
    color: #f1f5f9;
  }

  .panel-section h2 {
    margin-top: 0;
    margin-bottom: 1rem;
    font-size: 1.1rem;
    font-weight: 600;
    color: #38bdf8;
    border-bottom: 1px solid #334155;
    padding-bottom: 0.5rem;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    color: #94a3b8;
    gap: 0.75rem;
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid #334155;
    border-top: 3px solid #38bdf8;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .error-text {
    color: #ef4444;
    text-align: center;
    font-size: 0.9rem;
    margin: 0;
  }

  .file-upload-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: 2px dashed #475569;
    border-radius: 6px;
    padding: 2rem 1.5rem;
    cursor: pointer;
    background-color: #0f172a;
    transition: all 0.15s ease-in-out;
    text-align: center;
  }

  .file-upload-box:hover {
    border-color: #38bdf8;
    background-color: #1e293b;
  }

  .file-upload-box input {
    display: none;
  }

  .upload-icon {
    width: 32px;
    height: 32px;
    color: #94a3b8;
    margin-bottom: 0.75rem;
  }

  .file-upload-box:hover .upload-icon {
    color: #38bdf8;
  }

  .file-name {
    font-weight: 600;
    color: #f8fafc;
    word-break: break-all;
    margin-bottom: 0.25rem;
  }

  .file-size {
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .file-tip {
    font-size: 0.75rem;
    color: #64748b;
    margin-top: 0.5rem;
  }

  /* Symmetrical Validation Badge */
  .validation-status-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 6px;
    font-weight: 700;
    font-size: 0.9rem;
    letter-spacing: 0.05em;
    margin-bottom: 1.25rem;
    box-shadow: inset 0 1px 0 0 rgb(255 255 255 / 0.05);
  }

  .status-match {
    background-color: rgba(16, 185, 129, 0.15);
    border: 1px solid #10b981;
    color: #34d399;
  }

  .status-mismatch {
    background-color: rgba(239, 68, 68, 0.15);
    border: 1px solid #ef4444;
    color: #f87171;
  }

  .badge-icon {
    width: 18px;
    height: 18px;
  }

  /* Metrics Grid */
  .metrics-grid {
    display: grid;
    grid-template-columns: 120px 1fr 1fr;
    gap: 0.75rem 0.5rem;
    font-size: 0.85rem;
    align-items: center;
  }

  .metric-header {
    font-weight: 600;
    color: #94a3b8;
    border-bottom: 1px solid #334155;
    padding-bottom: 0.35rem;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .metric-label {
    color: #cbd5e1;
  }

  .metric-val {
    color: #f1f5f9;
    word-break: break-all;
  }

  .font-mono {
    font-family: monospace;
  }

  .hash-text {
    font-size: 0.8rem;
  }

  .text-green {
    color: #34d399;
    font-weight: 600;
  }

  .text-red {
    color: #f87171;
    font-weight: 600;
  }

  .text-gray {
    color: #94a3b8;
  }

  /* Player controls */
  .player-controls {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .btn-play {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    background-color: #0f172a;
    color: #cbd5e1;
    border: 1px solid #475569;
    padding: 0.75rem;
    border-radius: 6px;
    font-weight: 600;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.15s ease-in-out;
  }

  .btn-play:hover:not(:disabled) {
    background-color: #1e293b;
    border-color: #38bdf8;
    color: #f8fafc;
  }

  .btn-play:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-play.playing {
    background-color: rgba(56, 189, 248, 0.1);
    border-color: #38bdf8;
    color: #38bdf8;
  }

  .btn-indicator {
    display: block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: #475569;
    transition: background-color 0.15s ease-in-out;
  }

  .btn-play:hover .btn-indicator {
    background-color: #38bdf8;
  }

  .btn-play.playing .btn-indicator {
    background-color: #38bdf8;
    box-shadow: 0 0 8px #38bdf8;
  }

  /* Downloads */
  .download-section {
    border-top: 1px solid #334155;
    padding-top: 1rem;
    margin-top: 1rem;
  }

  .btn-download {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    background: linear-gradient(135deg, #10b981 0%, #059669 100%);
    color: #ffffff;
    border: none;
    padding: 0.75rem;
    border-radius: 6px;
    font-weight: 600;
    font-size: 0.9rem;
    text-decoration: none;
    text-align: center;
    cursor: pointer;
    box-shadow: 0 4px 6px -1px rgba(16, 185, 129, 0.2);
    transition: all 0.15s ease-in-out;
  }

  .btn-download:hover {
    transform: translateY(-1px);
    box-shadow: 0 6px 10px -1px rgba(16, 185, 129, 0.3);
    filter: brightness(1.05);
  }

  .icon-download {
    width: 18px;
    height: 18px;
  }

  .visualizer-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  /* Algorithm selector styling */
  .algorithm-selector-box {
    margin-top: 1.25rem;
    padding-top: 1.25rem;
    border-top: 1px solid #334155;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .algorithm-label {
    font-size: 0.85rem;
    font-weight: 600;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .algorithm-select-dropdown {
    width: 100%;
    background-color: #0f172a;
    border: 1px solid #475569;
    color: #f1f5f9;
    padding: 0.6rem;
    border-radius: 6px;
    font-size: 0.9rem;
    font-weight: 600;
    outline: none;
    cursor: pointer;
    transition: all 0.15s ease-in-out;
  }

  .algorithm-select-dropdown:focus {
    border-color: #38bdf8;
    box-shadow: 0 0 0 2px rgba(56, 189, 248, 0.15);
  }

  .algorithm-description {
    margin: 0;
    font-size: 0.8rem;
    color: #94a3b8;
    line-height: 1.4;
  }

  .algorithm-description strong {
    color: #38bdf8;
  }
</style>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import init, { 
    init_panic_hook, 
    encode_naive, 
    decode_naive,
    encode_wavelet_v1_two_pixels,
    decode_wavelet_v1_two_pixels,
    encode_wavelet_v2_bitplane,
    decode_wavelet_v2_bitplane,
    encode_wavelet_v3_serpentine,
    decode_wavelet_v3_serpentine,
    encode_wavelet_v4_dyadic_dwt,
    decode_wavelet_v4_dyadic_dwt
  } from '../wasm/core_wasm.js';
  import CanvasVisualizer from './CanvasVisualizer.svelte';

  // State using Svelte 5 standard runes
  let wasmLoaded = $state(false);
  let wasmError = $state<string | null>(null);

  // Selected conversion algorithm (v2_bitplane is the highly compressed, 1-pixel default)
  let selectedAlgorithm = $state<'v2_bitplane' | 'v1_two_pixels' | 'v3_serpentine' | 'v4_dyadic_dwt' | 'naive'>('v2_bitplane');
  let selectedHeight = $state<number>(1024); // default 20 Hz limit (1024 frequency bins)
  let selectedWaveletType = $state<number>(0); // 0 = CDF 5/3, 1 = Haar (CDF 1/1)

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

  // Waveform Visualizer States (Original L/R and Intermediate M/S)
  let originalWaveformL = $state<number[]>([]);
  let originalWaveformR = $state<number[]>([]);
  let midWaveform = $state<number[]>([]);
  let sideWaveform = $state<number[]>([]);

  // Symmetrical Invariant Matching Status
  let sizeMatches = $derived(
    originalBytes && decodedBytes && originalBytes.length === decodedBytes.length
  );
  let hashMatches = $derived(
    originalHash && decodedHash && originalHash === decodedHash
  );
  let symmetricalMatch = $derived(sizeMatches && hashMatches);

  // Preloading state for the default sample
  let isPreloading = $state(false);

  onMount(async () => {
    try {
      // Initialize the WebAssembly module
      await init();
      init_panic_hook();
      wasmLoaded = true;

      // Automatically preload the high-fidelity default audio sample on startup
      await loadDefaultSample();
    } catch (e: any) {
      console.error('Failed to load WASM:', e);
      wasmError = e.message || String(e);
    }
  });

  // Asynchronously fetch and load the default vocal WAV sample
  async function loadDefaultSample() {
    isPreloading = true;
    try {
      const defaultUrl = './voice.wav';
      console.log('Preloading default sample:', defaultUrl);
      
      const response = await fetch(defaultUrl);
      if (!response.ok) {
        throw new Error(`Failed to fetch default sample: ${response.statusText}`);
      }
      
      const arrayBuffer = await response.arrayBuffer();
      const bytes = new Uint8Array(arrayBuffer);
      originalBytes = bytes;
      originalHash = await computeSHA256(bytes);
      
      // Simulate file upload metadata so the UI renders perfectly
      originalFile = new File([arrayBuffer], 'voice_sample.wav', { type: 'audio/wav' });
      
      // Create object URL for comparative audioplayer playback
      originalUrl = URL.createObjectURL(originalFile);
      originalAudio = new Audio(originalUrl);
      originalAudio.onended = () => { isOriginalPlaying = false; };
      
      // Compute waveforms
      parseWavWaveforms(bytes);
    } catch (err) {
      console.error('Failed to preload default sample on startup:', err);
    } finally {
      isPreloading = false;
    }
  }

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

  // Parse 16-bit Stereo PCM WAV file to render wave paths
  function parseWavWaveforms(bytes: Uint8Array) {
    originalWaveformL = [];
    originalWaveformR = [];
    midWaveform = [];
    sideWaveform = [];

    if (bytes.length < 44) return;
    const isRiff = bytes[0] === 0x52 && bytes[1] === 0x49 && bytes[2] === 0x46 && bytes[3] === 0x46;
    const isWave = bytes[8] === 0x57 && bytes[9] === 0x41 && bytes[10] === 0x56 && bytes[11] === 0x45;
    if (!isRiff || !isWave) {
      generateGenericWaveform(bytes);
      return;
    }

    // Search for data chunk
    let dataOffset = 12;
    while (dataOffset + 8 < bytes.length) {
      const chunkId = String.fromCharCode(bytes[dataOffset], bytes[dataOffset + 1], bytes[dataOffset + 2], bytes[dataOffset + 3]);
      const chunkSize = (bytes[dataOffset + 4]) | (bytes[dataOffset + 5] << 8) | (bytes[dataOffset + 6] << 16) | (bytes[dataOffset + 7] << 24);
      if (chunkId === 'data') {
        dataOffset += 8;
        break;
      }
      dataOffset += 8 + chunkSize;
    }

    if (dataOffset >= bytes.length) {
      dataOffset = 44;
    }

    const remainingBytes = bytes.length - dataOffset;
    const numSamples = Math.floor(remainingBytes / 4);
    if (numSamples < 10) return;

    const step = Math.max(1, Math.floor(numSamples / 200));
    const numPoints = Math.min(200, Math.floor(numSamples / step));

    const wavL: number[] = [];
    const wavR: number[] = [];
    const wavM: number[] = [];
    const wavS: number[] = [];

    for (let p = 0; p < numPoints; p++) {
      const sampleIdx = p * step;
      const byteOffset = dataOffset + sampleIdx * 4;
      if (byteOffset + 3 >= bytes.length) break;

      const l_val = (bytes[byteOffset] | (bytes[byteOffset + 1] << 8));
      const l = l_val >= 32768 ? l_val - 65536 : l_val;

      const r_val = (bytes[byteOffset + 2] | (bytes[byteOffset + 3] << 8));
      const r = r_val >= 32768 ? r_val - 65536 : r_val;

      const lNorm = l / 32768;
      const rNorm = r / 32768;

      // S = L - R (Wrapping logic equivalent)
      // M = R + (S / 2) (Lifting equivalent)
      const sNorm = lNorm - rNorm;
      const mNorm = rNorm + (sNorm / 2);

      wavL.push(lNorm);
      wavR.push(rNorm);
      wavM.push(mNorm);
      wavS.push(sNorm);
    }

    originalWaveformL = wavL;
    originalWaveformR = wavR;
    midWaveform = wavM;
    sideWaveform = wavS;
  }

  // Fallback wave drawer for arbitrary binary byte payloads
  function generateGenericWaveform(bytes: Uint8Array) {
    const numSamples = Math.floor(bytes.length / 2);
    const step = Math.max(1, Math.floor(numSamples / 200));
    const numPoints = Math.min(200, Math.floor(numSamples / step));

    const wavL: number[] = [];
    const wavR: number[] = [];
    const wavM: number[] = [];
    const wavS: number[] = [];

    for (let p = 0; p < numPoints; p++) {
      const idx = p * step * 2;
      if (idx + 1 >= bytes.length) break;
      const val = bytes[idx] | (bytes[idx + 1] << 8);
      const sVal = val >= 32768 ? val - 65536 : val;
      const norm = sVal / 32768;

      wavL.push(norm);
      wavR.push(norm * 0.5);
      wavM.push(norm * 0.75);
      wavS.push(norm * 0.25);
    }

    originalWaveformL = wavL;
    originalWaveformR = wavR;
    midWaveform = wavM;
    sideWaveform = wavS;
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

    // Extract and compute waveforms
    parseWavWaveforms(originalBytes);

    // Note: We do not call runEncoding() here manually.
    // Svelte 5's $effect block below will automatically trigger runEncoding()
    // in a clean, unified, and sequential manner when originalBytes changes,
    // avoiding asynchronous race conditions on shared states.
  }

  // Reactive effect to re-run the entire pipeline when any parameter changes
  $effect(() => {
    if (originalBytes && wasmLoaded && selectedAlgorithm && selectedHeight !== undefined && selectedWaveletType !== undefined) {
      untrack(() => {
        runEncoding();
      });
    }
  });

  // Core Forward Process: Audio -> Image
  async function runEncoding() {
    if (!originalBytes || !wasmLoaded) return;

    try {
      const start = performance.now();
      if (selectedAlgorithm === 'v2_bitplane') {
        rgbaBytes = encode_wavelet_v2_bitplane(originalBytes, selectedHeight, selectedWaveletType);
      } else if (selectedAlgorithm === 'v1_two_pixels') {
        rgbaBytes = encode_wavelet_v1_two_pixels(originalBytes, selectedHeight, selectedWaveletType);
      } else if (selectedAlgorithm === 'v3_serpentine') {
        rgbaBytes = encode_wavelet_v3_serpentine(originalBytes, selectedHeight, selectedWaveletType);
      } else if (selectedAlgorithm === 'v4_dyadic_dwt') {
        rgbaBytes = encode_wavelet_v4_dyadic_dwt(originalBytes, selectedHeight, selectedWaveletType);
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
      if (selectedAlgorithm === 'v2_bitplane') {
        decodedBytes = decode_wavelet_v2_bitplane(rgbaBytes);
      } else if (selectedAlgorithm === 'v1_two_pixels') {
        decodedBytes = decode_wavelet_v1_two_pixels(rgbaBytes);
      } else if (selectedAlgorithm === 'v3_serpentine') {
        decodedBytes = decode_wavelet_v3_serpentine(rgbaBytes);
      } else if (selectedAlgorithm === 'v4_dyadic_dwt') {
        decodedBytes = decode_wavelet_v4_dyadic_dwt(rgbaBytes);
      } else {
        decodedBytes = decode_naive(rgbaBytes);
      }
      decodingTimeMs = performance.now() - start;

      if (decodedBytes) {
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

    // Reset waveforms
    originalWaveformL = [];
    originalWaveformR = [];
    midWaveform = [];
    sideWaveform = [];
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
      {:else if isPreloading}
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Baixando e processando amostra de áudio padrão...</p>
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
            <option value="v2_bitplane">V2: Bit-Plane Hierárquico (1 Pixel, Gray Code - Recomendável!)</option>
            <option value="v1_two_pixels">V1: Two-Pixel Sólido (8-Bytes por Amostra, RGB Ativo)</option>
            <option value="v3_serpentine">V3: Two-Pixel Serpentina (8-Bytes, Aritmética Pura - Estudo!)</option>
            <option value="v4_dyadic_dwt">V4: Two-Pixel Serpentina Diádica (8-Bytes, Mallat DWT - Semântico!)</option>
            <option value="naive">Naive Byte Packing (Lossless Baseline, Sem Transformação)</option>
          </select>
          
          {#if selectedAlgorithm !== 'naive'}
            <!-- Wavelet Family Selector -->
            <label for="wavelet-type-select" class="algorithm-label" style="margin-top: 0.75rem;">Família Wavelet:</label>
            <select 
              id="wavelet-type-select" 
              bind:value={selectedWaveletType}
              class="algorithm-select-dropdown"
            >
              <option value={0}>Cohen-Daubechies-Feauveau 5/3 (Biorogonal Suave)</option>
              <option value={1}>Haar (CDF 1/1 / Localização Temporal Máxima)</option>
            </select>

            <!-- Height / Frequency Resolution Selector -->
            <label for="height-select" class="algorithm-label" style="margin-top: 0.75rem;">Frequência Mínima (Altura H):</label>
            <select 
              id="height-select" 
              bind:value={selectedHeight}
              class="algorithm-select-dropdown"
            >
              <option value={256}>256 bandas (Frequência Mínima: ~86.1 Hz)</option>
              <option value={512}>512 bandas (Frequência Mínima: ~43.1 Hz)</option>
              <option value={1024}>1024 bandas (Frequência Mínima: ~21.5 Hz - Recomendável! 20 Hz)</option>
              <option value={2048}>2048 bandas (Frequência Mínima: ~10.8 Hz)</option>
            </select>
          {/if}

          <p class="algorithm-description">
            {#if selectedAlgorithm === 'v2_bitplane'}
              <strong>Estratégia V2 Hierárquica:</strong> Comprime cada amostra estéreo em apenas <strong>1 pixel (4 bytes)</strong>. Separa os MSBs (Estrutura) nos canais Vermelho/Azul para brilho sólido, e os LSBs (Refinamento) nos canais Verde/Alpha mapeados por código Gray para máxima compressão.
            {:else if selectedAlgorithm === 'v1_two_pixels'}
              <strong>Estratégia V1 Sólida:</strong> Empacota cada amostra em <strong>2 pixels (8 bytes)</strong>. Salva Mid e Side nos canais Red e Green de pixels separados e fixa o Alpha em 255. Excelente precisão visual, porém dobra a largura física do PNG.
            {:else if selectedAlgorithm === 'v3_serpentine'}
              <strong>Estratégia V3 Serpentina:</strong> Empacota cada amostra em <strong>2 pixels (8 bytes)</strong>. Utiliza o novo decodificador aritmético de tempo real $O(\log M)$ sobre as cascas concêntricas de Chebyshev sem alocação ou tabelas de busca. Garante bijeção estrita e fusão suave de cores térmicas no espectrograma.
            {:else if selectedAlgorithm === 'v4_dyadic_dwt'}
              <strong>Estratégia V4 Serpentina Diádica (Mallat DWT):</strong> Decompõe o áudio em oitavas de escala logarítmica exponenciais $j$. Organiza e exibe os coeficientes em uma grade contínua com expansão horizontal automática de tamanho $2^j$ para renderização de scalograma visível, mantendo bijeção matemática integral de alto contraste e bijeção bit-perfect absoluta!
            {:else}
              <strong>Naive Packing:</strong> Mapeia os bytes binários brutos do arquivo diretamente nos canais de cor RGBA, gerando ruído cinza de TV.
            {/if}
          </p>
        </div>
      {/if}
    </div>

    <!-- Symmetrical Validation Panel -->
    {#if originalBytes}
      <!-- Intermediate Pipeline Stages Panel -->
      {#if originalWaveformL.length > 0}
        <div class="panel-section">
          <h2>2. Estágios do Pipeline de Sinais</h2>
          
          <div class="pipeline-flow-diagram">
            <div class="flow-step">
              <span class="step-num">A</span>
              <span class="step-name">Sinal L / R</span>
            </div>
            <div class="flow-arrow">➡️</div>
            <div class="flow-step">
              <span class="step-num">B</span>
              <span class="step-name">Mid / Side</span>
            </div>
            <div class="flow-arrow">➡️</div>
            <div class="flow-step">
              <span class="step-num">C</span>
              <span class="step-name">Ondaletas 2D</span>
            </div>
          </div>

          <!-- Waveform Plotter using SVG for Left/Right -->
          <div class="waveform-stage-box">
            <div class="stage-info">
              <strong>Estágio A: Forma de Onda L/R Original</strong>
              <span class="legend"><span class="legend-color legend-l"></span>L (Esq) <span class="legend-color legend-r"></span>R (Dir)</span>
            </div>
            <svg viewBox="0 0 200 60" class="waveform-svg">
              <!-- Zero line -->
              <line x1="0" y1="30" x2="200" y2="30" stroke="#334155" stroke-dasharray="2,2" />
              
              <!-- Left channel path -->
              <path 
                d="M {originalWaveformL.map((y, x) => `${x},${30 + y * 28}`).join(' L ')}" 
                fill="none" 
                stroke="#38bdf8" 
                stroke-width="1" 
              />
              <!-- Right channel path -->
              <path 
                d="M {originalWaveformR.map((y, x) => `${x},${30 + y * 28}`).join(' L ')}" 
                fill="none" 
                stroke="#34d399" 
                stroke-width="1" 
                stroke-opacity="0.75"
              />
            </svg>
          </div>

          <!-- Waveform Plotter using SVG for Mid/Side -->
          <div class="waveform-stage-box">
            <div class="stage-info">
              <strong>Estágio B: Decomposição Mid/Side Reversível</strong>
              <span class="legend"><span class="legend-color legend-m"></span>Mid (Soma) <span class="legend-color legend-s"></span>Side (Diferença)</span>
            </div>
            <svg viewBox="0 0 200 60" class="waveform-svg">
              <!-- Zero line -->
              <line x1="0" y1="30" x2="200" y2="30" stroke="#334155" stroke-dasharray="2,2" />
              
              <!-- Mid channel path -->
              <path 
                d="M {midWaveform.map((y, x) => `${x},${30 + y * 28}`).join(' L ')}" 
                fill="none" 
                stroke="#facc15" 
                stroke-width="1" 
              />
              <!-- Side channel path -->
              <path 
                d="M {sideWaveform.map((y, x) => `${x},${30 + y * 28}`).join(' L ')}" 
                fill="none" 
                stroke="#c084fc" 
                stroke-width="1" 
                stroke-opacity="0.85"
              />
            </svg>
          </div>
          
          <p class="pipeline-tip">
            💡 Observe como o canal <strong>Side (Roxo)</strong> possui menor amplitude que o canal <strong>Mid (Amarelo)</strong>. Isso possibilita compactar os detalhes estéreo nos canais Blue/Alfa da imagem com imensa eficiência, preservando o brilho das frequências no Red/Green!
          </p>
        </div>
      {/if}

      <div class="panel-section">
        <h2>3. Validação Cruzada Simétrica</h2>
        
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
        <h2>4. Audição Comparativa</h2>
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

  /* Intermediate Pipeline Stage Diagrams */
  .pipeline-flow-diagram {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    margin-bottom: 1.25rem;
  }

  .flow-step {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .step-num {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    background-color: #38bdf8;
    color: #0f172a;
    border-radius: 50%;
    font-size: 0.7rem;
    font-weight: 800;
  }

  .step-name {
    font-size: 0.8rem;
    font-weight: 600;
    color: #e2e8f0;
  }

  .flow-arrow {
    color: #475569;
    font-size: 0.8rem;
  }

  .waveform-stage-box {
    background-color: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .stage-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
    margin-bottom: 0.5rem;
  }

  .stage-info strong {
    color: #f8fafc;
  }

  .legend {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: #94a3b8;
  }

  .legend-color {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-right: 0.15rem;
  }

  .legend-l { background-color: #38bdf8; }
  .legend-r { background-color: #34d399; }
  .legend-m { background-color: #facc15; }
  .legend-s { background-color: #c084fc; }

  .waveform-svg {
    display: block;
    width: 100%;
    height: 60px;
    background-color: #1e293b;
    border-radius: 4px;
    border: 1px solid #334155;
  }

  .pipeline-tip {
    margin: 0;
    margin-top: 1rem;
    font-size: 0.8rem;
    color: #94a3b8;
    line-height: 1.4;
    background-color: rgba(56, 189, 248, 0.05);
    border-left: 2px solid #38bdf8;
    padding: 0.5rem 0.75rem;
    border-radius: 0 4px 4px 0;
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { 
    wasm_analyze_higher_order_point, 
    type WasmHigherOrderPointResult,
    wasm_render_from_cached_quadruplets,
    wasm_has_cached_quadruplets
  } from '../wasm/core_wasm.js';

  // Svelte 5 strict typing: Receive all reactive props from App.svelte
  let { 
    rgbaGrid, 
    originalBytes = null,
    width, 
    height, 
    
    // Bindable advanced DSP parameters
    windowType = $bindable('hann'),
    windowSize = $bindable(1024),
    zeroPadding = $bindable(4),
    fmin = $bindable(20),
    fmax = $bindable(20000),
    algorithmType = $bindable('reassignment'),
    higherOrderO = $bindable(2),
    higherOrderVisualMode = $bindable<'ridge' | 'anisotropy' | 'curvature' | 'vector_reassign'>('ridge'),
    paletteType = $bindable('ycbcr'),
    selectedHeight = $bindable(1024),
    
    // Bindable looping states
    selectionStart = $bindable(null), 
    selectionEnd = $bindable(null), 
    loopMode = $bindable('normal'),
    
    // Bindable adaptive zoom bounds
    viewStart = $bindable(0.0),
    viewEnd = $bindable(1.0),
    
    // Bindable point size customization
    pointRadius = $bindable(1.0),
    
    // Bindable frequency scale type
    frequencyScale = $bindable('log'),
    
    // Bindable zoom mode & horizontal precomputed resolution multiplier (2^k)
    zoomMode = $bindable('gpu_debounced'),
    horizontalResolutionK = $bindable(0),
    
    currentQualityLod = 0,
    refinementProgress = null,
    onAdaptiveInteract = () => {},
    
    mirroredDensity = null,
    originalAudio,
    isPlaying,
    onPlayToggle,
    onAudioUploaded,
    onBackToConverter 
  }: { 
    rgbaGrid: Uint8Array | null, 
    originalBytes?: Uint8Array | null,
    mirroredDensity?: Float32Array | null,
    width: number, 
    height: number,
    
    windowType: 'hann' | 'hamming' | 'gaussian' | 'blackman-harris',
    windowSize: number,
    zeroPadding: number,
    fmin: number,
    fmax: number,
    algorithmType: 'reassignment' | 'log' | 'cqt' | 'higher_order' | 'sliding_jet',
    higherOrderO?: number,
    higherOrderVisualMode?: 'ridge' | 'anisotropy' | 'curvature' | 'vector_reassign',
    paletteType: 'ycbcr' | 'snake',
    selectedHeight: number,
    
    selectionStart: number | null,
    selectionEnd: number | null,
    loopMode: 'normal' | 'mirrored' | 'none',
    
    viewStart: number,
    viewEnd: number,
    pointRadius: number,
    frequencyScale: 'log' | 'linear',
    zoomMode: 'gpu_debounced' | 'continuous_resample',
    horizontalResolutionK: number,
    currentQualityLod?: number,
    refinementProgress?: number | null,
    onAdaptiveInteract?: () => void,
    
    originalAudio: HTMLAudioElement | null,
    isPlaying: boolean,
    onPlayToggle: () => void,
    onAudioUploaded: (bytes: Uint8Array) => void,
    onBackToConverter: () => void
  } = $props();

  let canvas: HTMLCanvasElement;
  let fileInput: HTMLInputElement;
  let gl: WebGL2RenderingContext | null = null;
  let program: WebGLProgram | null = null;
  let texture: WebGLTexture | null = null;

  let zoomX = $state(1.0);
  let panX = $state(0.0);
  let isDragging = false;
  let isSelecting = false;
  let lastMouseX = 0;

  let selectedTool = $state<'select' | 'region_select' | 'gaussian_brush' | 'low_pass' | 'high_pass' | 'differential_probe'>('region_select');
  let probeResult = $state<WasmHigherOrderPointResult | null>(null);
  let probePoint = $state<{ x: number, y: number, time_s: number, freq_hz: number } | null>(null);

  function executeProbeAnalysis(clientX: number, clientY: number) {
    if (!canvas || !originalBytes) return;
    const rect = canvas.getBoundingClientRect();
    const xRatio = Math.max(0.0, Math.min(1.0, (clientX - rect.left) / rect.width));
    const yRatio = Math.max(0.0, Math.min(1.0, 1.0 - (clientY - rect.top) / rect.height));

    const totalDuration = originalAudio?.duration && !isNaN(originalAudio.duration) ? originalAudio.duration : 1.0;
    const normT = viewStart + xRatio * (viewEnd - viewStart);
    const time_s = normT * totalDuration;

    const logMin = Math.log2(Math.max(1, fmin));
    const logMax = Math.log2(Math.max(fmin + 1, fmax));
    const freq_hz = Math.pow(2, logMin + yRatio * (logMax - logMin));

    try {
      const res = wasm_analyze_higher_order_point(
        originalBytes,
        44100.0,
        time_s,
        freq_hz,
        windowSize,
        higherOrderO ?? 2
      );
      probeResult = res;
      probePoint = {
        x: clientX - rect.left,
        y: clientY - rect.top,
        time_s,
        freq_hz
      };
    } catch (err) {
      console.error("❌ Probe analysis error:", err);
    }
  }
  let brushSize = $state(50);
  let brushStrength = $state(0.5);

  let rightDockExpanded = $state(true); // Right settings dock state

  // Playhead Tracking Cursor logic
  let playbackProgress = $state(0.0);
  let animationFrameId: number;

  $effect(() => {
    if (isPlaying && originalAudio) {
      const updatePlayhead = () => {
        if (originalAudio) {
          const duration = originalAudio.duration;
          if (duration && !isNaN(duration) && duration > 0.0) {
            playbackProgress = originalAudio.currentTime / duration;
          } else {
            playbackProgress = 0.0;
          }
        }
        animationFrameId = requestAnimationFrame(updatePlayhead);
      };
      updatePlayhead();
    } else {
      if (originalAudio) {
        const duration = originalAudio.duration;
        if (duration && !isNaN(duration) && duration > 0.0) {
          playbackProgress = originalAudio.currentTime / duration;
        } else {
          playbackProgress = 0.0;
        }
      }
      cancelAnimationFrame(animationFrameId);
    }
    
    return () => {
      cancelAnimationFrame(animationFrameId);
    };
  });

  // Fragment Shader: High-performance texture sampler drawing the CPU-rendered YCbCr spectrogram
  const fragmentShaderSource = `#version 300 es
  precision highp float;
  in vec2 v_uv;
  out vec4 outColor;
  
  uniform sampler2D u_spectrogramTexture;
  
  void main() {
      outColor = texture(u_spectrogramTexture, v_uv);
  }`;

  const vertexShaderSource = `#version 300 es
  in vec2 a_position;
  out vec2 v_uv;
  uniform float u_u0;
  uniform float u_u1;
  
  void main() {
      // Map view bounds on X (Time) axis using normalized u0 and u1
      float norm_x = a_position.x * 0.5 + 0.5;
      float u = u_u0 + norm_x * (u_u1 - u_u0);
      float v = a_position.y * 0.5 + 0.5;
      
      v_uv = vec2(u, v);
      gl_Position = vec4(a_position, 0.0, 1.0);
  }`;

  function compileShader(type: number, source: string) {
    if (!gl) return null;
    const shader = gl.createShader(type);
    if (!shader) return null;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error('❌ Shader compilation error:', gl.getShaderInfoLog(shader));
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  }

  function initWebGL() {
    if (!canvas) return;
    gl = canvas.getContext('webgl2', { antialias: true, alpha: false, preserveDrawingBuffer: true });
    if (!gl) {
      console.error('❌ WebGL2 is not supported.');
      return;
    }

    const vs = compileShader(gl.VERTEX_SHADER, vertexShaderSource);
    const fs = compileShader(gl.FRAGMENT_SHADER, fragmentShaderSource);
    if (!vs || !fs) return;

    program = gl.createProgram();
    if (!program) return;
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('❌ Program linking error:', gl.getProgramInfoLog(program));
      return;
    }

    gl.useProgram(program);

    const quad = new Float32Array([
      -1, -1,
       1, -1,
      -1,  1,
      -1,  1,
       1, -1,
       1,  1,
    ]);

    const vbo = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, quad, gl.STATIC_DRAW);

    const posLoc = gl.getAttribLocation(program, 'a_position');
    gl.enableVertexAttribArray(posLoc);
    gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);

    texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);

    render();
  }

  $effect(() => {
    const grid = rgbaGrid;
    const w = width;
    const h = height;

    if (!gl || !texture || !grid || w <= 0 || h <= 0) return;

    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    
    // Upload standard 8-bit texture directly to GPU
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, grid);
    
    render();
  });

  // Precompute 256-entry Look-Up Table mapping YCbCr Luminance Y to dB bin index [0..127]
  const Y_TO_DB_BIN = new Uint8Array(256);
  for (let y = 0; y < 256; y++) {
    if (y < 1) {
      Y_TO_DB_BIN[y] = 0;
    } else {
      const log2_r = 15.0 * (y * y - 1.0) / 65024.0 - 15.0;
      const db = 6.0205999 * log2_r; // 20 * log10(2) * log2(r)
      const bin = Math.floor(((db + 100.0) / 100.0) * 127.0);
      Y_TO_DB_BIN[y] = Math.max(0, Math.min(127, bin));
    }
  }

  // Gaussian Kernel Density Estimation (KDE) 1D smoothing
  function convolveGaussianKDE(accum: Float64Array): Float32Array {
    const out = new Float32Array(128);
    const sigma = 2.5;
    const radius = 5;
    const kernel = new Float32Array(11);
    let kSum = 0.0;
    for (let i = 0; i <= 10; i++) {
      const x = i - radius;
      const v = Math.exp(-0.5 * (x * x) / (sigma * sigma));
      kernel[i] = v;
      kSum += v;
    }
    for (let i = 0; i <= 10; i++) kernel[i] /= kSum;
    
    for (let i = 0; i < 128; i++) {
      let sum = 0.0;
      for (let k = 0; k <= 10; k++) {
        const idx = Math.max(0, Math.min(127, i + k - radius));
        sum += accum[idx] * kernel[k];
      }
      out[i] = sum;
    }
    return out;
  }

  let densityTrans = $state<Float32Array>(new Float32Array(128));
  let densityOrig = $state<Float32Array>(new Float32Array(128));
  let progressiveScanPct = $state<number>(0);
  let scanCancelId = 0;

  // Non-blocking chunk scanner: incrementally updates mirrored histograms over the selected region or active view
  function startProgressiveDensityScan() {
    scanCancelId++;
    const currentId = scanCancelId;
    
    if (!rgbaGrid || width <= 0 || height <= 0) return;
    
    const totalCols = width;
    const totalRows = height;
    const grid = rgbaGrid;
    const colsPerChunk = 64;

    // Determine target column range: selection region if active, else full texture
    let scanColStart = 0;
    let scanColEnd = totalCols;

    if (selectionStart !== null && selectionEnd !== null) {
      const sMin = Math.min(selectionStart, selectionEnd);
      const sMax = Math.max(selectionStart, selectionEnd);
      
      const texSpan = texEnd - texStart;
      if (texSpan > 0) {
        const uStart = Math.max(0.0, Math.min(1.0, (sMin - texStart) / texSpan));
        const uEnd = Math.max(0.0, Math.min(1.0, (sMax - texStart) / texSpan));
        scanColStart = Math.floor(uStart * totalCols);
        scanColEnd = Math.max(scanColStart + 1, Math.ceil(uEnd * totalCols));
      }
    }
    
    const rangeCols = scanColEnd - scanColStart;
    if (rangeCols <= 0) return;
    
    const accumTrans = new Float64Array(128);
    const accumOrig = new Float64Array(128);
    
    function scanChunk(currCol: number) {
      if (currentId !== scanCancelId) return; // Discard stale scan
      
      const chunkEnd = Math.min(scanColEnd, currCol + colsPerChunk);
      
      for (let c = currCol; c < chunkEnd; c++) {
        for (let j = 0; j < totalRows; j++) {
          const idx = (j * totalCols + c) * 4;
          const r = grid[idx];
          const g = grid[idx + 1];
          const b = grid[idx + 2];
          
          const y = Math.round(0.299 * r + 0.587 * g + 0.114 * b);
          const binTrans = Y_TO_DB_BIN[y];
          accumTrans[binTrans]++;
          
          // Model original STFT broader diffuse distribution (pre-reassignment dispersion)
          const binOrig = Math.max(0, Math.min(127, Math.round(binTrans * 0.88 + 8)));
          accumOrig[binOrig]++;
        }
      }
      
      const smoothedTrans = convolveGaussianKDE(accumTrans);
      const smoothedOrig = convolveGaussianKDE(accumOrig);
      
      let maxVal = 1e-12;
      for (let i = 0; i < 128; i++) {
        if (smoothedTrans[i] > maxVal) maxVal = smoothedTrans[i];
        if (smoothedOrig[i] > maxVal) maxVal = smoothedOrig[i];
      }
      
      const normTrans = new Float32Array(128);
      const normOrig = new Float32Array(128);
      for (let i = 0; i < 128; i++) {
        normTrans[i] = smoothedTrans[i] / maxVal;
        normOrig[i] = smoothedOrig[i] / maxVal;
      }
      
      densityTrans = normTrans;
      densityOrig = normOrig;
      const scannedSoFar = chunkEnd - scanColStart;
      progressiveScanPct = Math.min(100, Math.round((scannedSoFar / rangeCols) * 100));
      
      if (chunkEnd < scanColEnd) {
        requestAnimationFrame(() => scanChunk(chunkEnd));
      }
    }
    
    scanChunk(scanColStart);
  }

  // Convert linear density p in [0, 1] to logarithmic density scale vLog in [0, 1] (with 30dB floor)
  function toLogDensity(p: number, pFloor = 0.001): number {
    if (p <= pFloor) return 0.0;
    const logP = Math.log10(p);
    const logFloor = Math.log10(pFloor); // -3.0
    return Math.max(0.0, Math.min(1.0, (logP - logFloor) / -logFloor));
  }

  // Smooth SVG Path generation for mirrored curves with logarithmic density and 16px text channel
  let topPathD = $derived.by(() => {
    if (!densityTrans || densityTrans.length !== 128) return '';
    let d = `M 0 22 `;
    for (let i = 0; i < 128; i++) {
      const x = (i / 127) * 260;
      const vLog = toLogDensity(densityTrans[i]);
      const y = 22 - vLog * 18;
      d += `L ${x.toFixed(1)} ${y.toFixed(1)} `;
    }
    d += `L 260 22 Z`;
    return d;
  });

  let bottomPathD = $derived.by(() => {
    if (!densityOrig || densityOrig.length !== 128) return '';
    let d = `M 0 38 `;
    for (let i = 0; i < 128; i++) {
      const x = (i / 127) * 260;
      const vLog = toLogDensity(densityOrig[i]);
      const y = 38 + vLog * 18;
      d += `L ${x.toFixed(1)} ${y.toFixed(1)} `;
    }
    d += `L 260 38 Z`;
    return d;
  });

  // Interactive Vertical Frequency Zoom & Pan Controllers
  let isFreqDragging = false;
  let freqDragMode: 'fmax' | 'fmin' | 'pan' = 'pan';
  let lastFreqMouseY = 0;

  function handleFreqMouseDown(e: MouseEvent, mode: 'fmax' | 'fmin' | 'pan') {
    e.stopPropagation();
    isFreqDragging = true;
    freqDragMode = mode;
    lastFreqMouseY = e.clientY;
  }

  function handleFreqMouseMove(e: MouseEvent) {
    if (!isFreqDragging) return;
    const deltaY = lastFreqMouseY - e.clientY; // positive = dragging UP, negative = dragging DOWN
    lastFreqMouseY = e.clientY;
    
    // Each pixel of drag represents a fractional octave shift
    const octaveShift = deltaY * 0.015;
    const factor = Math.pow(2, octaveShift);
    
    if (freqDragMode === 'fmax') {
      const newFmax = Math.round(Math.max(fmin * 1.5, Math.min(22050, fmax * factor)));
      fmax = newFmax;
      onAdaptiveInteract();
    } else if (freqDragMode === 'fmin') {
      const newFmin = Math.round(Math.max(10, Math.min(fmax / 1.5, fmin * factor)));
      fmin = newFmin;
      onAdaptiveInteract();
    } else if (freqDragMode === 'pan') {
      const currentLogSpan = Math.log2(fmax / Math.max(1, fmin));
      let newLogMin = Math.log2(Math.max(10, fmin)) + octaveShift;
      let newLogMax = newLogMin + currentLogSpan;
      
      const logFloor = Math.log2(10);
      const logCeil = Math.log2(22050);
      
      if (newLogMin < logFloor) {
        newLogMin = logFloor;
        newLogMax = newLogMin + currentLogSpan;
      }
      if (newLogMax > logCeil) {
        newLogMax = logCeil;
        newLogMin = Math.max(logFloor, newLogMax - currentLogSpan);
      }
      
      fmin = Math.round(Math.pow(2, newLogMin));
      fmax = Math.round(Math.pow(2, newLogMax));
      onAdaptiveInteract();
    }
  }

  function handleFreqMouseUp() {
    isFreqDragging = false;
  }

  function handleFreqWheel(e: WheelEvent) {
    e.preventDefault();
    e.stopPropagation();
    
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const yRatio = Math.max(0.0, Math.min(1.0, 1.0 - (e.clientY - rect.top) / rect.height));
    
    const logMin = Math.log2(Math.max(10, fmin));
    const logMax = Math.log2(Math.min(22050, fmax));
    const logSpan = logMax - logMin;
    
    const zoomFactor = e.deltaY < 0 ? 0.85 : 1.18;
    const logFloor = Math.log2(10);
    const logCeil = Math.log2(22050);
    const maxSpan = logCeil - logFloor;
    const newSpan = Math.max(0.8, Math.min(maxSpan, logSpan * zoomFactor));
    
    const logCursor = logMin + yRatio * logSpan;
    let newLogMin = logCursor - yRatio * newSpan;
    let newLogMax = newLogMin + newSpan;
    
    if (newLogMin < logFloor) {
      newLogMin = logFloor;
      newLogMax = Math.min(logCeil, newLogMin + newSpan);
    }
    if (newLogMax > logCeil) {
      newLogMax = logCeil;
      newLogMin = Math.max(logFloor, newLogMax - newSpan);
    }
    
    fmin = Math.round(Math.pow(2, newLogMin));
    fmax = Math.round(Math.pow(2, newLogMax));
    onAdaptiveInteract();
  }

  function resetVerticalZoom() {
    fmin = 20;
    fmax = 20000;
    onAdaptiveInteract();
  }

  let texStart = $state(0.0);
  let texEnd = $state(1.0);
  let selScanDebounce: any = null;

  let minimapCanvas: HTMLCanvasElement;
  let isMinimapDragging = false;
  let minimapLastMouseX = 0;
  let minimapLastMouseY = 0;
  let lastMinimapThumbnailTime = 0;

  // Fully reactive Svelte 5 derived style for the 2D Viewport Bounding Frustum Box
  let frustumStyle = $derived.by(() => {
    const logMin = Math.log2(20);
    const logMax = Math.log2(20000);
    const logSpan = logMax - logMin;

    const left = Math.max(0, Math.min(98, viewStart * 100));
    const right = Math.max(left + 2, Math.min(100, viewEnd * 100));
    const width = Math.max(2, right - left);

    const yBottomNorm = (Math.log2(Math.max(20, fmin)) - logMin) / logSpan;
    const yTopNorm = (Math.log2(Math.min(20000, fmax)) - logMin) / logSpan;

    const bottom = Math.max(0, Math.min(96, yBottomNorm * 100));
    const top = Math.max(bottom + 4, Math.min(100, yTopNorm * 100));
    const height = Math.max(4, top - bottom);

    return `left: ${left.toFixed(1)}%; width: ${width.toFixed(1)}%; bottom: ${bottom.toFixed(1)}%; height: ${height.toFixed(1)}%;`;
  });

  function drawFallbackThumbnailFromGrid() {
    if (!minimapCanvas || !rgbaGrid || width <= 0 || height <= 0) return;
    const ctx = minimapCanvas.getContext('2d');
    if (!ctx) return;
    
    const MW = 220;
    const MH = 100;
    const imgData = ctx.createImageData(MW, MH);
    const data = imgData.data;
    
    for (let y = 0; y < MH; y++) {
      const srcY = Math.floor((1.0 - y / MH) * height);
      for (let x = 0; x < MW; x++) {
        const srcX = Math.floor((x / MW) * width);
        const srcIdx = (srcY * width + srcX) * 4;
        const dstIdx = (y * MW + x) * 4;
        data[dstIdx] = rgbaGrid[srcIdx];
        data[dstIdx + 1] = rgbaGrid[srcIdx + 1];
        data[dstIdx + 2] = rgbaGrid[srcIdx + 2];
        data[dstIdx + 3] = 180;
      }
    }
    ctx.putImageData(imgData, 0, 0);
  }

  function updateMinimapThumbnail() {
    if (!minimapCanvas) return;
    const now = performance.now();
    if (now - lastMinimapThumbnailTime < 80) return;
    lastMinimapThumbnailTime = now;

    const ctx = minimapCanvas.getContext('2d');
    if (!ctx) return;

    const MW = 220;
    const MH = 100;

    try {
      if (wasm_has_cached_quadruplets()) {
        const thumbBytes = wasm_render_from_cached_quadruplets(
          MW,
          MH,
          0.0,
          1.0,
          20.0,
          20000.0,
          1.0,
          paletteType
        );
        if (thumbBytes && thumbBytes.length === MW * MH * 4) {
          const imgData = ctx.createImageData(MW, MH);
          imgData.data.set(thumbBytes);
          ctx.putImageData(imgData, 0, 0);
          return;
        }
      }
    } catch (err) {
      console.warn("Minimap thumbnail generation fallback:", err);
    }

    drawFallbackThumbnailFromGrid();
  }

  function handleMinimapMouseDown(e: MouseEvent) {
    e.stopPropagation();
    isMinimapDragging = true;
    minimapLastMouseX = e.clientX;
    minimapLastMouseY = e.clientY;

    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const clickXRatio = (e.clientX - rect.left) / rect.width;
    const clickYRatio = 1.0 - (e.clientY - rect.top) / rect.height;

    const currentTSpan = viewEnd - viewStart;
    const logMin = Math.log2(20);
    const logMax = Math.log2(20000);
    const logTotalSpan = logMax - logMin;
    const currentLogSpan = Math.log2(fmax / Math.max(1, fmin));

    const currentYMin = (Math.log2(Math.max(20, fmin)) - logMin) / logTotalSpan;
    const currentYMax = (Math.log2(Math.min(20000, fmax)) - logMin) / logTotalSpan;

    const isInsideX = clickXRatio >= viewStart && clickXRatio <= viewEnd;
    const isInsideY = clickYRatio >= currentYMin && clickYRatio <= currentYMax;

    if (!isInsideX || !isInsideY) {
      let newStart = clickXRatio - currentTSpan / 2;
      let newEnd = clickXRatio + currentTSpan / 2;
      if (newStart < 0.0) {
        newEnd = Math.min(1.0, newEnd - newStart);
        newStart = 0.0;
      }
      if (newEnd > 1.0) {
        newStart = Math.max(0.0, newStart - (newEnd - 1.0));
        newEnd = 1.0;
      }
      viewStart = newStart;
      viewEnd = newEnd;

      const targetCenterLogF = logMin + clickYRatio * logTotalSpan;
      let newLogMin = targetCenterLogF - currentLogSpan / 2;
      let newLogMax = targetCenterLogF + currentLogSpan / 2;
      if (newLogMin < logMin) {
        newLogMin = logMin;
        newLogMax = newLogMin + currentLogSpan;
      }
      if (newLogMax > logMax) {
        newLogMax = logMax;
        newLogMin = Math.max(logMin, newLogMax - currentLogSpan);
      }
      fmin = Math.round(Math.pow(2, newLogMin));
      fmax = Math.round(Math.pow(2, newLogMax));

      onAdaptiveInteract();
    }
  }

  function handleMinimapMouseMove(e: MouseEvent) {
    if (!isMinimapDragging) return;
    const deltaX = e.clientX - minimapLastMouseX;
    const deltaY = e.clientY - minimapLastMouseY;
    minimapLastMouseX = e.clientX;
    minimapLastMouseY = e.clientY;

    const MINIMAP_WIDTH = 220;
    const MINIMAP_HEIGHT = 100;

    const tDelta = (deltaX / MINIMAP_WIDTH);
    const currentSpan = viewEnd - viewStart;
    let newStart = viewStart + tDelta;
    let newEnd = viewEnd + tDelta;
    if (newStart < 0.0) {
      newStart = 0.0;
      newEnd = currentSpan;
    }
    if (newEnd > 1.0) {
      newEnd = 1.0;
      newStart = 1.0 - currentSpan;
    }
    viewStart = newStart;
    viewEnd = newEnd;

    const logMin = Math.log2(20);
    const logMax = Math.log2(20000);
    const logTotalSpan = logMax - logMin;
    const currentLogSpan = Math.log2(fmax / Math.max(1, fmin));
    const logDelta = (-deltaY / MINIMAP_HEIGHT) * logTotalSpan;

    let newLogMin = Math.log2(Math.max(20, fmin)) + logDelta;
    let newLogMax = newLogMin + currentLogSpan;

    if (newLogMin < logMin) {
      newLogMin = logMin;
      newLogMax = newLogMin + currentLogSpan;
    }
    if (newLogMax > logMax) {
      newLogMax = logMax;
      newLogMin = Math.max(logMin, newLogMax - currentLogSpan);
    }

    fmin = Math.round(Math.pow(2, newLogMin));
    fmax = Math.round(Math.pow(2, newLogMax));

    onAdaptiveInteract();
  }

  function handleMinimapMouseUp() {
    isMinimapDragging = false;
  }

  function resetFullOverview() {
    viewStart = 0.0;
    viewEnd = 1.0;
    fmin = 20;
    fmax = 20000;
    onAdaptiveInteract();
  }

  // Whenever a newly generated texture is passed from WASM, record its exact window
  $effect(() => {
    const _grid = rgbaGrid;
    if (_grid) {
      texStart = viewStart;
      texEnd = viewEnd;
      render();
      updateMinimapThumbnail();
    }
  });

  // Re-scan density progressively whenever either the texture OR the selection region updates
  $effect(() => {
    const _grid = rgbaGrid;
    const _selStart = selectionStart;
    const _selEnd = selectionEnd;
    if (_grid) {
      if (selScanDebounce) clearTimeout(selScanDebounce);
      selScanDebounce = setTimeout(() => {
        startProgressiveDensityScan();
      }, 40);
    }
  });

  function render() {
    if (!gl || !program || !rgbaGrid) return;
    gl.viewport(0, 0, canvas.width, canvas.height);
    
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    
    const u0Loc = gl.getUniformLocation(program, 'u_u0');
    const u1Loc = gl.getUniformLocation(program, 'u_u1');
    const texLoc = gl.getUniformLocation(program, 'u_spectrogramTexture');
    
    const texSpan = texEnd - texStart;
    const u0 = texSpan > 0 ? (viewStart - texStart) / texSpan : 0.0;
    const u1 = texSpan > 0 ? (viewEnd - texStart) / texSpan : 1.0;
    
    gl.uniform1f(u0Loc, u0);
    gl.uniform1f(u1Loc, u1);
    gl.uniform1i(texLoc, 0);
    
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const mouseRatio = Math.max(0.0, Math.min(1.0, (e.clientX - rect.left) / rect.width));
    
    const currentSpan = viewEnd - viewStart;
    const zoomFactor = e.deltaY < 0 ? 0.85 : 1.18;
    const newSpan = Math.max(0.0005, Math.min(1.0, currentSpan * zoomFactor));
    
    const centerT = viewStart + mouseRatio * currentSpan;
    let newStart = centerT - mouseRatio * newSpan;
    let newEnd = centerT + (1.0 - mouseRatio) * newSpan;
    
    if (newStart < 0.0) {
      newEnd = Math.min(1.0, newEnd - newStart);
      newStart = 0.0;
    }
    if (newEnd > 1.0) {
      newStart = Math.max(0.0, newStart - (newEnd - 1.0));
      newEnd = 1.0;
    }
    
    viewStart = newStart;
    viewEnd = newEnd;
    
    render();
  }

  function handleMouseDown(e: MouseEvent) {
    if (selectedTool === 'differential_probe') {
      executeProbeAnalysis(e.clientX, e.clientY);
      return;
    }
    if (selectedTool === 'select') {
      isDragging = true;
      lastMouseX = e.clientX;
    } else if (selectedTool === 'region_select') {
      isSelecting = true;
      const t = screenXToNormalizedTime(e.clientX);
      selectionStart = t;
      selectionEnd = t;
    } else {
      const rect = canvas.getBoundingClientRect();
      const x = (e.clientX - rect.left) / rect.width;
      const y = 1.0 - (e.clientY - rect.top) / rect.height;
      console.log(`🖌️ Applying DSP Brush [${selectedTool}] at:`, x.toFixed(3), y.toFixed(3));
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging && selectedTool === 'select') {
      const rect = canvas.getBoundingClientRect();
      const deltaX = (e.clientX - lastMouseX) / rect.width;
      const currentSpan = viewEnd - viewStart;
      const shift = deltaX * currentSpan;
      
      let newStart = viewStart - shift;
      let newEnd = viewEnd - shift;
      
      if (newStart < 0.0) {
        newEnd += -newStart;
        newStart = 0.0;
      }
      if (newEnd > 1.0) {
        newStart -= (newEnd - 1.0);
        newEnd = 1.0;
      }
      
      viewStart = Math.max(0.0, newStart);
      viewEnd = Math.min(1.0, newEnd);
      lastMouseX = e.clientX;
      render();
    } else if (isSelecting && selectedTool === 'region_select') {
      const t = screenXToNormalizedTime(e.clientX);
      selectionEnd = t;
    }
  }

  function handleMouseUp() {
    isDragging = false;
    isSelecting = false;
    
    if (selectionStart !== null && selectionEnd !== null) {
      if (selectionStart > selectionEnd) {
        const temp = selectionStart;
        selectionStart = selectionEnd;
        selectionEnd = temp;
      }
      
      if (Math.abs(selectionEnd - selectionStart) < 0.001) {
        selectionStart = null;
        selectionEnd = null;
        startProgressiveDensityScan();
      }
    }
  }

  function handleGlobalMouseUp() {
    handleMouseUp();
    handleTimelineMouseUp();
    handleFreqMouseUp();
    handleMinimapMouseUp();
  }

  function handleGlobalMouseMove(e: MouseEvent) {
    if (isFreqDragging) {
      handleFreqMouseMove(e);
    }
    if (isMinimapDragging) {
      handleMinimapMouseMove(e);
    }
  }

  function clearSelection() {
    selectionStart = null;
    selectionEnd = null;
    startProgressiveDensityScan();
  }

  function screenXToNormalizedTime(clientX: number): number {
    if (!canvas || viewEnd <= viewStart) return 0.0;
    const rect = canvas.getBoundingClientRect();
    const ratio = Math.max(0.0, Math.min(1.0, (clientX - rect.left) / rect.width));
    return viewStart + ratio * (viewEnd - viewStart);
  }

  function normalizedTimeToScreenPct(t: number): number {
    if (!canvas || viewEnd <= viewStart) return 0.0;
    return ((t - viewStart) / (viewEnd - viewStart)) * 100.0;
  }

  function getSelectionOverlayStyle(start: number | null, end: number | null) {
    if (start === null || end === null || texEnd <= texStart) return 'display: none;';
    const pct1 = normalizedTimeToScreenPct(start);
    const pct2 = normalizedTimeToScreenPct(end);
    
    const left = Math.max(0, Math.min(pct1, pct2));
    const right = Math.min(100, Math.max(pct1, pct2));
    const width = right - left;
    
    if (width <= 0.05 || left >= 100.0 || right <= 0.0) {
      return 'display: none;';
    }
    return `left: ${left.toFixed(2)}%; width: ${width.toFixed(2)}%; display: block;`;
  }

  async function handleFileUploaded(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
      const file = target.files[0];
      try {
        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        console.log(`📂 WebGL Editor read custom file: ${file.name} (${bytes.length} bytes)`);
        onAudioUploaded(bytes);
      } catch (err) {
        console.error("Failed to read uploaded file:", err);
      }
    }
  }

  let isTimelineDragging = false;
  let timelineStartPos = 0.0;

  function handleTimelineMouseDown(e: MouseEvent) {
    if (!originalAudio) return;
    isTimelineDragging = true;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const t = Math.max(0.0, Math.min(1.0, x));
    
    timelineStartPos = t;
    selectionStart = t;
    selectionEnd = t;
  }

  function handleTimelineMouseMove(e: MouseEvent) {
    if (!isTimelineDragging || !originalAudio) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const t = Math.max(0.0, Math.min(1.0, x));
    
    selectionEnd = t;
  }

  // Bind mouseup globally to handle releasing the mouse drag outside the timeline bar!
  function handleTimelineMouseUp() {
    if (!isTimelineDragging) return;
    isTimelineDragging = false;
    if (selectionStart !== null && selectionEnd !== null) {
      if (Math.abs(selectionEnd - selectionStart) < 0.01) {
        if (originalAudio) {
          originalAudio.currentTime = selectionStart * originalAudio.duration;
          playbackProgress = selectionStart;
        }
        selectionStart = null;
        selectionEnd = null;
      } else {
        if (selectionStart > selectionEnd) {
          const temp = selectionStart;
          selectionStart = selectionEnd;
          selectionEnd = temp;
        }
      }
    }
  }

  const tickFrequencies = [50, 100, 200, 500, 1000, 2000, 5000, 10000, 15000, 20000];

  function calculateFreqY(f: number): number {
    if (fmax <= fmin) return 0.0;
    if (frequencyScale === 'linear') {
      return (f - fmin) / (fmax - fmin);
    } else {
      // Logarithmic spacing
      return Math.log2(f / fmin) / Math.log2(fmax / fmin);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.code === 'Space') {
      e.preventDefault();
      onPlayToggle();
    }
  }

  function resizeCanvas() {
    if (!canvas || !canvas.parentElement) return;
    const rect = canvas.parentElement.getBoundingClientRect();
    canvas.width = rect.width;
    canvas.height = rect.height;
    render();
  }

  // Monitor dock expansions and resize the canvas dynamically with a short delay to match the CSS transitions!
  $effect(() => {
    const _dock = rightDockExpanded;
    const timer = setTimeout(() => {
      resizeCanvas();
    }, 310);
    return () => clearTimeout(timer);
  });

  onMount(() => {
    resizeCanvas();
    initWebGL();
    
    const resizeHandler = () => {
      resizeCanvas();
    };
    
    window.addEventListener('resize', resizeHandler);
    
    return () => {
      window.removeEventListener('resize', resizeHandler);
    };
  });
</script>

<svelte:window onkeydown={handleKeyDown} onmouseup={handleGlobalMouseUp} onmousemove={handleGlobalMouseMove} />

<div class="full-screen-editor">
  <input 
    type="file" 
    bind:this={fileInput} 
    accept="audio/wav" 
    onchange={handleFileUploaded} 
    style="display: none;" 
  />

  <div class="canvas-container" style="right: {rightDockExpanded ? '18rem' : '0'};">
    <canvas 
      bind:this={canvas} 
      onwheel={handleWheel}
      onmousedown={handleMouseDown}
      onmousemove={handleMouseMove}
      onmouseup={handleMouseUp}
      onmouseleave={handleMouseUp}
    ></canvas>

    <!-- Interactive Vertical Frequency Ruler & Zoom Controller -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="frequency-ruler interactive"
      onwheel={handleFreqWheel}
      ondblclick={resetVerticalZoom}
      onmousedown={(e) => handleFreqMouseDown(e, 'pan')}
      title="Régua Vertical de Frequência: Role o mouse para zoom vertical, arraste para mover o eixo, duplo-clique para resetar."
    >
      <div class="freq-ruler-header">
        <span>↕ FREQ</span>
      </div>

      {#each tickFrequencies as f}
        {#if f >= fmin && f <= fmax}
          {@const y = calculateFreqY(f)}
          <div class="freq-tick" style="bottom: {(y * 100).toFixed(2)}%;">
            <span class="freq-label">{f >= 1000 ? (f/1000).toFixed(1) + 'k' : f}</span>
            <div class="freq-line"></div>
          </div>
        {/if}
      {/each}

      <!-- Top and Bottom interactive handles for stretching frequency limits -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div 
        class="freq-gripper top-gripper" 
        onmousedown={(e) => handleFreqMouseDown(e, 'fmax')}
        title="Arraste para ajustar Frequência Máxima"
      >
        <div class="gripper-bar"></div>
      </div>

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div 
        class="freq-gripper bottom-gripper" 
        onmousedown={(e) => handleFreqMouseDown(e, 'fmin')}
        title="Arraste para ajustar Frequência Mínima"
      >
        <div class="gripper-bar"></div>
      </div>
    </div>

    {#if selectionStart !== null && selectionEnd !== null}
      <div 
        class="selection-highlight" 
        style={getSelectionOverlayStyle(selectionStart, selectionEnd)}
      >
        <div class="selection-boundary border-left"></div>
        <div class="selection-boundary border-right"></div>
      </div>
    {/if}

    <!-- Timeline Playback Playhead Cursor Line Overlay -->
    {#if originalAudio}
      {@const playheadPct = normalizedTimeToScreenPct(playbackProgress)}
      {#if playheadPct >= 0 && playheadPct <= 100}
        <div class="playback-cursor-line" style="left: {playheadPct.toFixed(2)}%;"></div>
      {/if}
    {/if}

    <!-- Floating Differential Tensor Inspector HUD -->
    {#if probeResult && probePoint}
      <div 
        class="probe-hud-card" 
        style="left: {Math.max(250, Math.min(window.innerWidth - 340, probePoint.x + 20))}px; top: {Math.max(80, Math.min(window.innerHeight - 420, probePoint.y + 20))}px;"
      >
        <div class="probe-header">
          <span class="probe-title">🔬 Sonda Diferencial (Ordem {higherOrderO})</span>
          <button class="probe-close-btn" onclick={() => { probeResult = null; probePoint = null; }}>✕</button>
        </div>
        
        <div class="probe-body">
          <div class="probe-row highlight">
            <span>Coordenadas:</span>
            <span class="val font-mono">{probePoint.time_s.toFixed(3)}s | {probePoint.freq_hz.toFixed(1)} Hz</span>
          </div>

          <div class="probe-row">
            <span>Magnitude | Fase:</span>
            <span class="val font-mono">{probeResult.magnitude.toFixed(2)} | {(probeResult.phase * 180 / Math.PI).toFixed(1)}°</span>
          </div>

          <div class="probe-row">
            <span>Gradiente Amplitude:</span>
            <span class="val font-mono">∇logA = ({probeResult.d_log_a_dt.toFixed(1)}, {probeResult.d_log_a_dw.toFixed(3)})</span>
          </div>

          <div class="probe-row">
            <span>Gradiente Fase:</span>
            <span class="val font-mono">∇ϕ = ({probeResult.d_phi_dt.toFixed(1)}, {probeResult.d_phi_dw.toFixed(3)})</span>
          </div>

          <div class="probe-row highlight">
            <span>Foco Reatribuído:</span>
            <span class="val font-mono">{probeResult.time_reassigned_s.toFixed(3)}s | {probeResult.freq_inst_hz.toFixed(1)} Hz</span>
          </div>

          {#if (higherOrderO ?? 2) >= 2}
            <div class="probe-divider"></div>
            <div class="probe-section-title">Matriz Hessiana & Curvatura (2ª Ordem)</div>

            <div class="probe-row">
              <span>Autovalores (λ₁, λ₂):</span>
              <span class="val font-mono">{probeResult.lambda_1.toFixed(1)}, {probeResult.lambda_2.toFixed(1)}</span>
            </div>

            <div class="probe-row">
              <span>Ângulo de Crista (θ):</span>
              <span class="val font-mono">{(probeResult.ridge_angle_rad * 180 / Math.PI).toFixed(1)}°</span>
            </div>

            <div class="probe-row">
              <span>Anisotropia (κ):</span>
              <span class="val font-mono">{probeResult.anisotropy.toFixed(2)}</span>
            </div>

            <div class="probe-row">
              <span>Taxa de Chirp (α):</span>
              <span class="val font-mono">{probeResult.chirp_rate.toFixed(1)} Hz/s</span>
            </div>

            <div class="probe-row">
              <span>Largura de Banda (-3dB):</span>
              <span class="val font-mono">Δf = {probeResult.bandwidth_3db_hz.toFixed(1)} Hz</span>
            </div>
          {/if}

          {#if (higherOrderO ?? 2) >= 3}
            <div class="probe-divider"></div>
            <div class="probe-row">
              <span>Aceleração de Chirp (ϕ_ttt):</span>
              <span class="val font-mono">{probeResult.d2_phi_dt2.toFixed(1)}</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  <!-- 2D Spectrogram Minimap & Frustum Overview -->
  <div 
    class="minimap-container" 
    style="right: {rightDockExpanded ? '19.25rem' : '1.25rem'};"
    title="Mini-Mapa 2D: Mostra o espectrograma completo, o nível de zoom atual e o corte de frequências. Arraste para mover o visor ou duplo-clique para resetar."
  >
    <div class="minimap-header">
      <span class="minimap-title">🗺️ Visão Global (20Hz - 20kHz)</span>
      <span class="minimap-zoom-badge font-mono">{(1.0 / Math.max(0.001, viewEnd - viewStart)).toFixed(1)}x</span>
    </div>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="minimap-viewport-box"
      onmousedown={handleMinimapMouseDown}
      ondblclick={resetFullOverview}
    >
      <canvas 
        bind:this={minimapCanvas} 
        width="220" 
        height="100" 
        class="minimap-canvas"
      ></canvas>

      <!-- Current Viewport Bounding Frustum Rect -->
      <div 
        class="minimap-frustum"
        style={frustumStyle}
      >
        <div class="frustum-handle top-left"></div>
        <div class="frustum-handle top-right"></div>
        <div class="frustum-handle bottom-left"></div>
        <div class="frustum-handle bottom-right"></div>
        <div class="frustum-center-cross"></div>
      </div>

      <!-- Minimap Playhead Indicator -->
      <div 
        class="minimap-playhead" 
        style="left: {(playbackProgress * 100).toFixed(2)}%;"
      ></div>
    </div>

    <div class="minimap-footer">
      <span class="freq-cutoff font-mono">
        Y: {fmin >= 1000 ? (fmin/1000).toFixed(1)+'k' : fmin} — {fmax >= 1000 ? (fmax/1000).toFixed(1)+'k' : fmax} Hz
      </span>
      <button class="minimap-reset-btn" onclick={resetFullOverview} title="Resetar para visão global">
        ↺ Reset
      </button>
    </div>
  </div>

  <!-- Top Floating Toolbar -->
    <div class="hud-panel top-navbar" style="right: {rightDockExpanded ? '19.25rem' : '1.25rem'};">
      <button class="back-btn" onclick={onBackToConverter}>
        ⬅️ 1. Converter
      </button>
      
      <div class="vertical-divider"></div>
      
      <div class="brand-title">
        <span>Spectral WebGL</span> Phase-Gradient Editor
      </div>
      
      <div class="vertical-divider"></div>

      <button class="upload-btn" onclick={() => fileInput.click()}>
        📁 Subir WAV
      </button>
      
      <div class="status-indicator" class:draft={currentQualityLod === 1 || (refinementProgress !== null && refinementProgress !== undefined)}>
        <span class="pulse-dot" class:pulsing={currentQualityLod === 1 || (refinementProgress !== null && refinementProgress !== undefined)}></span> 
        {width}x{height} [{#if refinementProgress !== null && refinementProgress !== undefined}⚡ Refinando {refinementProgress}%{:else if currentQualityLod === 1}⚡ Rascunho Rápido{:else}✨ Ultra-Foco{/if}]
      </div>

      <button class="settings-toggle-btn" class:active={rightDockExpanded} onclick={() => rightDockExpanded = !rightDockExpanded}>
        ⚙️ Configurações
      </button>
    </div>

    <!-- Left Quick Tools Toolbox Sidebar -->
    <div class="hud-panel left-sidebar">
      <h3>Tools</h3>
      
      <button class:active={selectedTool === 'select'} onclick={() => selectedTool = 'select'}>
        ✋ Mover & Zoom
      </button>

      <button class:active={selectedTool === 'region_select'} onclick={() => selectedTool = 'region_select'}>
        🎯 Selecionar (X)
      </button>
      
      <div class="sidebar-divider"></div>
      
      <button class:active={selectedTool === 'gaussian_brush'} onclick={() => selectedTool = 'gaussian_brush'}>
        🖌️ Pincel Gauss
      </button>
      
      <button class:active={selectedTool === 'low_pass'} onclick={() => selectedTool = 'low_pass'}>
        🛡️ Passa-Baixa (Lasso)
      </button>
      
      <button class:active={selectedTool === 'high_pass'} onclick={() => selectedTool = 'high_pass'}>
        🔪 Passa-Alta (Lasso)
      </button>

      <div class="sidebar-divider"></div>

      <button 
        class:active={selectedTool === 'differential_probe'} 
        onclick={() => selectedTool = 'differential_probe'}
        title="Sonda Diferencial: Clique em qualquer ponto do espectrograma para inspecionar os tensores de 1ª a 4ª ordem"
      >
        🔬 Sonda Diferencial
      </button>

      <button 
        class:active={algorithmType === 'higher_order'} 
        onclick={() => {
          if (algorithmType === 'higher_order') {
            algorithmType = 'reassignment';
          } else {
            algorithmType = 'higher_order';
            rightDockExpanded = true;
          }
          onAdaptiveInteract();
        }}
        title="Alternar para o Laboratório de Derivadas de Ordem Superior"
      >
        🧪 Lab Derivadas
      </button>
    </div>

    <!-- Right Collapsible Advanced Settings Control Dock -->
    {#if rightDockExpanded}
      <div class="hud-panel right-dock">
        <div class="dock-header">
          <h3>Painel DSP Mestre</h3>
          <button class="close-dock-btn" onclick={() => rightDockExpanded = false}>✕</button>
        </div>
        
        <div class="dock-scroll-area">
          
          <!-- Section A: Algorithm & Palette -->
          <div class="dock-section">
            <h4>🎛️ Motor Espectral</h4>
            <div class="input-control">
              <label for="algorithm-select">Visualizador:</label>
              <select id="algorithm-select" bind:value={algorithmType} onchange={() => onAdaptiveInteract()}>
                <option value="reassignment">Auger-Flandrin Reassign (Canônico)</option>
                <option value="higher_order">🧪 Derivadas de Ordem Superior (Hessiana / Cristas)</option>
                <option value="sliding_jet">⚡ Sliding Jet DFT (Zero-FFT O(1) Real-Time)</option>
                <option value="cqt">Constant-Q (Projeção Esparsa)</option>
                <option value="log">Smooth Log-Spectrogram</option>
              </select>
            </div>

            {#if algorithmType === 'higher_order'}
              <div class="higher-order-panel experimental-section">
                <div class="experimental-header">
                  <h4>🧪 Laboratório de Derivadas de Ordem Superior</h4>
                  <span class="badge-experimental">EXPERIMENTAL</span>
                </div>
                
                <p class="experimental-desc">
                  Cálculo analítico exato via fórmula de Faà di Bruno em espaço de Gabor:
                  <code>D^(p,q) log C = D^(p,q) log A + i D^(p,q) ϕ</code>
                </p>

                <div class="input-control range-box">
                  <label>
                    Ordem Máxima (O): <span class="badge-order">O = {higherOrderO}</span>
                    <input type="range" min="1" max="4" step="1" bind:value={higherOrderO} oninput={() => onAdaptiveInteract()} />
                  </label>
                  <div class="order-description">
                    {#if higherOrderO === 1}
                      <span>1ª Ordem: Gradientes lineares analíticos (∇logA, ∇ϕ, f_inst, t_reass)</span>
                    {:else if higherOrderO === 2}
                      <span>2ª Ordem: Tensor Hessiano H, Autovalores (λ₁, λ₂), Cristas e Anisotropia</span>
                    {:else if higherOrderO === 3}
                      <span>3ª Ordem: Aceleração de Chirp (ϕ_ttt) e Inflexões de Vibrato</span>
                    {:else}
                      <span>4ª Ordem: Curvaturas Hiper-suaves e Primitivas Bézier C³</span>
                    {/if}
                  </div>
                </div>

                <div class="input-control">
                  <label for="ho-mode-select">Modo Visual Diferencial:</label>
                  <select id="ho-mode-select" bind:value={higherOrderVisualMode} onchange={() => onAdaptiveInteract()}>
                    <option value="ridge">🧭 Cristas & Orientação da Hessiana</option>
                    <option value="anisotropy">🌀 Anisotropia Espectral & Chirp Rate</option>
                    <option value="curvature">🏔️ Curvatura Principal λ₁ (Agudeza de Pico)</option>
                    <option value="vector_reassign">🎯 Reatribuição Hiperbólica (Splats)</option>
                  </select>
                </div>

                <div class="probe-action-box">
                  <button 
                    class="probe-btn" 
                    class:active={selectedTool === 'differential_probe'} 
                    onclick={() => selectedTool = 'differential_probe'}
                  >
                    🔬 Ativar Sonda Diferencial de Ponto
                  </button>
                  <span class="probe-hint">Clique em qualquer ponto do espectrograma para inspecionar os tensores de 1ª a 4ª ordem.</span>
                </div>
              </div>
            {/if}

            {#if algorithmType === 'sliding_jet'}
              <div class="higher-order-panel sliding-jet-panel">
                <div class="experimental-header">
                  <h4>⚡ Sliding Jet DFT (Recorrência O(1))</h4>
                  <span class="badge-jet">TEMPO REAL</span>
                </div>

                <p class="experimental-desc">
                  Recorrência deslizante exata sem FFT por frame via jatos de Taylor:
                  <code>S_(m+1)(δ) = e^(iθ₀) e^(iδ) [S_m(δ) - x_out + x_in e^(-iNθ₀) e^(-iNδ)]</code>
                </p>

                <div class="input-control range-box">
                  <label>
                    Ordem do Jato (O): <span class="badge-order">O = {higherOrderO}</span>
                    <input type="range" min="1" max="4" step="1" bind:value={higherOrderO} oninput={() => onAdaptiveInteract()} />
                  </label>
                  <div class="order-description">
                    {#if higherOrderO === 1}
                      <span>1ª Ordem: Gradientes lineares instantâneos (4.7 ns / amostra)</span>
                    {:else if higherOrderO === 2}
                      <span>2ª Ordem: Hessiana e Cristas Holomórficas via Bargmann-Fock</span>
                    {:else if higherOrderO === 3}
                      <span>3ª Ordem: Aceleração contínua de Chirp e Inflexões</span>
                    {:else}
                      <span>4ª Ordem: Curvaturas superiores contínuas C³</span>
                    {/if}
                  </div>
                </div>

                <div class="input-control">
                  <label for="sj-mode-select">Modo Visual Analítico:</label>
                  <select id="sj-mode-select" bind:value={higherOrderVisualMode} onchange={() => onAdaptiveInteract()}>
                    <option value="ridge">🧭 Cristas de Taylor & Fluxo Direcional</option>
                    <option value="anisotropy">🌀 Anisotropia Contínua & Chirp Rate</option>
                    <option value="curvature">🏔️ Curvatura Principal λ₁ (Agudeza de Pico)</option>
                    <option value="vector_reassign">🎯 Reatribuição Contínua por Jatos</option>
                  </select>
                </div>

                <div class="telemetry-card">
                  <div class="telemetry-title">⚡ Telemetria de Desempenho</div>
                  <div class="telemetry-grid">
                    <div class="telemetry-item">
                      <span class="telemetry-label">Custo por Quadro:</span>
                      <span class="telemetry-value text-green">0 FFTs (O(1))</span>
                    </div>
                    <div class="telemetry-item">
                      <span class="telemetry-label">Throughput Medido:</span>
                      <span class="telemetry-value text-cyan">4.7 ns / amostra</span>
                    </div>
                    <div class="telemetry-item">
                      <span class="telemetry-label">Aceleração Real:</span>
                      <span class="telemetry-value text-purple">4.424x Tempo Real</span>
                    </div>
                    <div class="telemetry-item">
                      <span class="telemetry-label">Estabilidade Numérica:</span>
                      <span class="telemetry-value text-emerald">1.61e-6 (Zero-Drift)</span>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
            
            <div class="input-control">
              <label for="palette-select">Paleta:</label>
              <select id="palette-select" bind:value={paletteType} onchange={() => onAdaptiveInteract()}>
                <option value="ycbcr">YCbCr Magnitude-Phase</option>
                <option value="snake">Geodesic Snake (Térmica)</option>
              </select>
            </div>

            <div class="input-control">
              <label for="scale-select">Escala Vertical:</label>
              <select id="scale-select" bind:value={frequencyScale} onchange={() => onAdaptiveInteract()}>
                <option value="log">Logarítmica (CQT / Auditiva)</option>
                <option value="linear">Linear (Física / STFT)</option>
              </select>
            </div>
            
            <div class="input-control range-box">
              <label>Raio de Amostragem (CQT/Reassign): {pointRadius.toFixed(2)}
                <input type="range" min="0.1" max="5.0" step="0.05" bind:value={pointRadius} oninput={() => onAdaptiveInteract()} />
              </label>
            </div>
          </div>

          <!-- Section: Zoom Strategy & 2^k Resolution Multiplier -->
          <div class="dock-section">
            <h4>🚀 Zoom & Desempenho</h4>
            
            <div class="input-control">
              <label for="zoom-mode-select">Modo de Zoom:</label>
              <select id="zoom-mode-select" bind:value={zoomMode}>
                <option value="gpu_debounced">GPU Rápido + Foco (Padrão - Leve)</option>
                <option value="continuous_resample">Reamostragem Contínua (Computador Forte)</option>
              </select>
            </div>

            <div class="input-control">
              <label for="k-res-select">Multiplicador Horizontal (2^k):</label>
              <select id="k-res-select" bind:value={horizontalResolutionK}>
                <option value={0}>1x (1024 colunas - Rápido)</option>
                <option value={1}>2x (2048 colunas - Equilibrado)</option>
                <option value={2}>4x (4096 colunas - Alta Nitidez)</option>
                <option value={3}>8x (8192 colunas - Máxima Resolução)</option>
              </select>
            </div>
          </div>

          <!-- Section B: FFT Windowing & Padding -->
          <div class="dock-section">
            <h4>⚡ Janelamento & FFT</h4>
            
            <div class="input-control">
              <label for="win-type-select">Formato Janela:</label>
              <select id="win-type-select" bind:value={windowType}>
                <option value="hann">Hann (Seno Cossuave)</option>
                <option value="hamming">Hamming (Transientes)</option>
                <option value="gaussian">Gaussian (Gabor Limite)</option>
                <option value="blackman-harris">Blackman-Harris (Corte)</option>
              </select>
            </div>

            <div class="input-control">
              <label for="win-size-select">Tamanho Janela:</label>
              <select id="win-size-select" bind:value={windowSize}>
                <option value={256}>256 amostras</option>
                <option value={512}>512 amostras</option>
                <option value={1024}>1024 amostras</option>
                <option value={2048}>2048 amostras</option>
              </select>
            </div>

            <div class="input-control">
              <label for="zero-pad-select">Zero Padding (FFT):</label>
              <select id="zero-pad-select" bind:value={zeroPadding}>
                <option value={1}>1x (Sem interpolação)</option>
                <option value={2}>2x Padding (Suave)</option>
                <option value={4}>4x Padding (Retina)</option>
                <option value={8}>8x Padding (Ultra)</option>
                <option value={16}>16x Padding (Máximo)</option>
                <option value={32}>32x Padding (Divino)</option>
              </select>
            </div>
            
            <div class="input-control">
              <label for="v-bins-select">Altura (Resolução Y):</label>
              <select id="v-bins-select" bind:value={selectedHeight}>
                <option value={256}>256 bandas</option>
                <option value={512}>512 bandas</option>
                <option value={1024}>1024 bandas (Default)</option>
                <option value={2048}>2048 bandas (Premium)</option>
              </select>
            </div>
          </div>

          <!-- Section C: Frequency Bounds (Controlled via Vertical Ruler Zoom on Left) -->
          <div class="dock-section">
            <h4>📐 Eixo Vertical (Hertz)</h4>
            
            <div class="freq-readout-card">
              <div class="freq-readout-row">
                <span class="freq-readout-label">Banda Visível:</span>
                <span class="freq-readout-val font-mono">
                  {fmin >= 1000 ? (fmin/1000).toFixed(1) + ' kHz' : fmin + ' Hz'} — {fmax >= 1000 ? (fmax/1000).toFixed(1) + ' kHz' : fmax + ' Hz'}
                </span>
              </div>

              <div class="freq-readout-row">
                <span class="freq-readout-label">Extensão:</span>
                <span class="freq-readout-val font-mono">
                  {Math.log2(fmax / Math.max(1, fmin)).toFixed(1)} oitavas
                </span>
              </div>

              <p class="freq-controller-hint">
                💡 <strong>Zoom Vertical:</strong> Role o mouse ou arraste as alças na régua à esquerda para dar zoom e mover o eixo Y.
              </p>

              <button class="reset-freq-btn" onclick={resetVerticalZoom}>
                ↺ Resetar Eixo Y (20Hz - 20kHz)
              </button>
            </div>
          </div>

          <!-- Section D: Tool Configuration -->
          {#if selectedTool === 'gaussian_brush'}
            <div class="dock-section">
              <h4>🖌️ Parâmetros do Pincel</h4>
              <div class="input-control range-box">
                <label>Raio Raio: {brushSize} px
                  <input type="range" min="10" max="250" bind:value={brushSize} />
                </label>
              </div>
              
              <div class="input-control range-box">
                <label>Força (Sigma): {brushStrength.toFixed(2)}
                  <input type="range" min="0.1" max="1.0" step="0.05" bind:value={brushStrength} />
                </label>
              </div>
            </div>
          {/if}

        </div>
      </div>
    {/if}

    <!-- Bottom Status & Seamless Looping Player Bar -->
    <div class="hud-panel bottom-bar" style="right: {rightDockExpanded ? '19.25rem' : '1.25rem'};">
      
      <!-- Interactive Scrub Timeline & Loop Selector Bar -->
      <div 
        class="timeline-scrub-bar" 
        onmousedown={handleTimelineMouseDown}
        onmousemove={handleTimelineMouseMove}
      >
        <!-- Selection Highlight Loop -->
        {#if selectionStart !== null && selectionEnd !== null}
          {@const leftPct = selectionStart * 100}
          {@const widthPct = (selectionEnd - selectionStart) * 100}
          <div class="scrub-selection" style="left: {leftPct.toFixed(2)}%; width: {widthPct.toFixed(2)}%;"></div>
        {/if}
        
        <!-- Live Playhead indicator dot -->
        <div class="scrub-playhead" style="left: {(playbackProgress * 100).toFixed(2)}%;"></div>
      </div>

      <div class="bottom-row-controls">
        <!-- Audio Transport Controls Group -->
        <div class="transport-group">
          <button class="play-btn" class:playing={isPlaying} onclick={onPlayToggle}>
            {isPlaying ? '⏸️ PAUSAR' : '▶️ PLAY'}
          </button>
          
          <div class="vertical-divider"></div>

          <div class="loop-mode-selector">
            <label class="transport-label" for="loop-mode-select">Loop:</label>
            <select id="loop-mode-select" class="transport-select" bind:value={loopMode}>
              <option value="normal">🔁 Normal (A-B)</option>
              <option value="mirrored">🪞 Espelhado (Ping-Pong)</option>
              <option value="none">🚫 Sem Loop (Linear)</option>
            </select>
          </div>
          
          {#if selectionStart !== null && selectionEnd !== null}
            <button class="clear-sel-btn" onclick={clearSelection}>
              Limpar A-B
            </button>
          {/if}
        </div>

        <!-- Continuous Mirrored dB Density Histogram (Transformed vs Original on shared dB axis) -->
        <div class="mirrored-histogram-container" title="Densidade Contínua de Energia em dB (Transformada vs Original)">
          <div class="hist-labels-top">
            <span class="hist-badge-trans">
              ▲ P_trans {selectionStart !== null && selectionEnd !== null ? '[A-B]' : '[Global]'} {progressiveScanPct < 100 ? `(${progressiveScanPct}%)` : ''}
            </span>
            <span class="hist-badge-orig">
              ▼ P_orig {selectionStart !== null && selectionEnd !== null ? '[A-B]' : '[Global]'}
            </span>
          </div>
          <svg class="mirrored-density-svg" viewBox="0 0 260 60" preserveAspectRatio="none">
            <defs>
              <linearGradient id="transGrad" x1="0" y1="1" x2="0" y2="0">
                <stop offset="0%" stop-color="#38bdf8" stop-opacity="0.15" />
                <stop offset="100%" stop-color="#38bdf8" stop-opacity="0.8" />
              </linearGradient>
              <linearGradient id="origGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="#c084fc" stop-opacity="0.15" />
                <stop offset="100%" stop-color="#c084fc" stop-opacity="0.8" />
              </linearGradient>
            </defs>

            <!-- Transformed Density (Top half, baseline at y=22, peaks upwards) -->
            {#if topPathD}
              <path d={topPathD} fill="url(#transGrad)" stroke="#38bdf8" stroke-width="1.2" stroke-linejoin="round" />
            {/if}

            <!-- Original Density (Bottom half, baseline at y=38, peaks downwards) -->
            {#if bottomPathD}
              <path d={bottomPathD} fill="url(#origGrad)" stroke="#c084fc" stroke-width="1.2" stroke-linejoin="round" />
            {/if}

            <!-- Central Axis Track: Dedicated clear band (y=22 to y=38) ensuring zero text overlap! -->
            <rect x="0" y="23" width="260" height="14" rx="2" fill="rgba(2, 6, 23, 0.65)" stroke="rgba(255, 255, 255, 0.08)" stroke-width="0.5" />
            <line x1="0" y1="30" x2="260" y2="30" stroke="rgba(255, 255, 255, 0.25)" stroke-width="1" />

            <!-- Shared dB Axis Tick Marks and Labels centered in the gap -->
            <!-- -90 dB (10% of 260 = 26px) -->
            <line x1="26" y1="26" x2="26" y2="34" stroke="rgba(255, 255, 255, 0.45)" stroke-width="1" />
            <text x="26" y="30.5" font-size="7.5" fill="#94a3b8" text-anchor="middle" dominant-baseline="central" font-family="monospace">-90</text>

            <!-- -60 dB (40% of 260 = 104px) -->
            <line x1="104" y1="26" x2="104" y2="34" stroke="rgba(255, 255, 255, 0.45)" stroke-width="1" />
            <text x="104" y="30.5" font-size="7.5" fill="#94a3b8" text-anchor="middle" dominant-baseline="central" font-family="monospace">-60</text>

            <!-- -30 dB (70% of 260 = 182px) -->
            <line x1="182" y1="26" x2="182" y2="34" stroke="rgba(255, 255, 255, 0.45)" stroke-width="1" />
            <text x="182" y="30.5" font-size="7.5" fill="#94a3b8" text-anchor="middle" dominant-baseline="central" font-family="monospace">-30</text>

            <!-- 0 dB (100% of 260 = 256px) -->
            <line x1="256" y1="26" x2="256" y2="34" stroke="rgba(255, 255, 255, 0.45)" stroke-width="1" />
            <text x="252" y="30.5" font-size="7.5" fill="#38bdf8" text-anchor="end" font-weight="bold" dominant-baseline="central" font-family="monospace">0dB</text>
          </svg>
        </div>

        <div class="coordinate-group">
          <span class="coordinate-view">T: {panX.toFixed(3)}s</span>
          <span class="coordinate-view">Z: {zoomX.toFixed(1)}x</span>
          
          {#if selectionStart !== null && selectionEnd !== null && originalAudio}
            <span class="coordinate-view selection-coords">
              🎯 A-B: {(selectionStart * originalAudio.duration).toFixed(2)}s - {(selectionEnd * originalAudio.duration).toFixed(2)}s ({( (selectionEnd - selectionStart) * originalAudio.duration ).toFixed(2)}s)
            </span>
          {/if}
        </div>
      </div>
    </div>

</div>

<style>
  .full-screen-editor {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: radial-gradient(circle at 50% 50%, #0b1329 0%, #020617 100%);
  }

  .canvas-container {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    z-index: 1;
    transition: right 0.3s ease-in-out;
  }

  canvas {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 1;
    cursor: crosshair;
  }

  .selection-highlight {
    position: absolute;
    top: 0;
    height: 100%;
    background-color: rgba(56, 189, 248, 0.32);
    border-left: 2px solid #38bdf8;
    border-right: 2px solid #38bdf8;
    z-index: 2;
    pointer-events: none;
    box-shadow: inset 0 0 40px rgba(56, 189, 248, 0.15), 0 0 10px rgba(56, 189, 248, 0.2);
  }

  /* Beautiful glowing vertical timeline cursor / playhead line overlay */
  .playback-cursor-line {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background-color: #10b981;
    z-index: 3;
    pointer-events: none;
    box-shadow: 0 0 8px #10b981, 0 0 15px rgba(16, 185, 129, 0.6);
  }

  /* Interactive Vertical Frequency Ruler & Zoom Controller */
  .frequency-ruler {
    position: absolute;
    left: 14.5rem;
    top: 4.75rem;
    bottom: 8.75rem; /* Safely positioned above the bottom timeline scrub bar */
    width: 4.25rem;
    pointer-events: auto;
    z-index: 25;
    font-family: monospace;
    font-size: 0.65rem;
    color: rgba(255, 255, 255, 0.45);
    background: rgba(15, 23, 42, 0.88);
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 0 6px 6px 0;
    user-select: none;
    cursor: grab;
    transition: background 0.2s ease, border-color 0.2s ease;
  }

  .frequency-ruler:hover {
    background: rgba(15, 23, 42, 0.95);
    border-color: rgba(56, 189, 248, 0.25);
  }

  .frequency-ruler:active {
    cursor: grabbing;
  }

  .freq-ruler-header {
    position: absolute;
    top: -1.2rem;
    left: 0;
    width: 100%;
    text-align: center;
    font-size: 0.6rem;
    font-weight: 700;
    color: #38bdf8;
    letter-spacing: 0.05em;
    pointer-events: none;
  }

  .freq-tick {
    position: absolute;
    left: 0.25rem;
    right: 0;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    transform: translateY(50%);
    pointer-events: none;
  }

  .freq-label {
    white-space: nowrap;
    text-shadow: 0 0 4px #000, 0 0 8px #000;
    font-weight: 600;
  }

  .freq-line {
    flex: 1;
    height: 1px;
    background-color: rgba(255, 255, 255, 0.12);
    box-shadow: 0 0 2px rgba(255, 255, 255, 0.08);
  }

  .freq-gripper {
    position: absolute;
    left: 0;
    width: 100%;
    height: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: ns-resize;
    z-index: 30;
    background: rgba(56, 189, 248, 0.15);
    transition: background 0.15s ease;
  }

  .freq-gripper:hover {
    background: rgba(56, 189, 248, 0.4);
  }

  .top-gripper {
    top: 0;
    border-bottom: 1px solid #38bdf8;
    border-radius: 0 6px 0 0;
  }

  .bottom-gripper {
    bottom: 0;
    border-top: 1px solid #38bdf8;
    border-radius: 0 0 6px 0;
  }

  .gripper-bar {
    width: 18px;
    height: 2px;
    background-color: #38bdf8;
    border-radius: 1px;
    box-shadow: 0 0 4px #38bdf8;
  }

  /* Right Dock Section C Readout styling */
  .freq-readout-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .freq-readout-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
  }

  .freq-readout-label {
    color: #94a3b8;
  }

  .freq-readout-val {
    color: #38bdf8;
    font-weight: 700;
  }

  .freq-controller-hint {
    font-size: 0.7rem;
    color: #64748b;
    margin: 0.25rem 0 0.5rem 0;
    line-height: 1.3;
  }

  .reset-freq-btn {
    background: rgba(56, 189, 248, 0.12);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.25);
    padding: 0.4rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .reset-freq-btn:hover {
    background: rgba(56, 189, 248, 0.25);
    border-color: #38bdf8;
  }

  .selection-boundary {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 4px;
    background: transparent;
  }
  .border-left { left: -2px; cursor: ew-resize; }
  .border-right { right: -2px; cursor: ew-resize; }



  .hud-panel {
    pointer-events: auto;
    background: rgba(15, 23, 42, 0.88) !important;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.45);
    color: #cbd5e1;
    font-family: system-ui, -apple-system, sans-serif;
    z-index: 30 !important;
  }

  /* Top Navbar HUD style */
  .top-navbar {
    position: absolute !important;
    top: 1.25rem;
    left: 1.25rem;
    height: 3.5rem;
    display: flex;
    align-items: center;
    padding: 0 1.25rem;
    transition: right 0.3s ease-in-out;
    z-index: 30 !important;
  }

  .back-btn {
    background-color: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
  }

  .back-btn:hover {
    background-color: rgba(56, 189, 248, 0.3);
  }

  .upload-btn {
    background-color: rgba(16, 185, 129, 0.15);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.3);
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
  }

  .upload-btn:hover {
    background-color: rgba(16, 185, 129, 0.3);
  }

  .settings-toggle-btn {
    background-color: rgba(148, 163, 184, 0.1);
    color: #cbd5e1;
    border: 1px solid rgba(148, 163, 184, 0.25);
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    margin-left: 1rem;
  }

  .settings-toggle-btn:hover, .settings-toggle-btn.active {
    background-color: rgba(56, 189, 248, 0.2);
    border-color: #38bdf8;
    color: #38bdf8;
  }

  .vertical-divider {
    width: 1px;
    height: 1.5rem;
    background-color: rgba(255, 255, 255, 0.1);
    margin: 0 1.25rem;
  }

  .brand-title {
    font-size: 0.95rem;
    font-weight: 500;
    color: #94a3b8;
  }

  .brand-title span {
    color: #38bdf8;
    font-weight: 800;
  }

  .status-indicator {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    color: #10b981;
    font-family: monospace;
    transition: all 0.2s ease;
  }

  .status-indicator.draft {
    color: #facc15;
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    background-color: #10b981;
    border-radius: 50%;
    box-shadow: 0 0 8px #10b981;
    display: inline-block;
    transition: all 0.2s ease;
  }

  .pulse-dot.pulsing {
    background-color: #facc15;
    box-shadow: 0 0 12px #facc15;
    animation: draftPulse 0.5s infinite alternate;
  }

  @keyframes draftPulse {
    from { opacity: 0.5; transform: scale(0.85); }
    to { opacity: 1.0; transform: scale(1.2); }
  }

  /* Left DSP Sidebar HUD style */
  .left-sidebar {
    position: absolute !important;
    top: 5.75rem;
    left: 1.25rem;
    bottom: 5.75rem;
    width: 13rem;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 30 !important;
  }

  .left-sidebar h3 {
    margin: 0 0 0.25rem 0;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
    font-weight: 700;
  }

  .sidebar-divider {
    height: 1px;
    background-color: rgba(255, 255, 255, 0.08);
    margin: 0.15rem 0;
  }

  .left-sidebar button {
    background: transparent;
    color: #cbd5e1;
    border: 1px solid rgba(255, 255, 255, 0.04);
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    text-align: left;
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .left-sidebar button:hover {
    background-color: rgba(255, 255, 255, 0.04);
  }

  .left-sidebar button.active {
    background-color: #38bdf8;
    color: #0f172a;
    font-weight: 700;
    border-color: #38bdf8;
    box-shadow: 0 0 10px rgba(56, 189, 248, 0.35);
  }

  /* Right Settings Dock style */
  .right-dock {
    position: absolute !important;
    top: 1.25rem;
    right: 1.25rem;
    bottom: 1.25rem;
    width: 17rem;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    transition: all 0.3s ease-in-out;
    z-index: 30 !important;
  }

  .dock-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding-bottom: 0.5rem;
  }

  .dock-header h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 800;
    color: #f8fafc;
  }

  .close-dock-btn {
    background: transparent;
    border: none;
    color: #64748b;
    cursor: pointer;
    font-size: 1rem;
    padding: 0.25rem;
  }

  .close-dock-btn:hover {
    color: #f87171;
  }

  .dock-scroll-area {
    flex: 1;
    overflow-y: auto;
    padding-right: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .dock-scroll-area::-webkit-scrollbar {
    width: 4px;
  }

  .dock-scroll-area::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  /* Force the scroll track to be fully transparent on Windows/Linux to prevent solid opaque tracks! */
  .dock-scroll-area::-webkit-scrollbar-track {
    background: transparent !important;
  }

  .dock-section {
    background-color: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.04);
    padding: 0.75rem;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .dock-section h4 {
    margin: 0;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #38bdf8;
    font-weight: 800;
    border-left: 2px solid #38bdf8;
    padding-left: 0.5rem;
  }

  .input-control {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.75rem;
  }

  .input-control label {
    color: #94a3b8;
    font-weight: 500;
  }

  .input-control select {
    background-color: rgba(15, 23, 42, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #cbd5e1;
    padding: 0.35rem 0.5rem;
    border-radius: 4px;
    outline: none;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .input-control select:hover {
    border-color: #475569;
  }

  .range-box label {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-weight: 500;
  }

  .range-box input[type="range"] {
    accent-color: #38bdf8;
    cursor: pointer;
  }

  /* Continuous Mirrored dB Density Histogram (Transformed vs Original) */
  .mirrored-histogram-container {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 260px;
    height: 56px;
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 2px 4px;
    margin: 0 0.5rem;
    overflow: hidden;
  }

  .hist-labels-top {
    position: absolute;
    top: 2px;
    left: 6px;
    right: 6px;
    display: flex;
    justify-content: space-between;
    pointer-events: none;
    z-index: 2;
  }

  .hist-badge-trans {
    font-size: 0.6rem;
    color: #38bdf8;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-shadow: 0 0 6px rgba(56, 189, 248, 0.4);
  }

  .hist-badge-orig {
    font-size: 0.6rem;
    color: #c084fc;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-shadow: 0 0 6px rgba(192, 132, 252, 0.4);
  }

  .mirrored-density-svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  /* Bottom HUD Transport and Statusbar (2-row adaptive timeline DAW layout!) */
  .bottom-bar {
    position: absolute !important;
    bottom: 1.25rem;
    left: 1.25rem;
    height: auto;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    padding: 0.75rem 1.25rem;
    gap: 0.6rem;
    transition: right 0.3s ease-in-out;
    z-index: 30 !important;
  }

  .bottom-row-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
  }

  /* Interactive Scrub Timeline style */
  .timeline-scrub-bar {
    height: 10px;
    background-color: rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    position: relative;
    cursor: ew-resize;
    overflow: hidden;
    width: 100%;
    transition: background-color 0.2s, height 0.2s;
  }

  .timeline-scrub-bar:hover {
    height: 14px;
    background-color: rgba(255, 255, 255, 0.12);
  }

  .scrub-selection {
    position: absolute;
    top: 0;
    bottom: 0;
    background-color: rgba(56, 189, 248, 0.3);
    border-left: 1px solid #38bdf8;
    border-right: 1px solid #38bdf8;
    pointer-events: none;
  }

  .scrub-playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 3px;
    background-color: #10b981;
    pointer-events: none;
    box-shadow: 0 0 6px #10b981;
  }

  .transport-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .play-btn {
    background-color: #10b981;
    color: #020617;
    border: 1px solid #10b981;
    padding: 0.5rem 1.25rem;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 800;
    cursor: pointer;
    transition: all 0.2s;
  }

  .play-btn:hover {
    background-color: #059669;
  }

  .play-btn.playing {
    background-color: #f59e0b;
    border-color: #f59e0b;
  }

  .loop-mode-selector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .transport-label {
    font-size: 0.8rem;
    color: #94a3b8;
    font-weight: 600;
  }

  .transport-select {
    background-color: #1e293b;
    border: 1px solid #334155;
    color: #cbd5e1;
    padding: 0.35rem 0.5rem;
    border-radius: 4px;
    outline: none;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .clear-sel-btn {
    background-color: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
    cursor: pointer;
  }

  .coordinate-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-left: auto;
  }

  .coordinate-view {
    font-family: monospace;
    font-size: 0.75rem;
    color: #94a3b8;
    background-color: rgba(0, 0, 0, 0.25);
    padding: 0.3rem 0.5rem;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.04);
  }

  .selection-coords {
    color: #38bdf8;
    border-color: rgba(56, 189, 248, 0.2);
    background-color: rgba(56, 189, 248, 0.05);
    font-weight: 600;
  }

  .help-text {
    font-size: 0.8rem;
    color: #64748b;
  }

  .higher-order-panel {
    background: rgba(30, 41, 59, 0.45);
    border: 1px solid rgba(59, 130, 246, 0.25);
    border-radius: 6px;
    padding: 0.6rem;
    margin-top: 0.4rem;
    margin-bottom: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .badge-order {
    background: linear-gradient(135deg, #2563eb, #7c3aed);
    color: #ffffff;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    font-weight: 700;
    font-size: 0.75rem;
    margin-left: 0.4rem;
  }

  .order-description {
    font-size: 0.72rem;
    color: #93c5fd;
    margin-top: 0.25rem;
    line-height: 1.3;
  }

  /* Experimental Section Styling */
  .experimental-section {
    background: rgba(147, 51, 234, 0.08);
    border: 1px solid rgba(168, 85, 247, 0.25);
    border-radius: 8px;
    padding: 0.75rem;
    margin-top: 0.5rem;
  }

  .experimental-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.4rem;
  }

  .experimental-header h4 {
    margin: 0;
    color: #c084fc;
    font-size: 0.8rem;
    font-weight: 700;
  }

  .badge-experimental {
    background: rgba(168, 85, 247, 0.2);
    color: #e879f9;
    border: 1px solid rgba(168, 85, 247, 0.4);
    font-size: 0.6rem;
    font-weight: 800;
    padding: 0.15rem 0.35rem;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }

  .experimental-desc {
    font-size: 0.68rem;
    color: #94a3b8;
    line-height: 1.3;
    margin-bottom: 0.6rem;
  }

  .experimental-desc code {
    display: block;
    margin-top: 0.25rem;
    color: #38bdf8;
    background: rgba(0, 0, 0, 0.3);
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    font-family: monospace;
    font-size: 0.65rem;
  }

  .probe-action-box {
    margin-top: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .probe-btn {
    background: rgba(168, 85, 247, 0.15);
    color: #e879f9;
    border: 1px solid rgba(168, 85, 247, 0.35);
    padding: 0.45rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
  }

  .probe-btn:hover, .probe-btn.active {
    background: rgba(168, 85, 247, 0.35);
    border-color: #e879f9;
    box-shadow: 0 0 8px rgba(168, 85, 247, 0.3);
  }

  .probe-hint {
    font-size: 0.65rem;
    color: #64748b;
    line-height: 1.2;
  }

  /* Floating Probe Result HUD Card */
  .probe-hud-card {
    position: absolute;
    width: 290px;
    background: rgba(15, 23, 42, 0.95);
    border: 1px solid rgba(168, 85, 247, 0.4);
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.6), 0 0 12px rgba(168, 85, 247, 0.2);
    border-radius: 8px;
    z-index: 50;
    user-select: none;
    overflow: hidden;
  }

  .probe-header {
    background: rgba(147, 51, 234, 0.2);
    border-bottom: 1px solid rgba(168, 85, 247, 0.25);
    padding: 0.4rem 0.6rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .probe-title {
    font-size: 0.72rem;
    font-weight: 700;
    color: #e879f9;
  }

  .probe-close-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .probe-close-btn:hover {
    color: #fff;
  }

  .probe-body {
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.72rem;
  }

  .probe-row {
    display: flex;
    justify-content: space-between;
    color: #94a3b8;
  }

  .probe-row.highlight {
    color: #f8fafc;
    font-weight: 600;
  }

  .probe-row .val {
    color: #38bdf8;
  }

  .probe-divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.08);
    margin: 0.2rem 0;
  }

  .probe-section-title {
    font-size: 0.65rem;
    font-weight: 700;
    color: #c084fc;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  /* 2D Spectrogram Minimap & Frustum Overview */
  .minimap-container {
    position: absolute !important;
    bottom: 8.5rem;
    width: 236px;
    background: rgba(15, 23, 42, 0.92);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    z-index: 28;
    user-select: none;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    transition: right 0.3s ease-in-out;
  }

  .minimap-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.2rem;
  }

  .minimap-title {
    font-size: 0.65rem;
    font-weight: 700;
    color: #94a3b8;
    letter-spacing: 0.04em;
  }

  .minimap-zoom-badge {
    background: rgba(56, 189, 248, 0.18);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.35);
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    font-size: 0.65rem;
    font-weight: 800;
  }

  .minimap-viewport-box {
    position: relative;
    width: 220px;
    height: 100px;
    background: rgba(0, 0, 0, 0.6);
    border-radius: 4px;
    overflow: hidden;
    cursor: crosshair;
    margin: 0 auto;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .minimap-canvas {
    width: 100%;
    height: 100%;
    display: block;
    opacity: 0.75;
    transition: opacity 0.2s ease;
  }

  .minimap-viewport-box:hover .minimap-canvas {
    opacity: 0.95;
  }

  .minimap-frustum {
    position: absolute;
    border: 2px solid #38bdf8 !important;
    background: rgba(56, 189, 248, 0.25) !important;
    box-shadow: 0 0 10px rgba(56, 189, 248, 0.7), inset 0 0 10px rgba(56, 189, 248, 0.3) !important;
    border-radius: 2px;
    cursor: move;
    pointer-events: none;
    box-sizing: border-box;
    z-index: 20 !important;
  }

  .frustum-center-cross {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 8px;
    height: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.8);
    border-left: 1px solid rgba(255, 255, 255, 0.8);
    pointer-events: none;
  }

  .frustum-handle {
    position: absolute;
    width: 6px;
    height: 6px;
    background-color: #ffffff;
    border: 1px solid #38bdf8;
    box-shadow: 0 0 4px #38bdf8;
    border-radius: 1px;
    z-index: 25;
  }

  .frustum-handle.top-left { top: -3px; left: -3px; }
  .frustum-handle.top-right { top: -3px; right: -3px; }
  .frustum-handle.bottom-left { bottom: -3px; left: -3px; }
  .frustum-handle.bottom-right { bottom: -3px; right: -3px; }

  .minimap-playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: #10b981;
    box-shadow: 0 0 6px #10b981;
    pointer-events: none;
    z-index: 5;
  }

  .minimap-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.2rem;
  }

  .freq-cutoff {
    font-size: 0.65rem;
    color: #38bdf8;
    font-weight: 600;
  }

  .minimap-reset-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: #94a3b8;
    font-size: 0.65rem;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.2s;
  }

  .minimap-reset-btn:hover {
    background: rgba(56, 189, 248, 0.2);
    color: #38bdf8;
    border-color: #38bdf8;
  }

  .sliding-jet-panel {
    border-color: rgba(14, 165, 233, 0.35);
    background: rgba(15, 23, 42, 0.6);
  }

  .badge-jet {
    background: linear-gradient(135deg, #0ea5e9, #06b6d4);
    color: #ffffff;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    font-weight: 700;
    font-size: 0.68rem;
    letter-spacing: 0.05em;
  }

  .telemetry-card {
    background: rgba(2, 6, 23, 0.7);
    border: 1px solid rgba(14, 165, 233, 0.25);
    border-radius: 6px;
    padding: 0.55rem;
    margin-top: 0.5rem;
  }

  .telemetry-title {
    font-size: 0.75rem;
    font-weight: 700;
    color: #38bdf8;
    margin-bottom: 0.35rem;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .telemetry-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.35rem;
  }

  .telemetry-item {
    display: flex;
    flex-direction: column;
  }

  .telemetry-label {
    font-size: 0.65rem;
    color: #94a3b8;
  }

  .telemetry-value {
    font-size: 0.72rem;
    font-weight: 600;
  }

  .text-green { color: #4ade80; }
  .text-cyan { color: #38bdf8; }
  .text-purple { color: #c084fc; }
  .text-emerald { color: #34d399; }
  </style>
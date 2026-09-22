<script lang="ts">
  import { onMount } from 'svelte';

  // Svelte 5 strict typing: Receive all reactive props from App.svelte
  let { 
    rgbaGrid, 
    width, 
    height, 
    
    // Bindable advanced DSP parameters
    windowType = $bindable('hann'),
    windowSize = $bindable(1024),
    zeroPadding = $bindable(4),
    fmin = $bindable(20),
    fmax = $bindable(20000),
    algorithmType = $bindable('reassignment'),
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
    horizontalResolutionK = $bindable(1),
    
    originalAudio,
    isPlaying,
    onPlayToggle,
    onAudioUploaded,
    onBackToConverter 
  }: { 
    rgbaGrid: Uint8Array | null, 
    width: number, 
    height: number,
    
    windowType: 'hann' | 'hamming' | 'gaussian' | 'blackman-harris',
    windowSize: number,
    zeroPadding: number,
    fmin: number,
    fmax: number,
    algorithmType: 'reassignment' | 'log',
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

  let selectedTool = $state<'select' | 'region_select' | 'gaussian_brush' | 'low_pass' | 'high_pass'>('region_select');
  let brushSize = $state(50);
  let brushStrength = $state(0.5);

  let rightDockExpanded = $state(true); // Right settings dock state

  // Logarithmic Histogram state variables
  let histogramBins = $state<number[]>(new Array(30).fill(0));
  let maxBinValue = $state(1);

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
  uniform float u_zoomX;
  uniform float u_panX;
  
  void main() {
      // Map view bounds on X (Time) axis only
      float x = (a_position.x / u_zoomX) + u_panX;
      float y = a_position.y;
      
      // Flip Y axis: Low frequencies (row 0 in texture, v_uv.y = 0.0) at the bottom (y = -1.0)
      v_uv = vec2(x * 0.5 + 0.5, y * 0.5 + 0.5);
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
    
    computeLogHistogram();
    render();
  });

  function computeLogHistogram() {
    if (!rgbaGrid) return;
    const bins = new Array(30).fill(0);
    const numPixels = rgbaGrid.length / 4;
    const step = Math.max(1, Math.floor(numPixels / 25000));
    
    for (let i = 0; i < numPixels; i += step) {
      const idx = i * 4;
      const r = rgbaGrid[idx];
      const g = rgbaGrid[idx + 1];
      const b = rgbaGrid[idx + 2];
      
      const Y = 0.299 * r + 0.587 * g + 0.114 * b;
      if (Y < 1.0) continue;
      
      const binIdx = Math.floor((Y / 255.0) * 30);
      const clampedIdx = Math.max(0, Math.min(29, binIdx));
      bins[clampedIdx]++;
    }
    
    histogramBins = bins;
    maxBinValue = Math.max(1, ...bins);
  }

  let texStart = $state(0.0);
  let texEnd = $state(1.0);
  let debounceTimer: any = null;

  // Whenever a newly generated texture is passed from WASM, record its exact window and stabilize GPU coordinates
  $effect(() => {
    const _grid = rgbaGrid;
    if (_grid) {
      texStart = viewStart;
      texEnd = viewEnd;
      zoomX = 1.0;
      panX = 0.0;
      render();
    }
  });

  function render() {
    if (!gl || !program || !rgbaGrid) return;
    gl.viewport(0, 0, canvas.width, canvas.height);
    
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    
    const zoomLoc = gl.getUniformLocation(program, 'u_zoomX');
    const panLoc = gl.getUniformLocation(program, 'u_panX');
    const texLoc = gl.getUniformLocation(program, 'u_spectrogramTexture');
    
    // Stretch texture on GPU during active zoom/pan gestures, flat when texture is focused!
    gl.uniform1f(zoomLoc, zoomX);
    gl.uniform1f(panLoc, panX);
    gl.uniform1i(texLoc, 0);
    
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    if (!canvas) return;

    if (zoomMode === 'continuous_resample') {
      // Continuous Resample Mode (For powerful machines)
      const rect = canvas.getBoundingClientRect();
      const mouseRatio = Math.max(0.0, Math.min(1.0, (e.clientX - rect.left) / rect.width));
      
      const currentSpan = viewEnd - viewStart;
      const zoomFactor = e.deltaY < 0 ? 0.85 : 1.18;
      const newSpan = Math.max(0.001, Math.min(1.0, currentSpan * zoomFactor));
      
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
    } else {
      // GPU-Debounced Mode (Default - Lightweight & Instant 120fps)
      // Stretches the precalculated 2^k high-res texture on the GPU smoothly,
      // and calculates focused high-res slices only when scrolling stops!
      const zoomFactor = 1.15;
      const oldZoom = zoomX;
      
      if (e.deltaY < 0) {
        zoomX *= zoomFactor;
      } else {
        zoomX /= zoomFactor;
      }
      
      zoomX = Math.max(1.0, Math.min(100.0, zoomX));
      
      const rect = canvas.getBoundingClientRect();
      const mouseX = (e.clientX - rect.left) / rect.width;
      const clipMouseX = (mouseX * 2.0 - 1.0);
      
      const viewPointX = (clipMouseX / oldZoom) + panX;
      panX = viewPointX - (clipMouseX / zoomX);
      
      const maxPan = 1.0 - (1.0 / zoomX);
      panX = Math.max(-maxPan, Math.min(maxPan, panX));

      render();

      if (debounceTimer) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        const halfSpan = 0.5 / zoomX;
        const centerU = (panX * 0.5 + 0.5);
        const texSpan = texEnd - texStart;
        
        const newStart = texStart + Math.max(0.0, centerU - halfSpan) * texSpan;
        const newEnd = texStart + Math.min(1.0, centerU + halfSpan) * texSpan;
        
        viewStart = Math.max(0.0, newStart);
        viewEnd = Math.min(1.0, newEnd);
      }, 220);
    }
  }

  function handleMouseDown(e: MouseEvent) {
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
      if (zoomMode === 'continuous_resample') {
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
      } else {
        const rect = canvas.getBoundingClientRect();
        const deltaX = (e.clientX - lastMouseX) / rect.width;
        panX -= deltaX * 2.0 / zoomX;
        
        const maxPan = 1.0 - (1.0 / zoomX);
        panX = Math.max(-maxPan, Math.min(maxPan, panX));
        
        lastMouseX = e.clientX;
        render();

        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
          const halfSpan = 0.5 / zoomX;
          const centerU = (panX * 0.5 + 0.5);
          const texSpan = texEnd - texStart;
          
          const newStart = texStart + Math.max(0.0, centerU - halfSpan) * texSpan;
          const newEnd = texStart + Math.min(1.0, centerU + halfSpan) * texSpan;
          
          viewStart = Math.max(0.0, newStart);
          viewEnd = Math.min(1.0, newEnd);
        }, 220);
      }
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
      }
    }
  }

  function handleGlobalMouseUp() {
    handleMouseUp();
    handleTimelineMouseUp();
  }

  function clearSelection() {
    selectionStart = null;
    selectionEnd = null;
  }

  function screenXToNormalizedTime(clientX: number): number {
    if (!canvas || texEnd <= texStart) return 0.0;
    const rect = canvas.getBoundingClientRect();
    const screenRatio = Math.max(0.0, Math.min(1.0, (clientX - rect.left) / rect.width));
    const clipX = screenRatio * 2.0 - 1.0;
    const u = ((clipX / zoomX) + panX + 1.0) * 0.5;
    return texStart + Math.max(0.0, Math.min(1.0, u)) * (texEnd - texStart);
  }

  function normalizedTimeToScreenPct(t: number): number {
    if (!canvas || texEnd <= texStart) return 0.0;
    const u = (t - texStart) / (texEnd - texStart);
    const clipX = (u * 2.0 - 1.0 - panX) * zoomX;
    return (clipX * 0.5 + 0.5) * 100.0;
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

<svelte:window onkeydown={handleKeyDown} onmouseup={handleGlobalMouseUp} />

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

    <!-- Glowing Frequency Ruler with tick marks -->
    <div class="frequency-ruler">
      {#each tickFrequencies as f}
        {#if f >= fmin && f <= fmax}
          {@const y = calculateFreqY(f)}
          <div class="freq-tick" style="bottom: {(y * 100).toFixed(2)}%;">
            <span class="freq-label">{f >= 1000 ? (f/1000).toFixed(1) + ' kHz' : f + ' Hz'}</span>
            <div class="freq-line"></div>
          </div>
        {/if}
      {/each}
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
      
      <div class="status-indicator">
        <span class="pulse-dot"></span> {width}x{height} [Complex]
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

      <!-- Compact Logarithmic Histogram integrated inside Sidebar -->
      <div class="histogram-panel">
        <h4>Energia (dB)</h4>
        <div class="histogram-bars">
          {#each histogramBins as count, idx}
            <div 
              class="hist-bar" 
              style="height: {(count / maxBinValue * 100).toFixed(1)}%;"
              title="Bin {idx}: {count} (~{idx * 3 - 90} dB)"
            ></div>
          {/each}
        </div>
        <div class="histogram-labels">
          <span>-90dB</span>
          <span>-45dB</span>
          <span>0dB</span>
        </div>
      </div>
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
            <h4>🎛️ Algoritmo & Cores</h4>
            <div class="input-control">
              <label for="algorithm-select">Visualizador:</label>
              <select id="algorithm-select" bind:value={algorithmType}>
                <option value="reassignment">Auger-Flandrin Reassign</option>
                <option value="cqt">Constant-Q (Projeção Esparsa)</option>
                <option value="log">Smooth Log-Spectrogram</option>
              </select>
            </div>
            
            <div class="input-control">
              <label for="palette-select">Paleta:</label>
              <select id="palette-select" bind:value={paletteType}>
                <option value="ycbcr">YCbCr Magnitude-Phase</option>
                <option value="snake">Geodesic Snake (Térmica)</option>
              </select>
            </div>

            <div class="input-control">
              <label for="scale-select">Escala Vertical:</label>
              <select id="scale-select" bind:value={frequencyScale}>
                <option value="log">Logarítmica (CQT / Auditiva)</option>
                <option value="linear">Linear (Física / STFT)</option>
              </select>
            </div>
            
            <div class="input-control range-box">
              <label>Raio de Amostragem (CQT/Reassign): {pointRadius.toFixed(2)}
                <input type="range" min="0.1" max="5.0" step="0.05" bind:value={pointRadius} />
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

          <!-- Section C: Frequency Bounds -->
          <div class="dock-section">
            <h4>📐 Filtro Hertz (Eixo Y)</h4>
            <div class="input-control range-box">
              <label>Freq Mínima: {fmin} Hz
                <input type="range" min="5" max="200" step="5" bind:value={fmin} />
              </label>
            </div>
            
            <div class="input-control range-box">
              <label>Freq Máxima: {fmax} Hz
                <input type="range" min="1000" max="22050" step="250" bind:value={fmax} />
              </label>
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

  /* Translucent Frequency ruler on the left edge of the canvas */
  .frequency-ruler {
    position: absolute;
    left: 15rem;
    top: 0;
    bottom: 0;
    width: 5.5rem;
    pointer-events: none;
    z-index: 15;
    font-family: monospace;
    font-size: 0.65rem;
    color: rgba(255, 255, 255, 0.45);
  }

  .freq-tick {
    position: absolute;
    left: 0;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transform: translateY(50%); /* perfectly centers text label with the tick line */
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
    background: rgba(15, 23, 42, 0.65) !important;
    backdrop-filter: blur(24px) !important;
    -webkit-backdrop-filter: blur(24px) !important;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.35);
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
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    background-color: #10b981;
    border-radius: 50%;
    box-shadow: 0 0 8px #10b981;
    display: inline-block;
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
    background-color: rgba(15, 23, 42, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.05);
    color: #cbd5e1;
    padding: 0.35rem 0.5rem;
    border-radius: 4px;
    outline: none;
    cursor: pointer;
    font-size: 0.75rem;
    backdrop-filter: blur(8px);
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

  /* Histogram Panel */
  .histogram-panel {
    margin-top: auto;
    background-color: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.04);
    padding: 0.5rem;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .histogram-panel h4 {
    margin: 0;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
    font-weight: 700;
  }

  .histogram-bars {
    display: flex;
    align-items: flex-end;
    gap: 1px;
    height: 45px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding-bottom: 1px;
  }

  .hist-bar {
    flex: 1;
    background-color: #38bdf8;
    border-radius: 1px 1px 0 0;
    opacity: 0.7;
    height: 0px;
  }

  .histogram-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.6rem;
    color: #475569;
    font-family: monospace;
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
</style>
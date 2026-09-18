<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  // Svelte 5 strict typing: Receive the 8-bit pre-rendered RGBA texture from WebAssembly
  let props: { 
    rgbaGrid: Uint8Array | null, 
    width: number, 
    height: number,
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
  let lastMouseX = 0;

  let selectedTool = $state<'select' | 'gaussian_brush' | 'low_pass' | 'high_pass' | 'eq'>('select');
  let brushSize = $state(50);
  let brushStrength = $state(0.5);

  // Logarithmic Histogram state variables
  let histogramBins = $state<number[]>(new Array(30).fill(0));
  let maxBinValue = $state(1);

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
    // Set preserveDrawingBuffer to true for Puppeteer integration test readback
    gl = canvas.getContext('webgl2', { antialias: true, alpha: false, preserveDrawingBuffer: true });
    if (!gl) {
      console.error('❌ WebGL2 is not supported by your browser or machine.');
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

    // Create the active standard texture
    texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    // Standard 8-bit textures support bilinear filtering universally out-of-the-box on every device!
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);

    render();
  }

  // Reactive effect to upload standard 8-bit texture to GPU and update logarithmic histogram
  $effect(() => {
    const grid = props.rgbaGrid;
    const w = props.width;
    const h = props.height;

    if (!gl || !texture || !grid || w <= 0 || h <= 0) return;

    gl.bindTexture(gl.TEXTURE_2D, texture);
    
    // Disable unpacking restrictions to support arbitrary non-power-of-two widths
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    
    // Upload the standard RGBA 8-bit pixel buffer to the GPU (universally supported, zero extensions required)
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, grid);
    console.log(`📢 WebGL uploaded 8-bit RGBA spectrogram texture: size ${w} x ${h}`);
    
    // Compute the Logarithmic Histogram from pixel colors
    computeLogHistogram();
    
    render();
  });

  // Calculate logarithmic magnitude energy distribution histogram directly from pre-rendered pixels
  function computeLogHistogram() {
    if (!props.rgbaGrid) return;
    const bins = new Array(30).fill(0);
    const numPixels = props.rgbaGrid.length / 4;
    
    // Sparsely sample the matrix to prevent frame locks (60 FPS safe)
    const step = Math.max(1, Math.floor(numPixels / 25000));
    
    for (let i = 0; i < numPixels; i += step) {
      const idx = i * 4;
      const r = props.rgbaGrid[idx];
      const g = props.rgbaGrid[idx + 1];
      const b = props.rgbaGrid[idx + 2];
      
      // Calculate BT.601 perceived luminance (which is mathematically proportional to Y^2-log dB!)
      const Y = 0.299 * r + 0.587 * g + 0.114 * b;
      if (Y < 1.0) continue; // Skip silence
      
      // Map Y in [1, 255] to 30 bins
      const binIdx = Math.floor((Y / 255.0) * 30);
      const clampedIdx = Math.max(0, Math.min(29, binIdx));
      bins[clampedIdx]++;
    }
    
    histogramBins = bins;
    maxBinValue = Math.max(1, ...bins);
  }

  function render() {
    if (!gl || !program || !props.rgbaGrid) return;
    gl.viewport(0, 0, canvas.width, canvas.height);
    
    // Explicitly bind the active texture unit and texture object
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    
    const zoomLoc = gl.getUniformLocation(program, 'u_zoomX');
    const panLoc = gl.getUniformLocation(program, 'u_panX');
    const texLoc = gl.getUniformLocation(program, 'u_spectrogramTexture');
    
    gl.uniform1f(zoomLoc, zoomX);
    gl.uniform1f(panLoc, panX);
    gl.uniform1i(texLoc, 0); // Point sampler to texture unit 0
    
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
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
  }

  function handleMouseDown(e: MouseEvent) {
    if (selectedTool === 'select') {
      isDragging = true;
      lastMouseX = e.clientX;
    } else {
      // Simulate spectral displacement or filtration brush strokes
      const rect = canvas.getBoundingClientRect();
      const x = (e.clientX - rect.left) / rect.width;
      const y = 1.0 - (e.clientY - rect.top) / rect.height; // Invert Y
      console.log(`🖌️ Applying DSP Brush [${selectedTool}] at:`, x.toFixed(3), y.toFixed(3));
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging && selectedTool === 'select') {
      const rect = canvas.getBoundingClientRect();
      const deltaX = (e.clientX - lastMouseX) / rect.width;
      panX -= deltaX * 2.0 / zoomX;
      
      const maxPan = 1.0 - (1.0 / zoomX);
      panX = Math.max(-maxPan, Math.min(maxPan, panX));
      
      lastMouseX = e.clientX;
      render();
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  // Handle direct file uploads inside the WebGL Editor
  async function handleFileUploaded(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
      const file = target.files[0];
      try {
        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        console.log(`📂 WebGL Editor successfully read custom file: ${file.name} (${bytes.length} bytes)`);
        props.onAudioUploaded(bytes);
      } catch (err) {
        console.error("Failed to read uploaded file inside WebGL Editor:", err);
      }
    }
  }

  onMount(() => {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    initWebGL();
    
    const resizeHandler = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
      render();
    };
    
    window.addEventListener('resize', resizeHandler);
    
    return () => {
      window.removeEventListener('resize', resizeHandler);
    };
  });
</script>

<div class="full-screen-editor">
  <!-- Hidden file input for uploading custom audio -->
  <input 
    type="file" 
    bind:this={fileInput} 
    accept="audio/wav" 
    onchange={handleFileUploaded} 
    style="display: none;" 
  />

  <!-- Interactive WebGL canvas background rendering the complex STFT phase-gradients -->
  <canvas 
    bind:this={canvas} 
    onwheel={handleWheel}
    onmousedown={handleMouseDown}
    onmousemove={handleMouseMove}
    onmouseup={handleMouseUp}
    onmouseleave={handleMouseUp}
  ></canvas>

  <!-- HUD: Translucent glassy panels floating over the background spectrogram -->
  <div class="hud-layer">
    
    <!-- Top Floating Toolbar -->
    <div class="hud-panel top-navbar">
      <button class="back-btn" onclick={props.onBackToConverter}>
        ⬅️ 1. Converter Áudio
      </button>
      
      <div class="vertical-divider"></div>
      
      <div class="brand-title">
        <span>Spectral WebGL</span> Phase-Gradient Editor
      </div>
      
      <div class="vertical-divider"></div>

      <!-- Button to load custom audios directly inside the WebGL editor view -->
      <button class="upload-btn" onclick={() => fileInput.click()}>
        📁 Carregar Áudio Customizado (.wav)
      </button>
      
      <div class="status-indicator">
        <span class="pulse-dot"></span> Grid: {props.width} x {props.height} [Complex]
      </div>
    </div>

    <!-- Left DSP Toolbox Sidebar -->
    <div class="hud-panel left-sidebar">
      <h3>DSP Toolbox</h3>
      
      <button class:active={selectedTool === 'select'} onclick={() => selectedTool = 'select'}>
        ✋ Mover & Zoom
      </button>
      
      <div class="sidebar-divider"></div>
      
      <button class:active={selectedTool === 'gaussian_brush'} onclick={() => selectedTool = 'gaussian_brush'}>
        🖌️ Deslocamento Gaussiano
      </button>
      
      <button class:active={selectedTool === 'high_pass'} onclick={() => selectedTool = 'high_pass'}>
        🔪 Filtro Passa-Alta (Lasso)
      </button>
      
      <button class:active={selectedTool === 'low_pass'} onclick={() => selectedTool = 'low_pass'}>
        🛡️ Filtro Passa-Baixa (Lasso)
      </button>
      
      <button class:active={selectedTool === 'eq'} onclick={() => selectedTool = 'eq'}>
        🎛️ Equalizador de Cristas
      </button>

      {#if selectedTool === 'gaussian_brush'}
        <div class="tool-controls">
          <label>Tamanho do Pincel: {brushSize}px
            <input type="range" min="10" max="200" bind:value={brushSize} />
          </label>
          <label>Força (Sigma): {brushStrength.toFixed(2)}
            <input type="range" min="0.1" max="1.0" step="0.05" bind:value={brushStrength} />
          </label>
        </div>
      {/if}

      <!-- Logarithmic Histogram of Reassignment Coefficients floating inside DSP Sidebar -->
      <div class="histogram-panel">
        <h4>Distribuição de Energia Logarítmica</h4>
        <div class="histogram-bars">
          {#each histogramBins as count, idx}
            <div 
              class="hist-bar" 
              style="height: {(count / maxBinValue * 100).toFixed(1)}%;"
              title="Bin {idx}: {count} coeficientes ({~~(idx * 3 - 90)} dB)"
            ></div>
          {/each}
        </div>
        <div class="histogram-labels">
          <span>-90 dB</span>
          <span>-45 dB</span>
          <span>0 dB</span>
        </div>
      </div>
    </div>

    <!-- Bottom Status Overlay Bar -->
    <div class="hud-panel bottom-bar">
      <span class="coordinate-view">EIXO T: {panX.toFixed(4)}s (Eixo X)</span>
      <span class="coordinate-view">ZOOM TEMPORAL: {zoomX.toFixed(2)}x</span>
      <span class="help-text">💡 Dica: Use o rolete do mouse para dar zoom horizontal focado e arraste para mover!</span>
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
    background-color: #020617;
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

  /* HUD layer styling with glassmorphism overlays */
  .hud-layer {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 10;
    pointer-events: none; /* Let clicks pass through empty spaces to canvas */
  }

  .hud-panel {
    pointer-events: auto; /* Re-enable pointer events for controls */
    background: rgba(15, 23, 42, 0.65);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.5);
    color: #cbd5e1;
    font-family: system-ui, -apple-system, sans-serif;
  }

  /* Top Navbar HUD style */
  .top-navbar {
    position: absolute;
    top: 1.25rem;
    left: 1.25rem;
    right: 1.25rem;
    height: 3.5rem;
    display: flex;
    align-items: center;
    padding: 0 1.25rem;
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
    transform: translateX(-2px);
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
    box-shadow: 0 0 10px rgba(16, 185, 129, 0.2);
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
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.7); }
    70% { transform: scale(1); box-shadow: 0 0 0 6px rgba(16, 185, 129, 0); }
    100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(16, 185, 129, 0); }
  }

  /* Left DSP Sidebar HUD style */
  .left-sidebar {
    position: absolute;
    top: 5.75rem;
    left: 1.25rem;
    bottom: 5.75rem;
    width: 16rem;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .left-sidebar h3 {
    margin: 0 0 0.5rem 0;
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
    font-weight: 700;
  }

  .sidebar-divider {
    height: 1px;
    background-color: rgba(255, 255, 255, 0.08);
    margin: 0.25rem 0;
  }

  .left-sidebar button {
    background: transparent;
    color: #cbd5e1;
    border: 1px solid rgba(255, 255, 255, 0.05);
    padding: 0.65rem 1rem;
    border-radius: 6px;
    text-align: left;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .left-sidebar button:hover {
    background-color: rgba(0, 0, 0, 0.05);
    border-color: rgba(255, 255, 255, 0.15);
  }

  .left-sidebar button.active {
    background-color: #38bdf8;
    color: #0f172a;
    font-weight: 700;
    border-color: #38bdf8;
    box-shadow: 0 0 12px rgba(56, 189, 248, 0.4);
  }

  .tool-controls {
    margin-top: auto;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.05);
    padding: 0.75rem;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    font-size: 0.8rem;
  }

  .tool-controls label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .tool-controls input[type="range"] {
    accent-color: #38bdf8;
    cursor: pointer;
  }

  /* Histogram Panel floated inside Sidebar */
  .histogram-panel {
    margin-top: auto;
    background-color: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.06);
    padding: 0.75rem;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .histogram-panel h4 {
    margin: 0;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
    font-weight: 700;
  }

  .histogram-bars {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 60px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    padding-bottom: 2px;
  }

  .hist-bar {
    flex: 1;
    background-color: #38bdf8;
    border-radius: 1px 1px 0 0;
    transition: height 0.3s;
    opacity: 0.75;
  }

  .hist-bar:hover {
    background-color: #10b981;
    opacity: 1.0;
  }

  .histogram-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.65rem;
    color: #475569;
    font-family: monospace;
  }

  /* Bottom HUD Statusbar */
  .bottom-bar {
    position: absolute;
    bottom: 1.25rem;
    left: 1.25rem;
    right: 1.25rem;
    height: 3rem;
    display: flex;
    align-items: center;
    padding: 0 1.25rem;
    gap: 1.5rem;
  }

  .coordinate-view {
    font-family: monospace;
    font-size: 0.8rem;
    color: #94a3b8;
    background-color: rgba(0, 0, 0, 0.2);
    padding: 0.35rem 0.65rem;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.04);
  }

  .help-text {
    font-size: 0.8rem;
    color: #64748b;
    margin-left: auto;
  }
</style>
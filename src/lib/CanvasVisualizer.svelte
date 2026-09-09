<script lang="ts">
  import { onMount } from 'svelte';

  // Props using Svelte 5 standard runes
  let { rgbaBytes = null }: { rgbaBytes: Uint8Array | null } = $props();

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let offscreenCanvas: HTMLCanvasElement | null = null;
  let offscreenCtx: CanvasRenderingContext2D | null = null;

  // Image Dimensions
  let width = $state(0);
  let height = $state(0);

  // Interaction State
  let scale = $state(1);
  let offsetX = $state(0);
  let offsetY = $state(0);
  let isDragging = false;
  let startX = 0;
  let startY = 0;

  // Hover/Inspection State
  let hoverX = $state(-1);
  let hoverY = $state(-1);
  let hoverR = $state(0); // Holds exact C_M value
  let hoverG = $state(0); // Holds exact C_S value

  // Intermediate Visual Mode: full (Complex), mid (Mono energy), side (Stereo width)
  let visualMode = $state<'full' | 'mid' | 'side'>('full');

  // Anti-Chaos: Bilinear image smoothing (default true for smooth spectral envelopes)
  let smoothSpectrogram = $state<boolean>(true);

  // Aspect Ratio Stretching: horizontal stretch factor to format as a wide Melgram (default 4x)
  let stretchFactor = $state<number>(4);

  // 1. Calculate canvas dimensions and reset view ONLY when the raw bytes change
  $effect(() => {
    if (rgbaBytes && rgbaBytes.length > 0) {
      // Read dimensions from the self-contained metadata header (big-endian)
      const w_png = (rgbaBytes[4] << 24) | (rgbaBytes[5] << 16) | (rgbaBytes[6] << 8) | rgbaBytes[7];
      const h = (rgbaBytes[8] << 24) | (rgbaBytes[9] << 16) | (rgbaBytes[10] << 8) | rgbaBytes[11];
      
      // Since we pack Mid and Side across 2 adjacent pixels, the visual width is exactly half of PNG width!
      width = w_png / 2;
      height = h;

      if (!offscreenCanvas) {
        offscreenCanvas = document.createElement('canvas');
      }
      offscreenCanvas.width = width;
      offscreenCanvas.height = height;
      offscreenCtx = offscreenCanvas.getContext('2d');

      resetView();
    } else {
      width = 0;
      height = 0;
      offscreenCanvas = null;
      offscreenCtx = null;
      resetView();
    }
  });

  // 2. Perform raw pixel channel filtering and render when bytes or visualMode changes
  $effect(() => {
    if (rgbaBytes && rgbaBytes.length > 0 && offscreenCtx) {
      const imageData = offscreenCtx.createImageData(width, height);
      
      // Slice out the 16-byte metadata header to render only the actual wavelet coefficients
      const coefOffset = 16;
      const w_png = width * 2;
      
      for (let r = 0; r < height; r++) {
        for (let c = 0; c < width; c++) {
          // Pixel A is at r * w_png + (c * 2). Pixel B is at offset_a + 4.
          const idx_a = r * w_png + (c * 2);
          const offset_a = coefOffset + idx_a * 4;
          const offset_b = offset_a + 4;
          
          if (offset_b + 3 >= rgbaBytes.length) break;

          // Extract high and low bytes of Mid (Pixel A)
          const mid_high = rgbaBytes[offset_a + 2]; // high byte (B channel of Pixel A)
          const mid_low  = rgbaBytes[offset_a + 3]; // low byte (A channel of Pixel A)
          
          // Extract high and low bytes of Side (Pixel B)
          const side_high = rgbaBytes[offset_b + 2]; // high byte (B channel of Pixel B)
          const side_low  = rgbaBytes[offset_b + 3]; // low byte (A channel of Pixel B)
          
          // Output pixel coordinate in Svelte's offscreen visual canvas (W x H)
          const out_idx = (r * width + c) * 4;
          
          if (visualMode === 'full') {
            // Render Mid on Red/Green, and Side on Blue
            imageData.data[out_idx]     = mid_high; // Mid High (Red)
            imageData.data[out_idx + 1] = mid_low;  // Mid Low (Green)
            imageData.data[out_idx + 2] = side_high; // Side High (Blue)
            imageData.data[out_idx + 3] = 255;      // Fully opaque
          } else if (visualMode === 'mid') {
            // Render only Mono Mid channels
            imageData.data[out_idx]     = mid_high;
            imageData.data[out_idx + 1] = mid_low;
            imageData.data[out_idx + 2] = 0;
            imageData.data[out_idx + 3] = 255;
          } else {
            // Render only Stereo Side channels
            imageData.data[out_idx]     = 0;
            imageData.data[out_idx + 1] = 0;
            imageData.data[out_idx + 2] = side_high;
            imageData.data[out_idx + 3] = 255;
          }
        }
      }
      offscreenCtx.putImageData(imageData, 0, 0);
      draw();
    }
  });

  // Animation frame loop
  $effect(() => {
    if (canvas && offscreenCanvas) {
      draw();
    }
  });

  // Reactive redraw trigger when smoothSpectrogram or stretchFactor changes
  $effect(() => {
    if (canvas && offscreenCanvas && smoothSpectrogram !== undefined && stretchFactor !== undefined) {
      draw();
    }
  });

  function resetView() {
    scale = 1;
    offsetX = 0;
    offsetY = 0;
    if (canvas && width > 0 && height > 0) {
      // Scale taking into account the horizontal stretch factor
      const visualWidth = width * stretchFactor;
      scale = Math.min((canvas.width - 40) / visualWidth, (canvas.height - 40) / height, 5);
      if (scale < 0.1) scale = 0.1;
      offsetX = (canvas.width - visualWidth * scale) / 2;
      offsetY = (canvas.height - height * scale) / 2;
    }
  }

  function draw() {
    if (!canvas) return;
    ctx = canvas.getContext('2d');
    if (!ctx || !offscreenCanvas) return;

    // Clear visible canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw grid background to show transparency
    drawGrid(ctx, canvas.width, canvas.height);

    // Apply transform and draw the offscreen canvas
    ctx.save();
    ctx.translate(offsetX, offsetY);
    // Stretch the time-axis (X) by multiplying by stretchFactor
    ctx.scale(scale * stretchFactor, scale);
    
    // Enable bilinear smoothing based on user preference to blend wavelet phases smoothly
    ctx.imageSmoothingEnabled = smoothSpectrogram;
    if (smoothSpectrogram) {
      ctx.imageSmoothingQuality = 'high';
    }
    
    ctx.drawImage(offscreenCanvas, 0, 0);
    ctx.restore();
  }

  function drawGrid(c: CanvasRenderingContext2D, w: number, h: number) {
    const size = 10;
    c.fillStyle = '#090d16'; // Extremely dark background
    c.fillRect(0, 0, w, h);
    
    c.fillStyle = '#111827'; // Subtle grid squares
    for (let x = 0; x < w; x += size * 2) {
      for (let y = 0; y < h; y += size * 2) {
        c.fillRect(x, y, size, size);
        c.fillRect(x + size, y + size, size, size);
      }
    }
  }

  // Event Handlers for Panning
  function handleMouseDown(e: MouseEvent) {
    if (!offscreenCanvas) return;
    isDragging = true;
    startX = e.clientX - offsetX;
    startY = e.clientY - offsetY;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!canvas || !offscreenCanvas) return;

    // 1. Handle Panning
    if (isDragging) {
      offsetX = e.clientX - startX;
      offsetY = e.clientY - startY;
      draw();
    }

    // 2. Handle Inspection / Pixel coordinates calculation
    const rect = canvas.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    // Convert screen coordinates back to image/pixel space coordinates (calibrated for stretchFactor)
    const imgX = Math.floor((mouseX - offsetX) / (scale * stretchFactor));
    const imgY = Math.floor((mouseY - offsetY) / scale);

    if (imgX >= 0 && imgX < width && imgY >= 0 && imgY < height && rgbaBytes) {
      hoverX = imgX;
      hoverY = imgY;

      const coefOffset = 16;
      const w_png = width * 2;
      const idx_a = imgY * w_png + (imgX * 2);
      const offset_a = coefOffset + idx_a * 4;
      const offset_b = offset_a + 4;
      
      if (offset_b + 3 < rgbaBytes.length) {
        // Reconstruct original Mid (C_M) and Side (C_S) u16 values from high and low bytes
        const u16_m = (rgbaBytes[offset_a + 2] << 8) | rgbaBytes[offset_a + 3];
        const u16_s = (rgbaBytes[offset_b + 2] << 8) | rgbaBytes[offset_b + 3];
        
        // Decode ZigZag to get the exact signed 16-bit coefficients!
        const m_val = (u16_m >> 1) ^ (-(u16_m & 1));
        const s_val = (u16_s >> 1) ^ (-(u16_s & 1));
        
        hoverR = m_val;
        hoverG = s_val;
      }
    } else {
      hoverX = -1;
      hoverY = -1;
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  // Event Handler for Zooming (Scroll)
  function handleWheel(e: WheelEvent) {
    if (!canvas || !offscreenCanvas) return;
    e.preventDefault();

    const rect = canvas.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    // Zoom factor
    const zoomFactor = 1.1;
    const nextScale = e.deltaY < 0 ? scale * zoomFactor : scale / zoomFactor;

    // Limit zoom scale between 0.1x and 100x
    if (nextScale < 0.1 || nextScale > 100) return;

    // Zoom centered on mouse cursor taking into account independent axis scaling
    offsetX = mouseX - (mouseX - offsetX) * (nextScale / scale);
    offsetY = mouseY - (mouseY - offsetY) * (nextScale / scale);
    scale = nextScale;

    draw();
  }

  onMount(() => {
    // Set initial size
    if (canvas) {
      canvas.width = canvas.parentElement?.clientWidth || 800;
      canvas.height = 400;
      draw();
    }

    const resizeObserver = new ResizeObserver(() => {
      if (canvas && canvas.parentElement) {
        canvas.width = canvas.parentElement.clientWidth;
        draw();
      }
    });
    
    if (canvas && canvas.parentElement) {
      resizeObserver.observe(canvas.parentElement);
    }

    return () => {
      resizeObserver.disconnect();
    };
  });
</script>

<div class="visualizer-container">
  <div class="canvas-header">
    <h3>Imagem Lossless Reversível (Grade de Pixels)</h3>
    
    {#if rgbaBytes}
      <div class="visual-mode-selector">
        <button 
          class="btn-toggle {visualMode === 'full' ? 'active' : ''}" 
          onclick={() => visualMode = 'full'}
          title="Espectro completo: Mid (L/R soma) mapeado para Vermelho/Verde e Side (L/R diferença) mapeado para Azul/Alfa"
        >
          Complexo (Completo)
        </button>
        <button 
          class="btn-toggle {visualMode === 'mid' ? 'active' : ''}" 
          onclick={() => visualMode = 'mid'}
          title="Mostra apenas a energia Mid (Mono central) do sinal. O canal Side é fixado no cinza neutro."
        >
          Apenas Mid (Mono / R-G)
        </button>
        <button 
          class="btn-toggle {visualMode === 'side' ? 'active' : ''}" 
          onclick={() => visualMode = 'side'}
          title="Mostra apenas o sinal Side (Espacialidade / Diferença estéreo). O canal Mid é fixado no cinza neutro."
        >
          Apenas Side (Estéreo / B-A)
        </button>
      </div>
    {/if}

    <button class="btn-secondary" onclick={resetView} disabled={!rgbaBytes}>
      Centralizar
    </button>
  </div>

  <!-- Semantic Stretching & Anti-Noise Controls -->
  {#if rgbaBytes}
    <div class="canvas-controls-bar">
      <label class="control-checkbox-label">
        <input type="checkbox" bind:checked={smoothSpectrogram} />
        <span class="checkbox-text">Suavizar Espectro (Fases GPU)</span>
      </label>

      <div class="control-stretch-selector">
        <span class="control-label">Estiramento Horizontal (Tempo):</span>
        <select bind:value={stretchFactor} class="stretch-dropdown">
          <option value={1}>1x (Físico/Quadrado)</option>
          <option value={2}>2x</option>
          <option value={4}>4x (Estilo Melgram)</option>
          <option value={8}>8x (Esticado Panorâmico)</option>
        </select>
      </div>
    </div>
  {/if}

  <div class="canvas-wrapper">
    <canvas
      bind:this={canvas}
      onmousedown={handleMouseDown}
      onmousemove={handleMouseMove}
      onmouseup={handleMouseUp}
      onmouseleave={handleMouseUp}
      onwheel={handleWheel}
    ></canvas>
  </div>

  {#if width > 0 && height > 0}
    <div class="meta-footer">
      <div class="dimensions">
        <strong>Resolução:</strong> {width} x {height} px ({width * height} pixels)
      </div>
      <div class="inspector">
        {#if hoverX !== -1}
          <strong>Pixel:</strong> X: {hoverX}, Y: {hoverY} | 
          <span class="color-text" style="color: #38bdf8;">C_M (Mid): {hoverR} | C_S (Side): {hoverG}</span>
        {:else}
          <em>Passe o mouse sobre os pixels para inspecionar</em>
        {/if}
      </div>
    </div>
  {:else}
    <div class="no-data">
      <p>Nenhuma imagem gerada. Carregue um áudio para visualizar os pixels equivalentes.</p>
    </div>
  {/if}
</div>

<style>
  .visualizer-container {
    display: flex;
    flex-direction: column;
    background-color: #0f172a;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 1rem;
    color: #f1f5f9;
    box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);
  }

  .canvas-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .canvas-header h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: #38bdf8;
  }

  .canvas-wrapper {
    position: relative;
    width: 100%;
    border-radius: 6px;
    overflow: hidden;
    cursor: grab;
  }

  .canvas-wrapper:active {
    cursor: grabbing;
  }

  canvas {
    display: block;
    width: 100%;
    background-color: #1e293b;
  }

  .meta-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 0.75rem;
    font-size: 0.85rem;
    background-color: #1e293b;
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    border: 1px solid #334155;
  }

  .inspector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .color-text {
    font-family: monospace;
    color: #38bdf8;
  }

  .btn-secondary {
    background-color: #334155;
    color: #f1f5f9;
    border: 1px solid #475569;
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.15s ease-in-out;
  }

  .btn-secondary:hover:not(:disabled) {
    background-color: #475569;
    border-color: #64748b;
  }

  .btn-secondary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .no-data {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 150px;
    background-color: #1e293b;
    border: 1px dashed #475569;
    border-radius: 6px;
    color: #94a3b8;
    text-align: center;
    font-style: italic;
  }

  /* Segmented Control Selector for Wavelet Planes */
  .visual-mode-selector {
    display: flex;
    background-color: #0f172a;
    border: 1px solid #334155;
    padding: 2px;
    border-radius: 6px;
    gap: 2px;
  }

  .btn-toggle {
    background: transparent;
    color: #94a3b8;
    border: none;
    padding: 0.35rem 0.65rem;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.15s ease-in-out;
  }

  .btn-toggle:hover:not(.active) {
    color: #f1f5f9;
    background-color: #1e293b;
  }

  .btn-toggle.active {
    background-color: #38bdf8;
    color: #0f172a;
    box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06);
  }

  /* Dynamic Controls Bar Styling */
  .canvas-controls-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: #1e293b;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .control-checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-size: 0.8rem;
    color: #cbd5e1;
    user-select: none;
  }

  .checkbox-text {
    font-weight: 600;
  }

  .control-stretch-selector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .control-label {
    font-size: 0.8rem;
    color: #94a3b8;
    font-weight: 500;
  }

  .stretch-dropdown {
    background-color: #0f172a;
    border: 1px solid #475569;
    color: #f1f5f9;
    border-radius: 4px;
    padding: 0.25rem 0.5rem;
    font-size: 0.8rem;
    font-weight: 600;
    outline: none;
    cursor: pointer;
    transition: border-color 0.15s ease-in-out;
  }

  .stretch-dropdown:focus {
    border-color: #38bdf8;
  }
</style>
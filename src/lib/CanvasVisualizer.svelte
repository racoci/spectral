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
  let hoverR = $state(0);
  let hoverG = $state(0);
  let hoverB = $state(0);
  let hoverA = $state(0);

  // Compute size and draw when the byte array changes
  $effect(() => {
    if (rgbaBytes && rgbaBytes.length > 0) {
      const pixelCount = rgbaBytes.length / 4;
      // Calculate approximately square dimensions
      width = Math.floor(Math.sqrt(pixelCount));
      if (width < 1) width = 1;
      height = Math.ceil(pixelCount / width);

      // Create or adjust offscreen canvas for rendering the raw pixels
      if (!offscreenCanvas) {
        offscreenCanvas = document.createElement('canvas');
      }
      offscreenCanvas.width = width;
      offscreenCanvas.height = height;
      offscreenCtx = offscreenCanvas.getContext('2d');

      if (offscreenCtx) {
        // Create standard ImageData buffer (W x H x 4)
        const imageData = offscreenCtx.createImageData(width, height);
        
        // Copy bytes. If rgbaBytes is smaller than the ImageData buffer,
        // it leaves the remainder as 0 (fully transparent black), which is perfect.
        imageData.data.set(rgbaBytes);
        offscreenCtx.putImageData(imageData, 0, 0);
      }

      // Reset zoom/pan on new image
      resetView();
    } else {
      width = 0;
      height = 0;
      offscreenCanvas = null;
      offscreenCtx = null;
      resetView();
    }
  });

  // Animation frame loop
  $effect(() => {
    if (canvas && offscreenCanvas) {
      draw();
    }
  });

  function resetView() {
    scale = 1;
    offsetX = 0;
    offsetY = 0;
    if (canvas && width > 0 && height > 0) {
      // Center the image in the visible canvas
      scale = Math.min((canvas.width - 40) / width, (canvas.height - 40) / height, 5);
      if (scale < 0.1) scale = 0.1;
      offsetX = (canvas.width - width * scale) / 2;
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
    ctx.scale(scale, scale);
    
    // Disable image smoothing to see crisp individual pixels when zoomed in
    ctx.imageSmoothingEnabled = false;
    
    ctx.drawImage(offscreenCanvas, 0, 0);
    ctx.restore();
  }

  function drawGrid(c: CanvasRenderingContext2D, w: number, h: number) {
    const size = 10;
    c.fillStyle = '#1e293b'; // Dark background
    c.fillRect(0, 0, w, h);
    
    c.fillStyle = '#334155'; // Slightly lighter squares
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

    // Convert screen coordinates back to image/pixel space coordinates
    const imgX = Math.floor((mouseX - offsetX) / scale);
    const imgY = Math.floor((mouseY - offsetY) / scale);

    if (imgX >= 0 && imgX < width && imgY >= 0 && imgY < height) {
      hoverX = imgX;
      hoverY = imgY;

      // Read pixel value from the offscreen canvas context
      if (offscreenCtx) {
        const pixel = offscreenCtx.getImageData(imgX, imgY, 1, 1).data;
        hoverR = pixel[0];
        hoverG = pixel[1];
        hoverB = pixel[2];
        hoverA = pixel[3];
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

    // Zoom centered on mouse cursor
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
    <button class="btn-secondary" onclick={resetView} disabled={!rgbaBytes}>
      Centralizar Visualização
    </button>
  </div>

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
          <span class="color-badge" style="background-color: rgba({hoverR},{hoverG},{hoverB},{hoverA/255})"></span>
          <span class="color-text">RGBA({hoverR}, {hoverG}, {hoverB}, {hoverA})</span>
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

  .color-badge {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 1px solid #f1f5f9;
    border-radius: 2px;
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
</style>

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  let { complexGrid, width, height }: { complexGrid: Float32Array | null, width: number, height: number } = $props();

  let canvas: HTMLCanvasElement;
  let gl: WebGL2RenderingContext | null = null;
  let program: WebGLProgram | null = null;
  let texture: WebGLTexture | null = null;

  let zoomX = $state(1.0);
  let panX = $state(0.0);
  let isDragging = false;
  let lastMouseX = 0;

  let selectedTool = $state<'select' | 'gaussian_brush' | 'high_pass'>('select');

  // Fragment Shader: Applies Y^2-log mapping, Radial Residual w, and BT.601 YCbCr directly on the GPU
  const fragmentShaderSource = `#version 300 es
  precision highp float;
  in vec2 v_uv;
  out vec4 outColor;
  
  uniform sampler2D u_complexTexture;
  
  void main() {
      // Texture returns vec2(Re, Im)
      vec2 z = texture(u_complexTexture, v_uv).rg;
      float r = length(z);
      
      // Absolute silence threshold (Y = 0)
      if (r < 3.0517578e-5) { // 2^-15
          outColor = vec4(0.0, 0.0, 0.0, 1.0);
          return;
      }
      
      // Y^2 - log encoding scale
      // Y = round( sqrt(1.0 + 65024.0 * (log2(r) + 15.0) / 15.0) )
      float log2_r = log2(r);
      float y_val = sqrt(1.0 + 65024.0 * (log2_r + 15.0) / 15.0);
      float Y = floor(y_val);
      Y = clamp(Y, 1.0, 255.0);
      
      // Lower bound of the magnitude interval for this Y
      float Amin_pow = -15.0 + 15.0 * (Y * Y - 1.0) / 65024.0;
      float A_min = exp2(Amin_pow);
      
      // Radial residual w = (r - A_min) * (z / r)
      float r_resid = r - A_min;
      
      // Upper bound to normalize residual
      float A_max = exp2(-15.0 + 15.0 * ((Y + 1.0) * (Y + 1.0) - 1.0) / 65024.0);
      float delta_A = A_max - A_min;
      
      // Normalize radial residual to [0, 1)
      float r_norm = r_resid / delta_A;
      vec2 w_norm = r_norm * (z / r);
      
      // Chrominance mappings
      float Cr_norm = -w_norm.x; // Inverted Real part
      float Cb_norm = w_norm.y;  // Imaginary part
      
      float Y_norm = Y / 255.0;
      
      // BT.601 YCbCr to RGB (assuming Cb/Cr are in [-1, 1])
      float R = Y_norm + 1.402 * Cr_norm;
      float G = Y_norm - 0.344136 * Cb_norm - 0.714136 * Cr_norm;
      float B = Y_norm + 1.772 * Cb_norm;
      
      // Clamp and output
      outColor = vec4(clamp(R, 0.0, 1.0), clamp(G, 0.0, 1.0), clamp(B, 0.0, 1.0), 1.0);
  }`;

  const vertexShaderSource = `#version 300 es
  in vec2 a_position;
  out vec2 v_uv;
  uniform float u_zoomX;
  uniform float u_panX;
  
  void main() {
      // Map view bounds
      float x = (a_position.x / u_zoomX) + u_panX;
      float y = a_position.y;
      
      v_uv = vec2(x * 0.5 + 0.5, 0.5 - y * 0.5); // Flip Y so low frequencies are at bottom
      gl_Position = vec4(a_position, 0.0, 1.0);
  }`;

  function compileShader(type: number, source: string) {
    if (!gl) return null;
    const shader = gl.createShader(type);
    if (!shader) return null;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error('Shader compilation error:', gl.getShaderInfoLog(shader));
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  }

  function initWebGL() {
    if (!canvas) return;
    gl = canvas.getContext('webgl2');
    if (!gl) {
      console.error('WebGL2 is not supported.');
      return;
    }
    
    // We require floating point textures
    gl.getExtension('EXT_color_buffer_float');

    const vs = compileShader(gl.VERTEX_SHADER, vertexShaderSource);
    const fs = compileShader(gl.FRAGMENT_SHADER, fragmentShaderSource);
    if (!vs || !fs) return;

    program = gl.createProgram();
    if (!program) return;
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('Program linking error:', gl.getProgramInfoLog(program));
      return;
    }

    gl.useProgram(program);

    // Full screen quad
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
    if (gl && texture && complexGrid && width > 0 && height > 0) {
      gl.bindTexture(gl.TEXTURE_2D, texture);
      // Upload the Float32Array (which is [Re, Im, Re, Im...]) as RG32F
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RG32F, width, height, 0, gl.RG, gl.FLOAT, complexGrid);
      render();
    }
  });

  function render() {
    if (!gl || !program || !complexGrid) return;
    gl.viewport(0, 0, canvas.width, canvas.height);
    
    // Calculate aspect ratio corrections internally in shader if needed, 
    // for now we pan and zoom normalized coordinates.
    const zoomLoc = gl.getUniformLocation(program, 'u_zoomX');
    const panLoc = gl.getUniformLocation(program, 'u_panX');
    
    gl.uniform1f(zoomLoc, zoomX);
    gl.uniform1f(panLoc, panX);
    
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomFactor = 1.1;
    const oldZoom = zoomX;
    
    if (e.deltaY < 0) {
      zoomX *= zoomFactor;
    } else {
      zoomX /= zoomFactor;
    }
    
    zoomX = Math.max(1.0, zoomX);
    
    // Zoom towards mouse pointer
    const rect = canvas.getBoundingClientRect();
    const mouseX = (e.clientX - rect.left) / rect.width; // 0 to 1
    const clipMouseX = (mouseX * 2.0 - 1.0); // -1 to 1
    
    // Adjust pan to keep the point under the cursor stationary
    const viewPointX = (clipMouseX / oldZoom) + panX;
    panX = viewPointX - (clipMouseX / zoomX);
    
    // Clamp pan bounds
    const maxPan = 1.0 - (1.0 / zoomX);
    panX = Math.max(-maxPan, Math.min(maxPan, panX));

    render();
  }

  function handleMouseDown(e: MouseEvent) {
    if (selectedTool === 'select') {
        isDragging = true;
        lastMouseX = e.clientX;
    } else {
        // Implement Brush Interactions
        console.log(`Using tool ${selectedTool} at`, e.clientX, e.clientY);
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging && selectedTool === 'select') {
      const rect = canvas.getBoundingClientRect();
      const deltaX = (e.clientX - lastMouseX) / rect.width;
      // We are panning, move viewport
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

  onMount(() => {
    const parent = canvas.parentElement;
    if (parent) {
      canvas.width = parent.clientWidth;
      canvas.height = parent.clientHeight;
    }
    initWebGL();
    window.addEventListener('resize', () => {
      if (parent) {
        canvas.width = parent.clientWidth;
        canvas.height = parent.clientHeight;
        render();
      }
    });
  });
</script>

<div class="editor-container">
  <div class="toolbar">
    <div class="tools-group">
      <button class:active={selectedTool === 'select'} onclick={() => selectedTool = 'select'}>
        ✋ Mover/Zoom
      </button>
      <button class:active={selectedTool === 'gaussian_brush'} onclick={() => selectedTool = 'gaussian_brush'}>
        🖌️ Pincel de Deslocamento
      </button>
      <button class:active={selectedTool === 'high_pass'} onclick={() => selectedTool = 'high_pass'}>
        🔪 Corte Passa-Alta
      </button>
    </div>
    
    <div class="status-group">
      <span class="zoom-status">Zoom: {zoomX.toFixed(2)}x</span>
    </div>
  </div>
  
  <div class="canvas-wrapper">
    <canvas 
      bind:this={canvas} 
      onwheel={handleWheel}
      onmousedown={handleMouseDown}
      onmousemove={handleMouseMove}
      onmouseup={handleMouseUp}
      onmouseleave={handleMouseUp}
    ></canvas>
  </div>
</div>

<style>
  .editor-container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 600px;
    background-color: #0f172a;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid #334155;
    box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background-color: #1e293b;
    border-bottom: 1px solid #334155;
  }

  .tools-group {
    display: flex;
    gap: 0.5rem;
  }

  button {
    background-color: #334155;
    color: #cbd5e1;
    border: 1px solid #475569;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  button:hover {
    background-color: #475569;
  }

  button.active {
    background-color: #0ea5e9;
    color: white;
    border-color: #0284c7;
    font-weight: 600;
  }

  .zoom-status {
    color: #94a3b8;
    font-size: 0.85rem;
    font-family: monospace;
  }

  .canvas-wrapper {
    flex-grow: 1;
    position: relative;
    width: 100%;
    height: 100%;
  }

  canvas {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    cursor: crosshair;
  }
</style>
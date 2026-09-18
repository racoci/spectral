<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { 
    decode_rg_to_coefficient, 
    get_color_r, 
    get_color_g, 
    get_color_b,
    decode_color_to_coefficient,
    wasm_encode_n,
    wasm_generate_v6_spectrogram,
    wasm_generate_v7_spectrogram,
    wasm_generate_v8_spectrogram,
    wasm_generate_complex_spectrogram
  } from '../wasm/core_wasm.js';

  // Props using Svelte 5 standard runes
  let { rgbaBytes = null }: { rgbaBytes: Uint8Array | null } = $props();

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let offscreenCanvas: HTMLCanvasElement | null = null;
  let offscreenCtx: CanvasRenderingContext2D | null = null;

  // Image Dimensions derived synchronously from props (Zero-loop, compile-time contract!)
  let width = $derived.by(() => {
    if (rgbaBytes && rgbaBytes.length > 0) {
      const w_png = (rgbaBytes[4] << 24) | (rgbaBytes[5] << 16) | (rgbaBytes[6] << 8) | rgbaBytes[7];
      const h = (rgbaBytes[8] << 24) | (rgbaBytes[9] << 16) | (rgbaBytes[10] << 8) | rgbaBytes[11];
      const original_len = (rgbaBytes[0] << 24) | (rgbaBytes[1] << 16) | (rgbaBytes[2] << 8) | rgbaBytes[3];
      const num_samples = Math.ceil(original_len / 4);

      // Detect if image is in V1 (2-pixel packing) or V2 (1-pixel packing) format
      const isTwoPixel = w_png * h > num_samples * 1.5;
      return isTwoPixel ? w_png / 2 : w_png;
    }
    return 0;
  });

  let height = $derived.by(() => {
    if (rgbaBytes && rgbaBytes.length > 0) {
      const h = (rgbaBytes[8] << 24) | (rgbaBytes[9] << 16) | (rgbaBytes[10] << 8) | rgbaBytes[11];
      return h;
    }
    return 0;
  });

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
  let hoverOctave = $state(-1); // scale level j
  let hoverK = $state(-1); // temporal index k
  let hoverFreqRange = $state(''); // "[lo, hi) Hz"
  let hoverCenterFreq = $state(''); // center frequency
  let isDyadicMode = $state(false); // true if packingVersion is 4

  // Intermediate Visual Mode: full (Complex), mid (Mono energy), side (Stereo width)
  let visualMode = $state<'full' | 'mid' | 'side'>('full');

  // Anti-Chaos: Bilinear image smoothing (default true for smooth spectral envelopes)
  let smoothSpectrogram = $state<boolean>(true);

  // Aspect Ratio Stretching: horizontal stretch factor to format as a wide Melgram (default 4x)
  let stretchFactor = $state<number>(4);

  // Color scheme selector: 'snake' (default Geodesic Snake) or 'ycbcr' (BT.601 Complex Phase-Magnitude)
  let colorScheme = $state<'snake' | 'ycbcr'>('snake');

  // Mathematical decoder mapping complex coefficient z = re + i * im to YCbCr Magnitude-Phase space (254-step log base 2^(15/254))
  const decibelLuminance = (re: number, im: number): { r_u: number, g_u: number, b_u: number } => {
    const abs_z = Math.sqrt(re * re + im * im);
    if (abs_z < 1e-12) {
      return { r_u: 0, g_u: 0, b_u: 0 }; // Reserved Y = 0 for absolute silence!
    }
    
    // Normalize to [2^-15, 1.0] range based on maximum 16-bit signed PCM amplitude
    const abs_z_norm = abs_z / 32768.0;
    
    // Logarithmic base b = 2^(15/254)
    const logBaseFactor = 15.0 / 254.0;
    const b = Math.pow(2.0, logBaseFactor);
    
    // Y = floor(log_b(|z_norm|)) + 255
    let Y = Math.floor(Math.log2(abs_z_norm) / logBaseFactor) + 255;
    Y = Math.max(1, Math.min(255, Y));
    
    // Recalculated estimate of Abs(z): A_z = b^(Y - 255)
    const A_z = Math.pow(2.0, (Y - 255) * logBaseFactor);
    
    const re_norm = re / 32768.0;
    const im_norm = im / 32768.0;
    
    // Denominator to scale the residual vector w to [-1.0, 1.0] perfectly
    const denom = A_z * (b - 1.0);
    const w_re = (re_norm - A_z * (re_norm / abs_z_norm)) / (denom > 1e-15 ? denom : 1e-15);
    const w_im = (im_norm - A_z * (im_norm / abs_z_norm)) / (denom > 1e-15 ? denom : 1e-15);
    
    // Cr = -Re(w), Cb = Im(w)
    const Cr = -w_re;
    const Cb = w_im;
    
    // Map chrominance offsets Cb, Cr to [16, 240] centered around 128
    const cb_byte = Math.max(16.0, Math.min(240.0, Cb * 112.0 + 128.0));
    const cr_byte = Math.max(16.0, Math.min(240.0, Cr * 112.0 + 128.0));
    
    // Convert YCbCr BT.601 to RGB color space, using the actual Y in [1, 255] as luminance
    const r_val = Y + 1.402 * (cr_byte - 128.0);
    const g_val = Y - 0.344136 * (cb_byte - 128.0) - 0.714136 * (cr_byte - 128.0);
    const b_val = Y + 1.772 * (cb_byte - 128.0);
    
    return {
      r_u: Math.max(0, Math.min(255, Math.round(r_val))),
      g_u: Math.max(0, Math.min(255, Math.round(g_val))),
      b_u: Math.max(0, Math.min(255, Math.round(b_val)))
    };
  };

  // Consolidated Svelte 5 Pipeline Effect (Zero-loop, Unidirectional Flow)
  $effect(() => {
    if (rgbaBytes && rgbaBytes.length > 0 && width > 0 && height > 0) {
      // Setup the offscreen canvas only when dimensions change
      if (!offscreenCanvas) {
        offscreenCanvas = document.createElement('canvas');
      }
      
      if (offscreenCanvas.width !== width || offscreenCanvas.height !== height) {
        offscreenCanvas.width = width;
        offscreenCanvas.height = height;
        offscreenCtx = offscreenCanvas.getContext('2d');
        // Isolate resetView from dependency tracking
        untrack(() => resetView());
      }
      
      if (offscreenCtx) {
        const imageData = offscreenCtx.createImageData(width, height);
        const coefOffset = 16;
        
        // Fetch parameters to identify algorithm format
        const w_png = (rgbaBytes[4] << 24) | (rgbaBytes[5] << 16) | (rgbaBytes[6] << 8) | rgbaBytes[7];
        const original_len = (rgbaBytes[0] << 24) | (rgbaBytes[1] << 16) | (rgbaBytes[2] << 8) | rgbaBytes[3];
        const num_samples = Math.ceil(original_len / 4);
        const isTwoPixel = w_png * height > num_samples * 1.5;
        const packingVersion = rgbaBytes[13];
        
        if (packingVersion === 9) {
          if (colorScheme === 'ycbcr') {
            const payload_start = 24 + width * height * 4;
            const original_wav = rgbaBytes.slice(payload_start);
            try {
              const complex_grid = wasm_generate_complex_spectrogram(original_wav, height, "hann");
              
              for (let r = 0; r < height; r++) {
                for (let c = 0; c < width; c++) {
                  // Invert vertically so low frequencies are at the bottom
                  const grid_y = height - 1 - r;
                  const complex_idx = (grid_y * width + c) * 2;
                  const re = complex_grid[complex_idx];
                  const im = complex_grid[complex_idx + 1];
                  
                  const { r_u, g_u, b_u } = decibelLuminance(re, im);
                  const out_idx = (r * width + c) * 4;
                  imageData.data[out_idx]     = r_u;
                  imageData.data[out_idx + 1] = g_u;
                  imageData.data[out_idx + 2] = b_u;
                  imageData.data[out_idx + 3] = 255;
                }
              }
            } catch (err) {
              console.error("Failed to generate complex spectrogram in UI:", err);
            }
          } else {
            // Default Geodesic Snake (already rasterized on the PNG!)
            for (let r = 0; r < height; r++) {
              for (let c = 0; c < width; c++) {
                const in_idx = 24 + (r * width + c) * 4;
                const out_idx = (r * width + c) * 4;
                if (in_idx + 3 < rgbaBytes.length) {
                  imageData.data[out_idx]     = rgbaBytes[in_idx];
                  imageData.data[out_idx + 1] = rgbaBytes[in_idx + 1];
                  imageData.data[out_idx + 2] = rgbaBytes[in_idx + 2];
                  imageData.data[out_idx + 3] = 255;
                }
              }
            }
          }
        } else if (packingVersion === 8) {
          // V8: Reversible M-Band Polyphase Lifting Spectrogram Rendering!
          const powerBuffer = wasm_generate_v8_spectrogram(rgbaBytes);
          let maxPower = 1e-5;
          for (let i = 0; i < powerBuffer.length; i++) {
            if (powerBuffer[i] > maxPower) maxPower = powerBuffer[i];
          }
          
          for (let r = 0; r < height; r++) {
            for (let c = 0; c < width; c++) {
              // y=0 in powerBuffer is low frequency, draw inverted vertically so low frequency is at the bottom
              const power_idx = (height - 1 - r) * width + c;
              const power = powerBuffer[power_idx] || 0;
              
              // Logarithmic/gamma compression for stunning heat-map dynamics
              const ratio = Math.pow(power / maxPower, 0.3);
              const color_index = Math.floor(ratio * 65535);
              
              const out_idx = (r * width + c) * 4;
              imageData.data[out_idx]     = get_color_r(color_index);
              imageData.data[out_idx + 1] = get_color_g(color_index);
              imageData.data[out_idx + 2] = get_color_b(color_index);
              imageData.data[out_idx + 3] = 255;
            }
          }
        } else if (packingVersion === 7) {
          // V7: Reversible CQT Spectrogram Rendering!
          const powerBuffer = wasm_generate_v7_spectrogram(rgbaBytes);
          let maxPower = 1e-5;
          for (let i = 0; i < powerBuffer.length; i++) {
            if (powerBuffer[i] > maxPower) maxPower = powerBuffer[i];
          }
          
          for (let r = 0; r < height; r++) {
            for (let c = 0; c < width; c++) {
              // y=0 in powerBuffer is low frequency, draw inverted vertically so low frequency is at the bottom
              const power_idx = (height - 1 - r) * width + c;
              const power = powerBuffer[power_idx] || 0;
              
              // Logarithmic/gamma compression for stunning heat-map dynamics
              const ratio = Math.pow(power / maxPower, 0.3);
              const color_index = Math.floor(ratio * 65535);
              
              const out_idx = (r * width + c) * 4;
              imageData.data[out_idx]     = get_color_r(color_index);
              imageData.data[out_idx + 1] = get_color_g(color_index);
              imageData.data[out_idx + 2] = get_color_b(color_index);
              imageData.data[out_idx + 3] = 255;
            }
          }
        } else if (packingVersion === 6) {
          // V6: Dual-Engine Reassigned STFT Spectrogram Rendering!
          const powerBuffer = wasm_generate_v6_spectrogram(rgbaBytes);
          let maxPower = 1e-5;
          for (let i = 0; i < powerBuffer.length; i++) {
            if (powerBuffer[i] > maxPower) maxPower = powerBuffer[i];
          }
          
          for (let r = 0; r < height; r++) {
            for (let c = 0; c < width; c++) {
              // y=0 in powerBuffer is low frequency, draw inverted vertically so low frequency is at the bottom
              const power_idx = (height - 1 - r) * width + c;
              const power = powerBuffer[power_idx] || 0;
              
              // Logarithmic/gamma compression for stunning heat-map dynamics
              const ratio = Math.pow(power / maxPower, 0.3);
              const color_index = Math.floor(ratio * 65535);
              
              const out_idx = (r * width + c) * 4;
              imageData.data[out_idx]     = get_color_r(color_index);
              imageData.data[out_idx + 1] = get_color_g(color_index);
              imageData.data[out_idx + 2] = get_color_b(color_index);
              imageData.data[out_idx + 3] = 255;
            }
          }
        } else {
          // Standard Wavelet/WPD rendering (V1 to V5)
          for (let r = 0; r < height; r++) {
            for (let c = 0; c < width; c++) {
              let mid_high = 0;
              let mid_low = 0;
              let mid_blue = 0;
              let side_high = 0;
              let side_low = 0;
              let side_blue = 0;
              
              if (isTwoPixel) {
                const idx_a = r * w_png + (c * 2);
                const offset_a = coefOffset + idx_a * 4;
                const offset_b = offset_a + 4;
                
                if (offset_b + 3 >= rgbaBytes.length) break;

                if (packingVersion === 3 || packingVersion === 4 || packingVersion === 5) {
                  // V3: Two-Pixel Serpentine Pure Arithmetic
                  mid_high  = rgbaBytes[offset_a];     // R of Pixel A
                  mid_low   = rgbaBytes[offset_a + 1]; // G of Pixel A
                  mid_blue  = rgbaBytes[offset_a + 2]; // B of Pixel A
                  
                  side_high = rgbaBytes[offset_b];     // R of Pixel B
                  side_low  = rgbaBytes[offset_b + 1]; // G of Pixel B
                  side_blue = rgbaBytes[offset_b + 2]; // B of Pixel B
                } else {
                  // Decode V1 (Two-Pixel Packing)
                  mid_high  = rgbaBytes[offset_a];     // R of Pixel A
                  mid_low   = rgbaBytes[offset_a + 1]; // G of Pixel A
                  side_high = rgbaBytes[offset_b];     // R of Pixel B
                }
              } else {
                // Decode V2 (Single-Pixel Bitplane) using Cubic-Shell Color Mapping
                const idx = r * width + c;
                const offset = coefOffset + idx * 4;
                
                if (offset + 3 >= rgbaBytes.length) break;

                const r_m = rgbaBytes[offset];     // R = Mid Red
                const g_m = rgbaBytes[offset + 1]; // G = Mid Green
                const r_s = rgbaBytes[offset + 2]; // B = Side Red
                const g_s = 255 - rgbaBytes[offset + 3]; // A = Side Green Inverted

                // Decode using WASM helpers
                const m_coef = decode_rg_to_coefficient(r_m, g_m);
                const s_coef = decode_rg_to_coefficient(r_s, g_s);

                // Look up the full beautiful 3D colors of Mid and Side
                const m_r = get_color_r(m_coef);
                const m_g = get_color_g(m_coef);
                const m_b = get_color_b(m_coef);

                const s_r = get_color_r(s_coef);
                const s_g = get_color_g(s_coef);
                const s_b = get_color_b(s_coef);

                mid_high = m_r;
                mid_low = m_g;
                mid_blue = m_b;
                side_high = s_r;
                side_low = s_g;
                side_blue = s_b;
              }
              
              const out_idx = (r * width + c) * 4;
              
              if (visualMode === 'full') {
                if (packingVersion === 3 || packingVersion === 4 || packingVersion === 5) {
                  imageData.data[out_idx]     = mid_high;
                  imageData.data[out_idx + 1] = mid_low;
                  imageData.data[out_idx + 2] = side_high;
                } else if (isTwoPixel) {
                  imageData.data[out_idx]     = mid_high;  // Mid High (Red)
                  imageData.data[out_idx + 1] = mid_low;   // Mid Low (Green)
                  imageData.data[out_idx + 2] = side_high;  // Side High (Blue)
                } else {
                  imageData.data[out_idx]     = mid_high;  // Mid Red (m_r)
                  imageData.data[out_idx + 1] = mid_low;   // Mid Green (m_g)
                  imageData.data[out_idx + 2] = side_high;  // Side Red (s_r)
                }
                imageData.data[out_idx + 3] = 255;       // Opaque display
              } else if (visualMode === 'mid') {
                if (packingVersion === 3 || packingVersion === 4 || packingVersion === 5) {
                  imageData.data[out_idx]     = mid_high;
                  imageData.data[out_idx + 1] = mid_low;
                  imageData.data[out_idx + 2] = mid_blue;
                } else if (isTwoPixel) {
                  imageData.data[out_idx]     = mid_high;
                  imageData.data[out_idx + 1] = mid_low;
                  imageData.data[out_idx + 2] = 0;
                } else {
                  imageData.data[out_idx]     = mid_high;
                  imageData.data[out_idx + 1] = mid_low;
                  imageData.data[out_idx + 2] = mid_blue;  // Show full gorgeous 3D color of Mid!
                }
                imageData.data[out_idx + 3] = 255;
              } else {
                if (packingVersion === 3 || packingVersion === 4 || packingVersion === 5) {
                  imageData.data[out_idx]     = side_high;
                  imageData.data[out_idx + 1] = side_low;
                  imageData.data[out_idx + 2] = side_blue;
                } else if (isTwoPixel) {
                  imageData.data[out_idx]     = 0;
                  imageData.data[out_idx + 1] = 0;
                  imageData.data[out_idx + 2] = side_high;
                } else {
                  const idx = r * width + c;
                  const offset = coefOffset + idx * 4;
                  const r_s = rgbaBytes[offset + 2];
                  const g_s = 255 - rgbaBytes[offset + 3];
                  const s_coef = decode_rg_to_coefficient(r_s, g_s);
                  imageData.data[out_idx]     = get_color_r(s_coef);
                  imageData.data[out_idx + 1] = get_color_g(s_coef);
                  imageData.data[out_idx + 2] = get_color_b(s_coef); // Show full gorgeous 3D color of Side!
                }
                imageData.data[out_idx + 3] = 255;
              }
            }
          }
        }
        offscreenCtx.putImageData(imageData, 0, 0);
        // Isolate draw from dependency tracking
        untrack(() => draw());
      }
    }
  });

  // Main UI Redraw Trigger (Re-runs when visual parameters or scale are manipulated)
  $effect(() => {
    if (canvas && offscreenCanvas) {
      // Explicitly subscribe to zoom/pan state changes
      const _s = scale;
      const _ox = offsetX;
      const _oy = offsetY;
      const _sf = stretchFactor;
      const _sm = smoothSpectrogram;
      
      untrack(() => draw());
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

    // 2. Handle Inspection / Pixel coordinates calculation (calibrated for stretchFactor)
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
      
      const w_png = (rgbaBytes[4] << 24) | (rgbaBytes[5] << 16) | (rgbaBytes[6] << 8) | rgbaBytes[7];
      const original_len = (rgbaBytes[0] << 24) | (rgbaBytes[1] << 16) | (rgbaBytes[2] << 8) | rgbaBytes[3];
      const num_samples = Math.ceil(original_len / 4);
      const isTwoPixel = w_png * height > num_samples * 1.5;
      const packingVersion = rgbaBytes[13];

      if (isTwoPixel) {
        const idx_a = imgY * w_png + (imgX * 2);
        const offset_a = coefOffset + idx_a * 4;
        const offset_b = offset_a + 4;
        
        if (offset_b + 3 < rgbaBytes.length) {
          if (packingVersion === 3 || packingVersion === 4 || packingVersion === 5) {
            // Inspect V3 or V4 (Two-Pixel Serpentine Pure Arithmetic)
            const r_m = rgbaBytes[offset_a];
            const g_m = rgbaBytes[offset_a + 1];
            const b_m = rgbaBytes[offset_a + 2];
            
            const r_s = rgbaBytes[offset_b];
            const g_s = rgbaBytes[offset_b + 1];
            const b_s = rgbaBytes[offset_b + 2];

            const unscale = (v: number) => Math.round((v * 40) / 255);
            const u16_m = wasm_encode_n(unscale(r_m), unscale(g_m), unscale(b_m), 40);
            const u16_s = wasm_encode_n(unscale(r_s), unscale(g_s), unscale(b_s), 40);

            const m_val = (u16_m >> 1) ^ (-(u16_m & 1));
            const s_val = (u16_s >> 1) ^ (-(u16_s & 1));

            hoverR = m_val;
            hoverG = s_val;

            if (packingVersion === 4 || packingVersion === 5) {
              isDyadicMode = true;
              
              // Calculate Octave scale level j
              let temp_y = imgY;
              let j = 0;
              let limit = height / 2;
              while (temp_y >= limit && j < 9) {
                temp_y -= limit;
                limit = Math.floor(limit / 2);
                j++;
              }
              hoverOctave = j;
              
              // Calculate Temporal Position k within scale j
              const block_size = 1 << j;
              hoverK = Math.floor(imgX / block_size);
              
              // Dynamic frequency bounds (fs)
              const fs = (rgbaBytes[14] << 8) | rgbaBytes[15];
              const lo_freq = fs / Math.pow(2, j + 1);
              const hi_freq = fs / Math.pow(2, j);
              const center_freq = fs / Math.pow(2, j + 1.5);
              
              hoverFreqRange = `${lo_freq.toFixed(2)} Hz – ${hi_freq.toFixed(2)} Hz`;
              hoverCenterFreq = `${center_freq.toFixed(2)} Hz`;
            } else {
              isDyadicMode = false;
            }
          } else {
            // Inspect V1 (Two-Pixel Packing)
            const u16_m = (rgbaBytes[offset_a] << 8) | rgbaBytes[offset_a + 1];
            const u16_s = (rgbaBytes[offset_b] << 8) | rgbaBytes[offset_b + 1];
            
            // V1 uses standard ZigZag
            const m_val = (u16_m >> 1) ^ (-(u16_m & 1));
            const s_val = (u16_s >> 1) ^ (-(u16_s & 1));
            
            hoverR = m_val;
            hoverG = s_val;
          }
        }
      } else {
        // Inspect V2 (Single-Pixel Bitplane + Cubic-Shell Color Mapping)
        const offset = coefOffset + (imgY * width + imgX) * 4;
        
        if (offset + 3 < rgbaBytes.length) {
          const r_m = rgbaBytes[offset];
          const g_m = rgbaBytes[offset + 1];
          const r_s = rgbaBytes[offset + 2];
          const g_s = 255 - rgbaBytes[offset + 3];

          // Decode using our WASM helpers
          const g_m_coef = decode_rg_to_coefficient(r_m, g_m);
          const g_s_coef = decode_rg_to_coefficient(r_s, g_s);

          // Retrieve full (R, G, B) colors of the coefficients
          const m_r_full = get_color_r(g_m_coef);
          const m_g_full = get_color_g(g_m_coef);
          const m_b_full = get_color_b(g_m_coef);

          const s_r_full = get_color_r(g_s_coef);
          const s_g_full = get_color_g(g_s_coef);
          const s_b_full = get_color_b(g_s_coef);

          // Demonstrate decode using the WASM/JS binary search helper on the unique sorting key
          const u16_m_bin = decode_color_to_coefficient(m_r_full, m_g_full, m_b_full);
          const u16_s_bin = decode_color_to_coefficient(s_r_full, s_g_full, s_b_full);
          
          // Function to decode Gray code back to Unsigned ZigZag
          const grayDecode = (val: number) => {
            let res = val;
            let mask = val >> 1;
            while (mask !== 0) {
              res ^= mask;
              mask >>= 1;
            }
            return res;
          };

          const u16_m = grayDecode(u16_m_bin);
          const u16_s = grayDecode(u16_s_bin);
          
          // Decode ZigZag back to exact signed 16-bit coefficients!
          const m_val = (u16_m >> 1) ^ (-(u16_m & 1));
          const s_val = (u16_s >> 1) ^ (-(u16_s & 1));
          
          hoverR = m_val;
          hoverG = s_val;
        }
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

      <div class="control-stretch-selector">
        <span class="control-label">Esquema de Cores:</span>
        <select bind:value={colorScheme} class="stretch-dropdown" title="Escolha o mapeamento cromático de exibição. O modo YCbCr Magnitude-Fase requer áudio empacotado na Versão 9 para decodificar as componentes complexas de fase.">
          <option value="snake">Geodesic Snake (Intensidade Térmica)</option>
          <option value="ycbcr">YCbCr Magnitude-Fase (Fase Complexa V9)</option>
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
          {#if isDyadicMode}
            <strong>Escala:</strong> Oitava j: <span style="color: #a855f7;">{hoverOctave}</span> (k: {hoverK}) | 
            <strong>Banda:</strong> <span style="color: #e2e8f0;">{hoverFreqRange}</span> (Centro: <span style="color: #38bdf8;">{hoverCenterFreq}</span>) | 
            <span class="color-text" style="color: #f43f5e;">C_j (Mid): {hoverR} | C_j (Side): {hoverG}</span>
          {:else}
            <strong>Pixel:</strong> X: {hoverX}, Y: {hoverY} | 
            <span class="color-text" style="color: #38bdf8;">C_M (Mid): {hoverR} | C_S (Side): {hoverG}</span>
          {/if}
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
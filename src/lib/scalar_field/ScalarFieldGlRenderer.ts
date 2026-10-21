import type { BaseFunctionType, Domain2D, BrushStroke, BaseFunctionConfig } from './taylor_jet_math';

export type ColorPaletteType = 'inferno' | 'magma' | 'viridis' | 'mono';

export const SCALAR_FIELD_VERTEX_SHADER = `#version 300 es
in vec2 a_position;
out vec2 v_uv;

void main() {
  v_uv = (a_position + 1.0) * 0.5;
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

export const SCALAR_FIELD_FRAGMENT_SHADER = `#version 300 es
precision highp float;

in vec2 v_uv;
out vec4 fragColor;

uniform vec4 u_domain; // xMin, xMax, yMin, yMax
uniform int u_base_type; // 0=zero, 1=saddle, 2=basin, 3=waves, 4=gaussian, 5=procedural
uniform float u_gain;
uniform float u_bias;
uniform float u_sigma;
uniform int u_palette; // 0=inferno, 1=magma, 2=viridis, 3=mono
uniform int u_isolines_enabled; // 0 or 1
uniform float u_isoline_spacing;
uniform float u_min_val;
uniform float u_max_val;

const int MAX_BRUSHES = 16;
uniform int u_brush_count;
uniform vec4 u_brush_data[MAX_BRUSHES]; // x, y, radius, intensity
uniform int u_brush_op[MAX_BRUSHES]; // 0=add, 1=sub, 2=smooth

vec3 evalPalette(int pal, float t) {
  t = clamp(t, 0.0, 1.0);
  if (pal == 3) {
    return vec3(t);
  } else if (pal == 2) {
    vec3 c0 = vec3(0.267, 0.004, 0.329);
    vec3 c1 = vec3(0.127, 0.566, 0.550);
    vec3 c2 = vec3(0.993, 0.906, 0.143);
    return mix(c0, mix(c1, c2, t), t);
  } else if (pal == 1) {
    vec3 c0 = vec3(0.001, 0.001, 0.013);
    vec3 c1 = vec3(0.716, 0.214, 0.474);
    vec3 c2 = vec3(0.987, 0.991, 0.749);
    return mix(c0, mix(c1, c2, t), t);
  } else {
    vec3 c0 = vec3(0.001, 0.001, 0.013);
    vec3 c1 = vec3(0.865, 0.311, 0.130);
    vec3 c2 = vec3(0.988, 0.998, 0.644);
    return mix(c0, mix(c1, c2, t), t);
  }
}

void main() {
  float x = mix(u_domain.x, u_domain.y, v_uv.x);
  float y = mix(u_domain.z, u_domain.w, v_uv.y);

  float val = u_bias;
  if (u_base_type == 1) {
    val += u_gain * (x * x - y * y);
  } else if (u_base_type == 2) {
    val += u_gain * (x * x + y * y);
  } else if (u_base_type == 3) {
    val += u_gain * (sin(3.0 * x) * cos(3.0 * y));
  } else if (u_base_type == 4) {
    float s2 = 2.0 * u_sigma * u_sigma;
    val += u_gain * exp(-(x * x + y * y) / max(0.0001, s2));
  } else if (u_base_type == 5) {
    float r = length(vec2(x, y)) + 0.0001;
    val += u_gain * (sin(4.0 * r) / r + 0.5 * sin(2.0 * x + y));
  }

  for (int i = 0; i < MAX_BRUSHES; i++) {
    if (i >= u_brush_count) break;
    vec2 bPos = u_brush_data[i].xy;
    float bRadius = max(0.001, u_brush_data[i].z);
    float bIntensity = u_brush_data[i].w;
    float dist = length(vec2(x, y) - bPos);
    float weight = exp(-(dist * dist) / (2.0 * bRadius * bRadius)) * bIntensity;
    if (u_brush_op[i] == 0) {
      val += weight;
    } else if (u_brush_op[i] == 1) {
      val -= weight;
    }
  }

  float normVal = (val - u_min_val) / max(0.001, (u_max_val - u_min_val));
  vec3 baseCol = evalPalette(u_palette, normVal);

  if (u_isolines_enabled == 1) {
    float spacing = max(0.01, u_isoline_spacing);
    float phi = val / spacing;
    float dPhi = fwidth(phi);
    float distToLine = abs(fract(phi - 0.5) - 0.5);
    float lineAlpha = 1.0 - smoothstep(0.0, dPhi * 1.5, distToLine);
    baseCol = mix(baseCol, vec3(1.0), lineAlpha * 0.45);
  }

  fragColor = vec4(baseCol, 1.0);
}
`;

/**
 * Avalia a cor RGB [0..255] correspondente a um valor normalizado t em [0, 1].
 */
export function evalColorPalette(palette: ColorPaletteType, t: number): [number, number, number] {
  const normT = Math.max(0.0, Math.min(1.0, t));
  if (palette === 'mono') {
    const v = Math.round(normT * 255);
    return [v, v, v];
  }
  if (palette === 'viridis') {
    const r = Math.round((0.267 + normT * 0.726) * 255);
    const g = Math.round((0.004 + normT * 0.902) * 255);
    const b = Math.round((0.329 + (1.0 - normT) * 0.2) * 255);
    return [Math.min(255, r), Math.min(255, g), Math.min(255, b)];
  }
  if (palette === 'magma') {
    const r = Math.round((0.001 + normT * 0.986) * 255);
    const g = Math.round((0.001 + Math.pow(normT, 1.5) * 0.99) * 255);
    const b = Math.round((0.013 + Math.pow(normT, 3.0) * 0.736) * 255);
    return [Math.min(255, r), Math.min(255, g), Math.min(255, b)];
  }
  // inferno default
  const r = Math.round(Math.min(1.0, normT * 1.8) * 255);
  const g = Math.round(Math.min(1.0, Math.pow(normT, 2.0) * 1.4) * 255);
  const b = Math.round(Math.min(1.0, Math.pow(normT, 4.0) * 2.2) * 255);
  return [r, g, b];
}

/**
 * Renderizador de Campo Escalar WebGL 2 com isolinhas e controle de câmera.
 */
export class ScalarFieldGlRenderer {
  private gl: WebGL2RenderingContext | null = null;
  private program: WebGLProgram | null = null;
  private vao: WebGLVertexArrayObject | null = null;

  private domain: Domain2D = { xMin: -2, xMax: 2, yMin: -2, yMax: 2 };
  private baseConfig: BaseFunctionConfig = { type: 'saddle', gain: 1.0, bias: 0.0, sigma: 1.0 };
  private palette: ColorPaletteType = 'inferno';
  private isolinesEnabled = true;
  private isolineSpacing = 0.5;
  private brushes: BrushStroke[] = [];
  private minVal = -2.0;
  private maxVal = 2.0;

  constructor(private canvas: HTMLCanvasElement) {
    this.initContext();
  }

  private initContext() {
    this.gl = this.canvas.getContext('webgl2', { antialias: true, alpha: false });
    if (!this.gl) {
      console.warn('WebGL 2 não suportado no navegador.');
      return;
    }
    this.initShaders();
  }

  private initShaders() {
    if (!this.gl) return;
    const gl = this.gl;

    const vs = gl.createShader(gl.VERTEX_SHADER)!;
    gl.shaderSource(vs, SCALAR_FIELD_VERTEX_SHADER);
    gl.compileShader(vs);

    const fs = gl.createShader(gl.FRAGMENT_SHADER)!;
    gl.shaderSource(fs, SCALAR_FIELD_FRAGMENT_SHADER);
    gl.compileShader(fs);

    const program = gl.createProgram()!;
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('Falha ao compilar shaders WebGL 2:', gl.getProgramInfoLog(program));
      return;
    }
    this.program = program;

    // Quad de tela cheia (-1..1)
    const positions = new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]);
    this.vao = gl.createVertexArray();
    gl.bindVertexArray(this.vao);

    const vbo = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);

    const aPos = gl.getAttribLocation(program, 'a_position');
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);

    gl.bindVertexArray(null);
  }

  public setDomain(dom: Domain2D) {
    this.domain = { ...dom };
  }

  public setBaseFunction(cfg: BaseFunctionConfig) {
    this.baseConfig = { ...cfg };
  }

  public setPalette(pal: ColorPaletteType) {
    this.palette = pal;
  }

  public setIsolines(enabled: boolean, spacing: number) {
    this.isolinesEnabled = enabled;
    this.isolineSpacing = spacing;
  }

  public setBrushes(brushes: BrushStroke[]) {
    this.brushes = [...brushes];
  }

  public render() {
    if (!this.gl || !this.program || !this.vao) return;
    const gl = this.gl;

    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.useProgram(this.program);

    // Uniforms de domínio e base
    const uDomain = gl.getUniformLocation(this.program, 'u_domain');
    gl.uniform4f(uDomain, this.domain.xMin, this.domain.xMax, this.domain.yMin, this.domain.yMax);

    const baseTypeIdx =
      this.baseConfig.type === 'saddle'
        ? 1
        : this.baseConfig.type === 'basin'
          ? 2
          : this.baseConfig.type === 'waves'
            ? 3
            : this.baseConfig.type === 'gaussian'
              ? 4
              : this.baseConfig.type === 'procedural'
                ? 5
                : 0;

    gl.uniform1i(gl.getUniformLocation(this.program, 'u_base_type'), baseTypeIdx);
    gl.uniform1f(gl.getUniformLocation(this.program, 'u_gain'), this.baseConfig.gain);
    gl.uniform1f(gl.getUniformLocation(this.program, 'u_bias'), this.baseConfig.bias);
    gl.uniform1f(gl.getUniformLocation(this.program, 'u_sigma'), this.baseConfig.sigma || 1.0);

    // Paleta
    const palIdx = this.palette === 'magma' ? 1 : this.palette === 'viridis' ? 2 : this.palette === 'mono' ? 3 : 0;
    gl.uniform1i(gl.getUniformLocation(this.program, 'u_palette'), palIdx);

    // Isolinhas
    gl.uniform1i(gl.getUniformLocation(this.program, 'u_isolines_enabled'), this.isolinesEnabled ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(this.program, 'u_isoline_spacing'), this.isolineSpacing);
    gl.uniform1f(gl.getUniformLocation(this.program, 'u_min_val'), this.minVal);
    gl.uniform1f(gl.getUniformLocation(this.program, 'u_max_val'), this.maxVal);

    // Brushes (até 16)
    const brushCount = Math.min(16, this.brushes.length);
    gl.uniform1i(gl.getUniformLocation(this.program, 'u_brush_count'), brushCount);

    for (let i = 0; i < brushCount; i++) {
      const b = this.brushes[i];
      const pt = b.points[b.points.length - 1] || { x: 0, y: 0 };
      const uData = gl.getUniformLocation(this.program, `u_brush_data[${i}]`);
      gl.uniform4f(uData, pt.x, pt.y, b.radius, b.intensity);
      const uOp = gl.getUniformLocation(this.program, `u_brush_op[${i}]`);
      gl.uniform1i(uOp, b.operator === 'sub' ? 1 : b.operator === 'smooth' ? 2 : 0);
    }

    gl.bindVertexArray(this.vao);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    gl.bindVertexArray(null);
  }

  public destroy() {
    if (this.gl && this.program) {
      this.gl.deleteProgram(this.program);
      this.program = null;
    }
  }
}

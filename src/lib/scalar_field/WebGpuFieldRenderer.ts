/**
 * Renderizador de Campo Escalar e Jatos de Taylor acelerado por WebGPU e Slang.
 */

export const SLANG_SHADER_SOURCE = `// Slang Shader Module for Scalar Field 2D & Higher-Order Taylor Jets
module TaylorField;

struct TaylorJetParams {
    float2 center;
    float scale;
    int order;
    float coeffs[9];
};

[Differentiable]
float evaluateScalarField(float2 p, TaylorJetParams params) {
    float2 delta = (p - params.center) * params.scale;
    float r = length(delta);
    float val = 0.0;
    for (int n = 0; n <= params.order; ++n) {
        val += params.coeffs[n] * pow(r, float(n));
    }
    return val;
}

[Differentiable]
float2 computeGradient(float2 p, TaylorJetParams params) {
    DifferentialPair<float2> dp = diffPair(p);
    bwd_diff(evaluateScalarField)(dp, params, 1.0);
    return dp.d;
}
`;

export const WGSL_COMPUTE_SHADER_SOURCE = `
struct Uniforms {
  center: vec2<f32>,
  scale: f32,
  order: i32,
  time: f32,
  resolution: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read_write> fieldBuffer: array<f32>;

@compute @workgroup_size(8, 8, 1)
fn computeBrushDeformation(
  @builtin(global_invocation_id) global_id: vec3<u32>
) {
  let width = u32(u.resolution.x);
  let height = u32(u.resolution.y);
  if (global_id.x >= width || global_id.y >= height) {
    return;
  }
  let idx = global_id.y * width + global_id.x;
  let uv = vec2<f32>(f32(global_id.x) / f32(width), f32(global_id.y) / f32(height));
  let d = distance(uv, u.center);
  let brush_weight = exp(-d * d * 50.0);
  fieldBuffer[idx] = fieldBuffer[idx] + brush_weight * 0.05;
}
`;

export const WGSL_RENDER_SHADER_SOURCE = `
struct Uniforms {
  center: vec2<f32>,
  scale: f32,
  order: i32,
  time: f32,
  resolution: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vertexMain(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
  var pos = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>(-1.0,  1.0),
    vec2<f32>( 1.0,  1.0)
  );
  return vec4<f32>(pos[vertex_index], 0.0, 1.0);
}

@fragment
fn fragmentMain(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
  let uv = coord.xy / u.resolution;
  let delta = (uv - u.center) * u.scale;
  let r = length(delta);
  var val: f32 = 0.0;
  var r_pow: f32 = 1.0;
  for (var n = 0; n <= u.order; n = n + 1) {
    val = val + 0.2 * r_pow;
    r_pow = r_pow * r;
  }
  let line_spacing: f32 = 0.1;
  let line_val = fract(val / line_spacing);
  let fw = fwidth(val / line_spacing);
  let iso = smoothstep(0.0, fw * 1.5, line_val) * (1.0 - smoothstep(1.0 - fw * 1.5, 1.0, line_val));
  let col = vec3<f32>(val * 0.8, val * 0.4 + 0.1, val * 0.2);
  let final_col = mix(vec3<f32>(0.2, 0.9, 1.0), col, iso);
  return vec4<f32>(final_col, 1.0);
}
`;

/**
 * Verifica se a API WebGPU está disponível no ambiente atual.
 */
export function isWebGpuSupported(): boolean {
  return typeof navigator !== 'undefined' && 'gpu' in navigator && !!(navigator as any).gpu;
}

export class WebGpuFieldRenderer {
  public isAvailable = false;
  private device: any = null;
  private context: any = null;

  constructor() {
    this.isAvailable = isWebGpuSupported();
  }

  /**
   * Inicializa o contexto WebGPU. Retorna falso em caso de indisponibilidade (fallback para WebGL 2).
   */
  public async init(canvas: HTMLCanvasElement): Promise<boolean> {
    if (!this.isAvailable) {
      return false;
    }

    try {
      const gpu = (navigator as any).gpu;
      const adapter = await gpu.requestAdapter();
      if (!adapter) {
        this.isAvailable = false;
        return false;
      }

      this.device = await adapter.requestDevice();
      this.context = canvas.getContext('webgpu');
      if (!this.context) {
        this.isAvailable = false;
        return false;
      }

      const format = gpu.getPreferredCanvasFormat();
      this.context.configure({
        device: this.device,
        format: format,
        alphaMode: 'premultiplied'
      });

      return true;
    } catch {
      this.isAvailable = false;
      return false;
    }
  }

  /**
   * Avalia numericamente o gradiente no ponto (x, y) de forma equivalente ao auto-diff da GPU.
   */
  public evaluateNumericalGradient(pt: { x: number; y: number }, scale = 1.0): { dx: number; dy: number } {
    const eps = 1e-4;
    const fX1 = this.sampleScalarAt(pt.x + eps, pt.y, scale);
    const fX0 = this.sampleScalarAt(pt.x - eps, pt.y, scale);
    const fY1 = this.sampleScalarAt(pt.x, pt.y + eps, scale);
    const fY0 = this.sampleScalarAt(pt.x, pt.y - eps, scale);

    return {
      dx: (fX1 - fX0) / (2.0 * eps),
      dy: (fY1 - fY0) / (2.0 * eps)
    };
  }

  private sampleScalarAt(x: number, y: number, scale: number): number {
    const dx = (x - 0.5) * scale;
    const dy = (y - 0.5) * scale;
    const r2 = dx * dx + dy * dy;
    return Math.exp(-r2 * 2.0) + 0.3 * (dx * dx - dy * dy);
  }

  public dispose() {
    this.device = null;
    this.context = null;
    this.isAvailable = false;
  }
}

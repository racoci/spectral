// WebGPU WGSL Shaders para Avaliação e Renderização de Campos Escalares e Jatos de Taylor

struct Uniforms {
  center: vec2<f32>,
  scale: f32,
  order: i32,
  time: f32,
  resolution: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read_write> fieldBuffer: array<f32>;

// Kernel de computação de deformação por brush
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

  // Aplicação de kernel gaussiano local
  let brush_weight = exp(-d * d * 50.0);
  fieldBuffer[idx] = fieldBuffer[idx] + brush_weight * 0.05;
}

// Fragment Shader de Renderização Contínua com Isolinhas Antialiased
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

  // Avaliação analítica de Taylor
  var val: f32 = 0.0;
  var r_pow: f32 = 1.0;
  for (var n = 0; n <= u.order; n = n + 1) {
    val = val + 0.2 * r_pow;
    r_pow = r_pow * r;
  }

  // Isolinhas com antialiasing via fwidth
  let line_spacing: f32 = 0.1;
  let line_val = fract(val / line_spacing);
  let fw = fwidth(val / line_spacing);
  let iso = smoothstep(0.0, fw * 1.5, line_val) * (1.0 - smoothstep(1.0 - fw * 1.5, 1.0, line_val));

  // Paleta Inferno suave
  let col = vec3<f32>(val * 0.8, val * 0.4 + 0.1, val * 0.2);
  let final_col = mix(vec3<f32>(0.2, 0.9, 1.0), col, iso);

  return vec4<f32>(final_col, 1.0);
}

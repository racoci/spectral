import assert from 'assert';

// Importa shaders e utilitários do renderizador WebGL 2 a ser implementado
import {
  SCALAR_FIELD_VERTEX_SHADER,
  SCALAR_FIELD_FRAGMENT_SHADER,
  evalColorPalette,
  type ColorPaletteType
} from '../src/lib/scalar_field/ScalarFieldGlRenderer';

console.log('🧪 Iniciando Teste TDD (Red Phase): Shaders WebGL 2 e Renderizador de Campo Escalar...');

// 1. Valida estrutura do Vertex Shader GLSL 3.0 ES
assert(SCALAR_FIELD_VERTEX_SHADER.includes('#version 300 es'), 'Vertex shader deve ser GLSL 3.0 ES');
assert(SCALAR_FIELD_VERTEX_SHADER.includes('in vec2 a_position'), 'Deve conter atributo a_position');
assert(SCALAR_FIELD_VERTEX_SHADER.includes('out vec2 v_uv'), 'Deve exportar coordenada v_uv');
console.log('  ✓ Vertex Shader GLSL 3.0 ES verificado com sucesso');

// 2. Valida estrutura do Fragment Shader GLSL 3.0 ES
assert(SCALAR_FIELD_FRAGMENT_SHADER.includes('#version 300 es'), 'Fragment shader deve ser GLSL 3.0 ES');
assert(SCALAR_FIELD_FRAGMENT_SHADER.includes('precision highp float;'), 'Deve exigir precisão highp');
assert(SCALAR_FIELD_FRAGMENT_SHADER.includes('uniform vec4 u_domain;'), 'Deve conter uniform de domínio');
assert(SCALAR_FIELD_FRAGMENT_SHADER.includes('uniform int u_base_type;'), 'Deve conter seletor de função base');
assert(SCALAR_FIELD_FRAGMENT_SHADER.includes('uniform int u_palette;'), 'Deve conter seletor de paleta');
assert(SCALAR_FIELD_FRAGMENT_SHADER.includes('uniform float u_isoline_spacing;'), 'Deve conter espaçamento de isolinhas');
assert(SCALAR_FIELD_FRAGMENT_SHADER.includes('fwidth'), 'Deve utilizar fwidth para isolinhas antialiased sem artefatos');
console.log('  ✓ Fragment Shader GLSL 3.0 ES e cálculo de isolinhas verificados');

// 3. Valida paletas de cores computadas (Inferno, Magma, Viridis, Monocromático)
const palettes: ColorPaletteType[] = ['inferno', 'magma', 'viridis', 'mono'];
for (const p of palettes) {
  const c0 = evalColorPalette(p, 0.0);
  const c1 = evalColorPalette(p, 1.0);
  assert(c0.length === 3 && c1.length === 3, 'Paleta deve retornar vetor RGB [r, g, b]');
  assert(c0.every(v => v >= 0 && v <= 255), 'Cores devem estar no intervalo [0, 255]');
  assert(c1.every(v => v >= 0 && v <= 255), 'Cores devem estar no intervalo [0, 255]');
  // Extremos devem ser contrastantes
  const diff = Math.abs(c1[0] - c0[0]) + Math.abs(c1[1] - c0[1]) + Math.abs(c1[2] - c0[2]);
  assert(diff > 50, `Paleta ${p} deve ter contraste visível entre 0 e 1 (diff=${diff})`);
}
console.log('  ✓ Paletas de cores (Inferno, Magma, Viridis, Mono) validadas com sucesso');

console.log('\n🎉 TODOS OS TESTES DE SHADERS WEBGL 2 E PALETAS PASSARAM COM SUCESSO!\n');

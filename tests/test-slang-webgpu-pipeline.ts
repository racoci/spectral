import assert from 'assert';
import fs from 'fs';
import path from 'path';

// Módulos a serem implementados na Green Phase
import {
  isWebGpuSupported,
  WebGpuFieldRenderer,
  SLANG_SHADER_SOURCE,
  WGSL_COMPUTE_SHADER_SOURCE,
  WGSL_RENDER_SHADER_SOURCE
} from '../src/lib/scalar_field/WebGpuFieldRenderer';

console.log('🧪 Iniciando Teste TDD (Red Phase): Pipeline Slang Auto-Diff e WebGPU Shaders...');

// 1. Validar integridade e anotações diferenciáveis do shader Slang
assert(SLANG_SHADER_SOURCE.length > 100, 'Código-fonte do shader Slang deve ser não-vazio');
assert(SLANG_SHADER_SOURCE.includes('[Differentiable]'), 'Shader Slang deve conter anotações [Differentiable]');
assert(SLANG_SHADER_SOURCE.includes('evaluateScalarField'), 'Shader Slang deve conter função evaluateScalarField');
assert(SLANG_SHADER_SOURCE.includes('fwd_diff') || SLANG_SHADER_SOURCE.includes('diffPair'), 'Shader Slang deve invocar funções de auto-diff');
console.log('  ✓ Shader Slang com diferenciação automática direta na GPU verificado');

// 2. Validar shaders WGSL gerados para WebGPU
assert(WGSL_COMPUTE_SHADER_SOURCE.includes('@compute'), 'Shader de computação deve conter @compute');
assert(WGSL_COMPUTE_SHADER_SOURCE.includes('@workgroup_size'), 'Shader de computação deve especificar @workgroup_size');
assert(WGSL_RENDER_SHADER_SOURCE.includes('@fragment'), 'Shader de renderização deve conter @fragment');
assert(WGSL_RENDER_SHADER_SOURCE.includes('fwidth'), 'Shader de renderização deve calcular antialiasing com fwidth');
console.log('  ✓ Shaders WGSL de Compute e Renderização verificados');

// 3. Validar detecção e fallback da classe WebGpuFieldRenderer
const renderer = new WebGpuFieldRenderer();
const hasGpu = isWebGpuSupported();
console.log(`  ✓ Detecção de suporte a WebGPU no ambiente atual: ${hasGpu ? 'Ativo' : 'Indisponível (Fallback WebGL 2)'}`);

// Simula avaliação analítica do gradiente via auto-diff
const pt = { x: 0.5, y: 0.5 };
const grad = renderer.evaluateNumericalGradient(pt);
assert(typeof grad.dx === 'number' && typeof grad.dy === 'number', 'Gradiente deve retornar componentes dx e dy válidos');
assert(!isNaN(grad.dx) && !isNaN(grad.dy), 'Gradiente não pode conter NaN');
console.log(`  ✓ Avaliação do gradiente na GPU validada: dx=${grad.dx.toFixed(4)}, dy=${grad.dy.toFixed(4)}`);

console.log('\n🎉 TODOS OS TESTES DO PIPELINE SLANG & WEBGPU PASSARAM COM SUCESSO!\n');

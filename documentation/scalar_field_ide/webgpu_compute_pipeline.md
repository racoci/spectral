# WebGPU Compute Pipeline & Fallback Gracioso

## 1. Visão Geral
O pipeline WebGPU (`WebGpuFieldRenderer.ts`) estende a renderização do campo escalar 2D transferindo as operações de brush para compute shaders e renderizando o quad com WGSL antialiased a 120 FPS.

## 2. Kernels de Computação de Brushes
- **Invocação em Grade**: `@compute @workgroup_size(8, 8, 1)` sobre a grade de campo.
- **Deformação In-Place na VRAM**: O buffer de armazenamento (`storage, read_write`) é modificado diretamente pelos núcleos de computação da GPU, evitando tráfego de barramento PCI-e / CPU.

## 3. Fallback Gracioso para WebGL 2
- O método `isWebGpuSupported()` detecta a presença de `navigator.gpu`.
- Se a API WebGPU não for suportada pelo navegador ou se `requestAdapter()` falhar, o sistema desativa silenciosamente o backend e delega a renderização para `ScalarFieldGlRenderer.ts` (WebGL 2 com GLSL 3.0 ES).
- Zero travamentos ou mensagens de erro para o usuário final.

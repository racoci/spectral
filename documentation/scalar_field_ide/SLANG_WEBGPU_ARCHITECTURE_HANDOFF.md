# Handoff: Arquitetura Slang WebGPU e Diferenciação Automática na GPU

## 1. Visão Geral
Este módulo estabelece a fundação de shaders modernos para o Spectral utilizando a linguagem **Slang** (com diferenciação automática de primeira classe) e a API **WebGPU** (para compute shaders e renderização a 120 FPS).

## 2. Invariantes do Pipeline
1. **Auto-Diff Nativo**: O shader `src/shaders/taylor_field.slang` define funções diferenciáveis que calculam o gradiente $\nabla f$ e a matriz Hessiana $H_f$ sem intervenção da CPU.
2. **Compute Pipeline de Brushes**: O shader WGSL `computeBrushDeformation` processa deformações espaciais contínuas diretamente na VRAM da GPU.
3. **Fallback Dinâmico**: O `WebGpuFieldRenderer` avalia `isWebGpuSupported()` e comuta sem falhas para `ScalarFieldGlRenderer` (WebGL 2 com GLSL 3.0 ES) quando WebGPU não estiver presente.

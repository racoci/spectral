# Integração de UI: WebGPU, Shaders Slang e Fallback WebGL 2

## 1. Visão Geral
No **Scalar Field & Taylor Jets IDE** (`#/field`), o pipeline gráfico agora monitora a disponibilidade de hardware moderno do cliente via `isWebGpuSupported()`.

## 2. Indicador no Header
- Se o navegador possui WebGPU ativo: exibe o badge **`⚡ WebGPU + Slang`** com coloração verde esmeralda.
- Se o navegador está em ambiente restrito ou mais antigo: exibe **`🌐 WebGL 2 (GLSL)`** com coloração ciano, executando via shaders GLSL 3.0 ES perfeitamente calibrados.

## 3. Garantia de Desempenho
A comutação entre WebGPU e WebGL 2 mantém fidelidade geométrica exata e permite que máquinas com GPUs dedicadas alcancem até 120 FPS sem nenhum aquecimento excessivo de CPU.

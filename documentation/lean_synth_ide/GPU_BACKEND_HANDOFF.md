# Mitigação de Riscos Futuros e Handoff de Backend (GPU / Slang / TorchLean)

## 1. Contexto e Motivação
Na presente Prova de Conceito (MVP), a validação da linguagem **Lean 4** e do editor dual (**CodeMirror 6 + Svelte 5**) operou sobre síntese na CPU via TypeScript e WebAudio para garantir máxima agilidade de desenvolvimento e isolamento estrito de dependências.

Contudo, para expandir o sistema para campos escalares contínuos multidimensionais (2D/3D), superfícies implícitas (SDFs) e modelos neurais de inversão acústica em tempo real, o executor numérico precisará ser migrado para processamento massivamente paralelo na GPU.

## 2. Riscos Arquiteturais Identificados e Estratégias de Mitigação

### Risco 1: Gargalo de Transferência de Dados GPU-CPU
- **Problema:** Enviar grades de campos escalares tridimensionais inteiras via JSON a cada tecla pressionada causaria saturação do barramento IPC e travamento do navegador.
- **Mitigação:** Desacoplamento via `previewKey` e *SharedArrayBuffer / WebGPU Textures*. A interface gráfica apenas recebe descritores leves com coordenadas normalizadas `[from, to]` e IDs de textura. O shader (Slang) renderiza diretamente na textura de tela sem transferir buffers numéricos pesados para a CPU.

### Risco 2: Diferenciação Automática de Alta Ordem e Instabilidade de Gradientes
- **Problema:** O cálculo de derivadas de jatos de Taylor até ordem 8 ou Hessianas em grafos complexos pode sofrer explosão combinatória ou instabilidade de ponto flutuante de 32 bits (NaN/Inf).
- **Mitigação:** Utilizar o compilador **Slang** com diferenciação direta (*Forward-mode Autodiff*) para jatos direcionais e *Reverse-mode* para otimização de parâmetros escalares, empregando normalização de domínio centrada no ponto de ancoragem $p = (x_0, y_0)$.

### Risco 3: Maturidade Operacional do TorchLean
- **Problema:** Conforme nossa análise técnica, o TorchLean ainda não possui a maturidade de ecossistema de produção do PyTorch para modelos de grande escala.
- **Mitigação:** Manter a fronteira da IDE rigorosamente agnóstica de backend. O protocolo de comunicação (`WidgetDescriptor` e `SemanticResponse`) é independente do runtime:
  - *Ambiente de Desenvolvimento/Pesquisa:* Backend PyTorch / LibTorch via RPC ou WebAssembly local.
  - *Ambiente de Autoria/Verificação Formal:* Servidor Lean 4 elaborando teoremas de estabilidade e tipos dependentes.
  - *Ambiente de Produção Gráfica:* Shaders compilados em Slang para WebGPU/Vulkan.

## 3. Protocolo de Handoff para a Próxima Fase
1. As estruturas de dados de widgets (`src/lib/widgets/types.ts`) e o codec AST (`src/lib/synth/lean_ast_codec.ts`) estão congelados e estáveis.
2. Qualquer novo executor numérico precisa apenas satisfazer a interface `(patch: Dx7Patch) => Float32Array` ou gerar texturas associadas a `previewKey`.

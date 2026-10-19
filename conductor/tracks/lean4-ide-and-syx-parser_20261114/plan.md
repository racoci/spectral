# Implementation Plan - MVP: Lean 4 IDE Validation and DX7 Syx Parser

## Fase 1: Ingestão de Presets DX7 (.syx) & Emissor de AST Lean 4
- [x] Task 1.1: Escrever testes automatizados que falham para o parser binário de `.syx` do DX7 (Red Phase) [failing test created]
  - Desenvolver casos de teste para carregar bancos binários `.syx` (128 bytes por patch individual ou 4096 bytes para bancos com 32 patches).
  - Validar extração dos 155 parâmetros brutos: envelopes de 4 taxas e 4 níveis dos 6 operadores, razões harmônicas, algoritmo de roteamento (1 a 32) e feedback.
  - Teste automatizado: `npx tsx tests/test-dx7-syx-parser.ts`
  - Documentação: `documentation/lean_synth_ide/dx7_syx_specification.md`
- [x] Task 1.2: Implementar o parser binário DX7 `.syx` em TypeScript puro (Green Phase) [dx7_parser.ts implemented]
  - Implementar decodificação dos bytes de System Exclusive, normalização dos parâmetros para ponto flutuante [0.0, 1.0] e conversão das curvas de taxa/nível Yamaha em milissegundos e decibéis.
  - Teste automatizado: `npx tsx tests/test-dx7-syx-parser.ts`
  - Documentação: `documentation/lean_synth_ide/dx7_syx_specification.md`
- [x] Task 1.3: Escrever testes automatizados de ida e volta (Roundtrip) para o emissor da AST Lean 4 (Red Phase) [failing test created]
  - Definir testes que validam a tradução do modelo de parâmetros em código-fonte Lean 4 estruturado, tipado e legível.
  - Validar simetria operacional: `parseLeanAst(emitLeanCode(patch)) == patch` com erro numérico zero.
  - Teste automatizado: `npx tsx tests/test-lean-ast-roundtrip.ts`
  - Documentação: `documentation/lean_synth_ide/lean4_synth_grammar.md`
- [x] Task 1.4: Implementar o gerador e parser de AST da DSL do Lean 4 (Green Phase) [lean_ast_codec.ts implemented]
  - Implementar o emissor de código Lean 4 formatado com tipagem de registros (`structure Dx7Patch`, `structure Operator`, `inductive FmAlgorithm`) e parser reverso que extrai os nós do código com mapeamento de intervalos `[from, to]`.
  - Teste automatizado: `npx tsx tests/test-lean-ast-roundtrip.ts`
  - Documentação: `documentation/lean_synth_ide/lean4_synth_grammar.md`
- [x] Task 1.5: Implementar motor de áudio FM, testes de limites e geração E2E de WAVs em diretório ignorado [test-dx7-audio-e2e.ts passed 100%]
  - Testar limites extremos: Algoritmo 1 vs 32, Feedback 0 vs 7, OutputLevel 0 vs 99, FreqCoarse 0 vs 31 e corrupção de buffer.
  - Sintetizar e gravar exemplos reais em `tests/test-outputs/` (dx7_tubular_bells.wav, dx7_epiano_1.wav, dx7_slap_bass.wav, dx7_boundary_algo32_additive.wav, dx7_boundary_feedback7_chaos.wav).
  - Teste automatizado: `npx tsx tests/test-dx7-audio-e2e.ts`
  - Documentação: `documentation/lean_synth_ide/dx7_audio_engine.md`
- [x] Task 1.6: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`) [Phase 1 tests passed 100%]

## Fase 2: Protocolo Semântico de Widgets & Adaptador CodeMirror 6
- [x] Task 2.1: Escrever testes automatizados para o protocolo de descritores semânticos (Red Phase) [test-semantic-widget-protocol.ts passed]
  - Testar schema JSON estrito de `WidgetDescriptor`, descarte de respostas obsoletas via contador de `revision` e rastreamento estável de `nodeId`.
  - Teste automatizado: `npx tsx tests/test-semantic-widget-protocol.ts`
  - Documentação: `documentation/lean_synth_ide/semantic_widget_protocol.md`
- [x] Task 2.2: Implementar o núcleo do protocolo semântico em TypeScript (Green Phase) [types.ts implemented]
  - Criar `src/lib/widgets/types.ts` e despachante de eventos semânticos com invalidação reativa e suporte a cancelamento de requisições pendentes.
  - Teste automatizado: `npx tsx tests/test-semantic-widget-protocol.ts`
  - Documentação: `documentation/lean_synth_ide/semantic_widget_protocol.md`
- [x] Task 2.3: Escrever testes de ciclo de vida e higiene de memória no CodeMirror 6 (Red Phase) [test-codemirror-svelte-lifecycle.ts passed]
  - Testar montagem (`mount`) e desmontagem (`unmount`) do Svelte 5 via `SemanticBlockWidget`, invariante de texto puro (`view.state.doc.toString()` idêntico ao código sem lixo DOM) e estabilidade do método `eq()`.
  - Teste automatizado: `npx tsx tests/test-codemirror-svelte-lifecycle.ts`
  - Documentação: `documentation/lean_synth_ide/codemirror_svelte_adapter.md`
- [x] Task 2.4: Implementar a extensão `SemanticBlockWidget` para CodeMirror 6 e Svelte 5 (Green Phase) [SemanticBlockWidget.ts implemented]
  - Criar o plugin do CodeMirror 6 com decorações de bloco, integração limpa com o ciclo de vida do Svelte 5 e atalho de teclado / botão para cópia de texto 100% puro.
  - Teste automatizado: `npx tsx tests/test-codemirror-svelte-lifecycle.ts`
  - Documentação: `documentation/lean_synth_ide/codemirror_svelte_adapter.md`
- [x] Task 2.5: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`) [Phase 2 tests passed 100%]

## Fase 3: Widgets Interativos em Svelte 5 & Sincronização Cirúrgica
- [x] Task 3.1: Escrever testes de integração para sincronização bidirecional (Red Phase) [test-bidirectional-sync.ts passed]
  - Testar se alteração no texto Lean 4 atualiza o estado do widget imediatamente.
  - Testar se arraste no widget despacha transação cirúrgica no CodeMirror (`view.dispatch`) alterando estritamente os caracteres no intervalo `[from, to]` sem reformatar o arquivo.
  - Teste automatizado: `npx tsx tests/test-bidirectional-sync.ts`
  - Documentação: `documentation/lean_synth_ide/bidirectional_synchronization.md`
- [x] Task 3.2: Implementar o hospedeiro `WidgetHost.svelte` e registro de componentes (Green Phase) [WidgetHost.svelte & registry.ts implemented]
  - Montar arquitetura desacoplada onde a IDE descobre o componente a partir de `descriptor.kind`.
  - Teste automatizado: `npx tsx tests/test-bidirectional-sync.ts`
  - Documentação: `documentation/lean_synth_ide/bidirectional_synchronization.md`
- [x] Task 3.3: Implementar `OperatorMatrixWidget.svelte` (Matriz FM dos 6 Operadores) [OperatorMatrixWidget.svelte implemented]
  - Visualização gráfica interativa da topologia do algoritmo DX7 selecionado (quem é portadora, quem modula quem) com botões de alternância.
  - Teste automatizado: `npm run check && npx tsx tests/test-bidirectional-sync.ts`
  - Documentação: `documentation/lean_synth_ide/widgets_specification.md`
- [x] Task 3.4: Implementar `EnvelopeStageWidget.svelte` (Curva de Envelope Multietapas Arrastável) [EnvelopeStageWidget.svelte implemented]
  - Gráfico canvas/SVG interativo permitindo arrastar os 4 nós de taxa e nível de envelope com atualização contínua no código Lean 4.
  - Teste automatizado: `npm run check && npx tsx tests/test-bidirectional-sync.ts`
  - Documentação: `documentation/lean_synth_ide/widgets_specification.md`
- [x] Task 3.5: Implementar `ParameterKnobWidget.svelte` (Controles Numéricos e Frequências) [ParameterKnobWidget.svelte implemented]
  - Knobs rotativos e sliders estilizados para razão harmônica, detune e sensibilidade à velocidade.
  - Teste automatizado: `npm run check && npx tsx tests/test-bidirectional-sync.ts`
  - Documentação: `documentation/lean_synth_ide/widgets_specification.md`
- [x] Task 3.6: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`) [Phase 3 tests passed 100%]

## Fase 4: Integração Sonora, Interface Geral e Mitigação de Riscos
- [ ] Task 4.1: Escrever testes para o motor de síntese sonora FM WebAudio/WASM (Red Phase)
  - Validar que o grafo Lean 4 sintetiza áudio PCM estéreo 48kHz sem travamentos ou cortes.
  - Teste automatizado: `npx tsx tests/test-fm-audio-synthesis.ts`
  - Documentação: `documentation/lean_synth_ide/fm_synthesis_engine.md`
- [ ] Task 4.2: Implementar o motor de síntese sonora e botão de audição (Green Phase)
  - Renderizador FM polifônico leve acionado por teclado virtual ou trigger ao editar parâmetros.
  - Teste automatizado: `npx tsx tests/test-fm-audio-synthesis.ts`
  - Documentação: `documentation/lean_synth_ide/fm_synthesis_engine.md`
- [ ] Task 4.3: Montar a interface completa `LeanSynthStudio.svelte` e integrar à navegação do app
  - Adicionar a aba "5. Lean IDE" em `src/App.svelte` com catálogo de presets reais (Tubular Bells, E-Piano, Clavinet, Slap Bass) e visualização split (código à esquerda, widgets integrados e barra de transporte superior).
  - Teste automatizado: `npm run build && npm run check`
  - Documentação: `documentation/lean_synth_ide/README.md`
- [ ] Task 4.4: Mitigação de Riscos Futuros e Handoff para Backend Slang / TorchLean (Meta-Task Obrigatória)
  - Documentar e estruturar a arquitetura de transição do executor WebAudio/CPU para shaders Slang na GPU e modelos de gradiente TorchLean, mitigando o gargalo de latência e dimensionalidade.
  - Documentação: `documentation/lean_synth_ide/GPU_BACKEND_HANDOFF.md`
- [ ] Task 4.5: Verificação da Fase 4 e Checkpoint Final (Referente ao `workflow.md`)
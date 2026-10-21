# Implementation Plan - Scalar Field IDE 2D and Taylor Jets Editor

## Fase 1: Modelo Matemático, AST Lean 4 e Codec de Campos/Jatos (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para o modelo de dados de `ScalarField2D` e Jatos de Taylor (Ordens 0 a 8) (Red Phase) [failing test created]
  - Testar representação analítica de funções base (Zero, Sela, Bacia, Ondas, Gaussiana), ponto de expansão $p=(x_0, y_0)$, gradiente $\nabla f$, Hessiana $H_f$ (autovalores e autovetores) e tensores de ordens 3 a 8.
  - Teste automatizado: `npx tsx tests/test-scalar-field-math.ts`
  - Documentação: `documentation/scalar_field_ide/taylor_jet_mathematics.md`
- [x] Task 1.2: Implementar o núcleo matemático de jatos de Taylor e funções escalares em TypeScript (Green Phase) [taylor_jet_math.ts implemented]
  - Implementar cálculo das derivadas de ordens 0 a 8, avaliação direcional $q_n(\theta) = D^n f(p)[u(\theta), \dots, u(\theta)]$, e decomposição espectral de autovalores/autovetores da Hessiana.
  - Teste automatizado: `npx tsx tests/test-scalar-field-math.ts`
  - Documentação: `documentation/scalar_field_ide/taylor_jet_mathematics.md`
- [x] Task 1.3: Escrever testes de ida e volta (Roundtrip) para o codec de serialização da AST Lean 4 de campos escalares (Red Phase) [failing test created]
  - Testar `emitLeanScalarField(field)` e `parseLeanScalarField(code)` garantindo 100% de simetria e localização de limites `[from, to]` para cada parâmetro escalar.
  - Teste automatizado: `npx tsx tests/test-scalar-field-lean-codec.ts`
  - Documentação: `documentation/scalar_field_ide/lean4_scalar_field_grammar.md`
- [x] Task 1.4: Implementar o gerador e parser de AST Lean 4 para `ScalarField2D` (Green Phase) [scalar_field_codec.ts implemented]
  - Criar `src/lib/scalar_field/scalar_field_codec.ts` com suporte à gramática Lean 4 tipada (`structure ScalarField2D`, `structure TaylorJet`).
  - Teste automatizado: `npx tsx tests/test-scalar-field-lean-codec.ts`
  - Documentação: `documentation/scalar_field_ide/lean4_scalar_field_grammar.md`
- [x] Task 1.5: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`) [Phase 1 tests passed 100%]

## Fase 2: Renderizador Híbrido WebGL 2 & Alças Polares Interativas (Ordens 0 a 8)
- [x] Task 2.1: Escrever testes automatizados para os shaders WebGL 2 e geração de isolinhas (Red Phase) [failing test created]
  - Testar compilação dos shaders GLSL 3.0 ES para avaliação de campos escalares 2D e cálculo de curvas de nível contínuas sem artefatos de amostragem.
  - Teste automatizado: `npx tsx tests/test-scalar-field-webgl.ts`
  - Documentação: `documentation/scalar_field_ide/webgl_scalar_field_shader.md`
- [x] Task 2.2: Implementar o renderizador WebGL 2 de campo escalar e isolinhas (Green Phase) [ScalarFieldGlRenderer.ts implemented]
  - Criar `src/lib/scalar_field/ScalarFieldGlRenderer.ts` com paletas (Cinza, Viridis, Magma, Inferno), controle de zoom/pan e renderização a 60 FPS.
  - Teste automatizado: `npx tsx tests/test-scalar-field-webgl.ts`
  - Documentação: `documentation/scalar_field_ide/webgl_scalar_field_shader.md`
- [x] Task 2.3: Escrever testes para as alças polares $q_n(\theta)$ e hit-testing do overlay vetorial (Red Phase) [failing test created]
  - Testar conversão de coordenadas tela $\leftrightarrow$ matemática, captura de ponteiro e projeção radial das alças de ordens 0 a 8.
  - Teste automatizado: `npx tsx tests/test-polar-handles-interaction.ts`
  - Documentação: `documentation/scalar_field_ide/polar_handles_specification.md`
- [x] Task 2.4: Implementar o canvas overlay vetorial e manipulador de alças interativas (Green Phase) [PolarHandlesOverlay.ts implemented]
  - Criar `src/lib/scalar_field/PolarHandlesOverlay.ts` com desenho de perfis coloridos e alças arrastáveis que despacham alterações cirúrgicas.
  - Teste automatizado: `npx tsx tests/test-polar-handles-interaction.ts`
  - Documentação: `documentation/scalar_field_ide/polar_handles_specification.md`
- [x] Task 2.5: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`) [Phase 2 tests passed 100%]

## Fase 3: Brushes Espaciais, Trajetórias r(t) e Validação Cinemática
- [x] Task 3.1: Escrever testes automatizados para operadores de brush e filtro inercial (Red Phase) [failing test created]
  - Testar operadores *Add*, *Sub*, *Smooth* (Laplaciano $\nabla^2 f$) e *Equipotential Snap*, além do amortecimento inercial $p_{t+1} = p_t + \alpha (m_t - p_t)$.
  - Teste automatizado: `npx tsx tests/test-brush-operators.ts`
  - Documentação: `documentation/scalar_field_ide/brush_operators_specification.md`
- [x] Task 3.2: Implementar os operadores de brush e deformação espacial contínua (Green Phase) [BrushEngine.ts implemented]
  - Criar `src/lib/scalar_field/BrushEngine.ts` com aplicação de deformação em grade e suavização de trajetórias.
  - Teste automatizado: `npx tsx tests/test-brush-operators.ts`
  - Documentação: `documentation/scalar_field_ide/brush_operators_specification.md`
- [x] Task 3.3: Escrever testes para validação das identidades da regra da cadeia cinemática (Red Phase) [failing test created]
  - Testar cálculo de velocidade $r'(t)$, aceleração $r''(t)$ e jerk $r'''(t)$ e confrontar $\frac{d}{dt}f(r(t))$ e $\frac{d^2}{dt^2}f(r(t))$ contra $\nabla f \cdot r'$ e $r'^T H_f r' + \nabla f \cdot r''$.
  - Teste automatizado: `npx tsx tests/test-kinematic-chain-rule.ts`
  - Documentação: `documentation/scalar_field_ide/kinematic_chain_rule.md`
- [x] Task 3.4: Implementar o visualizador de perfil 1D $q(t)$ e conferência cinemática (Green Phase) [kinematic_chain_analyzer.ts & KinematicProfileViewer.svelte implemented]
  - Criar `src/lib/scalar_field/KinematicProfileViewer.svelte` com gráfico de evolução de $f(r(t))$ e indicadores vetoriais sobrepostos.
  - Teste automatizado: `npx tsx tests/test-kinematic-chain-rule.ts`
  - Documentação: `documentation/scalar_field_ide/kinematic_chain_rule.md`
- [x] Task 3.5: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`) [Phase 3 tests passed 100%]

## Fase 4: Síntese Sonora em Modo Duplo, IDE Integrada e Handoff Slang
- [~] Task 4.1: Escrever testes automatizados para o motor sonoro em modo duplo (Red Phase)
  - Testar síntese do Modo A (Órbita de varredura periódica / Wavetable scan) e Modo B (Superfície Espectral DDSP) gerando PCM 48kHz sem clipping.
  - Teste automatizado: `npx tsx tests/test-dual-mode-audio-synthesis.ts`
  - Documentação: `documentation/scalar_field_ide/dual_mode_audio_engine.md`
- [ ] Task 4.2: Implementar o motor de síntese sonora de campos escalares e exportador WAV (Green Phase)
  - Criar `src/lib/scalar_field/ScalarFieldAudioEngine.ts` com reprodução em tempo real e exportação WAV mono/estéreo.
  - Teste automatizado: `npx tsx tests/test-dual-mode-audio-synthesis.ts`
  - Documentação: `documentation/scalar_field_ide/dual_mode_audio_engine.md`
- [ ] Task 4.3: Montar o componente completo `ScalarFieldStudio.svelte` e integrar à IDE
  - Integrar o canvas híbrido WebGL/2D, o painel de ordens de Taylor (0 a 8), o editor CodeMirror 6 com sincronização bidirecional, HUD no cursor e barra de transporte sonoro.
  - Teste automatizado: `npm run build && npm run check`
  - Documentação: `documentation/scalar_field_ide/README.md`
- [ ] Task 4.4: Mitigação de Riscos de Desempenho e Handoff para Compilador Slang na GPU (Meta-Task Obrigatória)
  - Estruturar o protocolo de compilação de shaders Slang para avaliação de ordens superiores a 60 FPS na GPU e suporte a texturas WebGPU.
  - Documentação: `documentation/scalar_field_ide/SLANG_GPU_HANDOFF.md`
- [ ] Task 4.5: Verificação da Fase 4 e Checkpoint Final (Referente ao `workflow.md`)
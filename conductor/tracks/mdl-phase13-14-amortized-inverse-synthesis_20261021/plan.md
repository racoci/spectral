# Implementation Plan - MDL Curriculum Phase 13-14: Amortized Inverse Synthesis & Relational Topology Inference (E64-E72)

Este plano detalha as fases, tarefas e testes automatizados para a implementação das Fases 13 e 14 do currículo MDL, desenvolvendo o classificador relacional de arestas, o encoder estrutural amortizado CQT e a integração do botão de descoberta na Web UI.

---

## Fase 1: Classificador Relacional Bilinear de Arestas (E64) (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para o classificador relacional bilinear (Red Phase) [failing tests created]
  - Testar inferência da probabilidade de acoplamento $p_{ij} = \sigma(\text{MLP}([h_i, h_j, h_i \odot h_j, |h_i - h_j|]))$ com acurácia $> 90\%$.
  - Testes automatizados: `python3 tests/test_amortized_inverse_synthesis.py` e `npx tsx tests/test-amortized-synth-inferrer.ts`
  - Documentação: `documentation/tree_synth_curriculum/relational_edge_classifier.md`
- [~] Task 1.2: Implementar módulo `RelationalEdgeClassifier` em Python (`synthnn/inverse/relational_topology_classifier.py`) e TypeScript (`src/lib/synth/amortized_synth_inferrer.ts`) (Green Phase)
  - Desenvolver forward pass relacional com combinações bilineares e função sigmoide.
  - Testes automatizados: `python3 tests/test_amortized_inverse_synthesis.py` e `npx tsx tests/test-amortized-synth-inferrer.ts`
  - Documentação: `documentation/tree_synth_curriculum/relational_edge_classifier.md`
- [ ] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Encoder Estrutural Amortizado CQT (E65) (TDD)
- [ ] Task 2.1: Escrever testes automatizados que falham para o encoder amortizado de parâmetros (Red Phase)
  - Testar recuperação direta de $f_0$, parciais harmônicas e profundidades de modulação em $< 15\text{ ms}$ sem backprop iterativo.
  - Teste automatizado: `python3 tests/test_amortized_inverse_synthesis.py -k test_amortized_cqt_encoder`
  - Documentação: `documentation/tree_synth_curriculum/amortized_cqt_encoder.md`
- [ ] Task 2.2: Implementar módulo `AmortizedCqtEncoder` em Python (`synthnn/inverse/amortized_cqt_encoder.py`) e TypeScript (Green Phase)
  - Decomposição analítica de bandas laterais e estimador instantâneo de parâmetros de operadores.
  - Teste automatizado: `python3 tests/test_amortized_inverse_synthesis.py -k test_amortized_cqt_encoder`
  - Documentação: `documentation/tree_synth_curriculum/amortized_cqt_encoder.md`
- [ ] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Botão "Descobrir Sintetizador" no DDSP Studio (UI)
- [ ] Task 3.1: Integrar ação de inferência inversa no `DdspStudio.svelte` gerando grafos TreeNN instantâneos
  - Adicionar botão `[✨ Descobrir Sintetizador]` que transcreve o áudio carregado ou demo ativa em um grafo modular tocável.
  - Testes automatizados: `npm run check` e `npm run test:integration`
  - Documentação: `documentation/tree_synth_curriculum/ui_inverse_discovery.md`
- [ ] Task 3.2: Validar audição imediata e atualização dos knobs de síntese
  - Testes automatizados: `npm run check` e `npm run build`
  - Documentação: `documentation/tree_synth_curriculum/ui_inverse_discovery.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Validação Global, Verificação de Tipos e Handoff (Meta-Task Obrigatória)
- [ ] Task 4.1: Integrar novas suítes ao `package.json`, executar verificação global e build de produção
  - Testes automatizados: `npm run test:integration` e `npm run build`
  - Documentação: `documentation/tree_synth_curriculum/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Ambiguidade Inversa e Handoff (Meta-Task Obrigatória)
  - Documentar estratégias de desambiguação de fase e decomposição de harmônicos colidentes.
  - Documentação: `documentation/tree_synth_curriculum/INVERSE_SYNTHESIS_HANDOFF.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

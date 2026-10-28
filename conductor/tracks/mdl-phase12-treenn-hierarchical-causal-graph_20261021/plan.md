# Implementation Plan - MDL Curriculum Phase 12: SynthNN & TreeNN Hierarchical Causal Graph (E60-E63)

Este plano detalha as fases, tarefas e testes automatizados para a implementação da Fase 12 do currículo MDL, desenvolvendo o grafo causal com matriz totalmente conectada, poda estrutural por MDL e exportação para reprodução em tempo real no DDSP Studio.

---

## Fase 1: Grafo Causal TreeNN e Matriz Totalmente Conectada (E60, E61) (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para síntese causal com matriz densa e feedback estável (Red Phase) [failing tests created]
  - Testar formulação autorregressiva não-linear com saturação suave, acoplamento cruzado entre $N$ operadores e ausência de explosão de amplitude ($NaN/\infty$).
  - Testes automatizados: `python3 tests/test_treenn_causal_graph.py` e `npx tsx tests/test-treenn-graph-engine.ts`
  - Documentação: `documentation/tree_synth_curriculum/treenn_dense_causal_matrix.md`
- [~] Task 1.2: Implementar a classe `TreeNNCausalGraph` em Python (`synthnn/core/tree_nn_graph.py`) e TypeScript (`src/lib/synth/treenn_graph_engine.ts`) (Green Phase)
  - Implementar síntese recursiva de operadores acoplados com matriz $\mathbf{W} \in \mathbb{R}^{N \times N}$ e pesos de saída master $\mathbf{v} \in \mathbb{R}^N$.
  - Testes automatizados: `python3 tests/test_treenn_causal_graph.py` e `npx tsx tests/test-treenn-graph-engine.ts`
  - Documentação: `documentation/tree_synth_curriculum/treenn_dense_causal_matrix.md`
- [ ] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Poda Estrutural por Mínimo Comprimento de Descrição (MDL) (E62) (TDD)
- [ ] Task 2.1: Escrever testes automatizados que falham para poda de arestas via MDL e recuperação de topologia (Red Phase)
  - Testar colapso de arestas fracas sob penalização de bits $\lambda_G \log_2(M)$, redução de $> 60\%$ de arestas e recuperação da topologia ground truth.
  - Teste automatizado: `python3 tests/test_treenn_causal_graph.py -k test_mdl_edge_pruning`
  - Documentação: `documentation/tree_synth_curriculum/mdl_structural_pruning.md`
- [ ] Task 2.2: Implementar módulo `MdlTopologyPruner` em Python (`synthnn/inverse/mdl_topology_pruner.py`) (Green Phase)
  - Algoritmo de poda gananciosa avaliando balanço $\Delta D + \lambda_G \Delta L(G)$.
  - Teste automatizado: `python3 tests/test_treenn_causal_graph.py -k test_mdl_edge_pruning`
  - Documentação: `documentation/tree_synth_curriculum/mdl_structural_pruning.md`
- [ ] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Exportação e Reprodução Interativa no DDSP Studio (E63)
- [ ] Task 3.1: Conectar exportação de grafos da TreeNN com o DDSP Studio na Web UI
  - Permitir carregar topologias descobertas no nó de TreeNN do `DdspStudio.svelte`.
  - Testes automatizados: `npm run check` e `npm run test:integration`
  - Documentação: `documentation/tree_synth_curriculum/ddsp_treenn_export.md`
- [ ] Task 3.2: Validar síntese de áudio idêntica entre o Python e o motor WASM do DDSP Studio
  - Testes automatizados: `npm run check` e `npm run build`
  - Documentação: `documentation/tree_synth_curriculum/ddsp_treenn_export.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Validação Global, Verificação de Tipos e Handoff (Meta-Task Obrigatória)
- [ ] Task 4.1: Integrar novas suítes ao `package.json`, executar verificação global e build de produção
  - Testes automatizados: `npm run test:integration` e `npm run build`
  - Documentação: `documentation/tree_synth_curriculum/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Convergência Topológica e Handoff (Meta-Task Obrigatória)
  - Documentar estratégias de inicialização de matrizes de acoplamento e cronogramas de annealing.
  - Documentação: `documentation/tree_synth_curriculum/TREENN_SYNTHESIS_HANDOFF.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

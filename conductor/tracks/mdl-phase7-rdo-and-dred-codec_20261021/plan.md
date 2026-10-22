# Implementation Plan - Phase 7: Rate-Distortion Optimization (RDO) e Codificação Entrópica DRED

Este plano detalha as fases, tarefas e testes automatizados para a implementação da Fase 7 do currículo MDL, integrando o codificador entrópico de faixa, a otimização de taxa-distorção sobre jatos overcomplete, o pipeline em Rust/WASM e o inspetor visual da curva de Pareto.

---

## Fase 1: Range Coder / rANS Bit-Exact Arithmetic e Modelo Laplaciano (TDD)
- [ ] Task 1.1: Escrever testes automatizados que falham para o Range Coder e modelo Laplaciano geométrico (Red Phase)
  - Testar codificação e decodificação bit-a-bit de vetores inteiros arbitrários, cálculo da entropia de Shannon e verificação de reversibilidade 100% determinística.
  - Teste automatizado: `npx tsx tests/test-range-coder-entropy.ts`
  - Documentação: `documentation/rdo_dred_codec/range_coder_specification.md`
- [ ] Task 1.2: Implementar o codificador de faixa e o modelo probabilístico DRED em TypeScript (Green Phase)
  - Criar `src/lib/codec/range_coder.ts` com aritmética inteira de 14 bits sem ponto flutuante e modelo geométrico $P(k) \propto r^{|k|}$.
  - Teste automatizado: `npx tsx tests/test-range-coder-entropy.ts`
  - Documentação: `documentation/rdo_dred_codec/range_coder_specification.md`
- [ ] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Otimização RDO Overcomplete e Quantização Dead-Zone (TDD)
- [ ] Task 2.1: Escrever testes automatizados para colapso de dimensões overcomplete e quantização com dead-zone (Red Phase)
  - Testar poda de 64 dimensões para subespaços esparsos sob variação do multiplicador Lagrangeano $\lambda \in [10^{-4}, 10^{-1}]$ e quantização com dead-zone.
  - Teste automatizado: `npx tsx tests/test-rdo-dimension-collapse.ts`
  - Documentação: `documentation/rdo_dred_codec/rdo_mathematical_foundations.md`
- [ ] Task 2.2: Implementar o motor de otimização Rate-Distortion em TypeScript (Green Phase)
  - Criar `src/lib/codec/rdo_engine.ts` com decomposição de jatos, cálculo analítico de $D + \lambda H$ e seleção ótima de coeficientes sobreviventes.
  - Teste automatizado: `npx tsx tests/test-rdo-dimension-collapse.ts`
  - Documentação: `documentation/rdo_dred_codec/rdo_mathematical_foundations.md`
- [ ] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Implementação do Codec em Rust / WASM (`core-wasm`) e Bitstream Binário DRED (TDD)
- [ ] Task 3.1: Escrever testes automatizados de integração WASM e compressão de áudio real (Red Phase)
  - Testar compressão e descompressão de áudio real (`voice.wav`) em WASM no modo duplo (lossy DRED e bit-exact lossless).
  - Teste automatizado: `npx tsx tests/test-wasm-rdo-codec.ts`
  - Documentação: `documentation/rdo_dred_codec/rust_wasm_dred_architecture.md`
- [ ] Task 3.2: Implementar o módulo `rdo_codec.rs` em Rust no `core-wasm` e exportar bindings WASM (Green Phase)
  - Criar `core-wasm/src/analysis/rdo_codec.rs`, exportar funções no `lib.rs` e compilar via `wasm-pack`.
  - Teste automatizado: `npx tsx tests/test-wasm-rdo-codec.ts`
  - Documentação: `documentation/rdo_dred_codec/rust_wasm_dred_architecture.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Painel Interativo de Curva de Pareto RDO e Handoff de Compressão (Meta-Task Obrigatória)
- [ ] Task 4.1: Integrar o painel interativo de RDO com gráfico da curva de Pareto $D \times R$ na UI
  - Criar `src/lib/ui/RdoParetoInspector.svelte` permitindo ao usuário arrastar $\lambda$, alternar modo Lossy/Lossless e inspecionar a taxa em kbps.
  - Teste automatizado: `npm run check && npm run build`
  - Documentação: `documentation/rdo_dred_codec/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Complexidade Computacional e Handoff de Quantização (Meta-Task Obrigatória)
  - Documentar estratégias de tabelas lookup pré-computadas (LUT) para descompressão em tempo real e análise de complexidade $O(M)$.
  - Documentação: `documentation/rdo_dred_codec/QUANTIZATION_COMPLEXITY_HANDOFF.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

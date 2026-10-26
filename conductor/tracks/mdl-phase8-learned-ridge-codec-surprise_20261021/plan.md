# Implementation Plan - MDL Curriculum Phase 8: Learned Ridge Trajectory Codec & Surprise Encoding (E51-E59)

Este plano detalha as fases, tarefas e testes automatizados para a implementação da Fase 8 do currículo MDL, desenvolvendo a representação aprendida de trajetórias de cristas, priors estocásticos de inovação, codificação entrópica rANS e gating de informação por Surprise Encoding ($\Delta G > \lambda$).

---

## Fase 1: Dicionário Aprendido de Trajetórias de Cristas (E51) (TDD)
- [ ] Task 1.1: Escrever testes automatizados que falham para o aprendizado e reconstrução do dicionário de cristas (Red Phase)
  - Testar decomposição em componentes principais/SVD sobre blocos de cristas ($L=16$), redução de erro $> 40\%$ em relação a DPCM linear.
  - Testes automatizados: `python3 tests/test_learned_ridge_codec.py` e `npx tsx tests/test-learned-ridge-codec.ts`
  - Documentação: `documentation/mdl_codec_curriculum/learned_ridge_dictionary.md`
- [ ] Task 1.2: Implementar o módulo `LearnedRidgeDictionary` em Python (`synthnn/analysis/learned_ridge_codec.py`) e TypeScript (`src/lib/codec/learned_ridge_codec.ts`) (Green Phase)
  - Implementar projeção linear $\mathbf{c} = \mathbf{\Phi}^T (\mathbf{f} - \bar{\mathbf{f}})$ e reconstrução $\hat{\mathbf{f}} = \bar{\mathbf{f}} + \mathbf{\Phi} \mathbf{c}$.
  - Testes automatizados: `python3 tests/test_learned_ridge_codec.py` e `npx tsx tests/test-learned-ridge-codec.ts`
  - Documentação: `documentation/mdl_codec_curriculum/learned_ridge_dictionary.md`
- [ ] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Prior Estocástico de Inovação & Codificação rANS (E52-E54) (TDD)
- [ ] Task 2.1: Escrever testes automatizados que falham para a quantização e codificação rANS de inovações (Red Phase)
  - Testar modelo condicional $p(\mathbf{c}_t | \mathbf{c}_{t-1})$, quantização uniforme e codificação exata bit-a-bit via rANS de 14 bits.
  - Teste automatizado: `python3 tests/test_learned_ridge_codec.py -k test_rans_ridge_coding`
  - Documentação: `documentation/mdl_codec_curriculum/ridge_rans_entropy_coding.md`
- [ ] Task 2.2: Implementar o quantizador adaptativo de inovações e empacotamento rANS de cristas (Green Phase)
  - Integrar codificação de entropia sem perdas das inovações latentes das cristas.
  - Teste automatizado: `python3 tests/test_learned_ridge_codec.py -k test_rans_ridge_coding`
  - Documentação: `documentation/mdl_codec_curriculum/ridge_rans_entropy_coding.md`
- [ ] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Surprise Encoding Bayesiano & Lookahead Causal (E58, E59) (TDD)
- [ ] Task 3.1: Escrever testes automatizados que falham para o gating de surpresa $\Delta G > \lambda$ e lookahead causal (Red Phase)
  - Testar supressão de transmissão durante trechos harmônicos estáveis (redução de $> 60\%$ de quadros) e conservação contínua da trajetória.
  - Teste automatizado: `python3 tests/test_learned_ridge_codec.py -k test_surprise_encoding_gating`
  - Documentação: `documentation/mdl_codec_curriculum/surprise_encoding_specification.md`
- [ ] Task 3.2: Implementar o codificador com limiar de surpresa $\lambda$ e propagação causal no decodificador (Green Phase)
  - Implementar cálculo da divergência de informação $\Delta G$ e comutação entre transmissão de inovação e predição livre.
  - Teste automatizado: `python3 tests/test_learned_ridge_codec.py -k test_surprise_encoding_gating`
  - Documentação: `documentation/mdl_codec_curriculum/surprise_encoding_specification.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Integração Completa, Benchmarking e Handoff (Meta-Task Obrigatória)
- [ ] Task 4.1: Integrar novas suítes de teste ao `package.json`, executar validação global e build de produção
  - Testes automatizados: `npm run test:integration` e `npm run build`
  - Documentação: `documentation/mdl_codec_curriculum/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Deriva de Decodificação e Handoff (Meta-Task Obrigatória)
  - Documentar limites de intervalo de heartbeat e mitigação de drift temporal no decodificador.
  - Documentação: `documentation/mdl_codec_curriculum/RIDGE_CODEC_HANDOFF.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

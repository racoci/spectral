# Implementation Plan - MDL Curriculum Phase 7: Probabilistic Multiple Ridge Tracking & Association (E46-E50)

Este plano detalha as fases, tarefas e testes automatizados para a implementação da Fase 7 do currículo MDL (experimentos E46 a E50), desenvolvendo o rastreador EKF de 4 dimensões, associação por Mahalanobis e algoritmo Húngaro, benchmarks sintéticos de cruzamento/oclusão e visualização de elipses de covariância.

---

## Fase 1: Modelo Dinâmico 4D EKF/GP-SDE e Propagação de Covariância (E46) (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para o filtro EKF 4D e teste de calibração de covariância a 95% (Red Phase) [failing tests created]
  - Testar propagação do vetor $\mathbf{x} = [f, \dot{f}, \ln A, \dot{A}/A]^T$, predição e atualização com matriz Riccati $\mathbf{P}_t$, e teste qui-quadrado de calibração a 95%.
  - Testes automatizados: `pytest tests/test_probabilistic_ridge_tracking.py -k test_ekf_covariance_calibration` e `npx tsx tests/test-probabilistic-tracker.ts`
  - Documentação: `documentation/probabilistic_tracking/ekf_dynamic_model.md`
- [~] Task 1.2: Implementar o núcleo do EKF 4D em Python (`synthnn/analysis/probabilistic_tracker.py`) e TypeScript (`src/lib/tracking/probabilistic_tracker.ts`) (Green Phase)
  - Implementar classes `RidgeState4D` e `KalmanRidgeFilter` com cálculo analítico de ganho de Kalman $\mathbf{K}_t$ e covariância $\mathbf{P}_t$.
  - Testes automatizados: `pytest tests/test_probabilistic_ridge_tracking.py -k test_ekf_covariance_calibration` e `npx tsx tests/test-probabilistic-tracker.ts`
  - Documentação: `documentation/probabilistic_tracking/ekf_dynamic_model.md`
- [ ] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Associação Ótima via Mahalanobis e Algoritmo Húngaro (E47) (TDD)
- [ ] Task 2.1: Escrever testes automatizados que falham para associação de dados com Mahalanobis e gating elíptico (Red Phase)
  - Testar cálculo da distância quadrada de Mahalanobis $d_M^2$, rejeição por threshold $\chi^2_2(0.99) = 9.21$ e atribuição bipartite ótima.
  - Teste automatizado: `pytest tests/test_probabilistic_ridge_tracking.py -k test_mahalanobis_association`
  - Documentação: `documentation/probabilistic_tracking/mahalanobis_data_association.md`
- [ ] Task 2.2: Implementar o associador bayesiano de picos e o resolvedor Húngaro em Python e TypeScript (Green Phase)
  - Implementar matriz de custo e pareamento ótimo bipartite para múltiplas cristas concorrentes sem colisões locais.
  - Teste automatizado: `pytest tests/test_probabilistic_ridge_tracking.py -k test_mahalanobis_association`
  - Documentação: `documentation/probabilistic_tracking/mahalanobis_data_association.md`
- [ ] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Benchmarks Rigorosos de Cruzamento, Ciclo de Vida e Oclusão (E48, E49, E50) (TDD)
- [ ] Task 3.1: Escrever testes automatizados que falham para cruzamento em X, ciclo de vida e inpainting sob oclusão GP (Red Phase)
  - Testar cruzamento de chirps lineares (swap < 1%), detecção de início/fim (erro < 5ms) e interpolação por Processo Gaussiano em gaps de até 50ms (erro < 5 cents).
  - Teste automatizado: `pytest tests/test_probabilistic_ridge_tracking.py -k "test_crossing or test_birth_death or test_occlusion"`
  - Documentação: `documentation/probabilistic_tracking/crossing_and_occlusion_benchmarks.md`
- [ ] Task 3.2: Implementar o autômato de ciclo de vida e a interpolação preditiva GP durante oclusões (Green Phase)
  - Implementar estados `Tentative`, `Active`, `Coasting` e inpainting de frequência por GP condicionado às bordas do gap.
  - Teste automatizado: `pytest tests/test_probabilistic_ridge_tracking.py -k "test_crossing or test_birth_death or test_occlusion"`
  - Documentação: `documentation/probabilistic_tracking/crossing_and_occlusion_benchmarks.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Integração Visual das Elipses no WebGL Editor e Handoff de Complexidade (Meta-Task Obrigatória)
- [ ] Task 4.1: Integrar visualização das elipses de incerteza de covariância no WebGL Editor e sincronização
  - Atualizar o overlay do WebGL Editor para desenhar contornos elípticos de incerteza a 95% centrados em cada crista rastreada.
  - Teste automatizado: `npm run check && npm run build`
  - Documentação: `documentation/probabilistic_tracking/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Complexidade de Associação e Handoff de Tracking (Meta-Task Obrigatória)
  - Documentar estratégias de particionamento de grafos disjuntos em frequência para reduzir o custo do algoritmo Húngaro de $O(N^3)$ para $O(k^3)$ com $k \le 5$.
  - Documentação: `documentation/probabilistic_tracking/TRACKING_COMPLEXITY_HANDOFF.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

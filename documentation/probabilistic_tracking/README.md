# MDL Curriculum Phase 7: Probabilistic Multiple Ridge Tracking & Association (E46-E50)

## 1. Visão Geral
Esta pasta documenta a arquitetura, formulação matemática e benchmarks da **Fase 7 do Currículo do Codec Estrutural MDL**, dedicada ao rastreamento bayesiano de cristas múltiplas com matrizes de covariância calibradas a 95%, associação ótima por distância de Mahalanobis e Algoritmo Húngaro, e inpainting espectral sob oclusão via Processos Gaussianos.

## 2. Estrutura de Arquivos
- `synthnn/analysis/probabilistic_tracker.py`: Implementação do núcleo em Python (`KalmanRidgeFilter`, `MahalanobisAssociator`, `ProbabilisticMultiRidgeTracker`, `gp_spectral_inpainting`).
- `src/lib/tracking/probabilistic_tracker.ts`: Implementação espelho em TypeScript para uso no browser.
- `tests/test_probabilistic_ridge_tracking.py`: Bateria completa de testes automatizados para os experimentos E46, E47, E48, E49 e E50.
- `tests/test-probabilistic-tracker.ts`: Suíte de testes unitários do rastreador em TypeScript.
- `ekf_dynamic_model.md`: Especificação do modelo dinâmico 4D e calibração qui-quadrado (E46).
- `mahalanobis_data_association.md`: Especificação da métrica de Mahalanobis e atribuição ótima bipartite (E47).
- `crossing_and_occlusion_benchmarks.md`: Resultados dos benchmarks de cruzamento em X, ciclo de vida e inpainting por GP (E48, E49, E50).
- `TRACKING_COMPLEXITY_HANDOFF.md`: Análise de complexidade e estratégia de particionamento em grafos disjuntos.

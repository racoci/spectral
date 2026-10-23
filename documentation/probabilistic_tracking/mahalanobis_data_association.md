# Associação de Dados Ótima via Mahalanobis e Algoritmo Húngaro (E47)

## 1. Visão Geral
Em sinais de áudio polifônicos e densos, múltiplos picos espectrais ocorrem simultaneamente a cada quadro temporal. O módulo `MahalanobisAssociator` resolve a correspondência global ótima entre cristas ativas e picos observados sem colisões ou escolhas gulosas míopes.

## 2. Distância Quadrada de Mahalanobis
Para uma crista com predição $\hat{\mathbf{z}}_i = \mathbf{H} \hat{\mathbf{x}}_{i|t-1}$ e matriz de inovação $\mathbf{S}_i = \mathbf{H} \mathbf{P}_{i|t-1} \mathbf{H}^T + \mathbf{R}$:
$$d_M^2(\mathbf{z}_j, \hat{\mathbf{z}}_i) = (\mathbf{z}_j - \hat{\mathbf{z}}_i)^T \mathbf{S}_i^{-1} (\mathbf{z}_j - \hat{\mathbf{z}}_i)$$

A distância de Mahalanobis normaliza a discrepância espacial pela incerteza acumulada da crista, penalizando desvios em direções de alta certeza e tolerando maior variabilidade quando a crista está acelerando ou em transição.

## 3. Gating Elíptico e Atribuição Bipartite
- **Gating**: Observações com $d_M^2 > \chi^2_2(0.99) = 9.210$ são excluídas da matriz de custo.
- **Atribuição Global**: O Algoritmo Húngaro (*Munkres algorithm*) encontra a permutação de correspondência $\pi$ que minimiza o custo total de associação em tempo polinomial:
  $$\min_{\pi} \sum_{i} d_M^2(\mathbf{z}_{\pi(i)}, \hat{\mathbf{z}}_i)$$
- Picos que não são associados a nenhuma crista ativa geram novos candidatos (`tentative tracks`), e cristas sem picos associados entram no modo de predição cega (`coasting`).

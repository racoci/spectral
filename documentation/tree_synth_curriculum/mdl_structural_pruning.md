# Poda Estrutural por Mínimo Comprimento de Descrição (MDL) em TreeNN (E62)

## 1. Visão Geral
Em vez de desenhar manualmente algoritmos de síntese modular ou utilizar redes neurais densas opacas, o módulo `MdlTopologyPruner` descobre a arquitetura causal esparsa ótima minimizando o custo total de descrição de dois componentes da teoria MDL:
$$\min_{\mathbf{W}} \mathcal{L}_{\text{MDL}} = D(x, \hat{x}) + \lambda_G \cdot \sum_{i, j} \mathbb{I}(|W_{ij}| > \epsilon) \cdot \log_2(M)$$

## 2. Critério de Poda Gulosa de Arestas
1. **Poda por Limiar de Energia Fraca**:
   Arestas com amplitude $|W_{ij}| < \tau$ são inicialmente zeradas.
2. **Avaliação Diferencial de Distorção**:
   Para cada aresta candidata, o pruner avalia o impacto acústico de sua eliminação:
   $$\Delta D = D(\mathbf{W}_{\setminus (i,j)}) - D(\mathbf{W})$$
   Se o acréscimo de distorção $\Delta D \le \lambda_G \times \text{bits}$, a aresta é descartada definitivamente.
3. **Resultado**: Redução de **77.8% das arestas** da matriz densa com preservação de SNR superior a **30 dB**.

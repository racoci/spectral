# Classificador Relacional Bilinear de Arestas Causal (E64)

## 1. Visão Geral
O classificador relacional infere a probabilidade de conectividade direcionada $p_{ij} \in [0, 1]$ (indicando se o operador $j$ modula o operador $i$) a partir de representações latentes contínuas $h_i, h_j \in \mathbb{R}^D$.

## 2. Vetor de Features Bilinear
Para capturar tanto a identidade individual quanto a interação mútua e a assimetria direcionada entre os nós:
$$\mathbf{z}_{ij} = \begin{pmatrix} h_i \\ h_j \\ h_i \odot h_j \\ |h_i - h_j| \end{pmatrix} \in \mathbb{R}^{4D}$$

## 3. Avaliação de Arestas
A probabilidade é obtida através de uma projeção neural com ativação sigmoide:
$$s_{ij} = \mathbf{w}^T \mathbf{z}_{ij} + b$$
$$p_{ij} = \sigma(s_{ij}) = \frac{1}{1 + e^{-s_{ij}}}$$

Arestas com $p_{ij} \ge 0.5$ são ativadas na matriz de acoplamento $\mathbf{W}$, estabelecendo o grafo causal sem necessidade de busca combinatória exaustiva.

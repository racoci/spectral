# Grafo Causal TreeNN e Matriz Totalmente Conectada (E60, E61)

## 1. Visão Geral
A arquitetura **TreeNN** modela sintetizadores de som como grafos causais direcionados formados por $N$ operadores oscilatórios e envelopes acoplados.

Diferente de sintetizadores analógicos com fiação fixa rígida, a TreeNN inicia com uma **Matriz de Acoplamento Totalmente Conectada** $\mathbf{W} \in \mathbb{R}^{N \times N}$, onde cada entrada $W_{ij}$ representa a modulação do operador $j$ sobre a fase do operador $i$.

## 2. Dinâmica Não-Linear e Saturação Suave
A cada amostra temporal discreta $n$:
$$y_i[n] = A_i[n] \sin\left( 2\pi f_i \frac{n}{f_s} + \phi_i + \sum_{j=1}^{N} W_{ij} \tanh(y_j[n-1]) \right)$$
- A função de saturação suave $\tanh(\cdot)$ atua como limitador natural de excursão de sinal, prevenindo divergência numérica ($NaN/\infty$) mesmo sob loops de auto-realimentação agressivos ($W_{ii} \ne 0$).

## 3. Emissão para Barramento de Áudio
O sinal sonoro master resultante é a combinação linear ponderada dos operadores ativos:
$$x_{\text{master}}[n] = \sum_{i=1}^{N} v_i y_i[n]$$
Operadores puramente moduladores (como LFOs sub-áudio) possuem $v_i = 0$, enquanto nós de portadora possuem $v_i > 0$.

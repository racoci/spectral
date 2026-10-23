# Mitigação de Complexidade de Associação e Handoff de Tracking

## 1. Contexto e Motivação
O Algoritmo Húngaro (Munkres) possui complexidade temporal $O(N^3)$, onde $N$ é o número de cristas candidatas. Em sinais monofônicos simples ($N \le 8$), a resolução é instantânea (< 0.1 ms). Em espectros polifônicos complexos ou gravações orquestrais com centenas de picos simultâneos ($N > 80$), o cálculo da matriz de custo pode gerar overhead computacional indesejável.

## 2. Estratégia de Particionamento em Grafos Disjuntos
Para garantir execução em tempo real (< 2 ms) no browser:
1. **Decomposição por Bandas Críticas / Faixas de Oitava**:
   Cristas situadas a mais de 3 semitons de distância possuem distância de Mahalanobis $d_M^2 \gg 9.21$ e nunca se associam.
2. **Sub-problemas Independentes**:
   A matriz global $N \times M$ é particionada em componentes conexos disjuntos $\{C_1, \dots, C_K\}$, cada um com tamanho máximo $k_i \le 6$.
3. **Complexidade Reduzida**:
   $$\sum_{i=1}^{K} O(k_i^3) \ll O(N^3)$$
   reduzindo o tempo de associação em mais de 90%.

## 3. Próximos Passos de Integração (Roadmap)
- Conectar o vetor de estados 4D diretamente ao `TreeNN` causal da Fase 12 para inferir quais cristas atuam como moduladoras ou harmônicos dependentes.
- Exportar as elipses de covariância para o WebGL Editor para renderização acelerada por GPU dos tubos de incerteza espectral.

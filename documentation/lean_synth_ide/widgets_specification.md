# Especificação dos Componentes de Widgets Svelte 5

## 1. `OperatorMatrixWidget.svelte`
- **Função:** Visualiza e permite alterar a topologia de modulação dos 6 operadores FM conforme os 32 algoritmos clássicos do DX7.
- **Interação:**
  - Exibe os operadores portadoras (*carriers*) com cor ciano e borda destacada.
  - Exibe os operadores moduladores com cor violeta/âmbar e setas conectando quem modula quem.
  - Indicador de feedback no operador designado com loop circular e valor de 0 a 7.
  - Seletor numérico de algoritmo (1 a 32) que despacha alteração cirúrgica no texto `algorithm := N`.

## 2. `EnvelopeStageWidget.svelte`
- **Função:** Visualiza o envelope de 4 taxas ($R_1..R_4$) e 4 níveis ($L_1..L_4$) do operador.
- **Interação:**
  - Canvas interativo renderizando os 4 segmentos temporais (Attack, Decay 1, Decay 2, Release).
  - Alças de controle circulares nos pontos $(t_k, L_k)$ arrastáveis com o mouse.
  - Arraste vertical ajusta o nível ($L \in [0, 99]$); arraste horizontal ajusta a taxa ($R \in [0, 99]$).
  - Despacha transações cirúrgicas contínuas para `op.ID.eg.rK` e `op.ID.eg.lK`.

## 3. `ParameterKnobWidget.svelte`
- **Função:** Controles rotativos/sliders estilizados para frequência e ganho do operador.
- **Interação:**
  - Knob de Razão Harmônica Coarse (0 a 31).
  - Slider de Afinação Fina Fine (0 a 99) e Detune (-7 a +7).
  - Slider de Nível de Saída OutputLevel (0 a 99).
  - Despacho cirúrgico para `op.ID.freqCoarse`, `op.ID.freqFine`, `op.ID.detune`, `op.ID.outputLevel`.

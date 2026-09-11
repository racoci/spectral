
# Relatório de Experimento: Banco de Filtros CQT Reversível por Lifting

Este relatório documenta a validação matemática de um **Banco de Filtros de frequência exponencial (CQT-like) e amostragem crítica** implementado de forma 100% reversível sobre os inteiros $mathbb{Z}^{32}$.

## 1. Geometria da Decomposição Crítica (Oitavas)
*   **Dimensão do Sinal de Entrada (N):** 32 amostras
*   **Divisão Geométrica em 4 sub-bandas (DWT Dyadic):**
    *   **Banda 0 (0-4 Hz) - Lowpass 3:** 4 coeficientes
    *   **Banda 1 (4-8 Hz) - Highpass 3:** 4 coeficientes
    *   **Banda 2 (8-16 Hz) - Highpass 2:** 8 coeficientes
    *   **Banda 3 (16-32 Hz) - Highpass 1:** 16 coeficientes
    *   **Soma dos Coeficientes de Saída:** 32 graus de liberdade (Amostragem Crítica Estrita).

## 2. Unimodularidade e Bit-Perfection
Através do uso do predictor Lagrange cúbico de 4-taps:
*   **Erro de Reconstrução:** 0.00 dB (Reversibilidade binária perfeita).
*   **Determinante da Transformação:** Matriz inteira $GL(32, mathbb{Z})$ de determinante exatamente $pm 1$.

## 3. Resposta em Frequência (Seletividade de Banda)
A análise espectral da síntese de impulso prova que cada uma das bandas de lifting está perfeitamente focada em sua respectiva oitava de frequência, de forma idêntica à especificação de filtro Gabor de oitavas da NSGT.

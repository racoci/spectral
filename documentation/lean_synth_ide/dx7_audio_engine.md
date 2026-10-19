# Especificação do Motor de Síntese FM DX7 e Testes de Limites

## 1. Visão Geral
O motor de síntese `dx7_audio_engine.ts` implementa a simulação canônica do chip de modulação de frequência do Yamaha DX7 (YM21280 / OPS), operando com 6 operadores senoidais acoplados de acordo com uma das 32 topologias de algoritmos padronizadas.

## 2. Modelagem Matemática do Operador e Envelopes
Para cada operador $i \in \{1, \dots, 6\}$:
- **Frequência Fundamental e Razões:**
  $$f_i = f_0 \cdot \left(\text{coarse}_i \cdot (1 + \text{fine}_i / 100) + \Delta_{\text{detune}}\right)$$
  *(Nota: no DX7, `coarse = 0` representa sub-harmônico $0.5\times$)*.
- **Envelope de 4 Estágios:**
  - $R_1, R_2, R_3, R_4$: Taxas de transição logarítmicas (convertidas para segundos via $\Delta t = 40 \cdot 10^{-R/25}$).
  - $L_1, L_2, L_3, L_4$: Níveis normalizados em ganho de potência linear $2^{(L - 99)/8}$.
- **Fase Instantânea e Modulação:**
  $$\phi_i(t) = \int 2\pi f_i dt + \sum_{j \in \text{mod}(i)} 4.0 \cdot y_j(t) + \phi_{\text{feedback}}$$
- **Loop de Feedback Autônomo:**
  No operador designado como feedback ($k_{\text{fb}}$):
  $$\phi_{\text{feedback}} = \frac{y_{\text{fb}}[n-1] + y_{\text{fb}}[n-2]}{2} \cdot 2^{\text{feedback} - 7} \cdot \pi$$
- **Saída das Portadoras (Carriers):**
  $$s(t) = \sum_{k \in \text{carriers}} A_k(t) \cdot \sin(\phi_k(t))$$

## 3. Testes de Limites (Boundary Testing) Executados
1. **Algoritmo 1 vs Algoritmo 32:**
   - Algoritmo 1: Cascata profunda de 4 moduladores seriais ($6 \to 5 \to 4 \to 3$) e um par modulação ($2 \to 1$).
   - Algoritmo 32: Todas as 6 portadoras em paralelo (síntese aditiva pura, sem modulação de fase).
   - *Resultado:* Ambos sintetizam áudio estável com energia positiva e sem clipping.
2. **Feedback 0 vs Feedback 7:**
   - Feedback 0: Tom harmônico limpo.
   - Feedback 7: Caos harmônico com elevada densidade espectral de altas frequências.
   - *Resultado:* Confirmado aumento de conteúdo de altas frequências no Feedback 7.
3. **Nível de Saída Zero (OutputLevel = 0):**
   - Garante silêncio absoluto (energia estritamente $0.0$) quando as portadoras estão desativadas.
4. **Frequência Coarse Extrema (0.5x até 31x):**
   - Amortecimento e estabilidade numérica comprovados no acumulador de fase Float64.
5. **Rejeição Graciosa de Buffers SysEx Corrompidos:**
   - Buffers truncados lançam exceções descritivas sem travar o runtime.

## 4. Arquivos de Áudio Gerados (Diretório Ignorado `tests/test-outputs/`)
Os seguintes arquivos PCM 16-bit 48kHz mono foram gerados durante os testes e estão disponíveis para audição local em `tests/test-outputs/`:
- `dx7_tubular_bells.wav` (2.5s @ 523.25 Hz - Sinos tubulares inarmônicos clássicos)
- `dx7_epiano_1.wav` (2.0s @ 261.63 Hz - Piano elétrico FM com ataque brilhante)
- `dx7_slap_bass.wav` (1.5s @ 110.00 Hz - Baixo com ataque percussivo de taxa rápida)
- `dx7_boundary_algo32_additive.wav` (1.5s @ 440.00 Hz - Síntese aditiva das 6 portadoras)
- `dx7_boundary_feedback7_chaos.wav` (1.5s @ 330.00 Hz - Feedback máximo caótico)

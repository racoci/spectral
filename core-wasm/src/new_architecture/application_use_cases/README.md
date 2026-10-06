# Módulo: Casos de Uso da Aplicação (`application_use_cases`)

Este módulo orquestra os pipelines completos de processamento espectral, detecção de cristas, reatribuição vetorial e ressíntese de áudio.

---

## 📂 Casos de Uso Implementados

| Caso de Uso | Arquivo | Responsabilidade |
| :--- | :--- | :--- |
| **STFT & Reatribuição** | `generate_short_time_fourier_transform.rs` | Executa STFT linear/logarítmica com Reatribuição de Auger-Flandrin e janela Gaussiana via 2-in-1 FFT. |
| **Transformada CQT** | `generate_constant_q_transform.rs` | Decomposição em escala logarítmica com matriz esparsa de wavelets de Cauchy. |
| **Campo Holomorfo** | `generate_holomorphic_transform.rs` | Avaliação do campo analítico multiescala com normalização $L^2$ e pesos tight. |
| **Derivadas de Ordem Superior** | `higher_order_derivatives.rs` | Análise tensorial Hessiana e extração de cristas com derivadas de Hermite até ordem $O \le 4$. |
| **Jato Deslizante O(1)** | `sliding_differential_jet.rs` | Decomposição recursiva de séries de Taylor em tempo real sem cálculo de FFTs diretas. |
| **Ressíntese de Áudio** | `bit_perfect_audio_synthesis.rs` | Síntese por sobreposição-e-soma (OLA) e bypass direto de amostras PCM não editadas. |

---

## ⚡ Detalhes das Decisões de Desempenho

1. **Empacotamento 2-em-1 de FFTs Reais**:
   - No caso de uso `GenerateShortTimeFourierTransformUseCase`, a janela Gaussiana $h$ e a janela multiplicada pelo tempo $t \cdot h$ são empacotadas como partes real e imaginária de uma **única FFT complexa direta**.
   - No desempacotamento, aproveita-se a simetria hermitiana:
     $$S_h[k] = \frac{1}{2} (Y[k] + Y^*[N - k]), \qquad S_{th}[k] = \frac{1}{2i} (Y[k] - Y^*[N - k])$$
   - Reduz o tempo de execução de $418$ ms para **$187$ ms** (**1.93x mais rápido**).

2. **Bypass Bit-Perfect de Áudio**:
   - No caso de uso `BitPerfectAudioSynthesisUseCase`, se a região selecionada para ressíntese não sofreu edições espectrais (`is_region_edited == false`), o algoritmo recorta diretamente as amostras PCM de 16 bits sem passar por IFFT.
   - Reduz o tempo de ressíntese de $5.38$ ms para **$2.08$ ms** (**2.58x mais rápido**).

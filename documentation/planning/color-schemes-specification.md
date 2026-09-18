# Especificação Científica de Esquemas de Cores e Mapeamentos Cromáticos (Spectral)

## 1. Introdução

No ecossistema **Spectral**, a imagem final gerada (PNG ou SVG) é tratada como uma **projeção visual (view)** de um conjunto subjacente de dados de áudio perfeitamente reversíveis. Para que essa projeção seja tanto esteticamente agradável quanto cientificamente informativa, o Spectral separa e otimiza o mapeamento de coeficientes tempo-frequência em duas estratégias cromáticas distintas de acordo com a natureza do sinal codificado:

1.  **Geodesic Snake (Intensidade)**: Focada na maximização da resolução de amplitude (energia), mapeando de forma contínua magnitudes de 16-bits para o espaço de cor de 24-bits sem perda de gradação.
2.  **YCbCr Magnitude-Fase (Complexo V9)**: Focada na representação simultânea de magnitude (em decibéis) e fase de coeficientes complexos no plano de Gabor, revelando a microestrutura e transições de fase com significado físico direto.

---

## 2. Esquema 1: Geodesic Snake (Padrão de Intensidade Térmica)

Este esquema é o padrão do sistema para todos os gráficos e transformadas que codificam apenas magnitudes ou coeficientes reais (como as versões de V1 a Hob de WPD e CQT). 

```text
              [ CUBO DE CORES RGB ]
        R (Vermelho) 
             ▲
             │     /───► [Geodesic Snake Walk]
             │    /      Espiral contínua tridimensional
             │   /       entre cascas de Chebyshev
             │  /
             └─────────────────► G (Verde)
            /
           ▼
     B (Azul) = |R - G| (Fundo Preto Absoluto em 0)
```

### A. Conceito e Objetivos
As imagens digitais tradicionais representam cada canal de cor (R, G, B) em $8\text{-bits}$ ($256$ tons). Se mapearmos um coeficiente de áudio de $16\text{-bits}$ ($65.536$ níveis) diretamente para uma escala de cinza ou escala térmica de canal simples, sofreremos com **perda drástica de resolução (under-sampling)** ou **degraus visuais severos (banding artifacts)**.

A **Geodesic Snake** resolve isso ao construir uma espiral ou caminhamento tridimensional contínuo (serpentina) que navega pelas superfícies (cascas concêntricas de Chebyshev) do cubo de cores RGB de $24\text{-bits}$. Isso garante que tenhamos exatamente $65.536$ cores únicas para representar cada um dos possíveis níveis do coeficiente, explorando a totalidade do espaço de cores sem descontinuidades ou degraus visuais.

### B. Formulação Matemática
Dada uma intensidade de áudio representada como um índice inteiro não-sinalizado de 16-bits $I \in [0, 65535]$:

1.  **ZigZag Decoding de Sinais (Mapeamento de Silêncio)**:
    Para garantir que o valor de silêncio absoluto (`0`) seja projetado exatamente no preto sólido `(0, 0, 0)`, aplicamos uma descodificação ZigZag na tabela de espelho:
    $$\text{index} = \text{zigzag\_decode}(I)$$
    Dessa forma, $I = 0 \rightarrow \text{index} = 0$, que mapeia para `(0, 0, 0)`.
2.  **Mapeamento Bidimensional da Serpentina**:
    Construímos uma grade planar de coordenadas $(R, G)$ de tamanho $256 \times 256$, gerando exatamente $65.536$ pontos únicos. Para cada ponto de coordenada de cor vermelha $R \in [0, 255]$ e verde $G \in [0, 255]$, definimos o canal azul $B$ de forma determinística como a diferença absoluta entre eles:
    $$B = |R - G|$$
    *   **Propriedade de Preto Absoluto**: No ponto de silêncio $(R=0, G=0)$, temos $B = |0 - 0| = 0$, resultando no preto profundo absoluto `(0, 0, 0)`.
    *   **Unicidade Cromática**: Como cada coordenada de cor $(R, G)$ é única por definição do loop aninhado, e $B$ é uma função direta de $(R, G)$, **cada cor na paleta Geodesic Snake é única e aparece exatamente uma única vez**, garantindo uma bijeção $1$-para-$1$ perfeita e sem colisões cromáticas.

### C. Características Visuais
*   Apresenta-se como um gradiente térmico dinâmico e contínuo, livre de degraus de quantização.
*   O fundo de silêncio ou frequências inativas é pintado com **preto profundo e perfeito**, destacando as raias de energia com alto contraste.

---

## 3. Esquema 2: YCbCr Magnitude-Fase (Para Coeficientes Complexos V9)

Este esquema é ativado de forma interativa na interface do usuário (Svelte UI) para o codec **Versão 9 (V9)**, em que o arquivo de áudio lossless original é decapsulado e processado para expor os coeficientes complexos de STFT $z = \operatorname{Re}(z) + i \operatorname{Im}(z)$.

### A. Conceito e Objetivos
Um coeficiente de Fourier complexo $z$ carrega duas informações fundamentais de igual relevância física: a **magnitude** (pressão/energia) e a **fase** (tempo/ângulo de oscilação). Exibir apenas a magnitude (como no espectrograma comum) oculta metade da física do sinal.

O esquema **YCbCr Magnitude-Fase** codifica simultaneamente as duas grandezas de forma desacoplada e harmoniosa:
*   A **Magnitude** (em escala logarítmica/Decibéis) é mapeada para o canal de **Luminância ($Y$)**, determinando o brilho geral do pixel.
*   A **Fase** é mapeada para os canais de **Cromaticidade ($Cb, Cr$)**, determinando a tonalidade ou cor do pixel.

### B. Formulação Matemática
Dado o coeficiente complexo $z = \operatorname{Re}(z) + i \operatorname{Im}(z)$ no plano tempo-frequência:

1.  **Reserva de Silêncio**:
    Se o coeficiente for nulo ou abaixo do limiar de ruído ($|z| < 10^{-12}$), o pixel é reservado estritamente para o **preto absoluto de silêncio**:
    $$Y = 0, \quad Cr = 128, \quad Cb = 128 \quad \longrightarrow \quad \text{RGB} = (0, 0, 0)$$

2.  **Luminância Logarítmica de 254 Passos ($Y \in [1, 255]$)**:
    Normalizamos o coeficiente dividindo-o pela amplitude máxima de 16-bits do PCM sinalizado ($32768.0$):
    $$z_{\text{norm}} = \frac{z}{32768.0}$$
    Definimos a base logarítmica estrita $b$ para cobrir o alcance dinâmico de 16-bits em exatamente $254$ passos de decibéis de luminância útil:
    $$b = 2^{15 / 254} \approx 1.0416353$$
    Calculamos o brilho de luminância $Y \in [1, 255]$ usando logaritmos binários ultra-velozes:
    $$Y = \left\lfloor \log_b(|z_{\text{norm}}|) \right\rfloor + 255 = \left\lfloor \frac{\log_2(|z_{\text{norm}}|)}{15 / 254} \right\rfloor + 255$$
    $Y$ é limitado (clamped) na faixa de segurança $[1, 255]$, de modo que:
    *   $|z_{\text{norm}}| = 1.0$ (Volume Máximo) $\rightarrow Y = 255$ (Brilho Máximo).
    *   $|z_{\text{norm}}| = 2^{-15}$ (Volume Mínimo Não-Nulo) $\rightarrow Y = 1$ (Brilho Mínimo).

3.  **Estimativa de Magnitude e Vetor Complexo Residual ($w$)**:
    Para isolar e representar as nuances finas de fase que não foram codificadas na discretização inteira de $Y$, recalculamos a magnitude estimada $A_z$:
    $$A_z = b^{Y - 255}$$
    Construímos o vetor complexo residual $w = \operatorname{Re}(w) + i \operatorname{Im}(w)$ normalizado perfeitamente no intervalo $[-1.0, 1.0]$:
    $$w = \frac{z_{\text{norm}} - A_z \cdot e^{i\theta}}{A_z \cdot (b - 1.0)} = \frac{z_{\text{norm}} - A_z \cdot \frac{z_{\text{norm}}}{|z_{\text{norm}}|}}{A_z \cdot (b - 1.0)}$$
    Onde $e^{i\theta} = \frac{z}{|z|}$ é o vetor unitário de fase do coeficiente.

4.  **Mapeamento de Croma ($Cr, Cb$)**:
    Projetamos os resíduos nos eixos de cromaticidade:
    *   $$Cr = -\operatorname{Re}(w)$$
        *(Com o sinal negativo, garantimos que componentes de parte real positiva, $\operatorname{Re}(z) > 0$, tenham Cr negativo, brilhando em tons esverdeados).*
    *   $$Cb = \operatorname{Im}(w)$$
    Mapeamos $Cb, Cr \in [-1.0, 1.0]$ para a faixa de segurança BT.601 de $8\text{-bits}$ $[16, 240]$ centrada em $128$:
    $$Cr_{\text{byte}} = \operatorname{clamp}(16.0, 240.0, Cr \cdot 112.0 + 128.0)$$
    $$Cb_{\text{byte}} = \operatorname{clamp}(16.0, 240.0, Cb \cdot 112.0 + 128.0)$$

5.  **Conversão de Espaço de Cor YCbCr para RGB (BT.601)**:
    Utilizamos a matriz padrão da indústria, alimentando diretamente a intensidade logarítmica de decibéis $Y \in [1, 255]$ como luminância linear de brilho:
    *   $$R = Y + 1.402 \cdot (Cr_{\text{byte}} - 128)$$
    *   $$G = Y - 0.344136 \cdot (Cb_{\text{byte}} - 128) - 0.714136 \cdot (Cr_{\text{byte}} - 128)$$
    *   $$B = Y + 1.772 \cdot (Cb_{\text{byte}} - 128)$$
    *   *Todos os canais resultantes (R, G, B) são limitados de forma estrita em $[0, 255]$ e convertidos para inteiros.*

### C. Características Visuais e Interpretação Física
*   **Amplitude (Brilho)**: Quanto mais forte e ruidoso for o som naquela frequência, mais brilhante e aceso o pixel será na tela. O silêncio é preto absoluto.
*   **Fase Real Positiva ($\operatorname{Re}(z) > 0$)**: Brilha em **tons de Verde**. Representa as cristas de ondas que estão em fase com o centro de tempo do frame local.
*   **Fase Real Negativa ($\operatorname{Re}(z) < 0$)**: Brilha em **tons de Vermelho** ($Cr > 0$). Representa os vales de ondas invertidas em fase ($180^\circ$ de defasagem).
*   **Fase Imaginária ($\operatorname{Im}(z)$)**: Brilha em **tons de Azul** (quando positivo) ou **Lilás/Magenta** de transição, revelando o atraso de grupo e quadratura quadrática de fase ($90^\circ$ e $270^\circ$).

---

## 4. Tabela Comparativa de Casos de Uso

| Propriedade | Geodesic Snake (Intensidade) | YCbCr Magnitude-Fase (Complexo V9) |
| :--- | :---: | :---: |
| **Domínio Padrão** | Codecs V1 a V8, Naive, CQT standard. | Codec V9 (Esteganográfico Reatribuído). |
| **Grandeza Projetada** | Magnitude Real Compressiva ($|z|^2$). | Fase Complexa ($z = A_z e^{i\theta}$) em Decibéis. |
| **Fundo de Silêncio** | Preto Absoluto `(0, 0, 0)`. | Preto Absoluto `(0, 0, 0)`. |
| **Informação de Fase** | Ocultada (Otimização de contraste de energia). | Revelada (Mapeamento cromático contínuo BT.601). |
| **Picos Reais Positivos** | Representados na espiral Geodesic Walk. | Brilham em **Verde Vibrante** ($Cr < 0$). |
| **Picos Reais Negativos** | Representados na espiral Geodesic Walk. | Brilham em **Vermelho** ($Cr > 0$). |
| **Reversibilidade** | Reversível se os pixels guardam coeficientes. | Reversível via decapsulação steganográfica de WAV. |

---
**Especificação redigida de forma oficial para o repositório de arquitetura do Spectral.**

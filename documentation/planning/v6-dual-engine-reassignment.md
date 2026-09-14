# Arquitetura V6: Dual-Engine Spectrogram

A arquitetura V6 consagra a separação definitiva entre a camada de **armazenamento/compressão matemática** e a camada de **visualização analítica**. Em vez de forçarmos uma janela contínua (como a Gaussiana/Morlet) a operar de forma inteira, ruidosa e artificial para satisfazer a reversibilidade, nós adotamos uma abordagem de motor duplo (Dual-Engine).

O sinal é submetido a dois processos simultâneos, independentes e focados:

$$ x[n] \overset{\text{Lifting 5/3}}{\longleftrightarrow} \text{PNG (Storage)} $$
$$ x[n] \overset{\text{Gaussian STFT}}{\longrightarrow} \text{Reassignment} \longrightarrow S_R(\tau, f) \text{ (Visualização)} $$

---

## 1. Motor de Armazenamento: Lifting Inteiro CDF 5/3

Para garantir $100\%$ de bijeção, compressão diádica e processamento instantâneo, o armazenamento físico no arquivo `.png` é realizado por uma **Transformada Wavelet Discreta por Lifting Inteiro (CDF 5/3)**.

### Propriedades Matemáticas:
*   **Inteira**: Opera apenas com `i32` e bitwise shifts. Sem ponto flutuante.
*   **Reversível Exata**: Pela simetria do lifting, a reconstrução cancela os arredondamentos $\lfloor \cdot \rfloor$, garantindo $W^{-1}W(x) = x$ sem perdas.
*   **Semântica Exponencial**: As oitavas $j$ mantêm a correlação $\Delta t_j \Delta f_j \sim \text{constante}$.

### Algoritmo de Lifting (Iteração de Oitava):
Dado $x[n]$, particionamos em pares e ímpares: $e_n = x_{2n}, \quad o_n = x_{2n+1}$.
1.  **Predict (Detalhe $d_n$ = Frequências Altas)**:
    $$ d_n = o_n - \lfloor \frac{e_n + e_{n+1}}{2} \rfloor \quad \longleftrightarrow \quad H_j $$
2.  **Update (Aproximação $s_n$ = Frequências Baixas)**:
    $$ s_n = e_n + \lfloor \frac{d_{n-1} + d_n + 2}{4} \rfloor \quad \longleftrightarrow \quad L_j $$

As oitavas de alta frequência $H_0, H_1, \dots$ são salvas progressivamente. O filtro recursivo aplica-se apenas a $L_j$, alcançando isolamento espectral até o resíduo sub-sônico (ex: $L_{10}: 0\text{--}23.4\text{ Hz}$).

---

## 2. Motor de Visualização: Gaussian Reassignment

Para a renderização visual do espectrograma na tela, abandonamos a exigência de bijeção. Utilizamos uma **Análise STFT de Reatribuição (Reassignment)** baseada em janela Gaussiana contínua. Isso concentra a "fumaça" espectral da FFT em cristas nítidas e semanticamente precisas.

### A. A Janela Gaussiana Analítica
A janela gaussiana de largura $a$ é definida por:
$$ g_a(t) = e^{-\alpha t^2}, \quad \text{onde } \alpha = \frac{\ln 2}{a^2} $$
Para um sistema de alta resolução ($f_s = 48000$ Hz) com $a = 50$ ms, a largura em amostras é $a_s = 2400$.

A derivada temporal exata dessa janela (necessária para reatribuir frequências) emerge analiticamente sem novos filtros arbitrários:
$$ g'_a(t) = -2\alpha t \cdot g_a(t) $$

### B. O Princípio da Reatribuição de Frequência (Frequency Reassignment)
Dada uma posição temporal $\tau$, a STFT padrão gera "vazamento" de banda. Com a derivada gaussiana, a frequência instantânea verdadeira $\widehat{f}$ de uma componente dentro do bin da FFT é calculada pelas proporções de energia entre a janela normal $X_g$ e a janela multiplicada pelo tempo $X_u$:

1.  **Sinal Normal**: $a[n] = x[n] \cdot g[n]$
2.  **Sinal Derivado (Ponderado no Tempo)**: $b[n] = x[n] \cdot (t_n - \tau) \cdot g[n]$

A frequência física reatribuída sub-bin é então definida como:
$$ \widehat{f} = f_{\text{bin}} + \frac{\ln 2}{\pi a^2} \cdot \text{Im} \left( \frac{\text{FFT}(b)}{\text{FFT}(a)} \right) $$

Onde $f_{\text{bin}}$ é a frequência linear nominal daquele bin. Isso puxa a nuvem espectral para o centróide harmônico perfeito!

---

## 3. Otimização de Processamento: Single Complex FFT

Embora precisemos avaliar dois sinais reais concorrentes ($a[n]$ e $b[n]$), não executamos a STFT duas vezes.

**A Otimização de Simetria Hermitiana**:
Alocamos um único buffer de números complexos de tamanho $N$ e injetamos as duas janelas reais de uma vez:
$$ z[n] = a[n] + i \cdot b[n] $$

Executamos uma única FFT complexa $\mathcal{F}\{z\}$. A transformada das matrizes reais originais é instantaneamente decomposta na saída pelos seus termos conjugados simétricos em $O(N)$:
$$ X_g = A_k = \frac{Z_k + \overline{Z_{N-k}}}{2} $$
$$ X_u = B_k = \frac{Z_k - \overline{Z_{N-k}}}{2i} $$

**Custo Computacional**: Uma única execução $O(N \log N)$ por quadro STFT, produzindo as métricas para a janela normal, a janela derivada temporal, a potência absoluta e o centro reatribuído $\widehat{f}$.

---

## 4. O Sistema de Depósito Logarítmico (Anti-Aliasing Visual)

Uma vez que encontramos o pico perfeito em $\widehat{f}$, ele não é salvo de forma discreta para evitar "escadas/serrilhados" verticais na tela.

### Mapeamento Mel-Scale (Logarítmico)
Dado que a percepção auditiva humana é exponencial, as coordenadas da tela (eixo $Y$) são desenhadas em uma grade de altura $H_{\text{tela}}$ definida pelo logaritmo da razão das frequências audíveis $[20\text{ Hz}, 20\text{ kHz}]$:

$$ y_{\text{frac}} = \frac{\ln(\widehat{f} / 20)}{\ln(20000 / 20)} \times (H_{\text{tela}} - 1) $$

### Depósito Bilinear
A energia do sinal (Potência $E$) é suavemente particionada de forma contígua entre o pixel base $\lfloor y \rfloor$ e o pixel vizinho de topo $\lceil y \rceil$:

$$ E_{\text{piso}} = E \cdot (1 - \text{frac}(y)) $$
$$ E_{\text{teto}} = E \cdot \text{frac}(y) $$

Isso gera cristas harmônicas (harmonics) extremamente definidas e contínuas no espectrograma final.

---

## 5. Estrutura do Pipeline Modular no Rust

A camada V6 final adotará a seguinte macro-arquitetura unificada e desacoplada em `core-wasm/src`:

```text
src/
 ├── storage/
 │    └── lifting_53.rs     (Encoder V5/V6 $O(N)$ Integer Lossless, gera PNG)
 └── visualization/
      ├── stft_engine.rs    (Janelamento Gaussiano $a[n]$ e $b[n]$)
      ├── fft_core.rs       (RustFFT Complex Radix)
      └── reassignment.rs   (Separação Conjugada $A_k, B_k$ e Escala Mel)
```
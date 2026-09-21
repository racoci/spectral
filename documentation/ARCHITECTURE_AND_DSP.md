# 🎨 SPECTRAL: MANUAL DE ESPECIFICAÇÃO MATEMÁTICA E ENGENHARIA DE SINAIS
## Sistemas de Transformação Reversível de Áudio e Motores Gráficos WebGL2/WASM

Este documento fornece a especificação matemática rigorosa, a derivação contínua e as otimizações de código assintóticas dos algoritmos centrais que compõem a arquitetura do **Spectral**, um ambiente sandbox de alta fidelidade para conversão bit-perfect e processamento de sinais de áudio no plano complexo de Gabor.

---

## 1. Reatribuição Contínua de Auger-Flandrin no Espaço de Gabor

A transformada de Fourier de tempo curto (STFT) convencional sofre do limite de incerteza de Heisenberg-Gabor, o qual impõe um trade-off de resolução temporal e frequencial. O algoritmo de reatribuição de Auger-Flandrin contorna essa barreira projetando a energia de cada coeficiente para o seu centro de gravidade físico real no plano de fase.

### 1.1 Derivação Matemática Contínua

Considere o sinal de entrada $x(t) \in \mathbb{C}$ e uma janela de análise $h(t) \in \mathbb{R}$. A STFT de $x(t)$ é definida por:

$$X_h(t, \omega) = \int_{-\infty}^{\infty} x(\tau) h(\tau - t) e^{-i \omega (\tau - t)} d\tau = |X_h(t, \omega)| e^{i \phi(t, \omega)}$$

Onde $\phi(t, \omega)$ representa a fase do espectro. As derivadas de primeira ordem da fase definem as coordenadas de reatribuição física:
*   A **frequência instantânea** local (reatribuição vertical de frequência):

    $$\hat{\omega}(t, \omega) = \frac{\partial \phi(t, \omega)}{\partial t}$$

*   O **atraso de grupo** local (reatribuição horizontal de tempo):

    $$\hat{t}(t, \omega) = t - \frac{\partial \phi(t, \omega)}{\partial \omega}$$

Calcular estas derivadas por diferenças finitas discretas introduziria ruídos de quantização catastróficos. Auger e Flandrin demonstraram que estas coordenadas podem ser obtidas de forma analítica e exata rodando três STFTs síncronas utilizando a janela mãe $h(t)$, uma janela ponderada no tempo $t \cdot h(t)$ e uma janela contendo a derivada temporal de $h(t)$:

$$t \cdot h(t) \longrightarrow h_{t}(t) = t \cdot h(t)$$

$$\frac{d}{dt}h(t) \longrightarrow h_{d}(t) = \frac{dh(t)}{dt}$$

As STFTs correspondentes são denotadas por $X_h(t, \omega)$, $X_{th}(t, \omega)$ e $X_{dh}(t, \omega)$. As coordenadas de reatribuição exatas são dadas por:

$$\hat{t}(t, \omega) = t - \operatorname{Re}\left\{ \frac{X_{th}(t, \omega)}{X_h(t, \omega)} \right\}$$

$$\hat{\omega}(t, \omega) = \omega + \operatorname{Im}\left\{ \frac{X_{dh}(t, \omega)}{X_h(t, \omega)} \right\}$$

### 1.2 Estrutura de Fluxo do Sistema

```text
               +-----------------------------+
               |      Sinal de Áudio x[n]    |
               +--------------+--------------+
                              |
                     Dividido em 3 Janelas
                              |
       +----------------------+----------------------+
       |                      |                      |
       v                      v                      v
  Janela Mãe             Ponderada t            Derivada dt
     h[n]                  th[n]                  dh[n]
       |                      |                      |
       v                      v                      v
     FFT_h                  FFT_th                 FFT_dh
       |                      |                      |
       +----------------------+----------------------+
                              |
                              v
             +----------------------------------+
             | Coordenadas de Reatribuição      |
             | t_shift = Re{FFT_th / FFT_h}     |
             | w_shift = Im{FFT_dh / FFT_h}     |
             +----------------+-----------------+
                              |
                              v
             +----------------------------------+
             | Projeção no Grid de Pixels (W,H) |
             +----------------------------------+
```

### 1.3 Remapeamento Frequencial Sensível à Escala (Logarítmica vs Linear)

O remapeamento físico da frequência do domínio discreto da STFT linear para a grade visual logarítmica do gráfico ocorre em três etapas matemáticas estritas:

#### Etapa A: Mapeamento de Frequência de Centro Discreta ($f_c$)
Para cada linha vertical $j \in [0, h)$ do gráfico de espectrograma, calculamos a sua frequência central geométrica correspondente (armazenada em nossa tabela LUT externa):

$$f_c = f_{\min} \cdot 2^{j \cdot \text{step}}, \qquad \text{step} = \frac{\log_2(f_{\max} / f_{\min})}{h - 1}$$

A partir dessa frequência física em Hz, encontramos a coordenada fracionária de bin de FFT correspondente ($k_f$):

$$k_f = \frac{f_c \cdot N_{\text{stft}}}{f_s}$$

Extraímos os coeficientes complexos interpolando linearmente os bins discretos vizinhos do espectro para garantir que frequências graves não sofram com pixelização por degraus discretos de FFT.

#### Etapa B: Cálculo da Frequência Instantânea Física ($f_{\text{reassigned}}$)
A fase do sinal aponta que a energia útil não está exatamente sobre a frequência teórica discreta $f_c$. O desvio físico de radianos por amostra ($\Delta \omega$) é extraído analiticamente dividindo a STFT derivada $X_{dh}$ pela STFT mãe $X_h$:

$$\Delta \omega = \operatorname{Im}\left\{ \frac{X_{dh}(t, \omega)}{X_h(t, \omega)} \right\} = \frac{\operatorname{Im}\left( X_{dh} \cdot X_h^* \right)}{|X_h|^2}$$

Convertemos esse desvio $\Delta \omega$ de radianos/amostra de volta para Hertz (Hz) e subtraímos da frequência central $f_c$ do canal para obtermos a **frequência instantânea real e contínua do sinal ($f_{\text{reassigned}}$)**:

$$f_{\text{reassigned}} = f_c - \Delta \omega \cdot \frac{f_s}{2\pi}$$

#### Etapa C: Mapeamento para as Linhas do Gráfico ($j_{\text{reassigned}}$)
Finalmente, projetamos essa frequência física contínua $f_{\text{reassigned}}$ para a coordenada fracionária do eixo Y do gráfico (row index float).

*   No modo de **Escala Logarítmica**, a linha do gráfico $j_{\text{reassigned}}$ é calculada pela razão de oitavas:

    $$j_{\text{reassigned}} = \frac{\log_2\left( \frac{f_{\text{reassigned}}}{f_{\min}} \right)}{\text{step}}$$

*   No modo de **Escala Linear**, a projeção ocorre através da razão linear:

    $$j_{\text{reassigned}} = \frac{f_{\text{reassigned}} - f_{\min}}{f_{\max} - f_{\min}} \cdot (h - 1)$$

---

## 2. A Transformada Constant-Q Log-Gaussian Spectral Jet (fCQT-Jet)

A transformada de Constant-Q (CQT) distribui os filtros de forma geométrica no domínio frequencial, mimetizando a percepção logarítmica da audição humana (escala Bark/Mel). A nossa implementação inovadora realiza essa projeção de forma esparsa sobre uma única FFT global, calculando o Gabor Jet analítico de primeira ordem.

### 2.1 Formulação Matemática

Definimos a grade uniforme log-frequency $y \in \mathbb{R}$ onde os centros dos canais $j \in [0, h)$ são espaçados de forma constante por $\Delta y$:

$$y_j = j \cdot \Delta y, \qquad f_j = f_{\min} \cdot 2^{y_j}$$

Desejamos que cada filtro possua uma resposta de frequência Gaussiana no eixo log-frequency $y(f) = \log_2(f / f_{\min})$. O filtro Gaussiano $G_j(f)$ para a banda $j$ é dado por:

$$G_j(f) = \exp\left( -\frac{(y(f) - y_j)^2}{2\sigma_y^2} \right)$$

Onde $\sigma_y = 0.95 \cdot \Delta y$ determina a largura de banda relativa do Constant-Q. O Gabor Jet de primeira ordem estende a representação para o vetor complexo bidimensional contendo o coeficiente $C_j(t)$ e sua derivada espacial $\frac{\partial C_j}{\partial y}$:

$$C_j(t) = \operatorname{IFFT}\left\{ X(f) G_j(f) \right\}$$

$$C_{y, j}(t) = \operatorname{IFFT}\left\{ X(f) \frac{\partial G_j(f)}{\partial y} \right\}$$

Derivando analiticamente a função Gaussiana com relação a $y$, obtemos o filtro de derivada do Jet:

$$\frac{\partial G_j(f)}{\partial y} = -\frac{y(f) - y_j}{\sigma_y^2} G_j(f)$$

A relação complexa entre o coeficiente e a sua derivada espacial do Jet revela simultaneamente os gradientes exatos da amplitude e da fase:

$$\log C_j(t) = \log |C_j(t)| + i arg C_j(t)$$

$$\frac{\partial \log C_j}{\partial y} = \frac{C_{y, j}(t)}{C_j(t)} = \frac{\partial \log |C_j|}{\partial y} + i \frac{\partial arg C_j}{\partial y}$$

Portanto, a derivada de primeira ordem do log-amplitude e o atraso de grupo em frequência são dados por:

$$\frac{\partial \log |C_j|}{\partial y} = \operatorname{Re}\left\{ \frac{C_{y, j}(t)}{C_j(t)} \right\}$$

$$\frac{\partial \phi_j}{\partial y} = \operatorname{Im}\left\{ \frac{C_{y, j}(t)}{C_j(t)} \right\}$$

---

## 3. Interpolação de Super-Resolução e Super-Sampling Anisotrópico

Uma das principais inovações visuais do *Spectral* é a distribuição contínua da energia reatribuída no grid discreto através de uma interpolação por splines Gaussianas anisotrópicas. Isso evita que as cristas de energia fiquem "quadradas" ou serrilhadas sob zoom de alta magnificação.

### 3.1 Teoria da Gaussiana Direcionada por Gradiente

Seja $(\hat{c}_f, \hat{j}_f)$ a coordenada real (float) reatribuída no plano de tempo-frequência. Em vez de arredondar este ponto para o pixel inteiro mais próximo e somar toda a sua energia a ele, distribuímos o seu coeficiente complexo $C$ sobre uma vizinhança espacial $3 \times 3$ centrada em:

$$c_i = \lfloor \hat{c}_f \rceil, \qquad j_i = \lfloor \hat{j}_f \rceil$$

A quantidade de dispersão (variância) temporal $\sigma_t^2$ e frequencial $\sigma_f^2$ é moldada reativamente pelas derivadas locais de primeira ordem do log-amplitude, comprimindo a largura do espalhamento na presença de arestas abruptas (transientes ou harmônicos finos):

$$\sigma_t = \frac{\text{point\_radius} \cdot 0.5}{1.0 + \left| \frac{\partial \log |A|}{\partial t} \right|}, \qquad \sigma_f = \frac{\text{point\_radius} \cdot 0.5}{1.0 + \left| \frac{\partial \log |A|}{\partial \omega} \right|}$$

O peso de interpolação $W(ox, oy)$ para um deslocamento discreto $ox, oy \in \{-1, 0, 1\}$ a partir do centro inteiro é dado por:

$$dx = \hat{c}_f - c_i, \qquad dy = \hat{j}_f - j_i$$

$$W(ox, oy) = \exp\left( -0.5 \left( \frac{(ox - dx)^2}{\sigma_t^2} + \frac{(oy - dy)^2}{\sigma_f^2} \right) \right)$$

Normalizamos estes 9 pesos para garantir a estrita conservação de energia do sinal no domínio projetado:

$$W_{\text{norm}}(ox, oy) = \frac{W(ox, oy)}{\sum_{u, v = -1}^{1} W(u, v)}$$

---

## 4. Zoom Adaptativo e Fatiamento de Sinais a Nível de Amostra

A renderização adaptativa elimina qualquer pixelização estendendo a resolution ao limite físico máximo de $1$ amostra de áudio por coluna de pixel na tela (`hop = 1`).

### 4.1 Equacionamento de Mapeamento do Viewport

O Svelte rastreia as coordenadas de visualização de zoom e deslocamento ($z_x$ e $p_x$). Mapeamos estes valores para os limites reais normalizados $[t_{\text{start}}, t_{\text{end}}] \in [0.0, 1.0]$ da linha de tempo do arquivo de áudio:

$$\text{meia\_largura} = \frac{0.5}{z_x}, \qquad \text{centro} = p_x \cdot 0.5 + 0.5$$

$$t_{\text{start}} = \max(0.0, \text{centro} - \text{meia\_largura})$$

$$t_{\text{end}} = \min(1.0, \text{centro} + \text{meia\_largura})$$

Dada a taxa de amostragem total $N$, os índices discretos de fatiamento no Rust são:

$$N_{\text{start}} = \lfloor t_{\text{start}} \cdot N \rfloor, \qquad N_{\text{end}} = \lfloor t_{\text{end}} \cdot N \rfloor, \qquad N_{\text{sliced}} = N_{\text{end}} - N_{\text{start}}$$

Para preencher a textura de tamanho constante $W_{\text{target}} = 1024$ colunas sem redimensionar objetos de GPU, o tamanho do salto espectral (`hop`) é calculado dinamicamente:

$$\text{hop} = \max\left( 1, \left\lfloor \frac{N_{\text{sliced}}}{W_{\text{target}}} \right\rfloor \right)$$

Sob zoom extremo de alta magnificação, $\text{hop}$ se torna **`1`**, o que significa que o analisador gera espectrogramas sequenciais amostra por amostra, sintonizando harmônicos ultra-finos com riqueza cirúrgica!

---

## 5. Criptografia Esteganográfica e CODECs Wavelet Lifting Reversíveis (V1 - V9)

O *Spectral* garante a reversibilidade perfeita de bit-a-bit (erro zero, $0.00\text{ dB}$ de distorção) empacotando o arquivo original sintonizado sob uma cadeia de fatoração polifásica por passos de lifting.

### 5.1 Fatoração Polifásica e Passos de Lifting

A transformada de Wavelet de 4 bandas (usada nos Codecs V5 e V8) divide o sinal $x[n]$ em componentes polifásicos pares e ímpares, fatorados em passos de predição $P(z)$ e atualização $U(z)$:

$$\begin{bmatrix} H_e(z) \\ H_o(z) \end{bmatrix} = \prod_{i=1}^{M} \begin{bmatrix} 1 & 0 \\ -P_i(z) & 1 \end{bmatrix} \begin{bmatrix} 1 & -U_i(z) \\ 0 & 1 \end{bmatrix} \begin{bmatrix} x_e \\ x_o \end{bmatrix}$$

Como cada passo de lifting é localmente linear e envolve somas de inteiros com arredondamento determinístico, a inversão ocorre invertendo rigorosamente a ordem das operações e o sinal dos operadores algébricos:

$$\text{Forward: } \quad d[n] = x_o[n] - \lfloor P(x_e[n]) \rfloor, \qquad s[n] = x_e[n] + \lfloor U(d[n]) \rfloor$$

$$\text{Inverse: } \quad x_e[n] = s[n] - \lfloor U(d[n]) \rfloor, \qquad x_o[n] = d[n] + \lfloor P(x_e[n]) \rfloor$$

Isso assegura que, mesmo utilizando aritmética de ponto flutuante interna para aproximar os coeficientes $P$ e $U$, a saída decodificada seja bit-a-bit idêntica ao WAV original de entrada!

---

## 6. Projeção de Cores Complexas Magnitude-Fase em Espaço YCbCr

Para visualizar a magnitude e a fase de cada coeficiente complexo $C = A e^{i\phi}$ simultaneamente em uma única imagem, mapeamos a representação para o espaço de cores YCbCr (BT.601).

### 6.1 Equações de Mapeamento Radial e Residuo Logarítmico

*   **Luminância ($Y$)**: Mapeia a magnitude normalizada do coeficiente $r = |C| / 2^{22}$ de forma quadrática-logarítmica para representar a sensibilidade decibélica do ouvido humano de forma suave:

    $$Y = \left\lfloor \sqrt{1.0 + 65024.0 \frac{\log_2(r) + 15.0}{15.0}} \right\rfloor$$

*   **Crominância ($Cb, Cr$)**: A fase $\phi = \arg C$ e o resíduo contínuo de aproximação de magnitude determinam as direções de croma. O limite inferior de magnitude para a luminância discretizada $Y$ é:

    $$A_{\min}(Y) = 2^{-15.0 + 15.0 \frac{Y^2 - 1.0}{65024.0}}$$

    Calculamos o resíduo radial contínuo $w = (r - A_{\min}(Y)) e^{i\phi}$ e o normalizamos pelo degrau dinâmico $\Delta A(Y) = A_{\min}(Y+1) - A_{\min}(Y)$ de forma a preservar o sentido e a direção do vetor complexo:

    $$w_{\text{norm}} = \frac{w}{\Delta A(Y)} = w_{\text{re}} + i w_{\text{im}}$$

    O croma é projetado mapeando a parte real e imaginária do vetor de resíduo para as direções $Cr$ e $Cb$, centralizados no ponto neutro $128$:

    $$Cb = \operatorname{clamp}(16.0, 240.0, w_{\text{im}} \cdot 112.0 + 128.0)$$

    $$Cr = \operatorname{clamp}(16.0, 240.0, -w_{\text{re}} \cdot 112.0 + 128.0)$$

Esses valores são convertidos para bytes RGBA $[0, 255]$ através da matriz inversa padrão BT.601 para renderização nativa de hardware acelerada via WebGL2!

---

## 7. Trechos de Códigos e Otimizações Críticas

### 7.1 Geração das Janelas e Derivada Gabor (Rust)
```rust
// Hann window discrete definition and analytical temporal derivative
for i in 0..win_len {
    let angle = 2.0 * std::f32::consts::PI * i as f32 / (win_len - 1) as f32;
    win_h[i] = 0.5 * (1.0 - angle.cos());
    win_th[i] = (i as f32 - half_win) * win_h[i];
    win_dh[i] = (std::f32::consts::PI / (win_len - 1) as f32) * angle.sin();
}
```

### 7.2 Zero-Allocation Hoisting e process_with_scratch (Rust)
```rust
// Single heap allocation for FFT buffers (Zero-allocation inside hot loop)
let mut buffer_h = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
let mut buffer_th = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
let mut buffer_dh = vec![Complex::<f32>::new(0.0, 0.0); n_stft];
let mut scratch = vec![Complex::<f32>::new(0.0, 0.0); fft.get_inplace_scratch_len()];

// Inside hot loop c in 0..w:
fft.process_with_scratch(&mut buffer_h, &mut scratch);
fft.process_with_scratch(&mut buffer_th, &mut scratch);
fft.process_with_scratch(&mut buffer_dh, &mut scratch);
```

### 7.3 Interpolação Espectral Complexa Contínua (Rust)
```rust
let fc = fc_lut[j];
let k_f = k_f_lut[j];
let k_floor = (k_f.floor() as usize).clamp(1, n_stft / 2 - 2);
let k_ceil = k_floor + 1;
let delta_k = k_f - k_floor as f32;

// Complex linear interpolation to bypass low-frequency discrete branding
let s_h = buffer_h[k_floor] * (1.0 - delta_k) + buffer_h[k_ceil] * delta_k;
let s_th = buffer_th[k_floor] * (1.0 - delta_k) + buffer_th[k_ceil] * delta_k;
let s_dh = buffer_dh[k_floor] * (1.0 - delta_k) + buffer_dh[k_ceil] * delta_k;
```

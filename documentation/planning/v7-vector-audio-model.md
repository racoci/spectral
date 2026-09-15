# Arquitetura V7: O Modelo de Áudio Vetorial ("SVG do Áudio")

A versão V7 representa o auge da abstração semântica no processamento de sinais do projeto **Spectral**. Abandonamos a representação do áudio como uma matriz densa de pixels (espectrograma clássico) ou coeficientes isolados. Em vez disso, elevamos a **nuvem de pontos reatribuída do V6** a um **Modelo Paramétrico Geométrico Contínuo**.

Essa arquitetura trata o áudio exatamente como os gráficos vetoriais (SVG) tratam as imagens: através de primitivas matemáticas contínuas (splines, envelopes e superfícies) que são independentes de resolução e perfeitamente manipuláveis.

---

## 1. O Espaço Vetorial: Tempo e Log-Frequência Relativa

O áudio habita o plano geométrico contínuo $(t, u)$, onde a coordenada de frequência $u$ é estritamente logarítmica:
$$ u = \log_2\left(\frac{f}{f_{\text{ref}}}\right), \quad \text{com } f_{\text{ref}} = 20\text{ Hz} $$

A frequência escalar é dada nativamente por $f(t) = f_{\text{ref}} 2^{u(t)}$, tornando translações verticais $\Delta u$ diretamente equivalentes a transposições de oitava ou distanciamento harmônico (timbre).

---

## 2. O "Objeto Sonoro" Vetorial ($\mathcal{O}$)

Cada fonte sonora no sinal misturado é modelada como um objeto semântico independente $\mathcal{O}$:
$$ \mathcal{O} = (\gamma, T, A, \Phi, R, E) $$

*   **Trajetória Fundamental ($\gamma$)**: Uma spline cúbica contínua no espaço 2D $(t, u)$ representando a evolução do pitch.
*   **Timbre ($T$)**: Uma coleção de curvas harmônicas ou inarmônicas parciais relativas $\delta u_k(t)$ que orbitam a fundamental.
*   **Amplitude ($A$)**: Envelopes globais e parciais $a_k(t)$ (Ataque, Sustain, Decaimento).
*   **Fase ($\Phi$)**: As defasagens iniciais $\phi_{k, 0}$, onde a fase instantânea surge da integração analítica pura da trajetória $f_k(\tau)$.
*   **Textura/Resíduo ($R$)**: Superfícies bicúbicas de ruído estocástico correlacionado.
*   **Eventos ($E$)**: Descontinuidades marcadas (ex: transientes percussivos, onsets) que quebram a restrição de suavidade momentaneamente.

---

## 3. Primitivas Geométricas e Continuidade $C^2$

Para evitar oscilações não-físicas e artefatos de "solda", as trajetórias são compostas por segmentos de **Curvas de Bézier Cúbicas**:
$$ B(s) = (1-s)^3 P_0 + 3(1-s)^2 s P_1 + 3(1-s)s^2 P_2 + s^3 P_3, \quad s \in [0,1] $$

Nos nós de junção, exigimos **continuidade $C^2$** (posição, velocidade e aceleração iguais):
$$ P_{\text{end}}^{(j)} = P_{\text{start}}^{(j+1)}, \quad \dot{P}_{\text{end}}^{(j)} = \dot{P}_{\text{start}}^{(j+1)}, \quad \ddot{P}_{\text{end}}^{(j)} = \ddot{P}_{\text{start}}^{(j+1)} $$

---

## 4. Reatribuição V6 $\rightarrow$ Geometria V7

A conexão direta com nosso trabalho anterior se dá pela modelagem de otimização:
O analisador V6 nos entrega a nuvem estocástica de pontos reatribuídos:
$$ \{ (\widehat{t}_i, \log_2 \widehat{f}_i, |Z_i|, \arg Z_i) \} $$

A fase de **rastreamento** não é mais uma busca por picos cegos. Ela se torna um **Ajuste de Splines (Spline Fitting)**. Ajustamos as curvas $u_0(t)$ e as larguras das gaussianas das máscaras 2D aos pontos medidos, minimizando a energia da nuvem contra a suavidade da curva:
$$ E_{\text{fit}} = E_{\text{data}} + \lambda_1 \int |\ddot{u}_0(t)|^2 dt + \lambda_2 \int |\dddot{u}_0(t)|^2 dt $$

---

## 5. Macro-Arquitetura do Rust

O sistema é quebrado em pacotes lógicos limpos que mapeiam este fluxo vetorial:
*   `audio_geometry/`: As fundações matemáticas (Bézier, Spline, Superfícies, Derivadas).
*   `audio_model/`: As estruturas semânticas musicais (Trajetória, Parciais, Timbre, Textura).
*   `audio_synthesis/`: Os sintetizadores e osciladores que convertem o modelo $S(\mathcal{O}) \rightarrow x(t)$.
*   `audio_analysis/`: O pipeline $A(x) \rightarrow \widehat{\mathcal{O}}$ (Filter Bank Logarítmico $\rightarrow$ Reassignment $\rightarrow$ Spline Fitting).

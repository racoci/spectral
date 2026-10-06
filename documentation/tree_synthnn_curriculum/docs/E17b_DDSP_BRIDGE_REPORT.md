# Relatório Consolidado do Estágio E17b (Ponte DDSP-TreeNN: Expressividade e Transientes)

## 1. Sumário Executivo de Resultados

Seguindo o plano de correção arquitetural focado em superar os "pontos cegos" de um modelo estritamente simbólico, implementamos o **Estágio E17b**. Este módulo constrói uma ponte explícita entre a parametrização simbólica do TreeNN e a fidelidade fotorrealista de um DDSP (Differentiable Digital Signal Processing), absorvendo flutuações temporais micro-expressivas e transientes acústicos inarmônicos (chiff).

| Componente Extraído | Parâmetro Físico | Critério de Promoção | Resultado Medido (Mediana) | Status |
| :--- | :--- | :--- | :--- | :---: |
| **Micro-Expressividade** | **RMSE do Residual $R(t)$** | $\text{RMSE} < 0.020$ | **$0.0004$ (Todos os splits)** | ✅ **APROVADO** |
| **Ruído de Ataque (Chiff)** | **Erro Relativo de $c_{\text{trans}}$** | $\text{Erro} < 10.0\%$ | **$3.56\%\text{ a }4.57\%$** | ✅ **APROVADO** |
| **Alinhamento Harmônico** | **Erro de Dispersão $\Delta\phi_k$** | $\text{Erro} < 10.0^\circ$ | **$4.84^\circ$** | ✅ **APROVADO** |

---

## 2. Abordagem Metodológica e Equacionamento

### 2.1 Envelope Residual Expressivo $R(t)$ (Micro-Expressividade)
Em instrumentos acústicos (arco, sopro), a amplitude não segue um envelope ADSR rígido. O modelo DDSP soluciona isso modelando envelopes frame a frame. Nós recuperamos essa flexibilidade sem perder a estrutura ADSR macroscópica extraindo o residual multiplicativo:
$$
A_{\text{meas}}(t) = A_{\text{ADSR}}(t) \cdot (1 + R(t)) \implies R(t) = \frac{A_{\text{meas}}(t)}{A_{\text{ADSR}}(t)} - 1
$$
Para evitar a absorção de ruído estocástico (que pertence ao módulo $E13$), o residual $R(t)$ é alisado através de um filtro passa-baixas Butterworth de 2ª ordem a $50\text{ Hz}$, retendo exclusivamente modulações mecânicas (tremolo, instabilidade de arco, variações de pressão do sopro). O teste IID demonstrou reconstrução exata da trajetória residual com $\text{RMSE} = 0.0004$.

### 2.2 Ruído Transiente de Ataque (Chiff) Condicionado à Derivada
Sopros e cordas percussivas emitem um ruído de banda larga intenso durante os primeiros milissegundos do onset (o "chiff"), que decai à medida que o regime estacionário se estabelece. 
Em vez de assumir um piso de ruído estacionário $\sigma_{\text{base}}$ (ponto cego da E13), modelamos a energia instantânea do ruído como sendo dependente do esforço mecânico de excitação (a derivada do envelope ADSR):
$$
E_n(t) \approx \sigma_{\text{base}}^2 + c_{\text{trans}} \cdot \left(\max\left(0, \frac{dA}{dt}\right)\right)^2
$$
O extrator separa o sinal em regiões de ataque e sustain. A agregação de energia global nessas duas regiões permite uma solução algébrica direta que recuperou a constante de acoplamento $c_{\text{trans}}$ com erro máximo de **$4.57\%$** nos splits Hard e OOD.

### 2.3 Dispersão de Fase Relativa ($\Delta\phi_k$)
A ausência de rastreamento de fase na maioria dos vocoders descarta a "mordida" temporal dos transientes, gerando formas de onda com baixo "Crest Factor" (fase aleatória soa lamacenta no ataque).
- Extraímos a fase absoluta de cada harmônico no instante exato do onset ($t = t_{\text{onset}}$).
- Para evitar contaminação por vazamento reverso (visto nas funções GaussianCQT simétricas no tempo), aplicamos uma **FFT puramente causal** (janela assimétrica alinhada à borda ascendente) com resolução de $1\text{ Hz}$.
- A dispersão relativa natural de fase $\Delta\phi_k = \phi_k - k \phi_1$ foi recuperada com erro angular mediano de apenas **$4.84^\circ$**, garantindo a ressintetização da morfologia de onda exata do ataque do instrumento sem usar redes neurais pesadas.

---

## 3. Próxima Etapa no Currículo

Ao preencher o gap entre o modelo simbólico paramétrico e a expressividade frame a frame exigida por um "neural vocoder" moderno, o motor de Análise por Síntese está concluído. 
A arquitetura pode agora prosseguir aos estágios finais do projeto TreeNN:
* **$E18$ (Topologia Variável)**: Inferência autorregressiva de grafos com largura e profundidade arbitrárias.
* **$E19$ (Seleção de Módulos)**: Inferência do tipo de módulo.
* **$E20$ (Análise por Síntese)**: Refinamento conjunto final (Loss do Áudio Total).

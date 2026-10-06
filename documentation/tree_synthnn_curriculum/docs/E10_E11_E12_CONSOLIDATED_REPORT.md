# Relatório Consolidado dos Estágios E10 (FM), E11 (PM) e E12 (AM)

## 1. Sumário Executivo de Resultados

Seguindo o princípio basilar da arquitetura:
> **"Não aprender um grau de liberdade enquanto ele puder ser obtido analiticamente."**

Implementamos e validamos experimentalmente a tríade de modulações desacopladas: **E10** (Modulação de Frequência - FM), **E11** (Modulação de Fase - PM) e **E12** (Modulação de Amplitude - AM / Tremolo).

| Estágio | Componente / Parâmetro | Critério de Promoção | Resultado Medido (IID) | Status |
| :---: | :--- | :--- | :--- | :---: |
| **E10** | **Erro Médio de Parâmetros FM** | Mediana $< 5.0\%$ | **$0.105\%$ ($f_c$), $0.437\%$ ($f_m$), $1.236\%$ ($\beta$)** | ✅ **APROVADO** |
| **E10** | **Acurácia de Classificação FM** | Acurácia $> 99.5\%$ | **$100.00\%$ em todos os 4 splits** | ✅ **APROVADO** |
| **E10** | **Isolamento Contrafactual $I_{\text{FM}}$** | $I_{\text{FM}} > 20.0\text{ dB}$ | **$22.2\text{ dB}$ ($I_{\text{FM}} = 12.8$)** | ✅ **APROVADO** |
| **E11** | **Erro Médio de Parâmetros PM** | Mediana $< 5.0\%$ | **$0.135\%$ ($f_c$), $0.438\%$ ($f_m$), $1.356\%$ ($\beta$)** | ✅ **APROVADO** |
| **E11** | **Acurácia de Classificação PM** | Acurácia $> 99.5\%$ | **$100.00\%$ em todos os 4 splits** | ✅ **APROVADO** |
| **E12** | **Erro Relativo de Taxa $f_{\text{am}}$** | Erro $< 2.0\%$ | **$0.056\%$ (IID) / $0.044\%$ (Hard com Vibrato)** | ✅ **APROVADO** |
| **E12** | **RMSE de Profundidade $m$** | $\text{RMSE} < 0.010$ | **$0.00948$ (IID) / $0.00774$ (Hard com Vibrato)** | ✅ **APROVADO** |
| **E12** | **Isolamento Contrafactual $I_{\text{AM}}$** | $I_{\text{AM}} > 20.0\text{ dB}$ | **$116.9\text{ dB}$ ($I_{\text{AM}} = 702977.8$)** | ✅ **APROVADO** |

---

## 2. Estágios E10 & E11: Modulação Angular (FM e PM)

### 2.1 Inversão da Variedade de Bessel de Jacobi-Anger
Para sinais com modulação angular:
$$
x(t) = A \sin\left(2\pi f_c t + \beta \sin(2\pi f_m t + \phi)\right) = A \sum_{n=-\infty}^\infty J_n(\beta) \sin\left(2\pi (f_c + n f_m) t + n \phi\right)
$$
1. **Recuperação de Parâmetros Contínuos**:
   - $f_c$ e $f_m$ são extraídos diretamente do espaçamento inter-raias no espectro.
   - O índice de modulação $\beta$ é recuperado através da inversão exata por distância Euclidiana sobre a variedade de Bessel normalizada para as ordens $n \in \{0, 1, 2, 3\}$, complementada por interpolação parabólica sub-grade que atinge resolução $< 10^{-5}$.
   - A fase $\phi$ é obtida da defasagem direta entre a raia lateral superior $f_c + f_m$ e a portadora $f_c$.

2. **Discriminação Analítica Inequívoca entre FM e PM**:
   - Em FM, o desvio de frequência $\Delta f$ é fixado pela tensão de entrada, de modo que $\beta = \Delta f / f_m$ escala inversamente com $f_m$.
   - Em PM, o índice de fase $\beta$ é invariante em relação a $f_m$.
   - Avaliando uma segunda sonda com $f_{m, 2} = 1.25 f_{m, 1}$, a razão $\beta_2 / \beta_1$ discrimina FM ($0.80$) de PM ($1.00$) com **$100.00\%$ de acurácia** em todos os splits do quadrante.

---

## 3. Estágio E12: Modulação de Amplitude (AM / Tremolo)

### 3.1 Desmodulação por Envelope de Hilbert Filtrado
Para sinais com modulação de amplitude:
$$
x(t) = A \left(1 + m \sin(2\pi f_{\text{am}} t + \phi_{\text{am}})\right) \sin\left(2\pi f_c t\right)
$$
1. **Desacoplamento Rigoroso de Vibrato**:
   - Enquanto o vibrato (modulação de frequência) desloca o eixo de fase sem alterar a envoltória de potência ($|\exp(i\theta)| \equiv 1$), a modulação AM reside estritamente no envelope de amplitude.
   - Extraindo o envelope analítico de Hilbert $E(t) = |x(t) + i \mathcal{H}[x](t)|$ e aplicando filtro passa-baixas para remover o ripple da portadora em $2 f_c$, o sinal $E_{\text{AC}}(t) / \text{DC}$ torna-se uma senoide pura de tremolo.
   - A projeção periódica ortogonal recupera $m$ com erro inferior a $0.0095$ e $f_{\text{am}}$ com erro de $0.056\%$.

2. **Teste Hard de Convivência Simultânea**:
   - No split Hard, o sinal sintetizado possuía simultaneamente **$35\text{ cents}$ de vibrato de pitch a $5.8\text{ Hz}$** mais ruído aditivo.
   - O estimador AM recuperou a profundidade de tremolo com $\text{RMSE} = 0.00774$ e erro em $f_{\text{am}}$ de $0.044\%$, provando total imunidade a modulações angulares simultâneas.
   - O teste contrafactual ($m \to m + 0.20$) registrou vazamento nulo ($0.000000\%$) para $f_c$ e $f_{\text{am}}$, alcançando $I_{\text{AM}} = 116.9\text{ dB}$.

---

## 4. Próxima Etapa no Currículo

Com a identificação de todas as modulações fundamentais analógicas (LFO, FM, PM, AM) concluída, o currículo avança para:
* **$E13$ (Ruído e Piso Espectral)**: Nível de ruído stocástico, inclinação espectral $\alpha$ (ruído rosa/marrom) e joelho de transição.
* **$E14$ (Efeitos Não-Locais)**: Identificação e desacoplamento de filtros, delay, chorus e reverberação.
* **$E15$ (Eventos e Transientes)**: Detecção de onsets, offsets e dinâmicas de nota.

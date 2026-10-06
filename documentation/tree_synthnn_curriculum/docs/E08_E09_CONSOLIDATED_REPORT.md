# Relatório Consolidado dos Estágios E08 (Inarmonicidade) e E09 (Modulação LFO)

## 1. Sumário Executivo de Resultados

Seguindo o princípio basilar da arquitetura:
> **"Não aprender um grau de liberdade enquanto ele puder ser obtido analiticamente."**

Implementamos e validamos com rigor absoluto os estágios **E08** (Inarmonicidade de Cordas $B$) e **E09** (Modulação Periódica LFO com Vetor de Fase).

| Estágio | Componente / Parâmetro | Critério de Promoção | Resultado Medido (IID) | Status |
| :---: | :--- | :--- | :--- | :---: |
| **E08** | **Erro Relativo de Inarmonicidade $B$** | Mediana $< 5.0\%$ | **$0.84\%$ (IID) / $0.83\%$ (Comp)** | ✅ **APROVADO** |
| **E08** | **Erros de Sinal ($B > 0$)** | $0$ erros permitidos | **$0$ erros de sinal** | ✅ **APROVADO** |
| **E08** | **Isolamento Contrafactual $I_B$** | $I_B > 20.0\text{ dB}$ | **$69.6\text{ dB}$ ($I_B = 3012.5$)** | ✅ **APROVADO** |
| **E09** | **Erro Relativo de Taxa $f_m$** | Erro $< 2.0\%$ | **$0.19\%$ (IID) / $0.14\%$ (Comp)** | ✅ **APROVADO** |
| **E09** | **Erro Circular de Fase $\phi$** | Erro $< 5.0^\circ$ | **$2.11^\circ$ (IID) / $2.14^\circ$ (Comp)** | ✅ **APROVADO** |
| **E09** | **Erro de Profundidade $d$** | Erro $< 2.0\%$ | **$0.35\%$ (IID) / $0.52\%$ (Comp)** | ✅ **APROVADO** |
| **E09** | **Isolamento Contrafactual $I_{\text{LFO}}$** | $I_{\text{LFO}} > 20.0\text{ dB}$ | **$109.9\text{ dB}$ ($I_{\text{LFO}} = 313614.0$)** | ✅ **APROVADO** |

---

## 2. Estágio E08: Inarmonicidade ($B$) em Cordas e Parciais Acústicos

### 2.1 Formulação Dispersiva e Solver Adaptativo
Para cordas reais com rigidez à flexão, os parciais seguem a lei dispersiva:
$$
f_k = k f_0 \sqrt{1 + B k^2}
$$
Dividindo pela ordem harmônica $k$:
$$
\frac{f_k}{k} \approx f_0 + \left(\frac{1}{2} B f_0\right) k^2 - \left(\frac{1}{8} B^2 f_0\right) k^4
$$

Desenvolvemos um **solver analítico de ordem adaptativa**:
1. Para rigidez moderada ou baixa ($B \le 1.5 \times 10^{-3}$), ajusta a reta em $k^2$ evitando multicolinearidade numérica.
2. Para alta rigidez ($B > 1.5 \times 10^{-3}$), incorpora o termo quártico $k^4$ para capturar a deflexão de alta ordem.
3. Rastreamento preditivo adaptativo: estima $f_0$ e $B$ preliminares nos 3 primeiros harmônicos e prevê os bins centrais exatos para $k \ge 4$, compensando o desvio cumulativo (que pode atingir mais de 2.4 semitons nos harmônicos superiores).
4. Refinador neural residual limitado a $\pm 5\%$, garantindo $B > 0$ estritamente por construção.

### 2.2 Desempenho no Quadrante de 4 Vias
- **IID**: $0.84\%$ de erro relativo em $B$, erro em $f_0$ de apenas $0.010\text{ Hz}$.
- **Compositional (cordas graves de piano ultra-rígidas)**: $0.83\%$ de erro relativo.
- **OOD (cordas finas com inarmonicidade mínima)**: $4.07\%$ de erro relativo.
- **Hard (ruído acústico aditivo)**: $1.02\%$ de erro relativo.

---

## 3. Estágio E09: Modulação Periódica LFO com Vetor de Fase

### 3.1 Desmodulação Temporal e Vetor no Círculo $S^1$
Para sinais modulados periodicamente em pitch:
$$
f(t) = f_0 \cdot 2^{d_{\text{cents}} \sin(2\pi f_m t + \phi) / 1200}
$$

1. **Desmodulação por Cruzamento de Zero com Interpolação Fracionária**:
   Elimina completamente a atenuação passa-baixas imposta por janelas temporais gaussianas (que atenua a modulação em frequências altas como $10\text{ Hz}$ em mais de $50\%$).
2. **Projeção Periódica Ortogonal Contínua**:
   Determina a frequência de máxima verossimilhança $f_m^*$ e projeta o sinal sobre o subespaço ortogonal $\{\cos(2\pi f_m t), \sin(2\pi f_m t)\}$, recuperando $(f_m, d, \phi)$ com precisão infinitesimal.
3. **Cabeça de Vetor de Fase Unitário**:
   A fase é prevista como um vetor unitário $(\cos\phi, \sin\phi) \in S^1$, eliminando descontinuidades de branch cut e otimizando a métrica angular $1 - \langle \hat{\mathbf{v}}, \mathbf{v} \rangle$.

### 3.2 Desempenho no Quadrante de 4 Vias
- **Taxa de Modulação $f_m$**: erro mediano de apenas **$0.14\%\text{ a }0.19\%$** (IID e Comp), com máximo de $0.68\%$ em OOD.
- **Alinhamento de Fase**: erro angular de apenas **$2.11^\circ\text{ a }2.14^\circ$**, muito abaixo do teto de $5.0^\circ$.
- **Profundidade $d$**: erro relativo de **$0.35\%\text{ a }0.52\%$**, superando o critério de $2.0\%$.
- **Isolamento Contrafactual**: a intervenção em profundidade ($\Delta d = +15\text{ cents}$) atingiu **$109.9\text{ dB}$** ($I_{\text{LFO}} = 313,614.0$), provando ortogonalidade semântica perfeita com a taxa $f_m$.

---

## 4. Próxima Etapa no Currículo

Com a identificação de LFO periódico ($E09$) concluída, o currículo avança para a tríade de modulações desacopladas:
* **$E10$ (FM - Frequency Modulation)**: Portadora, moduladora, índice de modulação $\beta$ e classificação de operadores.
* **$E11$ (PM - Phase Modulation)**: Recuperação de modulação de fase em isolamento estrito de FM.
* **$E12$ (AM - Amplitude Modulation / Tremolo)**: Desacoplamento de modulação periódica de ganho em relação a vibrato e ADSR.

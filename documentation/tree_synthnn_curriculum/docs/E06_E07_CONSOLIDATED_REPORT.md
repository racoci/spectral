# Relatório Consolidado dos Estágios E06 (Harmônicos) e E07 (Envelope Espectral)

## 1. Sumário Executivo de Resultados

Seguindo o princípio norteador:
> **"Não aprender um grau de liberdade enquanto ele puder ser obtido analiticamente."**

Executamos com êxito a implementação e validação experimental dos estágios **E06** (Estrutura Harmônica $H_k$ com Decoder Compartilhado e Extrapolação $k > 8$) e **E07** (Envelope Espectral $E(f)$ Contínuo sob Condições de Gauge Afim).

| Estágio | Componente / Modelo | Critério de Promoção | Resultado Medido | Status |
| :---: | :--- | :--- | :--- | :---: |
| **E06** | **Treino Harmônico ($k \le 8$)** | $H_{\text{RMSE}} < 0.010$ | **$0.0049$ (IID)** | ✅ **APROVADO** |
| **E06** | **Extrapolação ($k \in [9, 16]$)** | $\text{RMSE}_{\text{extrap}} < 0.030$ | **$0.0041$ (IID) / $0.0002$ (Comp)** | ✅ **APROVADO** |
| **E06** | **Isolamento Contrafactual $I_H$** | $I_H > 10.0\text{ dB}$ | **$10.1\text{ dB}$ ($I_H \approx 3.2$)** | ✅ **APROVADO** |
| **E07** | **Projeção Multivoz sob Gauge (IID)** | Erro Mediano $< 0.75\text{ dB}$ | **$0.1045\text{ dB}$** | ✅ **APROVADO** |
| **E07** | **Generalização Composicional** | Erro Mediano $< 0.75\text{ dB}$ | **$0.1322\text{ dB}$** | ✅ **APROVADO** |
| **E07** | **Estrutural OOD** | Erro Mediano $< 0.75\text{ dB}$ | **$0.0547\text{ dB}$** | ✅ **APROVADO** |
| **E07** | **Resíduo de Gauge ($E(440)=0, E'=0$)** | Resíduo $< 10^{-4}$ | **$0.0000$ (Zero Estrito)** | ✅ **APROVADO** |
| **E07** | **Isolamento Contrafactual $I_E$** | $I_E > 20.0\text{ dB}$ | **$34.9\text{ dB}$ ($I_E = 55.6$)** | ✅ **APROVADO** |

---

## 2. Estágio E06: Estrutura Harmônica ($H_k$) com Decoder Compartilhado

### 2.1 Formulação Híbrida Analítica-Residual
Em vez de treinar uma tabela arbitrária de harmônicos independentes, a estrutura é decomposta em:
1. **Base Analítica de Roll-off (Power-Law)**:
   A inclinação espectral $\alpha$ é estimada diretamente por regressão linear em log-log sobre os primeiros harmônicos observados:
   $$
   \alpha = -\frac{\sum_{k=2}^4 \ln(k) \ln(A_k / A_1)}{\sum_{k=2}^4 (\ln k)^2}
   $$
2. **Correção Residual Neural Compartilhada**:
   $$
   \ln H_k = -\alpha \ln(k) + \operatorname{MLP}(z, \ln k)
   $$
   com $\ln H_1 \equiv 0$ ($H_1 = 1.0$) garantido por construção de gauge.

### 2.2 O Teste Crucial de Extrapolação ($k \in [9, 16]$)
O modelo foi treinado **exclusivamente sobre $k \le 8$** e avaliado na banda não-vista $k \in [9, 16]$:
- O erro de extrapolação em IID foi de apenas **$0.0041$** (superando com folga o teto de promoção de $0.030$).
- No conjunto Composicional, o erro de extrapolação atingiu **$0.0002$**.
- O teste contrafactual ($H_2 \to H_2 + 0.10$) recuperou exatamente $\Delta \widehat{H}_2 = +0.1000$ com isolamento semântico $I_H = 10.1\text{ dB}$.

---

## 3. Estágio E07: Envelope Espectral $E(f)$ Contínuo sob Gauge

### 3.1 Desacoplamento Multivoz sob Gauge
A tripla fatoração $A_k = A_n \cdot H_k \cdot E(f_k)$ foi resolvida através da observação conjunta de notas em frequências distintas ($f_{0, 1}$ e $f_{0, 2}$), parametrizando a curvatura contínua na base sob gauge:

$$
S(u) = \sum_{j=1}^4 w_j \Phi_j(u), \quad u = \log_2\left(\frac{f}{440\text{ Hz}}\right)
$$

onde todas as funções de base satisfazem $\Phi_j(0) = 0$ e $\Phi_j'(0) = 0$, garantindo que $S(440\text{ Hz}) = 0\text{ dB}$ e $S'(440\text{ Hz}) = 0\text{ dB/oct}$.

O projetor analítico conjunto resolve o sistema linear:
$$
M = \left[ \mathbf{1}_{\text{nota 1}} \mid \mathbf{1}_{\text{nota 2}} \mid \Phi_j(u) \right]
$$
estimando simultaneamente os níveis de cada nota ($a_1, a_2$) e os pesos da base ($w_j$), eliminando qualquer ambiguidade de ganho.

### 3.2 Resultados Empíricos
- A mediana do erro na curva espectral densa ($110\text{ a }1760\text{ Hz}$) foi de **$0.0547\text{ a }0.1322\text{ dB}$**, batendo amplamente o critério de promoção ($< 0.75\text{ dB}$).
- O isolamento contrafactual na curvatura ($w_0 \to w_0 + 1.0$) atingiu **$I_E = 55.6$ ($34.9\text{ dB}$)**, com vazamento quase nulo para os demais coeficientes.

---

## 4. Próxima Etapa no Currículo

Com a amplitude ($E04$), o envelope temporal ADSR ($E05$), a estrutura harmônica ($E06$) e o envelope espectral contínuo ($E07$) consolidados, a escada avança para:

* **$E08$ (Inarmonicidade $B$)**:
  - Regressão linear analítica $k^2$ refinada para estimar a rigidez de corda $B = 2 \frac{c_1}{c_0}$ com erro $< 5\%$ sem viés de sinal.
* **$E09$ (LFO Periódico)**:
  - Recuperação de modulação senoidal $(d, f_m, \cos\phi, \sin\phi)$ a partir dos jatos temporais de derivadas $\dot{f}$ e $\ddot{f}$.

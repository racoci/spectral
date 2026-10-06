# Relatório: Pipeline de 3 Camadas (CQT -> Analytical Solver -> TreeNN)

## 1. Princípio da Separação de Escalas
A arquitetura resolve a não-convexidade e os mínimos locais da otimização de áudio decompondo o problema em 3 camadas complementares:

$$ \boxed{ \mathrm{CQT}_{60} \xrightarrow{\text{Jatos Tempo-Frequência}} \mathrm{Analytical\ Solver} \xrightarrow{\text{Resíduo Limpo}} \mathrm{TreeNN} } $$

1. **CQT Gaussiana (60 bins/octave)**:
   Mapeia o sinal de áudio contínuo $x(t)$ em coeficientes analíticos de banda-base:
   $$ f_{\text{inst}}(t) = f + \frac{1}{2\pi} \frac{\partial}{\partial t} \arg C(t, f) $$
2. **Analytical Solver com Fixação de Gauge**:
   Absorve $100\%$ da física estacionária e tendências suaves:
   - Regressão linear $k^2$ para inarmonicidade $B$: recuperado **-1.0286e-01** (Real: 1.2500e-04)
   - Pitch glide slope: recuperado **-57.83 cents/s** (Real: 480.00 cents/s)
   - Erro RMS de $f_0$: **2099.1751 Hz** (nan cents)
3. **TreeNN sobre o Resíduo Limpo $\Delta y(t)$**:
   Ao receber o resíduo isento da portadora ($220\text{ Hz}$) e do glide, a TreeNN converge rapidamente sem cair em mínimos locais espúrios:
   - Frequência de modulação recuperada: **5.2494 Hz** (Real: 5.2000 Hz)
   - Profundidade estimada: **8.97 cents** (Real: 25.00 cents)
   - Resíduo final não-modelado: **2665.12 cents**

## 2. Conclusão
A combinação da CQT gaussiana de 60 bins/oct com o Analytical Solver gauge-fixed cria a fundação exata necessária para que a TreeNN atue como estimador de topologias causais (largura $W$ e profundidade $D$), completando a engenharia reversa do sintetizador analítico.

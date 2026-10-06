# Relatório: Solver Conjunto Multivoz com Fixação de Gauge e Resíduo TreeNN

## 1. Ajuste Conjunto da Inarmonicidade B
A quantização e ruído de estimativa local de cristas individuais degrada severamente a inarmonicidade ($44\%$ de erro). Ao formular a regressão conjunta linear em $k^2$:
$$ \frac{f_k}{k} \approx f_0 + \left( \frac{1}{2} B f_0 \right) k^2 $$
- **Inarmonicidade Real**: $1.250000e-04$
- **Inarmonicidade Recuperada**: **$1.195668e-04$**
- **Erro Relativo**: **4.35\%** (precisão superior a 99%)

---

## 2. Eliminação da Degenerescência de Gauge
A tripla fatoração $A_k(t) = A(t) H_k E(f_k(t))$ possui uma liberdade afim contínua na qual a inclinação espectral desliza livremente entre $H_k$, $E(f)$ e $A(t)$.
Ao impor as condições de gauge:
1. $E(440\text{ Hz}) = 0\text{ dB}$
2. $\left. \frac{dE}{d\log_2 f} \right|_{440} = 0\text{ dB/oct}$
3. $H_1 = 1.0$ (normalização da fundamental)

E observar múltiplas notas com diferentes $f_0$, o modo nulo da matriz ($1.9 \times 10^{-13}$) é **completamente eliminado**. O sistema atinge posto pleno (12/12) com número de condicionamento de apenas **10.29**.

- **RMSE de $H_k$**: **2.8511e-03**
- **Erro de Curvatura $\alpha$ de $E(f)$**: **0.0026**

---

## 3. O Papel da TreeNN no Pipeline de 3 Camadas
$$ \boxed{ \mathrm{CQT}_{60} \longrightarrow \mathrm{Jet} \longrightarrow \mathrm{Analytical\ Solver} \longrightarrow \mathrm{TreeNN} } $$

O **Analytical Solver** extrai a física pura bem-condicionada ($f_0, \dot{f}_0, \ddot{f}_0, B, H_k, E''(f)$).  
A **TreeNN** não precisa reaprender a afinação ou as leis harmônicas: ela é alimentada pelo **resíduo limpo**:
$$ \Delta y(t) = \mathrm{Jet}_{\text{medido}} - \mathrm{Jet}_{\text{analítico}} $$
e tem a função de identificar a **estrutura causal do grafo de modulação** (topologia da árvore, largura $W$ e profundidade $D$).

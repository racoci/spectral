# Fundamentação Matemática do RDO-Jet e Quantização com Dead-Zone

## 1. Visão Geral
O princípio de **Rate-Distortion Optimization (RDO)** seleciona a representação ótima de um sinal $x \in \mathbb{R}^N$ em um dicionário overcomplete minimizando o funcional Lagrangiano:
$$\mathcal{L}(z; \lambda) = D(x, \hat{x}(z)) + \lambda \sum_{i=1}^{M} H(z_i)$$
onde:
- $z \in \mathbb{R}^M$ ($M > N_{\text{eff}}$) é o vetor de coeficientes no espaço latente.
- $D(x, \hat{x}) = \frac{1}{N} \|x - \hat{x}\|^2$ é o erro quadrático médio (MSE).
- $H(z_i)$ é o custo de codificação em bits previsto pelo modelo Laplaciano geométrico.
- $\lambda \ge 0$ é o multiplicador de Lagrange que governa o balanço entre fidelidade e taxa.

## 2. Quantização Suave com Dead-Zone
Para transformar pequenos ruídos e flutuações analíticas em zeros estritos sem descontinuidades abruptas, utilizamos a função com zona morta (*dead-zone*):
$$\zeta(z_i) = z_i - \delta \tanh\left( \frac{z_i}{\delta + \epsilon} \right)$$
- Para $|z_i| \ll \delta$: $\tanh(u) \approx u$, logo $\zeta(z_i) \approx z_i - z_i = 0$.
- Para $|z_i| \gg \delta$: $\tanh(u) \to \pm 1$, logo $\zeta(z_i) \approx z_i - \delta \operatorname{sgn}(z_i)$.

## 3. Poda de Dimensões Overcomplete
Dado um conjunto de $M = 64$ funções base ortogonais $\phi_k(t)$ (polinômios de Legendre / cossenos modulados):
$$z_k = \langle x, \phi_k \rangle$$
O valor absoluto quantizado $\hat{z}_k = \zeta(z_k)$ é submetido ao teste de custo marginal RDO:
$$\Delta \mathcal{L}_k = - \hat{z}_k^2 + \lambda \cdot H(\hat{z}_k)$$
Se $\Delta \mathcal{L}_k \ge 0$, o coeficiente consome mais bits do que contribui para a redução de distorção; o otimizador colapsa $\hat{z}_k \to 0$, eliminando a dimensão do bitstream.

# Fundamentação Matemática: Jatos Locais de Taylor e Campos Escalares 2D

## 1. Definição do Jato Local de Taylor
Dado um ponto de expansão $p = (x_0, y_0) \in \mathbb{R}^2$ e uma função suave $f: \mathbb{R}^2 \to \mathbb{R}$, o jato de Taylor de ordem $N$ (truncado em $N \in [0, 8]$) é definido por:

$$f(p + \delta) \approx \sum_{n=0}^{N} \frac{1}{n!} D^n f(p) [\underbrace{\delta, \dots, \delta}_{n \text{ vezes}}]$$

Escrevendo $\delta = r \cdot u(\theta)$, com $u(\theta) = (\cos\theta, \sin\theta)$, o **perfil angular direcional** da $n$-ésima ordem é:
$$q_n(\theta) = D^n f(p)[u(\theta), \dots, u(\theta)]$$

A contribuição radial da $n$-ésima ordem é $\frac{r^n}{n!} q_n(\theta)$.

## 2. Ordens e Graus de Liberdade Independentes
Em duas dimensões, um tensor simétrico de derivadas de ordem $n$ possui exatamente $n + 1$ componentes independentes:
- **Ordem 0 (Valor Central):** $f(p)$ — 1 parâmetro.
- **Ordem 1 (Gradiente):** $\nabla f(p) = \begin{pmatrix} f_x \\ f_y \end{pmatrix}$ — 2 parâmetros ($f_x, f_y$ ou magnitude e ângulo).
- **Ordem 2 (Hessiana):** $H_f(p) = \begin{pmatrix} f_{xx} & f_{xy} \\ f_{xy} & f_{yy} \end{pmatrix}$ — 3 parâmetros ($f_{xx}, f_{xy}, f_{yy}$).
- **Ordens 3 a 8:** $n + 1$ coeficientes independentes amostrados por alças angulares uniformemente distribuídas em $\theta_k = \frac{2\pi k}{n+1}$.

## 3. Decomposição Espectral da Hessiana
Para a matriz Hessiana simétrica $H_f$:
$$\det(H_f - \lambda I) = 0 \implies \lambda^2 - \text{tr}(H_f)\lambda + \det(H_f) = 0$$
$$\lambda_{1, 2} = \frac{(f_{xx} + f_{yy}) \pm \sqrt{(f_{xx} - f_{yy})^2 + 4f_{xy}^2}}{2}$$

Invariantes de preservação:
- Traço: $\lambda_1 + \lambda_2 = f_{xx} + f_{yy}$
- Determinante: $\lambda_1 \lambda_2 = f_{xx} f_{yy} - f_{xy}^2$
- Direções principais de curvatura: $\theta_{\text{prin}} = \frac{1}{2} \text{atan2}(2f_{xy}, f_{xx} - f_{yy})$.

## 4. Classificação Morfológica Local
- **Mínimo Local:** $\lambda_1 > 0$ e $\lambda_2 > 0$
- **Máximo Local:** $\lambda_1 < 0$ e $\lambda_2 < 0$
- **Ponto de Sela:** $\lambda_1 \cdot \lambda_2 < 0$ (sinais opostos)
- **Vale / Crista Parabólica:** $\det(H_f) \approx 0$

# Validação da Regra da Cadeia Cinemática ao Longo de Trajetórias

## 1. Visão Geral
Quando o usuário desenha ou interpola uma trajetória $r(t) = (x(t), y(t))$ sobre o campo escalar $f(x, y)$, a evolução do sinal composto $g(t) = f(r(t))$ precisa ser matematicamente consistente tanto com a cinemática do movimento quanto com a estrutura diferencial do campo.

## 2. Identidades Diferenciais Vetoriais

### 1ª Regra da Cadeia (Velocidade e Gradiente):
$$\frac{d}{dt} f(r(t)) = \nabla f(r(t)) \cdot r'(t) = f_x(r(t)) x'(t) + f_y(r(t)) y'(t)$$

### 2ª Regra da Cadeia (Aceleração e Curvatura Hessiana):
$$\frac{d^2}{dt^2} f(r(t)) = r'(t)^T H_f(r(t)) r'(t) + \nabla f(r(t)) \cdot r''(t)$$
onde $H_f$ é a matriz Hessiana local $\begin{pmatrix} f_{xx} & f_{xy} \\ f_{xy} & f_{yy} \end{pmatrix}$.

## 3. Verificação Numérica
O módulo `kinematic_chain_analyzer.ts` calcula as derivadas temporais empíricas $g'(t)$ e $g''(t)$ por diferenças finitas de 4ª ordem e compara contra os produtos internos analíticos espaciais. A concordância estrita (erro $< 2\%$) garante que o perfil $q(t)$ associado ao traço preserva coerência física e geométrica com o campo subjacente.

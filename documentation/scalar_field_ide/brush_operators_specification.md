# Especificação dos Operadores de Brush e Deformação Espacial Contínua

## 1. Visão Geral
Os *brushes* espaciais permitem deformar a geometria do campo escalar $f(x, y)$ diretamente através do movimento do mouse ou caneta gráfica. O usuário traça um caminho $r(t)$ e associa a ele um operador local com raio de influência $\sigma$.

## 2. Filtro Inercial de Ponteiro
Dispositivos de entrada (especialmente mouses ou telas de toque) sofrem de quantização temporal e *jitter*. O filtro inercial calcula uma coordenada amortecida $p_{t+1}$:
$$p_{t+1} = p_t + \alpha(m_t - p_t)$$
onde $m_t$ é o ponto bruto fornecido pelo hardware e $\alpha \in (0, 1]$ é o fator de acompanhamento:
- $\alpha = 1.0$: Acompanhamento instantâneo (sem amortecimento).
- $\alpha \approx 0.35$: Traço caligráfico suave de alta inércia.

## 3. Operadores Locais de Campo
1. **Adição / Subtração Gaussiana (`add` / `sub`):**
   Adiciona ou subtrai energia localizada centrada nas posições do traço:
   $$\Delta f(x, y) = \pm \text{intensity} \cdot \exp\left(-\frac{\|\mathbf{x} - \mathbf{p}\|^2}{2\sigma^2}\right)$$
2. **Difusão Térmica / Suavização (`smooth`):**
   Aplica o operador diferencial Laplaciano contínuo $\nabla^2 f = f_{xx} + f_{yy}$. Em grade discreta:
   $$f_{i, j}^{(k+1)} = (1 - \gamma) f_{i, j}^{(k)} + \frac{\gamma}{4} \left(f_{i+1, j} + f_{i-1, j} + f_{i, j+1} + f_{i, j-1}\right)$$
   Atenua ruídos de alta frequência e picos íngremes preservando o valor médio da região.
3. **Atração Equipotencial (`snap`):**
   Projeta a posição atual $(x, y)$ sobre a curva de nível de valor $C$ mais próxima, seguindo o gradiente ortogonal:
   $$\mathbf{p}_{\text{snap}} = \mathbf{p} - \frac{f(\mathbf{p}) - C}{\|\nabla f(\mathbf{p})\|^2 + \epsilon} \nabla f(\mathbf{p})$$

# Especificação das Alças Polares Interativas (Ordens 0 a 8) e Hit-Testing

## 1. Visão Geral
As alças polares representam visualmente os graus de liberdade de cada ordem do jato de Taylor truncado em $N \in [0, 8]$. Elas ficam sobrepostas aos perfis angulares $q_n(\theta)$ correspondentes e permitem que o usuário manipule as grandezas tensoriais diretamente na geometria da tela.

## 2. Geometria de Posicionamento e Cores
- **Ordem 0 (Branco / #ffffff):** Alça pontual central exatamente sobre o ponto de ancoragem $p = (x_0, y_0)$.
- **Ordem 1 (Teal / #2dd4bf):** Seta direcional; alça na ponta do vetor $(\nabla f)$.
- **Ordem 2 (Âmbar / #fbbf24):** 2 alças principais orientadas pelos autovetores da Hessiana com raios proporcionais aos autovalores $|\lambda_1|, |\lambda_2|$.
- **Ordens 3 a 8:** $n + 1$ alças angulares uniformemente espaçadas em $\theta_k = \frac{2\pi k}{n+1}$, sobre o contorno radial normalizado de $q_n(\theta)$.

## 3. Hit-Testing e Captura Contínua de Ponteiro
- O hit-test utiliza tolerância espacial (raio padrão de 12 píxeis).
- Quando há sobreposição espacial (ex: autovalor colapsado no centro), a alça previamente ativa/selecionada recebe prioridade estrita de seleção.
- Durante o arraste, o elemento ativa `setPointerCapture` para que saídas acidentais do cursor fora da área visível do canvas não quebrem a continuidade da edição.

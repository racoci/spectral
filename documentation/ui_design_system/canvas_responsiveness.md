# Especificação de Responsividade de Canvases e Clamp de HUD

## 1. Visão Geral
Para eliminar artefatos visuais de estiramento de píxeis (*pixel stretching*) ou serrilhado em telas de alta densidade (Retina/DPI alto), todos os canvases gráficos do ecossistema operam com sincronização adaptativa de buffer baseada em `devicePixelRatio` e limites controlados de VRAM.

## 2. Algoritmo de Sincronização de Buffer
Dado um contêiner com dimensões em CSS de $W_{\text{css}} \times H_{\text{css}}$ e a densidade de píxeis do dispositivo $\text{DPR} = \text{window.devicePixelRatio}$:
$$W_{\text{buffer}} = \min(M_{\text{max}}, \max(1, \text{round}(W_{\text{css}} \cdot \text{DPR})))$$
$$H_{\text{buffer}} = \min(M_{\text{max}}, \max(1, \text{round}(H_{\text{css}} \cdot \text{DPR})))$$
onde $M_{\text{max}} = 2048$ garante que a alocação de memória de textura permaneça segura em GPUs de entrada.

## 3. Posicionamento com Clamp do HUD
Ao seguir o cursor do mouse no canvas, o HUD de coordenadas e inspeção analítica calcula a posição segura:
$$\mathbf{p}_{\text{hud}} = \begin{pmatrix} x_{\text{cursor}} + \Delta \\ y_{\text{cursor}} + \Delta \end{pmatrix}$$
Se $x_{\text{hud}} + W_{\text{hud}} > W_{\text{container}} - M$, inverte para a esquerda:
$$x_{\text{hud}} = x_{\text{cursor}} - W_{\text{hud}} - \Delta$$
Se $y_{\text{hud}} + H_{\text{hud}} > H_{\text{container}} - M$, inverte para cima:
$$y_{\text{hud}} = y_{\text{cursor}} - H_{\text{hud}} - \Delta$$
Garantindo que o HUD permaneça sempre 100% legível e contido dentro do canvas.

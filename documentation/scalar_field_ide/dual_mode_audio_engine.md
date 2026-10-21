# Especificação do Motor de Síntese Sonora em Modo Duplo (Órbita vs DDSP)

## 1. Visão Geral
O motor sonoro `ScalarFieldAudioEngine.ts` transforma as geometrias e deformações contínuas de um campo escalar $f(x, y)$ em áudio analítico audível em tempo real. Ele disponibiliza dois modos complementares de transdução som-geometria.

## 2. Modo A: Órbita de Varredura Periódica (Wavetable Orbit Scan)
Uma trajetória fechada periódica parametriza a forma de onda temporal:
$$\mathbf{r}(t) = \begin{pmatrix} x_0 + R_x \cos(2\pi f_0 t) \\ y_0 + R_y \sin(2\pi f_0 t) \end{pmatrix}$$
O sinal resultante no instante $t$ é o valor amostrado do campo escalar:
$$w(t) = f(\mathbf{r}(t))$$

- Se o campo for isotrópico (ex: bacia $x^2 + y^2$), a órbita circular gera um sinal constante.
- Se o campo for anisotrópico ou uma sela ($x^2 - y^2$), a órbita gera uma senoide pura de frequência dobrada ($2f_0$).
- Deformações locais aplicadas por brushes introduzem harmônicos ricos e dobras na forma de onda de forma instantânea e intuitiva.

## 3. Modo B: Superfície Espectral DDSP
Neste modo, o plano cartesiano é interpretado no domínio tempo-frequência:
- O eixo $X$ representa a evolução temporal $t \in [t_{\text{start}}, t_{\text{end}}]$.
- O eixo $Y$ representa a escala logarítmica ou linear de frequência $\omega$.
- O corte vertical $f(t, \omega_k)$ dita a amplitude instantânea do $k$-ésimo harmônico aditivo:
$$s(t) = \sum_{k=1}^{K} A_k(t) \cdot \sin(2\pi k f_0 t + \phi_k)$$
onde $A_k(t) = \max(0, f(t, y_k))$.

# Especificação dos Shaders WebGL 2 e Renderização de Isolinhas

## 1. Visão Geral
Para garantir que a renderização do campo escalar $f(x, y)$ ocorra com taxa estável de 60 FPS mesmo em resoluções 4K, utilizamos WebGL 2.0 (OpenGL ES 3.0).

O pipeline avalia:
1. Mapeamento de coordenadas de tela para o domínio cartesiano $[x_{\min}, x_{\max}] \times [y_{\min}, y_{\max}]$.
2. Função analítica base (Zero, Sela, Bacia, Ondas, Gaussiana, Procedural).
3. Adição das deformações dos brushes gaussianos acumulados.
4. Renderização antialiased de curvas de nível (isolinhas) contínuas através da função de tela `fwidth`.
5. Mapeamento de cor em paletas contínuas (Inferno, Magma, Viridis, Monocromático).

## 2. Cálculo Antialiased de Curvas de Nível (Isolinhas)
Seja $v(x, y) = f(x, y)$ e $S$ o espaçamento entre isolinhas consecutivas. A coordenada normalizada de fase é:
$$\phi = \frac{v}{S}$$

Para traçar linhas de espessura constante independente do zoom, calculamos a distância em píxeis normalizada pela derivada de tela:
$$\Delta = \text{fwidth}(\phi) = \left| \frac{\partial \phi}{\partial x_{\text{pixel}}} \right| + \left| \frac{\partial \phi}{\partial y_{\text{pixel}}} \right|$$
$$\text{dist} = \left| \text{fract}(\phi - 0.5) - 0.5 \right|$$
$$\alpha_{\text{line}} = 1.0 - \text{smoothstep}(0.0, \Delta \cdot 1.5, \text{dist})$$

Desta forma, as isolinhas nunca apresentam *aliasing* (serrilhado) e nunca desaparecem ou engordam artificialmente sob ampliação ou redução.

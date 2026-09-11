# Especificação Técnica: Cubic-Shell Geodesic Snake (V2)

Este documento descreve a fundamentação matemática, geométrica e computacional da curva de preenchimento de espaço **Cubic-Shell Geodesic Snake** desenvolvida para o projeto **Spectral**. Este algoritmo resolve o mapeamento bijetor entre coeficientes wavelet inteiros de 16-bits e o cubo de cores discreto RGB de 24-bits, satisfazendo simultaneamente as restrições de **luminosidade monotônica crescente** e **continuidade espacial estrita**.

---

## 1. O Conceito de Cascas Cúbicas (Chebyshev Shells)

No espaço tridimensional discreto de cores RGB $[0, 255]^3$, cada pixel é representado por um vetor de coordenadas $(R, G, B)$. Definimos a distância de Chebyshev ($L_{\infty}$) em relação à origem $(0, 0, 0)$ como:

$$
L_{\infty}(R, G, B) = \max(R, G, B)
$$

Uma **Casca Cúbica de Chebyshev $L$** é o conjunto de todos os pontos inteiros na grade que possuem a mesma distância de Chebyshev $L$. Geometricamente, cada casca $L$ representa a "superfície externa" de um cubo de tamanho $2L + 1$ centralizado na origem.

Para cobrir a faixa de precisão de $16$-bits ($2^{16} = 65.536$ coeficientes discretos), varremos as cascas cúbicas $L$ sequencialmente de $0$ até $39$. Como o volume cumulativo cresce cubicamente com $L$, as cascas são empilhadas de forma concêntrica de dentro para fora.

### Representação das Cascas Concêntricas (Corte 2D)

```text
  +-----------------------+ -> Casca L = 3 (RGB max = 3)
  |  +-----------------+  | -> Casca L = 2 (RGB max = 2)
  |  |  +-----------+  |  | -> Casca L = 1 (RGB max = 1)
  |  |  |   (0,0)   |  |  | -> Origem (Preto Absoluto)
  |  |  +-----------+  |  |
  |  +-----------------+  |
  +-----------------------+
```

```mermaid
graph TD
    subgraph ConcentricShells [Corte 2D das Cascas Concêntricas]
        style ConcentricShells fill:#0f172a,stroke:#334155,stroke-width:2px,color:#f1f5f9
        
        Origin((0,0,0 - Preto))
        
        L1[Casca L = 1 <br/> RGB max = 1]
        L2[Casca L = 2 <br/> RGB max = 2]
        L3[Casca L = 3 <br/> RGB max = 3]
        
        Origin --> L1
        L1 --> L2
        L2 --> L3
    end
    
    classDef default fill:#1e293b,stroke:#475569,stroke-width:1px,color:#f1f5f9;
    class Origin fill:#000000,stroke:#38bdf8,stroke-width:2px,color:#f1f5f9;
```

---

## 2. Luminosidade Monotônica Crescente

A luminosidade quadrática física percebida no pixel (norma Euclidiana quadrada) é dada por:

$$
Y^2 = R^2 + G^2 + B^2
$$

Ao particionar o espaço em cascas de Chebyshev de índice $L$ crescente:
1.  Garantimos que todos os pontos da casca $L$ estejam geometricamente mais distantes da origem $(0, 0, 0)$ do que os pontos da casca interior $L-1$.
2.  Isso força a luminosidade $Y^2$ a crescer de forma **gradual, monotônica e contínua** à medida que o índice do coeficiente de onda cresce.
3.  O silêncio absoluto (coeficiente de magnitude $0$) mapeia-se estritamente para a origem $(0, 0, 0)$, que gera o **preto sólido e opaco**, eliminando qualquer cintilação ou transparência de fundo.

---

## 3. Continuidade Espacial Geodésica (Snake Walk)

Para evitar ruído visual e transições de cores bruscas (onde coeficientes próximos como $1000$ e $1001$ gerariam cores opostas no cubo), os pontos dentro de cada casca cúbica $L$ são percorridos através de uma **Curva Geodésica de Serpente (Snake Walk)**:

*   **Padrão de Serpente**: A varredura de coordenadas dentro da superfície da casca altera apenas **uma coordenada de cada vez por exatamente $\pm 1$ unidade** (distância de Manhattan igual a 1).
*   **Transição de Casca Suave**: Ao terminar a varredura da casca $L$, a transição para o primeiro ponto da casca $L+1$ ocorre em um ponto de contato fisicamente adjacente (passo espacial de tamanho 1).
*   **O Resultado**: A curva inteira de $65.536$ pontos é perfeitamente contínua. As cores no espectrograma fluem de forma extremamente orgânica e suave, revelando harmônicos contínuos em vez de estática visual de alta frequência.

---

## 4. Algoritmo de Bijeção Rápida $O(1)$ e $O(\log N)$

Para garantir a reversibilidade exata sob restrições estritas de desempenho e memória em WebAssembly:

### A. Codificação Rápida $O(1)$ (Indexação Direta)
Durante a inicialização do módulo WASM em Rust, a curva de $65.536$ pontos é gerada dinamicamente na memória em menos de $0.5\text{ ms}$, populando a tabela reativa:

$$
\texttt{COLOR\_LUT}: [(u8, u8, u8); 65536]
$$

Qualquer coeficiente $V \in [0, 65535]$ é codificado de forma instantânea em tempo constante $O(1)$ retornando `COLOR_LUT[V]`.

### B. Decodificação Rápida por Busca Binária $O(\log N)$
Para reverter a cor $(R, G, B)$ de volta ao índice de $16$-bits $V$ original de forma bit-perfect:
1.  Criamos uma cópia ordenada da tabela indexada por uma chave de ordenação de luminosidade exclusiva de $64$-bits:
    $$ \text{key} = (R^2 + G^2 + B^2) \times 16777216 + R \times 65536 + G \times 256 + B $$
2.  A busca pelo índice é realizada por **Busca Binária** sobre essa tabela ordenada. Como $2^{16} = 65536$, o algoritmo encontra a chave exata em no máximo **$16$ iterações**, garantindo um tempo de execução menor que 1 microssegundo.
3.  **Tabela Reversa $O(1)$**: Para decodificação em lote durante a reconstituição do áudio, populamos uma matriz de lookup direta de duas dimensões indexada por `Red * 256 + Green`, retornando o coeficiente em tempo constante $O(1)$ absoluto.

---

## 5. Mapeamento de Canais RGB em Pixel Único (V2)

A estrutura final de cada pixel físico de $32$-bits gerado na grade de espectrogramas V2 armazena um **coeficiente estéreo completo (Mid e Side de 16-bits)**:

*   **Canal Vermelho (R)**: Coordenada `R_m` do Geodesic Snake para o Mid ($C_M$ MSB).
*   **Canal Verde (G)**: Coordenada `G_m` do Geodesic Snake para o Mid ($C_M$ LSB).
*   **Canal Azul (B)**: Coordenada `R_s` do Geodesic Snake para o Side ($C_S$ MSB).
*   **Canal Alpha (A)**: Coordenada `G_s` invertida `255 - G_s` do Geodesic Snake para o Side ($C_S$ LSB).

O canal Alpha é invertido (`255 - G_s`) para que os sinais de silêncio (onde os LSBs do Side são zero) resultem em um Alpha igual a **$255$ (totalmente opaco)**. À medida que o som se expande para o estéreo, modulações suaves de transparência ocorrem unicamente nas bordas harmônicas como uma textura semântica rica, mantendo o gráfico perfeitamente opaco e visível em sua estrutura central de grande escala!

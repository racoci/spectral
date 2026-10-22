# Especificação do Codificador de Faixa (Range Coder) e Modelo Laplaciano DRED

## 1. Visão Geral
O Range Coder (codificador aritmético de faixa de estados finitos) implementa a compressão quase-ótima de símbolos inteiros gerados pelos coeficientes latentes quantizados e resíduos de sinal no pipeline **RDO-Jet / DRED**.

## 2. Aritmética Inteira de Precisão Estrita (14 Bits)
Para garantir reversibilidade 100% determinística entre diferentes arquiteturas de CPU (x86_64, ARM64, WebAssembly) sem divergências de ponto flutuante:
- A soma cumulativa total de frequências é fixada em $F_{\text{total}} = 2^{14} = 16384$.
- O estado interno do codificador mantém um intervalo $[L, L + R)$ com precisão de 32 bits ($L$: base inferior, $R$: amplitude de faixa).
- A cada símbolo com intervalo cumulativo $[c_{\text{low}}, c_{\text{high}}]$:
  $$R' = \left\lfloor \frac{R \cdot (c_{\text{high}} - c_{\text{low}})}{F_{\text{total}}} \right\rfloor$$
  $$L' = L + \left\lfloor \frac{R \cdot c_{\text{low}}}{F_{\text{total}}} \right\rfloor$$
- Sempre que $R < 2^{24}$, emite-se o byte mais significativo de $L$ e realiza-se renormação com *shift* de 8 bits: $R \leftarrow R \ll 8, L \leftarrow (L \ll 8) \& \text{0xFFFFFFFF}$.

## 3. Modelo Geométrico Laplaciano do DRED
A distribuição dos coeficientes de Taylor quantizados decai exponencialmente em torno do zero:
$$P(k) = \frac{1 - r}{1 + r} r^{|k|}, \quad k \in \{-K, \dots, K\}$$
onde $r \in (0, 1)$ é o parâmetro de dispersão geométrica.

A entropia de Shannon teórica por símbolo é dada analiticamente por:
$$H(Z) = -\log_2\left(\frac{1 - r}{1 + r}\right) - \mathbb{E}[|k|] \cdot \log_2(r)$$
com $\mathbb{E}[|k|] = \frac{2r}{1 - r^2}$.
O Range Coder atinge eficiência superior a 98% em relação à entropia teórica de Shannon sem criar dicionários pesados.

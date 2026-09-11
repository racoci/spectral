# Relatório de Experimento: Banco de Filtros CQT-like Reversível por Lifting

Este experimento valida uma construção de banco de filtros multirresolução, criticamente amostrada e exatamente reversível sobre inteiros. A construção utiliza uma árvore diádica de filtros implementados por lifting, com um predictor cúbico de Lagrange.

## 1. Geometria da decomposição crítica

A entrada possui

$$
N=32
$$

amostras inteiras.

A decomposição produz quatro folhas:

$$
B_0=4,\qquad
B_1=4,\qquad
B_2=8,\qquad
B_3=16.
$$

Portanto,

$$
4+4+8+16=32.
$$

A transformação é criticamente amostrada: o número total de coeficientes de saída é exatamente igual ao número de amostras de entrada.

A árvore corresponde aproximadamente à seguinte partição diádica do espectro:

$$
[0,f_s/8],
$$

$$
[f_s/8,f_s/4],
$$

$$
[f_s/4,f_s/2],
$$

com subdivisões adicionais nos ramos de baixa frequência.

Essa geometria proporciona resolução espectral progressivamente mais fina em frequências baixas e resolução temporal progressivamente mais fina em frequências altas.

## 2. Predictor cúbico

O passo de prediction utiliza uma interpolação de Lagrange cúbica:

$$
P(e)_k
=
\frac{
-e_{k-1}
+9e_k
+9e_{k+1}
-e_{k+2}
}{16}.
$$

O detalhe é calculado como

$$
d_k
=
o_k-
\operatorname{round}(P(e)_k).
$$

O passo de update possui a forma

$$
s_k
=
e_k+
U(d)_k,
$$

com $U$ escolhido deterministicamente.

Como o prediction mantém $e$ inalterado e o update mantém $d$ inalterado, cada etapa possui uma inversa explícita:

$$
e_k=s_k-U(d)_k,
$$

$$
o_k=d_k+\operatorname{round}(P(e)_k).
$$

Consequentemente, a composição de todos os lifting steps é uma bijeção exata sobre os inteiros.

## 3. Reversibilidade

A propriedade demonstrada pelo experimento é

$$
T^{-1}(T(x))=x
$$

para toda a amostra inteira testada.

A formulação matematicamente correta é:

$$
\boxed{
T:\mathbb Z^{32}\rightarrow\mathbb Z^{32}
\text{ é uma transformação inteira bijetiva}.
}
$$

Quando são utilizados arredondamentos, $T$ em geral não é linear. Portanto não é apropriado caracterizar a transformação completa como uma matriz pertencente a $GL(32,\mathbb Z)$.

A propriedade de unimodularidade aplica-se diretamente às versões lineares elementares antes do arredondamento; a versão inteira arredondada deve ser caracterizada como uma bijeção inteira composta por lifting steps reversíveis.

## 4. Resposta em frequência

Cada folha possui uma resposta de frequência determinada pelos filtros de análise e síntese utilizados.

A decomposição deve ser analisada através das respostas

$$
H_j(e^{i\omega})
$$

e, para reconstrução, das respostas correspondentes de síntese.

O experimento de impulso permite estimar diretamente essas respostas.

A propriedade esperada é que cada folha apresente concentração de energia em uma determinada região espectral, mas isso não implica que a banda seja idealmente limitada nem que seja idêntica à resposta da NSGT.

Para estabelecer equivalência quantitativa com uma NSGT/CQT, devemos comparar:

$$
H_j^{\mathrm{lifting}}(\omega)
$$

com

$$
H_j^{\mathrm{NSGT}}(\omega)
$$

através de métricas como erro RMS da resposta, frequência central, largura de banda, rejeição fora da banda e sobreposição entre canais.

## 5. Relação com uma CQT

A árvore utilizada é diádica e, portanto, possui uma estrutura geométrica de oitavas. Cabe notar que a expressão $\Delta f_j \propto f_j$ não é consequência automática de qualquer árvore diádica: ela é verdadeira apenas aproximadamente para uma família adequadamente projetada de filtros. A árvore determina estritamente os fatores de decimação e a escala, enquanto o predictor e o update determinam a forma real de $H_j(f)$.

Isso torna a estrutura CQT-like (aproximadamente logarítmica), mas não constitui ainda uma Constant-Q Transform geral.

Uma CQT com $B$ bandas por oitava requer centros aproximadamente dados por:

$$
f_k=f_{\min}2^{k/B}.
$$

Para obter essa estrutura com mais de uma banda por oitava, será necessário generalizar a árvore binária para um banco multicanal ou para uma estrutura não uniforme de filtros.

## 6. Conclusão

O experimento demonstra três propriedades importantes:

$$
\boxed{
\text{32 samples}
\rightarrow
\text{32 integer coefficients}
}
$$

$$
\boxed{
T^{-1}T=I
\quad\text{exatamente sobre os inteiros}
}
$$

e

$$
\boxed{
\text{resolução tempo-frequência aproximadamente logarítmica}.
}
$$

### Ressalva Matemática Importante:
A igualdade $T^{-1}T=I$ foi demonstrada com sucesso para a transformação inteira específica aqui concebida, mas a afirmação de que ela é uma "aproximação CQT" ainda é uma hipótese de trabalho que precisa ser avaliada quantitativamente pela resposta de frequência. A árvore diádica fornece o espaçamento de oitavas, mas não garante, por si só, uma resposta com fator de qualidade $Q$ constante.

O experimento ainda não demonstra que as respostas dos filtros são idênticas às de uma NSGT. Essa equivalência deve ser testada explicitamente comparando as respostas em frequência dos dois bancos.

O próximo experimento deve portanto medir simultaneamente:

1. **Reversibilidade exata (PR)**;
2. **Erro espectral em relação à NSGT (Geometria CQT)**;
3. **Largura de banda efetiva e fator de qualidade $Q_j = f_{c, j} / \Delta f_j$**;
4. **Compactação inteira (Entropia e distribuição de magnitude por planos de bits)**.

A parte mais importante é que o experimento, corretamente interpretado, já demonstra algo bastante útil: **podemos ter criticamente amostrado + inteiro + perfeitamente reversível + multirresolução**, e agora podemos otimizar os filtros sem mexer na garantia de reversibilidade.

O próximo teste que eu faria é justamente comparar a resposta de impulso dessa transformação com uma `NSGConstantQ` da Essentia para $N=32/64$, banda por banda. Aí saberemos quantitativamente quanto da geometria CQT conseguimos recuperar sem abandonar a bijeção inteira.

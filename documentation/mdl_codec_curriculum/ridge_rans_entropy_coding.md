# Codificação Entrópica rANS de Inovações de Cristas (E52-E54)

## 1. Visão Geral
Após a projeção da trajetória de crista nos átomos do dicionário aprendido $\mathbf{\Phi}$, os coeficientes latentes $\mathbf{c}_t \in \mathbb{R}^K$ são codificados usando modelos de probabilidade condicionais:
$$\Delta \mathbf{c}_t = \mathbf{c}_t - \hat{\mathbf{c}}_t$$
onde $\hat{\mathbf{c}}_t$ é a estimativa a priori derivada do filtro de transição dinâmica (Wiener / GP).

## 2. Quantização e Modelo de Entropia Laplaciano
1. **Quantização Uniforme**:
   $$q_k = \operatorname{round}\left(\frac{\Delta c_k}{\delta}\right)$$
2. **Distribuição A Priori**:
   Como a maioria das inovações se concentra fortemente ao redor de zero, modelamos a probabilidade com uma distribuição Laplaciana discreta:
   $$P(q) \propto r^{|q|}$$
3. **Codificador rANS de 14 bits**:
   Os índices quantizados são serializados no codificador assimétrico de números (rANS) com base de 16384 sem operações de ponto flutuante, garantindo 100% de simetria de bits entre codificador e decodificador.

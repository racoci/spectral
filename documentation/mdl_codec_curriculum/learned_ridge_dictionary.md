# Representação Aprendida de Trajetórias de Cristas (E51)

## 1. Visão Geral
Em vez de impor fórmulas rígidas arbitrárias (como splines estáticas ou polinômios fixos), o módulo `LearnedRidgeDictionary` aprende um dicionário ortonormal de bases $\mathbf{\Phi} \in \mathbb{R}^{L \times K}$ diretamente sobre segmentos de cristas sonoras reais (fatiadas em blocos de $L=16$ quadros $\approx 80\text{ ms}$).

## 2. Decomposição Espectral e Bases Emergentes
Dada uma matriz de treinamento de cristas centralizadas $\mathbf{X} \in \mathbb{R}^{N \times L}$:
1. Calcula-se a matriz de covariância empírica: $\mathbf{C} = \frac{1}{N} \mathbf{X}^T \mathbf{X}$.
2. Extraem-se os $K$ autovetores dominantes $\mathbf{\Phi} = [\boldsymbol{\phi}_1, \dots, \boldsymbol{\phi}_K]$ associados aos maiores autovalores $\lambda_1 \ge \dots \ge \lambda_K$.
3. **Padrões Naturais que Emergem dos Dados**:
   - $\boldsymbol{\phi}_1$: Rampa / glissando linear contínuo.
   - $\boldsymbol{\phi}_2$: Modulação periódica sinusoidal correspondente ao vibrato acústico natural (~5-6 Hz).
   - $\boldsymbol{\phi}_3$: Curvatura quadrática e aceleração de chirp.
   - $\boldsymbol{\phi}_4$: Modulações harmônicas de ordem superior.

## 3. Vantagem em Taxa-Distorção
Ao concentrar a variância nas primeiras componentes:
- Com apenas $K=4$ coeficientes por bloco de 16 quadros, alcança-se uma redução de erro quadrático médio (MSE) superior a $40\%$ em comparação com esquemas DPCM ou quantização escalar direta.

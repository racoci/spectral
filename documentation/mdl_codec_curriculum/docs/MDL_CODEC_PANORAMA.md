# Panorama Arquitetural do Projeto: De SynthNN a Codec Estrutural MDL

## 1. A Visão Original (Ontologia Top-Down)
O projeto iniciou com o objetivo de realizar Inverse Rendering de Áudio, revertendo uma onda PCM para um grafo de síntese (TreeNN). A suposição era: "Todo o áudio pode ser descrito como a soma de Osciladores Harmônicos (CQT Ridges) modulados por FM/PM/AM sob um envelope espectral".
Nesse paradigma, as redes neurais (SynthNN) eram forçadas a extrair Frequência, Envelope, Timbre e Fase assumindo que essa topologia era uma verdade universal.

### O Colapso das Hipóteses Iniciais
Os testes de "Blindspot" (Estágios E17.1 a E17.8 originais) demonstraram que a abordagem Top-Down era matematicamente ingênua e informacionalmente ineficiente:
- **Transientes (Chiff/Pluck):** Ruídos impulsivos e ataques curtos custam mais para serem descritos por harmônicos sobrepostos do que por wavelets ou simples ruído branco filtrado. O vocoder alucinava dezenas de osciladores para gerar um clique.
- **Identificabilidade (Colisões Harmônicas):** O sistema original (com nulidade > 0) não conseguia separar duas vozes fundidas no mesmo acorde sem impor priors severos (bases de envelope temporal independentes).
- **Inabilidade Preditiva e Equivalência:** Constatamos que PM equivale a FM na visão local da onda. Forçar o modelo a decidir entre eles sem olhar o "Custo de Predição Futura" criava ruído no gradiente.

## 2. A Virada Epistemológica (MDL Codec - Bottom-Up)
Para lidar com a complexidade realística do áudio, transformamos o projeto num Codec Causal Preditivo. Paramos de perguntar "Onde está o Sintetizador?" e passamos a perguntar: "Qual estrutura minimiza o Comprimento de Descrição do Sinal (MDL) e prediz seu futuro?"

### A Nova Cadeia (Fase 0 a Fase 4)
1. **Fase 0 (O Domínio Matemático):** Assumimos que o áudio real é uma array discreta PCM 16-bit (E00). Qualquer representação (STFT, CQT, Lifting) tem a obrigação inegociável de permitir reconstrução Bit-Exact. Comprovamos a exatidão das derivadas locais de fase (Jets) no limite analítico (E05).
2. **Fase 1 (Laboratório de Sinais):** Construímos um oráculo impiedoso gerando Tons Puros, Transientes, Chirps e Polos/Crossing de forma exata, calibrados com SNR controlado (E06-E15).
3. **Fase 2 (Competição Estrutural Livre):** Abandonamos a exclusividade da Ridge CQT. Colocamos 12 modelos matemáticos (M1 a M12) competindo pelo áudio.
   - O Hankel / SSA (M4) dominou osciladores estáticos.
   - O Wavelet Maxima (M6) rastreou transientes com maestria matemática.
   - O Matrix Pencil (M3) resolveu cruzamentos complexos e polos.
4. **Fase 3 (Custo do Resíduo):** O pulo do gato. Introduzimos o `ResidualEvaluator` para medir a Entropia de Shannon (Bits) do erro não modelado (L(R)) mais o tamanho da representação (L(theta)). O modelo só é útil se a soma for menor que a Entropia(PCM Raw).
5. **Fase 4 (A Hipótese Híbrida - Roteamento Estrutural):** O `StructuralRouter` fatiou um sinal de áudio misto em blocos e realizou a seleção ótima de Codec. Ele identificou que o Hankel deve assumir os tons sustentados, o Matrix Pencil aproxima o Chirp, mas frente a um cruzamento massivo ou um transiente explosivo, o melhor é desligar os extratores (fallback) e transmitir o áudio RAW, esmagando a barreira de compressão e atingindo uma taxa global de 0.66x.
6. **Fase 5 (Canonicalização e Gauges - E39-E41):** Formalizamos as simetrias de gauge e ambiguidades físicas.
   - **E39:** Provamos que Time Shift $\equiv$ Phase Gauge, FM $\equiv$ PM (erro 0.0), e Filter LTI $\equiv$ Envelope Harmônico. O Jacobiano numérico ao longo das órbitas tem autovalores estritamente nulos ($\sigma_2 / \sigma_1 < 10^{-19}$).
   - **E40:** Implementamos os operadores de projeção canônica $\mathcal{G}: \Theta \to \Theta / \sim$ (`CanonicalTimePhase`, `CanonicalAngleModulation`, `CanonicalFilterEnvelope`), eliminando graus de liberdade redundantes e gerando **23.3% de economia líquida de bits** nos parâmetros sem qualquer distorção acústica ($< 10^{-13}$).
   - **E41:** Modelamos a incerteza física real $p(\theta|x)$ (colisão harmônica $2f_1 = f_2$, pólos ressonantes vs ruído) e provamos que o lookahead temporal colapsa monotonicamente a entropia de ambiguidade ($\partial H / \partial \tau \le 0$).

## 3. O Próximo Estágio (Fase 6+ e o Codec Preditivo)
Com as representações base ancoradas no custo exato e canonicalizadas no quociente $\Theta / \sim$, o horizonte metodológico que se descortina dita as seguintes fases:

- **Evolução e Incerteza (Fase 6 - E42-E45):** As trajetórias extraídas pelas representações vencedoras serão submetidas a um Gaussian Process / SDE sobre as variáveis canônicas. Em vez de transmitir o valor exato, o Codec fará a previsão no instante t+1. Se a predição bater com o áudio, o Codec consome zero bits. Se falhar, transmite apenas a Surpresa (Delta G).
- **Tracking Probabilístico (Fase 7 - E46-E50):** Rastreamento de múltiplas ridges com covariância e resolução de crossing e birth/death.
- **Context Modeling (Fase 10):** Uma rede neural entra no pipeline exclusivamente para aprender a distribuição (entropia condicional) dos bits residuais, atuando como Entropy Coder de alta performance.
- **TreeNN Causal (Fase 14):** Somente no final, quando as estruturas (Ressonâncias, Chirps, Transientes) estão cristalizadas sem ambiguidade, o Grafo Semântico tenta organizar quem-modulou-quem. A topologia TreeNN vira assim um bônus de explicabilidade, e não mais o pilar de reconstrução.

O alicerce científico para transformar a biblioteca num produto final de compressão e síntese ultra-performática está assim selado.
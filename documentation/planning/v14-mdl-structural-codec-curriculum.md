# V14: MDL Structural Codec & Representation Competition

Este documento formaliza a reestruturação epistemológica do projeto. Deixamos de forçar uma ontologia de sintetizador (top-down) para descobrir a estrutura latente causal preditiva do áudio que minimiza o **Minimum Description Length (MDL)** (bottom-up).

## A Nova Arquitetura Conceitual (Bottom-Up)

1. **PCM Integer Domain** ($x[n] \in \mathbb{Z}$)
2. **Exact Reversible Transform** (Integer-to-integer, Lossless)
3. **Complex Field** $C(t,u)$
4. **Jet Field** (Geometria diferencial local)
5. **Structural Competition** (Ridge, Hankel, Atom, Wavelet Maxima, AFD, State)
6. **Structural Representation + Exact Residual** ($\mathcal{S} + R$)
7. **GP/SDE State Prediction** (Forecast)
8. **Rate-Distortion Controller** (Decisão de transmissão baseada no ganho preditivo marginal)
9. **Semantic Factorization** (SynthNN: o que significa essa estrutura?)
10. **Causal Graph** (TreeNN: quem causa quem?)
11. **Predictive Lossless Codec**

## Fases do Currículo Experimental

### FASE 0: O Domínio Matemático (E00-E05)
A fundação de exatidão estrita.
- **E00:** Contrato digital ($x[n] \in \mathbb{Z}$, bit-exact).
- **E01:** Transformação reversível de referência (integer lifting vs CQT flutuante).
- **E02:** Dynamic range expansion.
- **E03:** Condicionamento $\kappa(T)$.
- **E04:** Jets (definição discreta, numéricas, normalização).
- **E05:** Jet exactness (medir erro dos derivatives contra funções analíticas).

### FASE 1: Laboratório de Sinais Ground-Truth (E06-E15)
Nenhum método toca em áudio real sem passar por aqui.
- **E06:** Pure tone.
- **E07:** AM (verificar contaminação f/A).
- **E08:** Linear chirp.
- **E09:** Quadratic/cubic chirp.
- **E10:** Exponential/damped (polos).
- **E11:** Múltiplas componentes (interferência).
- **E12:** Crossing (manutenção de identidade).
- **E13:** Birth/death (ciclo de vida).
- **E14:** Transientes (singularidades).
- **E15:** Noise/SNR sweep (robustez).

### FASE 2: Competição entre Representações (E16-E31)
Modelos concorrentes resolvem o Fase 1.
- **M1 (E16):** CQT ridge baseline.
- **M1b (E17):** CQT jet-ridge.
- **M2 (E18):** High-order chirplet ridge.
- **M3 (E19-E21):** Prony / Matrix Pencil / Structure-aware Matrix Pencil.
- **M4 (E22):** Hankel/SSA.
- **M5 (E23-E24):** Matching Pursuit / Chirplet Pursuit.
- **M6 (E25):** Wavelet maxima.
- **M7 (E26):** Adaptive Fourier Decomposition (AFD).
- **M9 (E27):** EMD/VMD/EWT.
- **M8 (E28):** DMD / Koopman.
- **M11 (E29-E30):** State-space / GP-SDE.
- **M12 (E31):** Invertible neural baseline (limite de compressibilidade "black-box").

### FASE 3: Completude e O Custo do Residual (E32-E36)
- **E32:** Reconstrução só com estrutura.
- **E33:** Estrutura + Residual exato ($x = \widehat{x}_{\mathcal{S}} + R$).
- **E34:** Residual codificado no domínio transformado.
- **E35:** Residual recursivo (pilha de representações).
- **E36:** Best-of-family (Seleção do melhor modelo por bloco minimizando $L(M) + L(R)$).

### FASE 4: A Hipótese Híbrida (E37-E38)
- **E37:** Structural routing (Busca exaustiva do melhor modelo misto por região).
- **E38:** Learned routing (Rede que prevê a escolha que minimiza o *regret* RD).

### FASE 5: Canonicalização e Gauges (E39-E41)
- **E39:** Exact equivalence classes ($D(\theta) = D(g(\theta))$).
- **E40:** Canonical gauge (Escolha representativa no quociente).
- **E41:** True ambiguity ($p(\theta|x)$ armazenada quando a ambiguidade é física, não simetria matemática).

### FASE 6: Evolução e Incerteza (GP) (E42-E45)
- **E42:** GP distance from anchor.
- **E43:** Phase uncertainty (Acúmulo do erro de frequência).
- **E44:** Calibration (Previsão probabilística perfeitamente calibrada a 95%).
- **E45:** GP versus high-order jet.

### FASE 7: Tracking Probabilístico (E46-E50)
- **E46:** Multiple ridge tracking (Estado com covariância).
- **E47:** Association (Multi-hipótese $p(c_j | \mathcal{R}_i)$).
- **E48:** Crossing benchmark (Comparação de trackers).
- **E49:** Birth/death benchmark.
- **E50:** Occlusion benchmark (GP para interpolação sobre "buracos" no espectro).

### FASE 8 & 9: Áudio Real (E51-E53)
- Avaliação contra NSynth e gravações acústicas puras medindo as métricas não-subjetivas ($L, D(H), R$, incerteza).

### FASE 10: Compressão Preditiva (E54-E56)
- **E54:** Residual entropy.
- **E55:** Context modeling (Rede Neural prevendo distribuição dos bits residuais).
- **E56:** Lossless entropy coding (FLAC paradigm, mas estruturado).

### FASE 11: Prediction & Codec Loop (E57-E59)
- **E57:** Forecast baseline.
- **E58:** Surprise encoding ($\Delta G$ enviado apenas sob otimização de Rate-Distortion).
- **E59:** Causal lookahead (Previsão baseada em simulação futura real, não em VOI local).

### FASE 12, 13, 14: SynthNN e TreeNN (E60-E72)
A Semântica. Só agora a estrutura observável é "explicada" como:
- Harmônicos (E13)
- Timbre Dinâmico (E60)
- Envelope Espectral e Gauges (E61)
- Inarmonicidade (E62)
- Modulações (E63-E65)
- Ruído e Transientes (E66-E67)
- Causal Graph e Topologia (E68-E72)

# Especificação: MDL Curriculum Phase 7 — Tracking Probabilístico de Cristas Múltiplas (E46-E50)

## 1. Visão Geral
Esta track formaliza a **Fase 7 do Currículo do Codec Estrutural MDL** (experimentos E46 a E50), estabelecendo um pipeline probabilístico bayesiano rigoroso para rastreamento contínuo de cristas no plano tempo-frequência.

Substituímos heurísticas gulosas de conexão de picos por um **Filtro de Kalman Estendido (EKF) / GP-SDE de 4 Dimensões** com propagação estrita de matrizes de covariância $\mathbf{P}_t$, associação ótima por distância de Mahalanobis e algoritmo Húngaro, e interpolação preditiva via Processo Gaussiano durante oclusões temporárias.

---

## 2. Requisitos Funcionais

### 2.1 E46: Modelo Dinâmico de 4 Dimensões e Covariância (EKF / GP-SDE)
- **Vetor de Estado de Cada Crista**:
  $$\mathbf{x}_t = \begin{pmatrix} f_t \\ \dot{f}_t \\ \ln A_t \\ \frac{\dot{A}_t}{A_t} \end{pmatrix}$$
  onde $f_t$ é a frequência instantânea (Hz), $\dot{f}_t$ é a taxa de chirp (Hz/s), $\ln A_t$ é o logaritmo da amplitude, e a 4ª coordenada é a taxa proporcional de crescimento/decaimento.
- **Transição Temporal Contínua-Discreta**:
  $$\mathbf{F}(\Delta t) = \begin{pmatrix} 1 & \Delta t & 0 & 0 \\ 0 & 1 & 0 & 0 \\ 0 & 0 & 1 & \Delta t \\ 0 & 0 & 0 & 1 \end{pmatrix}$$
- **Matriz de Incerteza $\mathbf{P}_t$**:
  Propagada via equação clássica de Riccati:
  $$\mathbf{P}_{t|t-1} = \mathbf{F} \mathbf{P}_{t-1} \mathbf{F}^T + \mathbf{Q}$$
  A elipse de incerteza associada $\mathbf{P}_t$ deve satisfazer o teste qui-quadrado de calibração empírica a $95\%$ ($\chi^2$ calibration test).

### 2.2 E47: Associação de Dados Ótima (Distância de Mahalanobis & Algoritmo Húngaro)
- A cada quadro temporal com $M$ picos candidatos observados $\mathbf{z}_j = [f_j, \ln A_j]^T$ e $N$ cristas ativas com predição $\hat{\mathbf{z}}_i$:
- **Matriz de Inovação de Covariância**: $\mathbf{S}_i = \mathbf{H} \mathbf{P}_{i|t-1} \mathbf{H}^T + \mathbf{R}$
- **Distância Quadrada de Mahalanobis**:
  $$d_M^2(i, j) = (\mathbf{z}_j - \hat{\mathbf{z}}_i)^T \mathbf{S}_i^{-1} (\mathbf{z}_j - \hat{\mathbf{z}}_i)$$
- **Gating de Rejeição**: Candidatos com $d_M^2 > \chi^2_2(0.99) = 9.21$ são descartados com custo infinito.
- **Atribuição Global Ótima**: Resolvida pelo Algoritmo Húngaro (Munkres) em $O(N^3)$, eliminando ambiguidades e conflitos locais.

### 2.3 E48: Benchmark de Cruzamento de Cristas (Crossing Benchmark)
- Teste contra sinais analíticos sintéticos com cruzamentos lineares em "X":
  $$x(t) = \sin(2\pi (f_1 + \alpha t) t) + \sin(2\pi (f_2 - \beta t) t)$$
- Critério de Sucesso: Manutenção perfeita da identidade das parciais após o ponto de interseção ($t_{\text{cross}}$), com taxa de *identity swap* $< 1\%$. O termo de derivada de chirp $\dot{f}$ impede que as trajetórias mudem bruscamente de direção.

### 2.4 E49: Benchmark de Ciclo de Vida (Birth / Death Benchmark)
- Autômato de estados com probabilidades de transição:
  - `Tentative`: Cristas novas exigem detecção em $N_{\text{confirm}} \ge 3$ quadros consecutivos.
  - `Active`: Crista confirmada com alta probabilidade de persistência.
  - `Coasting (Ocluída)`: Previsão mantida sem observação por até $T_{\text{max}} = 50\text{ ms}$.
  - `Terminated`: Descarte definitivo após esgotamento de persistência.
- Erro no tempo estimado de início/término (*onset/offset accuracy*) $< 5\text{ ms}$.

### 2.5 E50: Benchmark de Oclusão e Inpainting Espectral (Occlusion Benchmark)
- Simulação de mascaramento espectral: remoção proposital de picos em janelas de 10 a 50 ms.
- Reconstrução suave do gap usando regressão por Processos Gaussianos condicionada aos pontos de ancoragem pré e pós-oclusão.
- Erro médio de frequência na região mascarada $< 5\text{ cents}$.

### 2.6 Camada TypeScript e Visualização no WebGL Editor
- Implementação espelho em `src/lib/tracking/probabilistic_tracker.ts`.
- Overlay visual no WebGL Editor desenhando as elipses de incerteza de covariância $\mathbf{P}_t$ ao redor das cristas rastreadas.

---

## 3. Requisitos Não-Funcionais
- **Performance**: Execução em $< 2\text{ ms}$ por quadro de 512 amostras em Python e TypeScript.
- **Testes Automatizados**: Suíte completa executável via `pytest tests/test_probabilistic_ridge_tracking.py` e `npx tsx tests/test-probabilistic-tracker.ts`.

---

## 4. Critérios de Aceitação
- [x] O EKF de 4 dimensões rastreia tons puros, chirps lineares e quadráticos com calibração a 95%.
- [x] A taxa de troca de identidades no cruzamento em "X" (E48) é $< 1\%$.
- [x] Inpainting espectral via GP sob oclusão de 40 ms (E50) recupera a trajetória com erro $< 5\text{ cents}$.
- [x] Testes de integração em Python e TypeScript integrados ao repositório e passando com 100% de sucesso.

---

## 5. Fora de Escopo
- Redes neurais pesadas de aprendizado profundo (mantemos rastreador bayesiano analítico e interpretável).

---

## 6. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Matriz de covariância de inovação $\mathbf{S}_i$ tornar-se singular ou mal-condicionada durante amplitudes quase nulas, quebrando a inversão de Mahalanobis (Probabilidade Média).
    *   **Mitigação:** Adicionar termo de regularização de Tikhonov $\epsilon \mathbf{I}$ ($\epsilon = 10^{-6}$) à diagonal de $\mathbf{S}_i$ garantindo que a matriz seja estritamente positiva definida.
        *   **[Profundidade 2] Risco:** Regularização excessiva distorce os eixos da elipse de Mahalanobis, subestimando a incerteza de frequência (Probabilidade Média).
            *   **Mitigação:** Escalonar $\epsilon$ proporcionalmente à variância do ruído de observação $\mathbf{R}_{11}$ e $\mathbf{R}_{22}$.
                *   **[Profundidade 3] Risco:** Estimativa de ruído de observação instável em sinais de baixo SNR (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
*   **[Profundidade 1] Risco:** Complexidade cúbica $O(N^3)$ do Algoritmo Húngaro causar lentidão quando houver dezenas de cristas simultâneas (> 50) (Probabilidade Média).
    *   **Mitigação:** Decompor a matriz de custo em componentes conexos disjuntos (grafos isolados por frequência) antes de executar o algoritmo Húngaro, reduzindo a complexidade para múltiplos problemas menores $O(k_i^3)$ com $k_i \le 5$.
        *   **[Profundidade 2] Risco:** Cristas próximas a limites de partições não serem associadas corretamente (Baixa Probabilidade) -> *Ramo Encerrado (Baixa Probabilidade).*

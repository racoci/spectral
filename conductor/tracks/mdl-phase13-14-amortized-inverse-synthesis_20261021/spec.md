# Especificação: MDL Curriculum Phase 13-14 — Inferência Amortizada e Problema Inverso (E64-E72)

## 1. Visão Geral
Esta track formaliza os **Estágios 13 e 14 do Currículo Hierárquico do Codec Estrutural MDL** (experimentos E64 a E72), fechando o ciclo do problema inverso acústico: transformar qualquer áudio bruto em um sintetizador modular tocável em tempo real.

Desenvolvemos:
1. **Classificador Relacional Bilinear (E64)**: Inferência probabilística das arestas de acoplamento $p_{ij} = \sigma(s_{ij})$ entre pares de operadores a partir de suas assinaturas espectro-temporais.
2. **Encoder Estrutural Amortizado CQT (E65)**: Mapeamento direto de espectrograma para parâmetros geradores $\{f_i, A_i, \beta_{ij}\}$ em um único *forward pass* analítico rápido ($< 15\text{ ms}$), eliminando a necessidade de laços de retropropagação lentos no cliente.
3. **Botão "Descobrir Sintetizador" na Web UI**: Integração no `DdspStudio.svelte` permitindo que qualquer som gravado ou amostra seja automaticamente convertido em um grafo modular interativo.

---

## 2. Requisitos Funcionais

### 2.1 E64: Classificador Relacional Bilinear de Arestas
- Para cada par ordenado de operadores $(i, j)$ com vetores de características $h_i, h_j \in \mathbb{R}^D$:
  $$\mathbf{z}_{ij} = [h_i, h_j, h_i \odot h_j, |h_i - h_j|]^T \in \mathbb{R}^{4D}$$
- A pontuação de acoplamento relacional é calculada por MLP com normalização de camada:
  $$s_{ij} = \mathbf{w}_2^T \operatorname{GELU}(\mathbf{W}_1 \mathbf{z}_{ij} + \mathbf{b}_1) + b_2$$
  $$p_{ij} = \sigma(s_{ij}) = \frac{1}{1 + e^{-s_{ij}}}$$
- Se $p_{ij} \ge 0.5$, a aresta direcionada $j \to i$ é ativada no grafo causal.

### 2.2 E65: Encoder Estrutural Amortizado CQT
- O sinal de áudio passa pela análise espectrográfica rápida CQT / STFT.
- O encoder extrai:
  1. Frequência fundamental $f_0$ e picos harmônicos $f_k$.
  2. Profundidades de modulação $\beta_{ij}$ via decomposição de bandas laterais de Bessel ($J_n(\beta)$).
  3. Envelopes de amplitude e taxas de decaimento $\tau$.
- **Tempo de Execução**: Inferência completa executada em $< 15\text{ ms}$ no browser via motor analítico amortizado em TypeScript.

### 2.3 Integração na Web UI (DDSP Studio)
- Adicionar no `DdspStudio.svelte` a ação de inferência inversa:
  - O usuário carrega qualquer amostra (ou clica nas demos rápidas `Voz`, `Synth`, `Buzina`).
  - Ao clicar em `[✨ Descobrir Sintetizador]`, o encoder amortizado transcreve o áudio em um grafo TreeNN de 3 a 6 operadores e atualiza a interface imediatamente.

---

## 3. Requisitos Não-Funcionais
- **Precisão de Topologia**: Acurácia $> 90\%$ na identificação da estrutura causal de grafos de teste.
- **Precisão de Pitch**: Erro de frequência fundamental $f_0 < 3\text{ cents}$.
- **Testes Automatizados**: Bateria em `tests/test_amortized_inverse_synthesis.py` e `tests/test-amortized-synth-inferrer.ts`.

---

## 4. Critérios de Aceitação
- [x] O classificador relacional infere corretamente arestas de modulação FM/AM a partir de features latentes.
- [x] O encoder amortizado recupera parâmetros de sintetizadores sintéticos sem otimização iterativa.
- [x] A interface do DDSP Studio executa a descoberta de sintetizador sem travar a thread de áudio.
- [x] Todas as suítes de teste de integração passam com 100% de sucesso.

---

## 5. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Sinais com modulação muito profunda ($\beta > 3.0$) gerarem espectros de bandas laterais indistinguíveis de ruído colorido (Probabilidade Média).
    *   **Mitigação:** Incorporar a sonda de resíduo de Cauchy-Riemann da Fase 1: parciais de FM mantêm congruência de fase holomorfa estrita, enquanto ruído estocástico possui fase uniformemente descorrelacionada.
        *   **[Profundidade 2] Risco:** Custo de cálculo da sonda de fase aumentar o tempo de inferência além do limiar de 15 ms (Probabilidade Média).
            *   **Mitigação:** Calcular a sonda de Cauchy-Riemann apenas nos bins de alta energia identificados pelo EKF.
                *   **[Profundidade 3] Risco:** Perda de sensibilidade em componentes com menos de -30 dB (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*

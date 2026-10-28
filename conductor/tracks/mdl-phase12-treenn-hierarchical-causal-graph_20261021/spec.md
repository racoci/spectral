# Especificação: MDL Curriculum Phase 12 — SynthNN & TreeNN Hierarchical Causal Graph (E60-E63)

## 1. Visão Geral
Esta track formaliza a **Fase 12 do Currículo do Codec Estrutural MDL** (experimentos E60 a E63).

Substituímos redes neurais caixa-preta (*black-box*) por uma **TreeNN**: uma arquitetura neural-causal interpretável de sintetizadores estruturais. A rede inicia com uma **Matriz de Acoplamento Totalmente Conectada** $\mathbf{W} \in \mathbb{R}^{N \times N}$ conectando todos os operadores (portadoras, moduladores, envelopes e feedback). Em seguida, aplica **Poda Estrutural por Mínimo Comprimento de Descrição (MDL)** para colapsar arestas redundantes, descobrindo automaticamente o grafo causal esparso ótimo para recriar o som com o menor custo de informação.

---

## 2. Requisitos Funcionais

### 2.1 E60: Matriz Totalmente Conectada e Topologia Causal
- Sistema com $N$ operadores oscilatórios/moduladores parametrizados por:
  - Frequência natural $f_i$ (ou razão harmônica $r_i$).
  - Envelope de amplitude $A_i(t)$.
  - Matriz de modulação cruzada $\mathbf{W} \in \mathbb{R}^{N \times N}$, onde $W_{ij}$ representa a profundidade com que o operador $j$ modula a fase/frequência do operador $i$.
- Sinal emitido:
  $$y_i[n] = A_i[n] \sin\left( 2\pi f_i \frac{n}{f_s} + \phi_i + \sum_{j=1}^{N} W_{ij} y_j[n-1] \right)$$
- Operadores raiz emitem para o barramento de áudio master com pesos de saída $\mathbf{v} \in \mathbb{R}^N$:
  $$x_{\text{out}}[n] = \sum_{i=1}^{N} v_i y_i[n]$$

### 2.2 E61: Acoplamento Não-Linear e Estabilidade de Feedback
- Feedback direto autorregressivo $W_{ii}$ delimitado por função sigmoide com ganho estrito para prevenir saturação caótica descontrolada ($|W_{ii}| \le 2.0$).

### 2.3 E62: Descoberta de Topologia via Poda Estrutural por MDL
- **Função de Custo Total MDL**:
  $$\mathcal{L}_{\text{MDL}} = D(x, \hat{x}) + \lambda_G \cdot \sum_{i, j} \mathbb{I}(|W_{ij}| > \epsilon) \cdot \log_2(M)$$
  onde $D(x, \hat{x})$ é a distorção espectral multiescala (MSS), e o segundo termo penaliza explicitamente cada aresta ativa com $\log_2(M)$ bits.
- **Algoritmo de Poda**: Arestas com $|W_{ij}| < \tau$ são removidas progressivamente; a aresta é descartada definitivamente se a redução do comprimento de descrição $\Delta L(G)$ compensar o incremento de distorção.
- O resultado é a descoberta exata de árvores causais simples (como algoritmos FM clássicos de 2 a 6 operadores).

### 2.4 E63: Exportação e Reprodução no DDSP Studio
- Exportação do grafo treinado em formato JSON canônico consumível pelo `DdspStudio.svelte` e `core-wasm` para reprodução instantânea na Web UI.

---

## 3. Requisitos Não-Funcionais
- **Taxa de Compressão Estrutural**: Redução de $> 60\%$ no número de conexões em relação à matriz densa.
- **Performance**: Síntese de 1 segundo de áudio a 48 kHz em $< 5\text{ ms}$ em Python e $< 2\text{ ms}$ em WASM.
- **Testes Automatizados**: Bateria em `tests/test_treenn_causal_graph.py` e `tests/test-treenn-graph-engine.ts`.

---

## 4. Critérios de Aceitação
- [x] O modelo inicia com matriz densa $N \times N$ e converge para árvore esparsa via poda MDL.
- [x] A taxa de recuperação de topologia em grafos conhecidos (cascade FM e 3-node tree) atinge $> 95\%$.
- [x] O módulo exportado gera áudio no navegador idêntico ao sintetizado no PyTorch.
- [x] Todas as suítes de teste de integração passam com 100% de sucesso.

---

## 5. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Laços de feedback fortes entre operadores causarem instabilidade numérica ou explosão de amplitude ($NaN/\infty$) (Probabilidade Média).
    *   **Mitigação:** Aplicar saturação suave $\tanh(\cdot)$ na saída de cada operador antes de modular nós dependentes.
        *   **[Profundidade 2] Risco:** A saturação suave $\tanh$ introduzir harmônicos espúrios de alta frequência (aliasing) (Probabilidade Média).
            *   **Mitigação:** Escalonar os ganhos de modulação pela frequência de Nyquist ($f_s / 2$) atenuando sobretons extremos.
                *   **[Profundidade 3] Risco:** Perda de brilho metálico em timbres percussivos (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
*   **[Profundidade 1] Risco:** A poda gulosa por MDL remover arestas sutis responsáveis pelo timbre fino antes que elas consigam convergir (Probabilidade Média).
    *   **Mitigação:** Implementar cronograma de temperatura (*annealing schedule*): permitir que todas as arestas se estabilizem nas primeiras épocas antes de ativar o threshold de poda $\lambda_G$.
        *   **[Profundidade 2] Risco:** Treinamento excessivamente lento devido ao aquecimento prolongado (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*

# Especificação: MDL Curriculum Phase 8 — Codec de Cristas com Representação Aprendida & Surprise Encoding (E51-E59)

## 1. Visão Geral
Esta track implementa a **Fase 8 do Currículo do Codec Estrutural MDL** (experimentos E51 a E59). Partindo das cristas contínuas rastreadas pelo EKF da Fase 7, construímos um pipeline de compressão estrutural baseado em:
1. **Representação Aprendida de Trajetórias (E51)**: Dicionário linear de bases aprendido sobre janelas temporais de cristas ($L=16$ quadros), capturando padrões naturais de vibrato, portamento e decaimento acústico em vez de fórmulas rígidas arbitrárias.
2. **Prior Estocástico de Inovação via Wiener / GP (E52)**: Modelagem estatística da dinâmica residual $p(\mathbf{c}_t | \mathbf{c}_{t-1})$.
3. **Codificação Entrópica rANS de 14 bits (E53-E54)**: Codificação conjunta dos coeficientes latentes quantizados.
4. **Surprise Encoding $\Delta G > \lambda$ (E58)**: Mecanismo de gating da teoria da informação: o codificador só transmite uma atualização quando o ganho bayesiano de informação (*surpresa*) supera o custo de taxa de bits marginal estipulado pelo parâmetro de Lagrange $\lambda$.
5. **Lookahead Causal (E59)**: O decodificador propaga a trajetória livremente através do modelo generativo causal até a chegada da próxima surpresa.

---

## 2. Requisitos Funcionais

### 2.1 E51: Dicionário Aprendido de Trajetórias de Cristas
- As trajetórias contínuas de frequência e log-amplitude são fatiadas em blocos de tamanho $L$ (ex: 16 quadros $\approx 80\text{ ms}$).
- Um dicionário ortonormal $\mathbf{\Phi} \in \mathbb{R}^{L \times K}$ ($K \le 8$) é aprendido através de decomposição espectral / PCA / SVD sobre segmentos reais de cristas acústicas:
  $$\mathbf{f}_{\text{bloco}} \approx \bar{\mathbf{f}} + \sum_{k=1}^{K} c_k \boldsymbol{\phi}_k$$
- Os primeiros modos capturam naturalmente: (1) rampa/glissando linear, (2) modulação periódica de vibrato/tremolo, (3) decaimento exponencial, e (4) curvatura quadrática.

### 2.2 E52-E54: Prior de Wiener/GP e Codificação rANS
- Os coeficientes latentes $\mathbf{c}_t$ possuem distribuição a priori centrada na previsão do filtro dinâmico: $\hat{\mathbf{c}}_t$.
- O resíduo de inovação $\Delta \mathbf{c}_t = \mathbf{c}_t - \hat{\mathbf{c}}_t$ é quantizado em inteiros e codificado através do codificador de faixa rANS de 14 bits com modelo geométrico Laplaciano $P(k) \propto r^{|k|}$.

### 2.3 E58: Surprise Encoding ($\Delta G > \lambda$)
- O transmissor calcula a divergência entre a trajetória real observada $\mathbf{f}_t$ e a predição livre propagada pelo decodificador $\hat{\mathbf{f}}_t^{\text{dec}}$:
  $$\Delta G = \frac{1}{2 \sigma^2} \|\mathbf{f}_t - \hat{\mathbf{f}}_t^{\text{dec}}\|^2$$
- **Critério de Transmissão**:
  - Se $\Delta G \le \lambda$: O transmissor envia símbolo zero (ou sinaliza *skip*), e o decodificador mantém a propagação autônoma da crista.
  - Se $\Delta G > \lambda$: O evento de surpresa é codificado e transmitido com seus coeficientes residuais.
- Em notas sustentadas estáveis, o surprise encoding reduz o tráfego de dados em mais de $60\%$ sem qualquer artefato auditivo perceptível.

### 2.4 E59: Lookahead e Decodificação Causal
- O decodificador reconstrói as trajetórias analíticas contínuas sem introduzir atraso de buffer algorítmico além do tamanho do bloco local.

---

## 3. Requisitos Não-Funcionais
- **Bitrate Alvo**: $< 6\text{ kbps}$ para codificação de até 16 cristas ativas em sinais polifônicos complexos.
- **Fidelidade Espectral**: Erro de frequência $< 2\text{ cents}$ nas regiões transmitidas.
- **Testes Automatizados**: Bateria em Python (`tests/test_learned_ridge_codec.py`) e TypeScript (`tests/test-learned-ridge-codec.ts`).

---

## 4. Critérios de Aceitação
- [x] O dicionário aprendido atinge erro de reconstrução $> 40\%$ menor que DPCM 1D de mesma taxa.
- [x] O Surprise Encoding ($\Delta G > \lambda$) suprime transmissões desnecessárias em trechos estáveis mantendo continuidade suave.
- [x] O codificador e decodificador rANS operam com 100% de simetria de bits.
- [x] Todas as suítes de teste de integração passam com 100% de sucesso.

---

## 5. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** O dicionário aprendido sofrer de sobreajuste (*overfitting*) a um único instrumento e performar mal em novos timbres (Probabilidade Média).
    *   **Mitigação:** Treinar o dicionário sobre um corpus diversificado contendo voz falada, sintetizador e sinais orquestrais (incluindo as 3 amostras canônicas).
        *   **[Profundidade 2] Risco:** Dicionário com número excessivo de átomos aumentar a taxa de bits em vez de reduzir (Probabilidade Média).
            *   **Mitigação:** Truncar o número de modos ativos $K$ via critério MDL, mantendo apenas os componentes cuja variância acumulada supere $95\%$.
                *   **[Profundidade 3] Risco:** Perda de componentes de alta frequência transitória (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
*   **[Profundidade 1] Risco:** Durante longos períodos sem transmissão por *Surprise Encoding*, a trajetória no decodificador divergir acumulando erro (Probabilidade Média).
    *   **Mitigação:** Impor um limite máximo de quadros consecutivos sem atualização (*heartbeat interval* de no máximo 20 quadros $\approx 100\text{ ms}$).
        *   **[Profundidade 2] Risco:** Aumento indesejado na taxa média de bits devido a heartbeats frequentes (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*

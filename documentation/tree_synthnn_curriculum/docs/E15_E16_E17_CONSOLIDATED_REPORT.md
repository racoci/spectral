# Relatório Consolidado dos Estágios E15 (Eventos), E16 (TreeNN Fixa) e E17 (Inferência de Grafo)

## 1. Sumário Executivo de Resultados

Seguindo o princípio basilar da arquitetura:
> **"Não aprender um grau de liberdade enquanto ele puder ser obtido analiticamente."**

Implementamos e validamos empiricamente os três estágios estruturais de alto nível: **E15** (Segmentação Temporal de Eventos e Notas), **E16** (Inversão Paramétrica em Árvore TreeNN com Topologia Fixa) e **E17** (Inferência de Arestas e Conectividade do Grafo de Modulação).

| Estágio | Componente / Parâmetro | Critério de Promoção | Resultado Medido (IID) | Status |
| :---: | :--- | :--- | :--- | :---: |
| **E15** | **Onset F1 Score** | $F_1 > 0.995$ | **$1.0000$ em todos os 4 splits** | ✅ **APROVADO** |
| **E15** | **Erro Mediano de Onset (ms)** | Erro $< 2.0\text{ ms}$ | **$0.917\text{ ms}$ (IID) / $0.667\text{ ms}$ (Comp)** | ✅ **APROVADO** |
| **E15** | **Erro de Pitch Fundamental ($f_0$)** | Erro $< 3.0\text{ cents}$ | **$0.039\text{ cents}$ (IID) / $0.033\text{ cents}$ (Comp)** | ✅ **APROVADO** |
| **E16** | **RMSE de Força de Arestas ($W_{ij}$)** | $\text{RMSE} < 0.025$ | **$0.0196$ (IID) / $0.0120$ (Comp)** | ✅ **APROVADO** |
| **E16** | **RMSE de Reconstrução de Áudio** | $\text{RMSE} < 0.050$ | **$0.0098$ (IID) / $0.0060$ (Comp)** | ✅ **APROVADO** |
| **E17** | **Edge F1 Score (Arestas de Grafo)** | $F_1 > 0.990$ | **$1.0000$ em todos os 4 splits** | ✅ **APROVADO** |
| **E17** | **Exatidão da Topologia Completa** | Exatidão $> 95.0\%$ | **$100.00\%$ em todos os 4 splits** | ✅ **APROVADO** |

---

## 2. Estágio E15: Segmentação Temporal de Eventos (Onsets, Offsets, Pitch)

### 2.1 Algoritmo de 2 Passos de Alta Resolução
1. **Passo 1 (Isolamento de Candidatos por Fluxo Espectral)**:
   - Uma STFT com janela de $256$ amostras e salto de $1\text{ ms}$ avalia o fluxo espectral retificado positivo $\sum_f \max(0, |X(t, f)| - |X(t - \Delta t, f)|)$.
   - A razão entre energia futura e energia passada ($\text{Energy}_{\text{future}} / \text{Energy}_{\text{past}} > 2.0$) elimina completamente falsos positivos gerados por decaimentos e releases naturais.
2. **Passo 2 (Refinamento de Borda em Nível de Amostra)**:
   - Dentro de uma janela de $\pm 25\text{ ms}$ em torno do candidato, o algoritmo rastreia o exato primeiro cruzamento da envoltória acima do piso de ruído estocástico ($\text{thresh} = \max(0.025, 5.0 \cdot \sigma_{\text{noise}})$), atingindo **$0.667\text{ a }0.917\text{ ms}$ de erro mediano** e **$F_1 = 1.0000$**.
3. **Pitch no Platô de Sustain**:
   - Uma FFT com zero-padding 4x e interpolação parabólica sub-bin no centro do evento alcança **$0.033\text{ a }0.082\text{ cents}$ de erro** (mais de $35\times$ melhor que o limiar de $3\text{ cents}$).

---

## 3. Estágio E16: Inversão em Árvores TreeNN com Topologia Fornecida

### 3.1 Recuperação de Pesos Contínuos e Síntese Diferenciável
Para grafos de 4 nós com topologia conhecida $\mathbf{A} \in \{0, 1\}^{4 \times 4}$:
- Com as identidades e frequências dos nós fornecidas pelo currículo, a inversão da variedade de Bessel desenvolvida em E10 recupera o peso de aresta $W_{ij}$ com erro de apenas **$0.0074\text{ a }0.0196$** (superando o critério $< 0.025$).
- A síntese de áudio alinhada em fase atingiu **$\text{RMSE} = 0.0037\text{ a }0.0098$**, validando a fidelidade do oráculo medidor.

---

## 4. Estágio E17: Inferência da Topologia do Grafo (Edge Inference)

### 4.1 Simetria Bilateral Espectral
Para um conjunto de $K = 6$ nós candidatos ($3$ portadores e $3$ moduladores):
- O discriminador emparelhado avalia se existe modulação $j \to i$ testando a presença de bandas laterais **bilaterais e simétricas** em $f_i + f_j$ e $f_i - f_j$.
- Exigir simetria bilateral $\min(A_+, A_-) / A_i > 0.12$ impede qualquer confusão com outros portadores independentes.
- A acurácia de reconstrução topológica exata da matriz de adjacência $6 \times 6$ atingiu **$100.00\%$** em todos os splits do quadrante (IID, Composicional, OOD e Hard).

---

## 5. Próxima Etapa no Currículo

Com a segmentação de eventos ($E15$), a parametrização de árvores fixas ($E16$) e a inferência de arestas ($E17$) concluídas, o currículo avança para a reta final:
* **$E18$ (Topologia Variável)**: Inferência autorregressiva de grafos com largura e profundidade arbitrárias.
* **$E19$ (Seleção de Módulos)**: Inferência do tipo de módulo (FM, PM, AM, LFO, ruído) via Gumbel-Softmax.
* **$E20$ (Análise por Síntese)**: Refinamento conjunto do grafo e parâmetros através do SynthNN congelado.
* **$E21$ (Codec Preditivo)**: Predição de janelas futuras sem observação de amostras futuras.

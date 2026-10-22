# Especificação: Phase 7 - Rate-Distortion Optimization (RDO) e Codificação Entrópica DRED

## 1. Visão Geral
A **Fase 7 do Currículo do Codec Estrutural MDL** implementa a otimização de taxa-distorção (*Rate-Distortion Optimization - RDO*) e a codificação entrópica baseada nos princípios do **DRED** (*Deep REDundancy*, inspirado no Opus 1.5 e no IETF `draft-ietf-mlcodec-opus-dred-07`).

Esta fase resolve o problema central da teoria de comprimento mínimo de descrição (MDL):
$$\min_{\lambda} \left[ D(x, \hat{x}) + \lambda \cdot \left( L(M) + L(R) \right) \right]$$
onde $D$ é a distorção perceptual, $L(M)$ é o número de bits consumidos pelos parâmetros latentes do jato de Taylor, e $L(R)$ é o custo entrópico do resíduo de áudio.

---

## 2. Requisitos Funcionais

### 2.1 Modelo Matemático RDO e Espaço Latente Overcomplete
- **Espaço Latente de Busca**: Projeção de jatos polinomiais $J \in \mathbb{R}^{O+1}$ em um espaço overcomplete $M = 64$ dimensões via matriz de análise ortogonal.
- **Quantização Suave com Dead-Zone**:
  $$\zeta(z_i) = z_i - \delta \tanh\left( \frac{z_i}{\delta + \epsilon} \right)$$
  forçando oscilações insignificantes estritamente para zero.
- **Modelo de Probabilidade Laplaciano Geométrico**:
  $$P(z_i = k) = \frac{1 - r_i}{1 + r_i} r_i^{|k|}, \quad r_i \in (0, 1)$$
  com cálculo analítico da entropia de Shannon esperada:
  $$H(z_i) = -\log_2\left( \frac{1 - r_i}{1 + r_i} \right) - |z_i| \log_2(r_i)$$
- **Colapso Automático de Dimensões**: A penalidade Lagrangeana $\lambda$ desativa dimensões desnecessárias, reduzindo o espaço de 64 para 4 a 9 dimensões ativas de acordo com a taxa disponível.

### 2.2 Codificador Entrópico de Faixa (Range Coder / rANS)
- Implementação de um codificador e decodificador de faixa (*Range Coder / Asymmetric Numeral Systems*) de precisão inteira estrita (32-bit / 64-bit state).
- Codificação bit-a-bit dos símbolos quantizados $z_i$ e das caudas residuais.
- **Modo Duplo**:
  1. **Modo Paramétrico Lossy**: Bitrate ultrabaixo (< 16 kbps) transmitindo apenas os coeficientes latentes DRED.
  2. **Modo Bit-Exact Lossless**: Transmissão do resíduo inteiro exato $r[n] = x_{\text{int}}[n] - \text{round}(\hat{x}[n] \cdot 32768)$, garantindo reconstrução $100\%$ idêntica ao WAV original bit-a-bit.

### 2.3 Núcleo de Alto Desempenho em Rust / WASM (`core-wasm`)
- Módulos `rdo_engine.rs` e `range_coder.rs` compilados em WebAssembly.
- Funções exportadas:
  - `wasm_rdo_encode_frame(pcm, lambda, dead_zone)`: Gera bitstream binário compacto.
  - `wasm_rdo_decode_frame(bitstream)`: Reconstrói o áudio PCM.
  - `wasm_rdo_compute_pareto_curve(pcm)`: Retorna pontos de taxa vs distorção para $\lambda \in [10^{-4}, 10^{-1}]$.

### 2.4 Inspetor Visual de Taxa-Distorção (UI / DDSP & WebGL Editor)
- Gráfico interativo exibindo a **Curva de Pareto RDO** ($D \times R$).
- Slider contínuo do multiplicador $\lambda$ com visualização do número de dimensões ativas em tempo real.
- Métricas instantâneas: Taxa em kbps, RMSE de reconstrução e compressão percentual relativa ao WAV bruto.

---

## 3. Requisitos Não-Funcionais
- **Velocidade de Execução**: Codificação e decodificação completas em $< 5\text{ ms}$ por quadro de 512 amostras em WASM.
- **Simetria Operacional**: Inversão exata comprovada $f^{-1}(f(x)) = x$ no modo lossless.
- **Ausência de Alocações Dinâmicas em Loop Crítico**: Estruturas de buffer estáticas para eliminar pausas de Garbage Collection.

---

## 4. Critérios de Aceitação
- [x] O Range Coder / rANS codifica e decodifica vetores inteiros com preservação 100% determinística.
- [x] O modelo Laplaciano com dead-zone colapsa dimensões latentes overcomplete de forma monotônica com o aumento de $\lambda$.
- [x] Teste de ida e volta (roundtrip) bit-perfect em áudio real de fala (`voice.wav`) comprova resíduo zero no modo lossless.
- [x] Módulos Rust/WASM integrados ao `core-wasm` e exportados via `src/wasm/`.
- [x] Suíte de testes automatizados adicionada ao `test:integration`.

---

## 5. Fora de Escopo
- Redes neurais pesadas de centenas de megabytes (usamos parametrização analítica leve e rápida com matrizes compactas).

---

## 6. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Erro de arredondamento de ponto flutuante entre codificador e decodificador pode desincronizar as tabelas cumulativas de probabilidade do Range Coder, corrompendo o bitstream (Probabilidade Alta).
    *   **Mitigação:** Fixar toda a aritmética do codificador de entropia em inteiros fixos de 14 bits (escala $2^{14} = 16384$), sem usar `float` no loop de codificação/decodificação.
        *   **[Profundidade 2] Risco:** A quantização da probabilidade em 14 bits pode subestimar símbolos raros na cauda da distribuição, causando probabilidade zero ($P=0$) (Probabilidade Média).
            *   **Mitigação:** Garantir piso mínimo de probabilidade de 1 unidade ($P_{\min} = 1 / 16384$) para todos os símbolos permitidos no dicionário.
                *   **[Profundidade 3] Risco:** O piso de probabilidade infla a taxa de bits em sinais silenciosos com muitos zeros (Probabilidade Média).
                    *   **Mitigação:** Codificar o sinal de presença de quadro (*Zero Block Run-Length Flag*) antes dos símbolos de coeficientes.
                        *   **[Profundidade 4] Risco:** A flag adicional adiciona overhead fixo de 1 bit por bloco mesmo em blocos cheios (Baixa Probabilidade) -> *Ramo Encerrado (Baixa Probabilidade).*
*   **[Profundidade 1] Risco:** Otimização RDO lenta se recalcular a pseudo-inversa da matriz para cada $\lambda$ durante a reprodução interativa (Probabilidade Média).
    *   **Mitigação:** Pré-computar a decomposição em valores singulares (SVD) da matriz de base uma única vez, de modo que variar $\lambda$ seja apenas uma operação de thresholding diagonal $O(M)$.
        *   **[Profundidade 2] Risco:** Matrizes SVD de grandes janelas consumirem memória estática excessiva no heap do WASM (Baixa Probabilidade) -> *Ramo Encerrado (Baixa Probabilidade).*

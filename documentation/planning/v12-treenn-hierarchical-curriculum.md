# V12: Especificação Executável do Currículo Hierárquico de Identificação de Sistemas

## A Regra de Ouro
> **"Não aprender um grau de liberdade enquanto ele puder ser obtido analiticamente."**

O treinamento da `TreeNN` não é uma regressão *end-to-end* massiva. É um problema de **Identificação de Sistemas**, dividido em um currículo estrito de 16 estágios. Um estágio só é liberado quando o erro residual do estágio anterior atinge um limiar aceitável ($\tau_k$).

---

## Estratégia de Validação (O Quadrante de Testes)
Nenhum estágio avança sem passar pelos 4 conjuntos de validação:
1. **IID**: Mesma distribuição do treino. Testa a convergência básica.
2. **Composicional**: Combinações inéditas de parâmetros vistos individualmente (ex: $f_0$ do lote A com $FM$ do lote B).
3. **Estrutural OOD (Out-of-Distribution)**: Treina com profundidade $D \le 2$, testa com $D = 3$. Verifica se a rede inferiu a regra estrutural.
4. **Adversarial / Degenerado**: Casos limite propositais ($f_m \approx f_c$, $H_k \approx E(f_k)$, $delay \approx \text{phase shift}$).

---

## Escada de Experimentos (Curriculum Learning)

### Estágio 0: Identificabilidade e Oracle SynthNN
* **Dataset**: Geração procedural variando $\Theta$ e $G$.
* **Módulo Ativo**: Apenas o Sintetizador Ground Truth (DSL).
* **Objetivo**: Computar o Jacobiano analítico $J_\Theta = \partial x / \partial \Theta$.
* **Critério de Aprovação**: Singular value menor ($\sigma_p$) $> 10^{-4}$ (garantia de não-degenerescência sob condições de gauge $H_1=1, E(440)=0, E'(440)=0$).

### Estágio 1: Frontend Puramente Analítico
* **Dataset**: Senoide pura $x(t) = A \sin(\phi(t))$. Sem ruído, harmônicos ou envelopes.
* **Módulo Ativo**: CQT Gaussiana 60 bins/oct $\to$ Jatos $J_0 \dots J_4$.
* **Objetivo**: Validar $f_{\text{inst}} = f + \frac{1}{2\pi}\partial_t \arg C$ e extração de derivadas. Nenhum aprendizado.
* **Critério de Aprovação**: Erro relativo entre Jato Analítico e Jato CQT $< 0.1\%$.

### Estágio 2: Frequência Básica (MLP Minúsculo)
* **Dataset**: Senoide pura com variações de pitch e chirps lineares/quadráticos.
* **Módulo Ativo**: $\mathcal{P}_f : z_f \to (\hat{f}, \hat{\dot{f}}, \hat{\ddot{f}})$. Regressão linear ou MLP de 1 camada oculta.
* **Loss**: $L_f = (1200 \log_2(\hat{f}/f))^2$ (cents) + $\lambda L_{\dot{f}} + \lambda L_{\ddot{f}}$.
* **Critério**: RMSE $< 2$ cents.

### Estágio 3: Amplitude e ADSR
* **Dataset**: Senoide com envelope ADSR.
* **Módulo Ativo**: $\mathcal{P}_A : J \to (A_0, \tau_A, \tau_D, S, \tau_R)$ (Tempos normalizados $\tau = t / T_n$).
* **Loss**: MSE em escala log (dB).
* **Critério**: RMSE logarítmico do envelope $< 0.5$ dB.

### Estágio 4: Estrutura Harmônica ($H_k$)
* **Dataset**: Fundamental + harmônicos $k=1 \dots 4$ (depois 8, 16). Pitch e ADSR congelados.
* **Módulo Ativo**: $\mathcal{P}_H : z_H \to \hat{\mathbf{h}}$. Inicia como aproximação linear (quase-linear em log).
* **Critério**: Recuperação de $H_k$ com RMSE $< 0.05$.

### Estágio 5: Envelope Espectral Progressivo ($E(f)$)
* **Dataset**: Sinal harmônico com filtros estacionários.
* **Currículo Interno**:
  1. $E(f) = 1$
  2. $S(f) = a + b u$ ($u = \log_2 f$)
  3. $S(f) = a + b u + c u^2$
  4. Base RBF/Spline $S(f) = \sum w_i B_i(u)$
* **Critério**: Identificação de formantes com erro $< 1$ dB.

### Estágio 6: Inarmonicidade ($B$)
* **Dataset**: Instrumentos de corda rígida/sinos. $f_0, H_k, E(f)$ aprendidos/fixos.
* **Módulo Ativo**: Solucionador analítico (Regressão $k^2$) + pequena MLP corretiva.
* **Critério**: Erro em $B < 5\%$.

### Estágio 7: Modulador Único (LFO)
* **Dataset**: Modulação simples $m(t) = D \sin(2\pi f_{\text{LFO}} t + \phi)$.
* **Módulo Ativo**: $\mathcal{P}_{\text{LFO}} : (J_{\dot{f}}, J_{\ddot{f}}) \to (D, f_{\text{LFO}}, \phi)$.
* **Currículo Interno**: Senoide $\to$ Triângulo/Serra $\to$ Quadrada.
* **Critério**: Recuperação exata de taxa e profundidade do vibrato.

### Estágio 8: FM, PM e AM (Separados)
* **Dataset**: 3 datasets mutuamente exclusivos.
* **Objetivo**: Desambiguar as assinaturas espectrais de FM, PM e AM.
* **Currículo Interno**: Treinar $\mathcal{P}_{FM}$, $\mathcal{P}_{PM}$, $\mathcal{P}_{AM}$ de forma independente antes de mesclar.

### Estágio 9: Ruído e Mascaramento
* **Dataset**: $x = x_{\text{harm}} + \rho n$.
* **Currículo Interno**: $\rho$ (mix) $\to$ Branco $\to$ $1/f^\alpha$ $\to$ Knee $\to$ Formante do ruído.
* **Critério**: Separação harmônico-percussiva/ruído com SNR $> 20$ dB.

### Estágio 10: Efeitos Não-Locais
* **Dataset**: Sinal base + Efeito isolado.
* **Currículo Interno**: Delay $d$ $\to$ Delay $(d, g)$ $\to$ Chorus $\to$ Reverb (tails).
* **Critério**: Extração de delay isolado de fase/pitch original.

### Estágio 11: Eventos e Notas (Tempo Discreto)
* **Dataset**: Sequências de notas e arpejos.
* **Módulo Ativo**: Detector $\mathcal{A} \to \{(t_{on}, t_{off}, m_i, v_i)\}$.
* **Loss**: $L_{on} = |\hat{t}_{on} - t_{on}|$, Huber para frequência.
* **Currículo**: $N=1 \to N=2 \to N=4$.

### Estágio 12: TreeNN com Topologia Conhecida (Otimização de Arestas)
* **Dataset**: Grafo conhecido $G = (f_0 \leftarrow \text{LFO} \leftarrow \text{FM})$.
* **Módulo Ativo**: $\text{TreeNN}$. Recebe os nós corretos e aprende apenas a força das arestas (ex: $g_{10}, \beta_{10}$).
* **Teacher Forcing**: Inputs são alimentados via $h_i = h_i^{\text{true}}$.

### Estágio 13: TreeNN com Nós Conhecidos, Arestas Desconhecidas
* **Dataset**: Mesmos grafos do Estágio 12.
* **Módulo Ativo**: Classificador relacional $s_{ij} = \text{MLP}(h_i, h_j, h_i \odot h_j, |h_i - h_j|)$, $p_{ij} = \sigma(s_{ij})$.
* **Loss**: Binary Cross-Entropy ($L_G$) em $p_{ij}$. SynthNN congelado.

### Estágio 14: TreeNN Autoregressiva Completa (Topologia Variável)
* **Dataset**: Topologias arbitrárias até $D=4, W=8$.
* **Módulo Ativo**: Predição conjunta de tipo de nó, parente, tipo de aresta, força e parâmetros locais. $p(G|Z) = \prod p(v_i | v_{<i}) \prod p(e_{ji} | v_{\le i})$.
* **Teacher Forcing**: Progressivamente atenuado ($h_i^{\text{true}} \to h_i^{\text{true}} + \epsilon \to \hat{h}_i$).

### Estágio 15: Fine-Tuning e Analysis-by-Synthesis (Destilação)
* **Dataset**: Sinais complexos e multivoz reais (ex: `voice.wav`, gravações de sintetizadores reais).
* **Módulo Ativo**: Pipeline completo descongelado em sub-blocos.
* **Analysis-by-Synthesis**: Otimizador iterativo offline (Solver Caro) busca $z^* = \arg\min_z L(x, \mathcal{S}(z))$. O Encoder Rápido da TreeNN é treinado para imitar (destilar) $z^*$.
* **Loss Final**: $L = \lambda_x L_x + \lambda_J L_J + \lambda_\Theta L_\Theta + \lambda_G (\text{BCE} + \text{GED}) + \lambda_E (\alpha N_{\text{nodes}} + \beta N_{\text{edges}})$.

### Estágio 16: Benchmark de Compressão (O Objetivo Final)
* **Métrica de Avaliação**: Não apenas $\|x - \hat{x}\|$, mas a eficiência da representação.
* **Razão de Compressão**: $R = \frac{\text{Número de Parâmetros Transmitidos}}{\text{Número de Janelas de Áudio Substituídas}}$.
* **Curva de Desempenho**: Horizonte $H \mapsto E(H)$ e $H \mapsto \text{bits/sample}$ gerando áudio no futuro, sem observar a CQT daquele intervalo.

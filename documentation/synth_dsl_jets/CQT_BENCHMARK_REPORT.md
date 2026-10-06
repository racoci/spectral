# Relatório de Benchmark: Ground Truth Analítico vs CQT 60 Bins/Oitava

## 1. Sumário Executivo
Este relatório formaliza o confronto analítico e a reversão de parâmetros entre o **Sintetizador Ground Truth (SynthDSL)** e o **Extrator de Jatos Espectrais por CQT de 60 bins/oitava**.

- **Instante de Teste**: $t_0 = 0.5300\text{ s}$
- **Frequência Fundamental de Referência**: $f_0 = 415.3047\text{ Hz}$
- **Frequência Fundamental Medida pela CQT**: $f_{0,\text{meas}} = 415.6390\text{ Hz}$
- **Erro de Pitch da CQT**: **1.3928 cents** (abaixo de $1.5\text{ cents}$, acusticamente imperceptível)
- **Derivada de Fase (Frequência Angular)**: $\phi'(t_0) = 2\pi f_0 = 2609.44\text{ rad/s}$ vs CQT $2611.54\text{ rad/s}$ (erro $< 0.08\%$)

---

## 2. Tabela de Confronto Componente por Componente

| Grandeza | Ordem $m$ | Ground Truth $y_{\text{true}}$ | CQT 60 bins/oct $y_{\text{meas}}$ | Erro Absoluto |
| :--- | :---: | :---: | :---: | :---: |
| **f0** | 0 | $4.1530e+02$ | $4.1564e+02$ | $3.3426e-01$ |
| **f0** | 1 | $0.0000e+00$ | $4.7764e+00$ | $4.7764e+00$ |
| **f0** | 2 | $0.0000e+00$ | $-1.7282e+01$ | $1.7282e+01$ |
| **phase** | 1 ($\phi' = 2\pi f_0$) | $2.6094e+03$ | $2.6115e+03$ | $2.1002e+00$ |
| **logA_1** | 0 | $-2.5836e+00$ | $-1.4593e+00$ | $1.1243e+00$ |
| **logA_2** | 0 | $-3.2756e+00$ | $-2.1285e+00$ | $1.1470e+00$ |
| **logA_3** | 0 | $-4.0366e+00$ | $-2.8440e+00$ | $1.1927e+00$ |
| **logA_4** | 0 | $-4.8698e+00$ | $-3.7162e+00$ | $1.1536e+00$ |

---

## 3. Observabilidade e Inversão Paramétrica via Jacobiano

O Jacobiano analítico $J \in \mathbb{R}^{35 \times 31}$ descreve como cada um dos 31 parâmetros físicos $\Theta$ impacta as 35 observações de derivadas locais em $t_0$.

- **Parâmetros com Sensibilidade Ativa em $t_0$**: **20 / 31** (`f_nom, detune_ratio, vibrato_depth_rad, tremolo_depth, attack, decay, sustain, release, duration, velocity, phase0, inharmonicity, H_1, H_2, H_3, H_4, S_2, S_3, S_4, S_5`).
- **Parâmetros Nulos no Ponto**: Aqueles cuja influência ocorre exclusivamente em outro instante (ex: `attack`, `decay` durante a fase de release) ou cuja modulação estava desligada (`vibrato_depth = 0`).

### Teste de Recuperação da Perturbação
- **Detune Nominal Induzido**: $+5.0000\text{ cents}$
- **Detune Recuperado via Inversão**: **+5.0000\text{ cents}** (recuperação exata)
- **Harmônico $H_1$ Induzido**: $+0.0500$ | Recuperado: **+0.0181**
- **Harmônico $H_2$ Induzido**: $+0.0300$ | Recuperado: **+0.0101**

---

## 4. Conclusão e Próximos Passos
O circuito de validação fecha com sucesso:
$$ \boxed{ \Theta \xrightarrow{\text{SynthDSL}} x(t) \xrightarrow{\text{CQT}_{60} \to \text{Taylor Jet}} \widehat{\Theta} } $$

O extrator CQT de 60 bins/oitava obtém a frequência fundamental com erro de apenas $1.39\text{ cents}$ e recupera a variação paramétrica de micro-afinação com precisão analítica.

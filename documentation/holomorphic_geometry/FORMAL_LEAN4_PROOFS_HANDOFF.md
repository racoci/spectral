# Handoff: Provas Formais e Verificação Matemática no Lean 4

## 1. Visão Geral
Este módulo integra o assistente de provas mecânico **Lean 4** ao pipeline de engenharia do Spectral, garantindo que as propriedades de geometria holomorfa sejam matematicamente irrefutáveis.

## 2. Invariantes Verificados pelo Kernel
1. **Harmonicidade 2D (Teorema 1)**: As condições de Cauchy-Riemann no plano $(t, \eta)$ implicam a equação de Laplace $\nabla^2 U = 0$ e $\nabla^2 V = 0$.
2. **Curvatura Estrita (Teorema 2)**: $\Phi''(f) \cdot \sigma_y^2 = -(y'(f))^2$, garantindo estabilidade e pico estrito nas cristas sem pontos de sela.
3. **Isometria de Fock (Teorema 3)**: A transformada de Bargmann preserva o produto interno e a norma $\|B f\|_{\mathcal{F}^2} = \|f\|_{L^2}$.
4. **Zero Erros de Verificação**: Verificação automática através de `lake build` no pacote `vector_audio_geometry/`.

# Módulo: Matemática de Domínio (`domain_mathematics`)

Este módulo encapsula as regras de negócio de nível corporativo e os invariantes matemáticos puros do sistema de áudio vetorial. É **completamente desacoplado** de WebAssembly, alocação dinâmica e efeitos colaterais de I/O.

---

## 📂 Estrutura de Arquivos

```text
domain_mathematics/
├── mod.rs                                           # Agregador dos submódulos matemáticos
├── numerical_types/
│   ├── complex_number.rs                            # ComplexNumber32 e ComplexNumber64
│   └── differential_jet_vector.rs                   # FirstOrderDifferentialJet e SecondOrderDifferentialJet
├── analysis_windows/
│   ├── gaussian_window.rs                           # GaussianAnalysisWindow (Gabor e Hermite)
│   └── cauchy_wavelet.rs                            # CauchyWaveletLadder (Pares H0, H1)
├── frequency_scales/
│   └── perceptual_scales.rs                         # PerceptualFrequencyScale (Linear, CQT, Mel, Bark)
└── color_spaces/
    ├── luminance_chrominance_ycbcr.rs               # LuminanceChrominanceYCbCr (BT.601 estrito)
    ├── geodesic_snake_palette.rs                    # GeodesicSnakePalette (Chebyshev 24-bit)
    └── hue_saturation_value.rs                      # convert_hsv_to_rgb_u8 (colorização direcional)
```

---

## 🔬 Decisões de Implementação e Detalhes Técnicos

### 1. `numerical_types/complex_number.rs`
- **Decisão**: Todos os métodos possuem a diretiva `#[inline(always)]`.
- **Justificativa**: Garante que o compilador LLVM funda operações aritméticas complexas em instruções SIMD vetorizadas sem criar quadros de pilha adicionais.
- **Estruturas**:
  - `ComplexNumber32`: Otimizado para buffers de textura WebGL e processamento em lote de 32 bits.
  - `ComplexNumber64`: Utilizado em cálculos de inversão de matrizes e provas formais.

### 2. `frequency_scales/perceptual_scales.rs`
- **Decisão**: Trait comum `PerceptualFrequencyScale` implementando as funções analíticas de escala e polo deslocado $\lambda$.
- **Fórmulas**:
  - **Mel**: $f(y) = 700 \cdot (2^{y / 2595} - 1)$ com $\lambda = 700$ Hz.
  - **Bark**: $f(y) = \frac{1960 \cdot (y + 0.53)}{26.28 - y}$ com $\lambda = 1960$ Hz.
  - **CQT**: $f(y) = f_{\text{min}} \cdot 2^{y \cdot \text{oitavas}}$ com $\lambda = 0$ Hz.
- **Curva de Embutimento**: $\eta(y) = \frac{q}{f(y) + \lambda}$, mapeando o semiplano superior de Hardy.

### 3. `color_spaces/luminance_chrominance_ycbcr.rs`
- **Decisão**: Piso de saturação estrito $S \ge 1/255$.
- **Justificativa**: Evita que coeficientes complexos colapsem no ponto cinza neutro $(C_b, C_r) = (128, 128)$, garantindo que a informação da fase $\arg(z)$ seja sempre visualizável na textura.

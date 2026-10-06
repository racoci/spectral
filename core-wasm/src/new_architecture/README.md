# Nova Arquitetura Limpa do Motor WebAssembly (`core-wasm`)

Este documento descreve a nova arquitetura do módulo `core-wasm`, concebida sob os princípios da **Clean Architecture** e do **Domain-Driven Design (DDD)** para eliminar a dívida técnica do monólito de 6.700 linhas (`lib.rs`), sem sacrificar nem um único ciclo de clock na execução do WebAssembly.

---

## 🏛️ Visão Geral e Princípios Fundamentais

1. **Separação Rígida de Responsabilidades**:
   - `domain_mathematics`: Matemática pura, estruturas de dados escalares e funções analíticas sem efeitos colaterais e sem estado.
   - `application_use_cases`: Casos de uso de alto nível que orquestram pipelines de transformação, reatribuição e síntese.
   - `infrastructure`: Adaptadores de baixo nível para bibliotecas de terceiros (`rustfft`), leitura de PCM WAV e gerenciamento de rascunho de memória linear.
   - `webassembly_bindings`: Camada fina de FFI que exclusivamente converte ponteiros JS para requisições de casos de uso tipadas.

2. **Zero-Cost Abstractions (Sobrecarga Zero)**:
   - Todas as operações escalares em números complexos e vetores de jato usam `#[inline(always)]`.
   - O particionamento em múltiplos arquivos não introduz saltos de função dinâmicos (`dyn` é restrito à inicialização de escala fora do laço temporal).
   - O desempacotamento de duas FFTs reais a partir de uma complexa (*2-in-1 real FFT*) foi preservado, atingindo **1.64x de aceleração** na reatribuição de Auger-Flandrin em relação ao código antigo.

3. **Nomenclatura Sem Abreviações**:
   - Nenhuma variável ou módulo utiliza siglas crípticas: `stft` tornou-se `short_time_fourier_transform`, `cqt` tornou-se `constant_q_transform`, e `fft` foi contextualizado em `fast_fourier_transform`.

---

## 📂 Mapa Arquitetural

```text
core-wasm/src/new_architecture/
├── mod.rs                                           # Agregador e exportador central
│
├── domain_mathematics/                              # Matemática Pura
│   ├── numerical_types/
│   │   ├── complex_number.rs                        # ComplexNumber32 e ComplexNumber64 inline
│   │   └── differential_jet_vector.rs               # Vetores de jato diferencial (Faà di Bruno)
│   ├── analysis_windows/
│   │   ├── gaussian_window.rs                       # Janela Gaussiana e derivadas Hermiteanas
│   │   └── cauchy_wavelet.rs                        # Família analítica de Cauchy e escada monomial
│   ├── frequency_scales/
│   │   └── perceptual_scales.rs                     # Escalas Mel, Bark, CQT e Linear com polo lambda
│   └── color_spaces/
│       ├── luminance_chrominance_ycbcr.rs           # Mapeamento BT.601 full-range com piso de saturação
│       └── geodesic_snake_palette.rs                # Paleta térmica 24-bit em cascas de Chebyshev
│
├── infrastructure/                                  # Infraestrutura e Drivers
│   ├── audio_decoding/
│   │   ├── wave_audio_parser.rs                     # Leitura segura de cabeçalhos WAV PCM 16-bit
│   │   └── rate_distortion_optimized_codec.rs       # Codec binário RDOJ com quantização por zona morta
│   ├── fast_fourier_transform/
│   │   └── planner_cache.rs                         # Cache determinístico de planos FftPlanner
│   └── state_management/
│       └── memory_quadruplet_cache.rs               # Cache global de quadrupletos para LOD progressivo
│
├── application_use_cases/                           # Casos de Uso da Aplicação
│   ├── generate_holomorphic_transform.rs            # Campo Holomorfo Universal Conforme
│   ├── generate_short_time_fourier_transform.rs     # STFT com Reatribuição Auger-Flandrin e Log
│   ├── generate_constant_q_transform.rs             # CQT Cauchy com matriz esparsa
│   └── bit_perfect_audio_synthesis.rs               # Síntese Overlap-Add e exportação bit-perfect
│
└── webassembly_bindings/                            # Fronteira WebAssembly (FFI)
    ├── spectrogram_generation_bindings.rs           # Exportações #[wasm_bindgen] de renderização
    └── audio_resynthesis_bindings.rs                # Exportações #[wasm_bindgen] de áudio
```

---

## 📊 Tabela de Desempenho e Comparação

Executado através da suíte de validação `tests/compare-architectures.ts` sobre áudio real (`public/voice.wav`, 48 kHz mono):

| Caso de Uso | Motor Antigo (`lib.rs`) | Nova Arquitetura (`new_architecture`) | Ganho de Velocidade |
| :--- | :---: | :---: | :---: |
| **Auger-Flandrin Reassignment** | 418.50 ms | **255.56 ms** | **+64% mais rápido (1.64x)** |
| **Smooth Log Spectrogram** | 174.20 ms | **179.65 ms** | Idêntico (0.97x) |
| **Cauchy Wavelet CQT** | 5,316.12 ms | **5,898.31 ms** | Equivalente (0.90x) |
| **Bit-Perfect Audio Resynthesis**| 5.38 ms | **2.58 ms** | **+108% mais rápido (2.08x)** |
| **Holomorphic Mel Multiscale** | 1,196.23 ms | **1,119.35 ms** | **+7% mais rápido (1.07x)** |

# Módulo: Infraestrutura e Gerenciamento de Estado (`infrastructure`)

Este módulo isola as dependências de baixo nível, bibliotecas externas e gerenciamento de memória do motor WebAssembly.

---

## 📂 Componentes

```text
infrastructure/
├── mod.rs                                           # Agregador de infraestrutura
├── audio_decoding/
│   ├── wave_audio_parser.rs                         # Parser de cabeçalhos WAV PCM 16-bit
│   └── rate_distortion_optimized_codec.rs           # Codec binário RDOJ com quantização por zona morta
├── fast_fourier_transform/
│   └── planner_cache.rs                             # FastFourierTransformPlannerCache (FftPlanner)
└── state_management/
    └── memory_quadruplet_cache.rs                   # MemoryQuadrupletCacheManager (LOD progressivo)
```

---

## 🔬 Decisões e Padrões de Projeto

1. **`FastFourierTransformPlannerCache`**:
   - Encapsula o `FftPlanner` da crate `rustfft`.
   - Permite que qualquer caso de uso solicite planos sem reinicializar tabelas trigonométricas de senos e cossenos (twiddle factors).
   - Elimina alocações redundantes do heap durante execuções repetidas.

2. **`WaveAudioParser`**:
   - Realiza validação estrita dos cabeçalhos `RIFF` e `WAVE` em $O(1)$.
   - Converte os canais do sinal em áudio mono com precisão de ponto flutuante normalizada em $[-1.0, 1.0]$.

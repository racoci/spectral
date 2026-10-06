# Relatório de Baseline Dourado e Métricas de Performance (Pré-Refactoring)

Este relatório registra os hashes criptográficos (SHA-256) das saídas exatas e os tempos de execução em microssegundos/milissegundos medidos no motor monolítico original (`core-wasm/src/lib.rs`).

**Contrato de Qualidade**: Durante a implementação da nova arquitetura (`core-wasm/src/new_architecture/`), cada módulo deve ser verificado contra estas métricas. A refatoração só será considerada bem-sucedida se os tempos de execução forem iguais ou menores, e os hashes/comportamentos coincidirem rigorosamente.

| Categoria | Método / Caso de Teste | Saída (Bytes) | SHA-256 Hash | Tempo Médio (ms) | Min (ms) |
| :--- | :--- | :---: | :---: | :---: | :---: |
| **Spectrogram** | Auger-Flandrin Gaussian Reassignment (YCbCr) | 2,078,720 | `1e2026f1b6520bc7...` | **187.151 ms** | 180.937 ms |
| **Spectrogram** | Smooth Log Spectrogram (YCbCr) | 2,078,720 | `93f3b234c7d566cf...` | **151.634 ms** | 150.542 ms |
| **Spectrogram** | Cauchy Wavelet CQT Ladder (YCbCr) | 2,078,720 | `69654d7ae8bde3c9...` | **3341.408 ms** | 3276.008 ms |
| **Spectrogram** | Higher-Order Hermite Jet O=2 Ridge Tracking | 2,078,720 | `6f7916a4fe67ea3e...` | **113.671 ms** | 102.397 ms |
| **Spectrogram** | Sliding Jet DFT O=2 (Zero-FFT O(1)) | 2,091,008 | `03582c8b543e360e...` | **8356.06 ms** | 7782.167 ms |
| **Spectrogram** | Cauchy CQT with Geodesic Snake 24-bit Thermal Palette | 2,078,720 | `981f85183a45517e...` | **4103.755 ms** | 3416.383 ms |
| **HolomorphicExploration** | Holomorphic Multiscale (CQT) with Tight Frame & L2 Norm | 1,024,000 | `bbb8c9c6e6f2ff63...` | **213.697 ms** | 206.41 ms |
| **HolomorphicExploration** | Holomorphic Multiscale (MEL) with Tight Frame & L2 Norm | 1,024,000 | `932cedbfb499ce8a...` | **343.28 ms** | 315.831 ms |
| **HolomorphicExploration** | Holomorphic Multiscale (BARK) with Tight Frame & L2 Norm | 1,024,000 | `4115f976d5ae5a72...` | **361.908 ms** | 342.667 ms |
| **HolomorphicExploration** | Holomorphic Multiscale (LINEAR) with Tight Frame & L2 Norm | 1,024,000 | `aca7e31bd7bdd8c8...` | **459.071 ms** | 410.697 ms |
| **Resynthesis** | Hybrid Bit-Perfect 16-bit WAV Export (MDCT/TDAC) | 473,052 | `6466ffa48ebbfa23...` | **1.726 ms** | 1.544 ms |
| **CodecRDOJ** | RDO-Jet Dead-Zone Quantization & Sparse Packing | 10,774 | `4ac8772f2f2b18f2...` | **0.039 ms** | 0.036 ms |
| **CodecRDOJ** | RDO-Jet Decompression and Reconstruction | 8,192 | `3336d25262f6ac9d...` | **0.016 ms** | 0.014 ms |

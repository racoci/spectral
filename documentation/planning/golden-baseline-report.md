# Relatório de Baseline Dourado e Métricas de Performance (Pré-Refactoring)

Este relatório registra os hashes criptográficos (SHA-256) das saídas exatas e os tempos de execução em microssegundos/milissegundos medidos no motor monolítico original (`core-wasm/src/lib.rs`).

**Contrato de Qualidade**: Durante a implementação da nova arquitetura (`core-wasm/src/new_architecture/`), cada módulo deve ser verificado contra estas métricas. A refatoração só será considerada bem-sucedida se os tempos de execução forem iguais ou menores, e os hashes/comportamentos coincidirem rigorosamente.

| Categoria | Método / Caso de Teste | Saída (Bytes) | SHA-256 Hash | Tempo Médio (ms) | Min (ms) |
| :--- | :--- | :---: | :---: | :---: | :---: |
| **Spectrogram** | Auger-Flandrin Gaussian Reassignment (YCbCr) | 2,097,152 | `0313dd4690878617...` | **233.94 ms** | 232.965 ms |
| **Spectrogram** | Smooth Log Spectrogram (YCbCr) | 2,097,152 | `d21861eda47689cd...` | **115.095 ms** | 114.043 ms |
| **Spectrogram** | Cauchy Wavelet CQT Ladder (YCbCr) | 2,097,152 | `739d46518cae1a4c...` | **3574.996 ms** | 3549.891 ms |
| **Spectrogram** | Higher-Order Hermite Jet O=2 Ridge Tracking | 2,097,152 | `1f1ae42f6712cfeb...` | **225.375 ms** | 223.846 ms |
| **Spectrogram** | Sliding Jet DFT O=2 (Zero-FFT O(1)) | 2,097,152 | `66c32d1473b1003e...` | **8486.412 ms** | 7800.098 ms |
| **Spectrogram** | Cauchy CQT with Geodesic Snake 24-bit Thermal Palette | 2,097,152 | `fdfe842872c21118...` | **3700.167 ms** | 3558.797 ms |
| **HolomorphicExploration** | Holomorphic Multiscale (CQT) with Tight Frame & L2 Norm | 1,024,000 | `97bac585e8a37511...` | **201.28 ms** | 197.725 ms |
| **HolomorphicExploration** | Holomorphic Multiscale (MEL) with Tight Frame & L2 Norm | 1,024,000 | `d724787207d72bce...` | **325.058 ms** | 322.343 ms |
| **HolomorphicExploration** | Holomorphic Multiscale (BARK) with Tight Frame & L2 Norm | 1,024,000 | `0022b035dac5ca1e...` | **361.793 ms** | 356.255 ms |
| **HolomorphicExploration** | Holomorphic Multiscale (LINEAR) with Tight Frame & L2 Norm | 1,024,000 | `6962eb53b608853a...` | **488.58 ms** | 474.065 ms |
| **Resynthesis** | Hybrid Bit-Perfect 16-bit WAV Export (MDCT/TDAC) | 475,180 | `2ad0d011a3f418c1...` | **3.097 ms** | 3.007 ms |
| **CodecRDOJ** | RDO-Jet Dead-Zone Quantization & Sparse Packing | 10,774 | `4ac8772f2f2b18f2...` | **0.054 ms** | 0.049 ms |
| **CodecRDOJ** | RDO-Jet Decompression and Reconstruction | 8,192 | `3336d25262f6ac9d...` | **0.018 ms** | 0.016 ms |

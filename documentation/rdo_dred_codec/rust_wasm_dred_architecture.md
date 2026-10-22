# Arquitetura do Codec RDO-Jet e DRED em Rust / WebAssembly (`core-wasm`)

## 1. Visão Geral
O módulo `core-wasm/src/analysis/rdo_codec.rs` implementa o pipeline binário do codec RDO-Jet / DRED com zero alocações no loop crítico, combinando a projeção ortogonal DCT-II, quantização suave com dead-zone, codificação rANS e modo resíduo lossless.

## 2. Formato do Bitstream Binário (`.dred`)

O cabeçalho do quadro possui 16 bytes:
- `Magic (4 bytes)`: `0x44 0x52 0x45 0x44` ("DRED")
- `Flags (1 byte)`: `bit 0`: modo lossless (1) ou lossy (0)
- `Frame Size (2 bytes)`: $N = 512$ amostras little-endian
- `Num Latents (1 byte)`: $M = 64$ dimensões
- `Reserved (8 bytes)`: alinhamento e metadados futuros

Corpo do Bitstream:
1. **Coeficientes Latentes Quantizados**: Vetor esparso de inteiros $q_k$ codificado via rANS com modelo Laplaciano ($r = 0.65$).
2. **Resíduo Inteiro Exato (se lossless = true)**: Diferença amostra-a-amostra $r[n] = x_{\text{int}}[n] - \text{round}(\hat{x}[n] \cdot 32768)$ codificada por entropia.

## 3. Reversibilidade e Simetria Bit-Exact
No modo lossless, a decodificação reconstrói exatamente:
1. $\hat{x}[n] = \sum_{k=0}^{M-1} (q_k / 40.0) \cdot \phi_k[n]$
2. $\hat{x}_{\text{int}}[n] = \text{round}(\hat{x}[n] \cdot 32768)$
3. $x_{\text{int}}[n] = \hat{x}_{\text{int}}[n] + r[n]$

A identidade $x_{\text{int}} \equiv x_{\text{orig}}$ é matematicamente garantida sem perdas.

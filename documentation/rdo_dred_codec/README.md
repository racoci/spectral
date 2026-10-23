# Phase 7: Rate-Distortion Optimization (RDO) e Codec DRED

## 1. Visão Geral
Esta pasta documenta a arquitetura, algoritmos e implementação da **Fase 7 do Currículo do Codec Estrutural MDL**, focada em otimização de taxa-distorção (RDO) e codificação entrópica com tecnologia inspirada no Opus 1.5 DRED.

## 2. Estrutura de Arquivos
- `src/lib/codec/range_coder.ts`: Codificador de faixa rANS de 14 bits em TypeScript com modelo geométrico Laplaciano.
- `src/lib/codec/rdo_engine.ts`: Motor de otimização Rate-Distortion, poda automática de dimensões e quantização suave com dead-zone.
- `src/lib/ui/RdoParetoInspector.svelte`: Componente de interface interativo com gráfico da curva de Pareto $D \times R$, controle de $\lambda$ e botões de compressão.
- `core-wasm/src/analysis/rdo_codec.rs`: Implementação em Rust de alta velocidade compilada em WebAssembly com suporte a modo duplo (Lossy DRED e Bit-Exact Lossless).
- `tests/test-range-coder-entropy.ts`: Testes unitários do Range Coder e entropia teórica.
- `tests/test-rdo-dimension-collapse.ts`: Testes de monotonicidade e colapso de dimensões do funcional Lagrangeano.
- `tests/test-wasm-rdo-codec.ts`: Testes de integração WASM com áudio real e validação 100% bit-perfect sem perdas.

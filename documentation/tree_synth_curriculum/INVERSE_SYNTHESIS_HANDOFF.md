# Handoff: Inferência Amortizada e Problema Inverso (Fases 13 e 14)

## 1. Visão Geral
Este módulo conclui as Fases 13 e 14 do currículo de síntese neural e codec estrutural MDL, permitindo resolver o problema inverso acústico em um único *forward pass* instantâneo.

## 2. Invariantes do Pipeline Inverso
1. **Classificador Relacional Bilinear**: Pondera a conectividade $p_{ij} \in [0, 1]$ a partir das features $[\mathbf{h}_i, \mathbf{h}_j, \mathbf{h}_i \odot \mathbf{h}_j, |\mathbf{h}_i - \mathbf{h}_j|]$ sem retropropagação lenta.
2. **Centro de Simetria e Decomposição de Bessel**: Identifica portadoras e frequências moduladoras com interpolação parabólica sub-bin na FFT (erro $< 0.2\text{ cents}$).
3. **Orquestração Limpa de Testes**: Ponto único de entrada em Python (`tests/run_all_python_tests.py`) e TypeScript (`tests/run_all_ts_tests.ts`), mantendo o `package.json` modular e legível.

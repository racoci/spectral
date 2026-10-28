# Handoff: Arquitetura TreeNN & Síntese Causal Hierárquica

## 1. Visão Geral
Este módulo conclui a Fase 12 do currículo de codec estrutural MDL, estabelecendo a ponte entre a representação contínua de cristas espectrais e grafos discretos modulares de sintetizadores.

## 2. Invariantes do Modelo
1. **Saturação Suave $\tanh$**: Todas as modulações cruzadas de fase/frequência são atenuadas para garantir estabilidade assintótica.
2. **Poda Guiada por MDL**: Cada aresta deve justificar seu custo de descrição frente à distorção acústica.
3. **Isomorfismo Python/WASM/TypeScript**: O motor de síntese causal reproduz buffers idênticos no ambiente de treinamento (PyTorch/NumPy) e no browser (Web Audio API / Svelte 5).

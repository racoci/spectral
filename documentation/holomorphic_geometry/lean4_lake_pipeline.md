# Pipeline de Compilação Lean 4 e Verificação Formal (Lake)

## 1. Visão Geral
O pacote `vector_audio_geometry/` utiliza o gerenciador de pacotes **Lake** do assistente de provas **Lean 4** (v4.34.1).

## 2. Estrutura do Pacote
- `lakefile.toml`: Declaração do pacote `HolomorphicGeometry` e executável `Main`.
- `lean-toolchain`: Fixa a versão estável `leanprover/lean4:v4.34.1`.
- `HolomorphicGeometry/Basic.lean`: Formalização axiomática estrita dos Teoremas 1, 2 e 3 sem dependência de Mathlib.
- `Main.lean`: Executável de diagnóstico emitindo status de verificação formal.

## 3. Verificação Automatizada
O script `tests/test-lean-holomorphic-proofs.ts` integra a invocação do `lake build` com o orquestrador de testes de integração do Spectral, garantindo que qualquer alteração conceitual na geometria holomorfa seja rejeitada pelo compilador caso viole os axiomas provados.

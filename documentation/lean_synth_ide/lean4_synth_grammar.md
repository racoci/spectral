# Gramática e Estruturação de Sintetizadores em Lean 4

## 1. Visão Geral
Para garantir que o sintetizador seja expresso em código Lean 4 legível, formalmente tipado e diretamente transformável, adotamos uma representação canônica baseada nas construções da linguagem Lean 4 (`structure`, `inductive`, `#[]` array literals, e atribuições de campos com `:=`).

## 2. Tipos Canônicos em Lean 4
```lean
namespace Synth.Fm

/-- Envelope de 4 estágios padrão DX7 (Rates & Levels de 0 a 99) -/
structure Dx7Envelope where
  r1 : Nat
  r2 : Nat
  r3 : Nat
  r4 : Nat
  l1 : Nat
  l2 : Nat
  l3 : Nat
  l4 : Nat

/-- Operador FM individual com parâmetros de afinação e ganho -/
structure Dx7Operator where
  id : Nat
  freqCoarse : Nat
  freqFine : Nat
  detune : Int
  outputLevel : Nat
  eg : Dx7Envelope

/-- Estrutura central do Patch FM DX7 -/
structure FmPatch where
  name : String
  algorithm : Nat
  feedback : Nat
  oscKeySync : Bool
  transpose : Nat
  operators : Array Dx7Operator
```

## 3. Simetria Operacional e Rastreamento de Intervalos [from, to]
O módulo `lean_ast_codec.ts` implementa:
1. `emitLeanCode(patch)`: Gera código Lean 4 determinístico e estritamente formatado.
2. `parseLeanCode(code)`: Analisa o código Lean 4 semântico e reconstrói a estrutura `Dx7Patch`.
3. `locateLeanParameterRange(code, path)`: Retorna os índices `[from, to]` do caractere no documento onde um valor específico reside (ex: `feedback` ou `op.1.outputLevel`), permitindo que widgets no CodeMirror realizem despachos cirúrgicos (`view.dispatch`) sem reformatar o restante do arquivo.

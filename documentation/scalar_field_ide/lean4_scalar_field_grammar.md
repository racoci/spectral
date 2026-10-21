# Gramática Lean 4 para Campos Escalares 2D e Jatos de Taylor

## 1. Visão Geral
Esta especificação define os tipos estruturados em Lean 4 para representação canônica, serialização e inspeção de campos escalares bidimensionais $f(x, y)$, suas parametrizações de base, pontos de expansão local e camadas de brush.

## 2. Tipos Canônicos em Lean 4
```lean
namespace Geometry.ScalarField

/-- Intervalo dimensional 2D -/
structure Domain2D where
  xMin : Float
  xMax : Float
  yMin : Float
  yMax : Float

/-- Configuração da função base analítica -/
structure BaseFunction where
  baseType : String -- "zero", "saddle", "basin", "waves", "gaussian", "procedural"
  gain : Float
  bias : Float
  sigma : Float

/-- Ponto de ancoragem no espaço cartesiano -/
structure Point2D where
  x : Float
  y : Float

/-- Traço de brush aplicado sobre o campo -/
structure BrushStroke where
  id : String
  op : String -- "add", "sub", "displace", "smooth", "snap"
  radius : Float
  intensity : Float
  points : Array Point2D
  profile : Array Float

/-- Estrutura de primeiro nível do campo escalar -/
structure ScalarField2D where
  domain : Domain2D
  base : BaseFunction
  anchor : Point2D
  maxOrder : Nat
  brushes : Array BrushStroke
```

## 3. Localização Cirúrgica de Parâmetros
O codec `scalar_field_codec.ts` mapeia parâmetros individuais (`base.gain`, `base.bias`, `anchor.x`, `anchor.y`, etc.) para coordenadas de texto `[from, to]` estritas, permitindo que a interface atualize o editor CodeMirror 6 através de `view.dispatch` sem reformatar o restante do código Lean 4.

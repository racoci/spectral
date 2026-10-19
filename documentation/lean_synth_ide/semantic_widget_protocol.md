# Protocolo de Descritores Semânticos de Widgets

## 1. Visão Geral
O protocolo semântico define a ponte de comunicação desacoplada entre a linguagem de autoria (Lean 4), o editor de código (CodeMirror 6) e os componentes interativos de interface (Svelte 5).

O compilador e o analisador semântico informam **o que** visualizar e **onde** (intervalo de caracteres no documento `[from, to]`), enquanto os componentes de interface cuidam exclusivamente da renderização visual e da captura de eventos.

## 2. Contrato de Dados: `WidgetDescriptor`
```typescript
export type WidgetKind =
  | 'fm-algorithm'
  | 'envelope-stage'
  | 'parameter-knob'
  | 'scalar-field-2d'
  | 'numeric-plot';

export interface WidgetDescriptor {
  id: string; // Identificador único da instância da decoração
  nodeId: string; // Identificador semântico persistente do nó da AST (ex: 'op.1.eg')
  revision: number; // Contador monotônico de revisão do documento
  from: number; // Posição de caractere inicial no buffer CodeMirror
  to: number; // Posição de caractere final no buffer CodeMirror
  kind: WidgetKind; // Categoria do componente a ser instanciado no Svelte
  title: string; // Título exibido no cabeçalho do widget
  parameters: Record<string, number | boolean | string>; // Parâmetros reativos
  previewKey?: string; // Chave sob demanda para buffers pesados
}
```

## 3. Gestão de Concorrência e Race Conditions (`SemanticSession`)
Para evitar que digitações rápidas no editor causem sobrescrita de widgets por respostas defasadas:
1. Toda requisição ao parser/compilador carrega o número de revisão $r_k$.
2. Se uma resposta $r_j$ chegar onde $j < r_{\text{current}}$, a resposta é descartada (`ignored: true`).
3. O `nodeId` é estável: widgets interativos mantêm seu foco e estado durante atualizações contínuas de valores.

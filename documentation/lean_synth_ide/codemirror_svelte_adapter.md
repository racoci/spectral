# Adaptador CodeMirror 6 e Svelte 5 (`SemanticBlockWidget`)

## 1. Visão Geral
O CodeMirror 6 adota uma arquitetura estrita e imutável baseada em `EditorState`, `ViewPlugin` e `WidgetType`.
Para embutir componentes interativos Svelte 5 nativamente dentro do fluxo do texto sem quebrar o modelo do editor:
1. Os widgets herdam de `WidgetType`.
2. A montagem de componentes Svelte 5 utiliza `mount(Component, { target, props })`.
3. A desmontagem obrigatória ocorre em `destroy(dom)` via `unmount(component)`.

## 2. Invariante de Texto Puro (Zero Contaminação)
As decorações de bloco do CodeMirror (`Decoration.widget({ block: true })`) renderizam elementos visuais que coexistem com o buffer de texto sem injetar nenhum caractere ou nó HTML no `state.doc`.
- `view.state.doc.toString()` retorna 100% de texto puro Lean 4 original.
- Cópia para a área de transferência (`Clipboard`) exporta o código-fonte matemático sem formatação.

## 3. Prevenção de Desperdício de Recursos via `eq(other)`
O método `eq(other)` é utilizado pelo mecanismo de virtualização do CodeMirror. Quando o usuário rola o documento (scroll), o CodeMirror só recria o widget se `eq()` retornar `false`:
```typescript
eq(other: WidgetType): boolean {
  return (
    other instanceof SemanticBlockWidget &&
    other.descriptor.nodeId === this.descriptor.nodeId &&
    other.descriptor.revision === this.descriptor.revision &&
    JSON.stringify(other.descriptor.parameters) === JSON.stringify(this.descriptor.parameters)
  );
}
```

## 4. Transações Cirúrgicas (Widget -> CodeMirror)
Quando o usuário altera um valor no widget (ex: arrasta um slider ou ponto de envelope), o widget despacha uma transação com `locateLeanParameterRange`:
```typescript
view.dispatch({
  changes: { from: range.from, to: range.to, insert: String(newValue) }
});
```
Isso garante que apenas o literal numérico é alterado, preservando todo o restante do arquivo, comentários e indentação.

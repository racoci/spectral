# Especificação da Sincronização Bidirecional (Editor <-> Widgets)

## 1. Princípio da Representação Dual Estrita
A IDE garante que toda alteração em tela é imediatamente refletida no código-fonte em Lean 4, e qualquer caractere digitado ou colado no CodeMirror atualiza os widgets instantaneamente.
A representação textual em Lean 4 permanece a **única fonte da verdade** matemática.

## 2. Direção A: Texto -> Widget (Reactive Compilation)
1. O CodeMirror emite um evento de atualização de documento (`ViewUpdate`).
2. A `SemanticSession` processa o novo texto com a revisão correspondente $r_k$.
3. Os descritores de widgets (`WidgetDescriptor[]`) são extraídos com seus parâmetros atualizados.
4. O `WidgetHost` do Svelte 5 recebe os novos parâmetros via Runes (`$derived(descriptor)`), atualizando a posição dos knobs, envelopes e da matriz sem reconstruir o DOM desnecessariamente.

## 3. Direção B: Widget -> Texto (Surgical Dispatch)
1. O usuário interage com um elemento gráfico (ex: move um ponto de envelope $R_1$, gira um knob ou clica em outro algoritmo FM).
2. O widget invoca o callback `onParamChange(paramPath, newValue)`.
3. A função `createParamUpdateTransaction(state, paramPath, newValue)` consulta o mapa de AST e localiza os índices exatos de caractere `[from, to]` do literal no arquivo.
4. O CodeMirror executa `view.dispatch({ changes: { from, to, insert: String(newValue) } })`.
5. Nenhum outro ponto do arquivo é tocado, prevenindo reindentação ou perda de comentários do usuário.

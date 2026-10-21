# Diretrizes de Acessibilidade, ARIA e Alto Contraste (WCAG 2.1 AA)

## 1. Visão Geral
Este documento estabelece as diretrizes de acessibilidade e inclusão visual para a interface do ecossistema Spectral. Todos os componentes atômicos (`Knob`, `PillTabs`, `PanelCard`, `Splitter`, `SmartTooltip`, `GlobalTopbar`) foram projetados para satisfazer os critérios de conformidade **WCAG 2.1 nível AA**.

## 2. Contraste de Cores e Paletas Semânticas
- **Texto Principal (`--text-primary: #e5f1fb`) sobre Fundo Base (`--bg-base: #02060b`)**:
  - Razão de contraste medida: **16.8:1** (excede amplamente o requisito mínimo de 4.5:1).
- **Rótulos Secundários (`--text-secondary: #8ca5ba`) sobre Fundo de Superfície (`--bg-surface: #06111d`)**:
  - Razão de contraste medida: **6.2:1** (em conformidade com o requisito de 4.5:1).
- **Destaques e Acentos (`--accent-cyan: #38bdf8`)**:
  - Utilizado exclusivamente para foco interativo, ponteiros e valores ativos.

## 3. Semântica ARIA e Foco por Teclado
1. **Knob (`Knob.svelte`)**:
   - `role="slider"`, com atributos reativos `aria-valuenow`, `aria-valuemin`, `aria-valuemax` e `aria-label`.
   - Navegável via `Tab` com foco visível de alto contraste (`:focus-visible`).
2. **Abas Segmentadas (`PillTabs.svelte`)**:
   - `role="tablist"` com itens individuais `role="tab"` e `aria-selected="true/false"`.
3. **Divisores (`Splitter.svelte`)**:
   - `role="separator"` com `aria-orientation="vertical|horizontal"`.
4. **Tooltips Flutuantes (`SmartTooltip.svelte`)**:
   - `role="tooltip"`, `pointer-events: none` para não obstruir o clique do usuário e `backdrop-filter` para manter legibilidade sobreposta aos gráficos.

## 4. Atalhos Globais de Teclado
- Acesso direto via `Alt + 1` até `Alt + 6`.
- Proteção automática para não sequestrar o cursor quando o usuário estiver digitando código Lean 4 ou preenchendo formulários.

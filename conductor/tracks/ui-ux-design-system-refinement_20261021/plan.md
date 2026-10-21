# Implementation Plan - UI/UX Design System Refinement & Visual Ergonomics

Este plano detalha as fases, tarefas e testes automatizados para a implementação do Design System unificado, dos componentes atômicos em `src/lib/ui/`, da navegação global com atalhos de teclado e da responsividade com `ResizeObserver` nas 6 ferramentas.

---

## Fase 1: Tokens Globais, Design System Base e Componentes Atômicos em `src/lib/ui/` (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para os componentes atômicos compartilhados (Red Phase) [failing test created]
  - Testar `Knob`, `PillTabs`, `PanelCard`, `Splitter` e `SmartTooltip` verificando contratos de propriedades, dispatch de eventos de valor e cálculo de limites.
  - Teste automatizado: `npx tsx tests/test-ui-atomic-components.ts`
  - Documentação: `documentation/ui_design_system/atomic_components.md`
- [x] Task 1.2: Implementar os tokens semânticos em `src/app.css` e os componentes atômicos em `src/lib/ui/` (Green Phase) [Knob, PillTabs, PanelCard, Splitter, SmartTooltip implemented]
  - Criar `Knob.svelte`, `PillTabs.svelte`, `PanelCard.svelte`, `Splitter.svelte` e `SmartTooltip.svelte` com tipagem estrita e reatividade Svelte 5 (Runes).
  - Teste automatizado: `npx tsx tests/test-ui-atomic-components.ts`
  - Documentação: `documentation/ui_design_system/atomic_components.md`
- [x] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`) [Phase 1 tests passed 100%]

---

## Fase 2: Topbar Global Avançada, Atalhos de Teclado e Indicador de Áudio (TDD)
- [x] Task 2.1: Escrever testes automatizados que falham para o roteador com atalhos `Alt+1..6` e indicador de áudio (Red Phase) [failing test created]
  - Testar mapeamento de teclas `Alt+1` a `Alt+6` para atualização de hash (`#/converter` até `#/field`) e detecção de nós de áudio ativos.
  - Teste automatizado: `npx tsx tests/test-topbar-navigation.ts`
  - Documentação: `documentation/ui_design_system/topbar_navigation.md`
- [x] Task 2.2: Implementar o componente `GlobalTopbar.svelte` e integrar a `src/App.svelte` (Green Phase) [GlobalTopbar.svelte & App.svelte integrated]
  - Criar `src/lib/ui/GlobalTopbar.svelte` com ícones SVG estilizados, badges de ferramenta e listener de atalhos globais.
  - Teste automatizado: `npx tsx tests/test-topbar-navigation.ts`
  - Documentação: `documentation/ui_design_system/topbar_navigation.md`
- [x] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`) [Phase 2 tests passed 100%]

---

## Fase 3: Responsividade Adaptativa com `ResizeObserver` e Integração nos 6 Estúdios (TDD)
- [x] Task 3.1: Escrever testes automatizados para redimensionamento de canvas e clamp de HUD (Red Phase) [failing test created]
  - Testar sincronização de buffer `width/height` vs `clientWidth/clientHeight` e função de clamp com margens de segurança na viewport.
  - Teste automatizado: `npx tsx tests/test-canvas-responsiveness.ts`
  - Documentação: `documentation/ui_design_system/canvas_responsiveness.md`
- [~] Task 3.2: Integrar os componentes atômicos e o `ResizeObserver` nos 6 estúdios (Green Phase)
  - Refatorar `WebGlEditor`, `DdspStudio`, `StnStudio`, `LeanSynthStudio` e `ScalarFieldStudio` para usar os componentes de `src/lib/ui/`.
  - Teste automatizado: `npm run check && npx tsx tests/test-canvas-responsiveness.ts`
  - Documentação: `documentation/ui_design_system/canvas_responsiveness.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Auditoria Visual Multi-Resolução e Handoff de Acessibilidade (Meta-Task Obrigatória)
- [ ] Task 4.1: Expandir a suíte E2E do Puppeteer (`tests/e2e-browser-suite.ts`) para auditoria multi-resolução
  - Executar auditoria em Desktop (1440x960) e Laptop (1280x720) gerando o catálogo visual completo de screenshots sem sobreposição.
  - Teste automatizado: `npm run test:e2e`
  - Documentação: `documentation/ui_design_system/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Acessibilidade, ARIA e Alto Contraste (Meta-Task Obrigatória)
  - Documentar estratégias de contraste WCAG AA, navegação por teclado e semântica ARIA para os controles interativos.
  - Documentação: `documentation/ui_design_system/ACCESSIBILITY_AND_CONTRAST.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

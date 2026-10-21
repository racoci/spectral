# Spectral UI Design System & Componentes Atômicos

## 1. Visão Geral
O **Spectral UI Design System** estabelece a identidade visual unificada, moderna e de alto contraste em todas as 6 ferramentas do projeto:
1. **Converter** (`#/converter`)
2. **WebGL Editor** (`#/editor`)
3. **DDSP Studio** (`#/synth`)
4. **STN Synth** (`#/stn`)
5. **Lean IDE** (`#/lean`)
6. **Field IDE** (`#/field`)

## 2. Arquitetura de Módulos
- `src/app.css`: Tokens globais de superfície, bordas, texto e paleta semântica Dark.
- `src/lib/ui/ui_helpers.ts`: Geometria do Knob, limites de Splitter e cálculo de SmartTooltip.
- `src/lib/ui/navigation_helpers.ts`: Definição canônica das rotas e atalhos `Alt+1..6`.
- `src/lib/ui/canvas_helpers.ts`: Sincronização de resolução com DPR e clamping inteligente de HUDs no cursor.
- `src/lib/ui/GlobalTopbar.svelte`: Barra de navegação unificada presente em todas as telas com indicador de áudio ativo.
- `src/lib/ui/Knob.svelte`: Rotary knob vetorial de 270° com suporte a drag, wheel e clique duplo.
- `src/lib/ui/PillTabs.svelte`: Abas e seletores segmentados.
- `src/lib/ui/PanelCard.svelte`: Cards de painel colapsáveis com badges.
- `src/lib/ui/Splitter.svelte`: Divisores arrastáveis seguros contra colapso de layout.
- `src/lib/ui/SmartTooltip.svelte`: Tooltip flutuante com anti-oclusão de ponteiro.

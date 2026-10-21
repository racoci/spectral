# Especificação: Refinamento de UI/UX, Design System Unificado e Ergonomia Visual (6 Ferramentas)

## 1. Visão Geral
Esta track tem como objetivo unificar, polir e elevar a qualidade visual, estética e ergonômica de todas as 6 ferramentas do ecossistema Spectral:
1. **Audio Converter** (`#/converter`)
2. **WebGL 2D Editor** (`#/editor`)
3. **DDSP & TreeNN Studio** (`#/synth`)
4. **STN Synth Studio** (`#/stn`)
5. **Lean 4 Synth IDE** (`#/lean`)
6. **Scalar Field & Taylor Jets IDE** (`#/field`)

O projeto migrará estilos fragmentados para um **Design System Atômico Unificado** (`src/lib/ui/`), garantindo responsividade contínua via `ResizeObserver`, clamp inteligente para tooltips/HUD, navegação avançada com atalhos de teclado e auditoria visual multi-resolução.

---

## 2. Requisitos Funcionais

### 2.1 Design System & Tokens Globais (`src/app.css`)
- **Tokens Semânticos de Cor e Superfície**:
  - `--bg-base`: Fundo mais profundo (`#02060b`).
  - `--bg-surface`: Superfície de cards e painéis (`#06111d`).
  - `--bg-elevated`: Elementos flutuantes e modais (`#0b1c2e`).
  - `--border-subtle`: Bordas de divisão (`#13273c`).
  - `--border-active`: Bordas em foco/seleção (`#2a5a84`).
  - `--text-primary`: Texto principal (`#e5f1fb`).
  - `--text-secondary`: Rótulos e metadados (`#8ca5ba`).
  - `--accent-cyan`: Ciano de destaque (`#38bdf8`).
  - `--accent-teal`: Teal para gradientes e status (`#2dd4bf`).
  - `--accent-amber`: Âmbar para curvaturas e alertas (`#fbbf24`).
  - `--accent-violet`: Violeta para estruturas Lean 4 (`#c084fc`).
- **Tipografia**:
  - Fonte monospace padronizada (`JetBrains Mono`, `Fira Code`, `ui-monospace`).
  - Escala proporcional de tamanhos (`10px`, `11px`, `12px`, `14px`, `16px`).

### 2.2 Barra Superior de Navegação Global (Topbar)
- Ícones SVG estilizados e rótulos concisos para as 6 ferramentas:
  - `1. Converter` (Ícone de conversão bidirecional)
  - `2. WebGL Editor` (Ícone de espectrograma / visualizador)
  - `3. DDSP Studio` (Ícone de síntese neural / formantes)
  - `4. STN Synth` (Ícone senoides + transientes + ruído)
  - `5. Lean IDE` (Ícone lambda λ de prova formal e FM)
  - `6. Field IDE` (Ícone nabla ∇² de campos e jatos)
- **Atalhos de Teclado Globais**:
  - `Alt + 1` até `Alt + 6` para alternar instantaneamente entre as telas.
- **Indicador de Áudio Ativo**:
  - Badge pulsante quando qualquer motor sonoro (Web Audio API) estiver em reprodução.

### 2.3 Biblioteca de Componentes Atômicos Compartilhados (`src/lib/ui/`)
1. **`Knob.svelte`**:
   - Controle giratório com arrasto vertical/angular, suporte a scroll wheel, reset por clique duplo e exibição de valor numérico formatado.
2. **`PillTabs.svelte`**:
   - Seletor de abas e modos com transição suave e indicador visual ativo.
3. **`PanelCard.svelte`**:
   - Contêiner de painel padronizado com cabeçalho recolhível, grip de arraste e badges informativas.
4. **`Splitter.svelte`**:
   - Divisor horizontal ou vertical arrastável com limites de largura mínima e cursor condizente.
5. **`SmartTooltip.svelte` / HUD no Cursor**:
   - Detecção de limites da viewport (`clamp`) para impedir que tooltips ou HUDs vazem da tela ou cubram o cursor do usuário.

### 2.4 Responsividade de Canvases com `ResizeObserver`
- Todos os elementos `<canvas>` WebGL 2 e 2D overlay devem utilizar `ResizeObserver` para sincronizar `canvas.width` e `canvas.height` em relação ao tamanho real renderizado do contêiner, eliminando distorções de escala (*pixel stretching*) sob redimensionamento de janela ou arraste de splitters.

### 2.5 Higiene do CodeMirror 6
- Padronização do padding de gutters, destaque de linha ativa, rolagem sincronizada e remoção de artefatos visuais de quebra em larguras menores.

---

## 3. Requisitos Não-Funcionais
- **Performance**: Manter 60 FPS estáveis mesmo durante redimensionamento contínuo de janelas.
- **Integridade de Tipos**: Verificação estrita via `svelte-check` com 0 erros.
- **Pureza de Texto**: O editor CodeMirror 6 deve continuar 100% puro e sem contaminação HTML/DOM.

---

## 4. Critérios de Aceitação
- [x] O `src/app.css` contém os tokens semânticos unificados.
- [x] Os componentes em `src/lib/ui/` estão implementados com testes unitários/integração.
- [x] A barra superior de navegação possui atalhos de teclado `Alt+1..6` e ícones SVG.
- [x] As 6 ferramentas utilizam os componentes compartilhados e não apresentam vazamento de layout.
- [x] A suíte `test:e2e` do Puppeteer executa em duas resoluções (Desktop 1440x960 e Laptop 1280x720) sem erros de console e gera screenshots nítidas.
- [x] Todas as 17 suítes de teste de integração passam com 100% de sucesso.
- [x] O build de produção do Vite compila com sucesso.

---

## 5. Fora de Escopo
- Alterações em algoritmos DSP de síntese ou compressão na camada Rust/WASM e Python.

---

## 6. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Redimensionamento contínuo de canvases via `ResizeObserver` durante arraste de splitters pode disparar *"ResizeObserver loop limit exceeded"* ou causar perda de contexto WebGL por desalocação excessiva de buffers (Probabilidade Média).
    *   **Mitigação:** Implementar `requestAnimationFrame` (rAF) throttling no observador, atualizando o `gl.viewport` e o buffer físico do canvas no máximo uma vez por quadro de animação (16.6ms).
        *   **[Profundidade 2] Risco:** O rAF throttling pode introduzir 1 quadro de atraso visual (*pixel stretching*) durante arrastos ultra-rápidos do mouse (Probabilidade Média).
            *   **Mitigação:** Aplicar interpolação CSS instantânea via `transform: scale()` durante o arraste e consolidar a resolução nativa do canvas no rAF subsequente.
                *   **[Profundidade 3] Risco:** A escala CSS pode causar leve embaçamento transitório em telas de alta densidade (Retina/DPI alto) (Probabilidade Média).
                    *   **Mitigação:** Multiplicar a resolução interna por `window.devicePixelRatio` com margem de segurança de 1.2x para que a escala seja sempre *downscaled* com superamostragem ao invés de esticada.
                        *   **[Profundidade 4] Risco:** Sobrecarga de memória de vídeo (VRAM) em GPUs móveis de entrada com buffers 1.2x (Probabilidade Média).
                            *   **Mitigação:** Limitar a dimensão máxima do buffer a `Math.min(2048, window.innerWidth * dpr)` em dispositivos de baixa VRAM.
                                *   **[Profundidade 5] Risco:** Em monitores 4K ultrawide, o limite de 2048px reduziria ligeiramente a resolução do espectrograma (Baixa Probabilidade) -> *Ramo Encerrado (Baixa Probabilidade).*
*   **[Profundidade 1] Risco:** Conflito de atalhos globais de teclado (`Alt+1..6`) com abas nativas do navegador ou com digitação dentro do CodeMirror 6 (Probabilidade Média).
    *   **Mitigação:** Bloquear a interceptação de `Alt+1..6` se o elemento ativo for um campo de entrada ou o editor CodeMirror (`!e.target.closest('.cm-content, input, textarea')`), exigindo `Esc` para desfoque prévio ou clique direto na barra.
        *   **[Profundidade 2] Risco:** Usuários podem não perceber que o editor está em foco e achar que o atalho falhou (Probabilidade Média).
            *   **Mitigação:** Exibir dica visual rápida no topo do editor (`[Pressione Esc para alternar ferramentas]`) quando `Alt` for pressionado dentro do CodeMirror.
                *   **[Profundidade 3] Risco:** Tooltip temporário poluir a visão do código fonte (Baixa Probabilidade) -> *Ramo Encerrado (Baixa Probabilidade).*


# Especificação dos Componentes Atômicos de UI e Geometria de Interação

## 1. Visão Geral
A pasta `src/lib/ui/` centraliza os componentes básicos de controle e layout, desacoplando a lógica matemática dos estúdios da apresentação visual e do tratamento de eventos do ponteiro.

## 2. Contratos de Geometria e Interação

### 2.1 Rotary Knob (`Knob.svelte`)
- **Mapeamento Angular**: Varredura de $270^\circ$, mapeando o intervalo $[V_{\min}, V_{\max}]$ para $[-135^\circ, +135^\circ]$ com $0^\circ$ posicionado no topo vertical.
- **Interação**:
  - Arraste vertical ou circular contínuo.
  - Rolagem por roda do mouse (*wheel*).
  - Clique duplo para restauração ao valor padrão (`defaultValue`).
  - `setPointerCapture` para manter o arraste ativo mesmo com o cursor fora dos limites visuais do componente.

### 2.2 Segmented Switch (`PillTabs.svelte`)
- Alternância de modos ou abas com transição estética sutil e indicação de foco acessível.

### 2.3 Container Padronizado (`PanelCard.svelte`)
- Cabeçalho com ícone, título em negrito, badges informativas e botão de recolher/expandir com animação suave.

### 2.4 Divisor Redimensionável (`Splitter.svelte`)
- Cálculo estrito de limites com `computeSplitterBounds` garantindo que painéis adjacentes respeitem larguras mínimas configuráveis.

### 2.5 Tooltip e HUD Inteligente (`SmartTooltip.svelte`)
- Função `computeSmartTooltipPosition` com detecção de bordas da janela (`viewport`), invertendo as coordenadas horizontal e verticalmente quando o elemento se aproxima das bordas direita ou inferior da tela.

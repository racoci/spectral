# Lean 4 Synth IDE: Estúdio de Síntese Modular e Editor Dual

## 1. Visão Geral
A **Lean 4 Synth IDE** é um ambiente de autoria e síntese sonora de alta performance com arquitetura dual:
1. **Editor de Código Puro (CodeMirror 6):** Exibe e manipula o programa formal em linguagem Lean 4 (`FmPatch`, `Dx7Operator`, `Dx7Envelope`), garantindo extração de texto 100% puro para a área de transferência (`clipboard`) sem nenhuma contaminação por marcações HTML de widgets.
2. **Widgets Semânticos Interativos (Svelte 5):** Cada módulo do sintetizador projeta descritores semânticos que montam controles visuais especializados:
   - *OperatorMatrixWidget:* Visualização e seleção da topologia dos 32 algoritmos FM com matriz de modulação e feedback.
   - *EnvelopeStageWidget:* Curva interativa arrastável por pontos representando as 4 taxas e 4 níveis de envelope por operador.
   - *ParameterKnobWidget:* Knobs e sliders rotativos de alta precisão para frequência coarse, fine, detune e sensibilidade de velocidade.
3. **Sincronização Cirúrgica Bidirecional:**
   - Movimentos nos widgets realizam transações pontuais no CodeMirror (`view.dispatch`) alterando exclusivamente os números afetados nos limites `[from, to]`.
   - Edições de texto no CodeMirror atualizam o estado dos widgets instantaneamente.
4. **Motor de Síntese em Tempo Real & Teclado Virtual:**
   - Síntese FM polifônica estéreo de 6 operadores em 48 kHz.
   - Teclado virtual de 2 oitavas (C3 a C5) com gatilho instantâneo de notas e acordes.
   - Importação direta de bancos reais de sintetizadores no formato `.syx` do Yamaha DX7 / Dexed.
   - Exportação para arquivo `.wav` de 16-bit PCM.

## 2. Estrutura de Arquivos da Implementação
- `src/lib/LeanSynthStudio.svelte`: Componente principal do estúdio com split redimensionável e transporte.
- `src/lib/synth/dx7_parser.ts`: Parser binário de arquivos SysEx (`.syx`).
- `src/lib/synth/dx7_presets.ts`: Banco de presets canônicos (`TUB BELLS`, `E.PIANO 1`, `SLAP BASS`).
- `src/lib/synth/dx7_audio_engine.ts`: Motor de síntese FM de 6 operadores e codificador WAV.
- `src/lib/synth/lean_ast_codec.ts`: Emissor, parser e localizador cirúrgico de limites `[from, to]` para código Lean 4.
- `src/lib/widgets/types.ts`: Contrato de dados do protocolo semântico (`WidgetDescriptor`).
- `src/lib/widgets/registry.ts`: Registro desacoplado de componentes visuais Svelte.
- `src/lib/editor/SemanticBlockWidget.ts`: Adaptador de ciclo de vida CodeMirror 6 para Svelte 5 (`mount`/`unmount`).

# Especificação da Barra de Navegação Global e Atalhos de Teclado

## 1. Visão Geral
A barra superior (`GlobalTopbar.svelte`) oferece uma experiência consistente de orientação entre as 6 ferramentas de áudio e síntese, integrando ícones SVG estilizados, atalhos de teclado globais rápidos e um monitor de atividade sonora em tempo real.

## 2. Mapa Canônico de Rotas e Atalhos

| Nº | Rota / Hash | Identificador | Rótulo | Atalho | Ícone / Propósito |
| :---: | :---: | :---: | :---: | :---: | :--- |
| **1** | `#/converter` | `converter` | 1. Converter | `Alt+1` | Conversão reversível bidirecional de áudio em imagens |
| **2** | `#/editor` | `editor` | 2. WebGL Editor | `Alt+2` | Visualizador de alta performance e espetrograma 2D |
| **3** | `#/synth` | `synth` | 3. DDSP Studio | `Alt+3` | Síntese aditiva neural baseada em formantes e TreeNN |
| **4** | `#/stn` | `stn` | 4. STN Synth | `Alt+4` | Decomposição Senoide + Transiente + Ruído |
| **5** | `#/lean` | `lean` | 5. Lean IDE | `Alt+5` | Ambiente dual de código puro Lean 4 e matriz FM DX7 |
| **6** | `#/field` | `field` | 6. Field IDE | `Alt+6` | Campos escalares 2D, jatos de Taylor e brushes |

## 3. Segurança e Prevenção de Conflitos
- O ouvinte de eventos de teclado do `window` detecta combinações `Alt + [1..6]`.
- Se o elemento atualmente focado na página for um campo de entrada de texto (`input`, `textarea`) ou o buffer de texto do CodeMirror 6 (`.cm-content`), o atalho é sumariamente ignorado para evitar interferência na digitação do usuário.

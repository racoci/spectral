# Especificação da Track: MVP — Validação da Representação de Sintetizadores em Lean 4 com Presets Reais e IDE Dual (CodeMirror 6 / Svelte 5)

## 1. Visão Geral
Construir uma Prova de Conceito (MVP) funcional para validar a adequação da linguagem **Lean 4** como linguagem de autoria, estruturação de AST e síntese sonora do projeto. O sistema fará a ingestão de presets reais do banco do **Dexed / Yamaha DX7** (formato `.syx`), traduzirá seus 155 parâmetros contínuos e topologia em um programa Lean 4 formal e fortemente tipado, e apresentará esse código em um editor **CodeMirror 6** com **widgets semânticos interativos em Svelte 5** sincronizados bidirecionalmente.

---

## 2. Diagrama de Fluxo e Arquitetura
- **Ingestão:** SyxFile -> [Parser TypeScript] -> AST Lean 4
- **Bidirecionalidade:** Código Puro CodeMirror <--> Descritores Semânticos <--> Widgets Svelte 5
- **Saída Audível:** Compilação do Parâmetro AST -> WebAudio Node Graph.

---

## 3. Requisitos Funcionais

1. **Ingestão e Parser de Dados Reais (`.syx`):** Extrair 155 parâmetros brutos de bancos binários DX7.
2. **Gramática e Representação em Lean 4:** Conversão da estrutura matricial de FM e de envelopes ADSR em texto tipado Lean 4.
3. **Protocolo Semântico de Widgets:** Definição do `WidgetDescriptor` com `from`, `to`, `nodeId` e `revision`.
4. **Editor Dual CodeMirror 6 + Svelte 5:** Extensão customizada que renderiza blocos Svelte dentro do texto sem poluir o código com tags invisíveis.
5. **Sincronização Bidirecional Estrita:** Eventos em UI mudam código-fonte `dispatch` e edição em texto realoca as alças na UI.
6. **Preview Sonoro Imediato:** Som 48kHz via WebAudio executando o preset validado.

---

## 4. Critérios de Aceitação (TDD)
- **Teste 1:** Extração DX7 `.syx` vs Parâmetros Lean extraídos tem distância zero (Roundtrip).
- **Teste 2:** Extração `view.state.doc.toString()` produz código-fonte perfeitamente limpo, desprovido de marcações Svelte.
- **Teste 3:** Transações cirúrgicas bidirecionais substituem o trecho `[from, to]` e nada mais.
- **Teste 4:** Reutilização via `eq()` virtualização protege contra Memory Leaks.
- **Teste 5:** O som produzido pela IDE espelha o *timbre* clássico DX7 do arquivo `.syx`.
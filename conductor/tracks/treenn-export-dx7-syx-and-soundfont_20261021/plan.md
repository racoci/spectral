# Implementation Plan - TreeNN and DX7 Patch Export to Yamaha SysEx (.syx) and SoundFont2 (.sf2)

Este plano detalha as fases, tarefas e testes automatizados para a implementação da exportação de patches de sintetizador para Yamaha DX7 SysEx (.syx) e SoundFont 2 (.sf2).

---

## Fase 1: Exportador Yamaha DX7 SysEx (.syx) (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para o serializador DX7 SysEx de voz única (155 bytes) e banco de 32 vozes (4104 bytes) (Red Phase) [failing tests created]
  - Testar montagem binária de 128 bytes por voz, cálculo de checksum de 7 bits em complemento de dois e cabeçalhos MIDI padrão.
  - Teste automatizado: `npx tsx tests/test-dx7-sysex-exporter.ts`
  - Documentação: `documentation/synth_dsl_jets/dx7_sysex_export_specification.md`
- [x] Task 1.2: Implementar módulo `src/lib/export/dx7_sysex_exporter.ts` (Green Phase)
  - Funções `exportDx7SingleVoiceSyx(patch)` e `exportDx7BankSyx(patches)` com sanitização de nome ASCII e validação de limites de operador.
  - Teste automatizado: `npx tsx tests/test-dx7-sysex-exporter.ts`
  - Documentação: `documentation/synth_dsl_jets/dx7_sysex_export_specification.md`
- [x] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Exportador SoundFont 2 (.sf2) (TDD)
- [x] Task 2.1: Escrever testes automatizados que falham para o empacotador de SoundFont 2 RIFF/sfbk (Red Phase) [failing tests created]
  - Testar integridade dos chunks `RIFF`, `INFO`, `sdta` (dados PCM 16-bit a 48 kHz) e `pdta` (tabelas de cabeçalho v2.04).
  - Teste automatizado: `npx tsx tests/test-soundfont2-exporter.ts`
  - Documentação: `documentation/synth_dsl_jets/soundfont2_export_specification.md`
- [x] Task 2.2: Implementar módulo `src/lib/export/soundfont2_exporter.ts` (Green Phase)
  - Gerador de container binário SF2 com síntese de 5 notas em oitavas (C2 a C6) e mapeamento de zonas de tecla.
  - Teste automatizado: `npx tsx tests/test-soundfont2-exporter.ts`
  - Documentação: `documentation/synth_dsl_jets/soundfont2_export_specification.md`
- [x] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Integração na Interface (LeanSynthStudio & DdspStudio)
- [x] Task 3.1: Adicionar botões `[🎹 Baixar .SYX (DX7)]` e `[🎼 Baixar SoundFont (.sf2)]` nas toolbars
  - Integrar disparo de download binário em `LeanSynthStudio.svelte` e `DdspStudio.svelte`.
  - Testes automatizados: `npm run check` e `npm run test:integration`
  - Documentação: `documentation/synth_dsl_jets/ui_hardware_export.md`
- [x] Task 3.2: Validar compatibilidade de download de arquivos binários sem corrupção
  - Testes automatizados: `npm run check` e `npm run build`
  - Documentação: `documentation/synth_dsl_jets/ui_hardware_export.md`
- [x] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Validação Global, Verificação de Tipos e Handoff (Meta-Task Obrigatória)
- [x] Task 4.1: Integrar novas suítes ao `tests/run_all_ts_tests.ts`, executar verificação global e build de produção
  - Testes automatizados: `npm run test:integration` e `npm run build`
  - Documentação: `documentation/synth_dsl_jets/README.md`
- [x] Task 4.2: Mitigação de Riscos de Interoperabilidade e Handoff (Meta-Task Obrigatória)
  - Documentar compatibilidade com sintetizadores de hardware (Yamaha DX7, Korg Volca FM) e samplers VST.
  - Documentação: `documentation/synth_dsl_jets/HARDWARE_INTEROPERABILITY_HANDOFF.md`
- [x] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

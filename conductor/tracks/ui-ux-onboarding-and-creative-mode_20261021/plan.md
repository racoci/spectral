# Implementation Plan - UI/UX Accessibility, 1-Click Demos, and Creative vs Expert Mode

Este plano detalha as fases, tarefas e testes automatizados para democratizar a interface web para usuários leigos e criativos, adicionando carregamento de demonstrações em 1 clique, alternador de modo Criativo vs Expert com vocabulário intuitivo, player comparativo A/B e modal de onboarding.

---

## Fase 1: Barra de Demonstrações Rápidas (1-Click Audio Demos) e Carregamento (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para o carregador de amostras curadas (`tests/test-sample-demos-loader.ts`) (Red Phase) [failing tests created]
  - Testar disponibilidade das amostras (`voice.wav`, `synth.wav`, `car-horn.wav`), carregamento de buffers binários e decodificação WAV 16-bit.
  - Teste automatizado: `npx tsx tests/test-sample-demos-loader.ts`
  - Documentação: `documentation/ui_design_system/audio_demos_loader.md`
- [x] Task 1.2: Implementar o módulo `src/lib/ui/demo_samples.ts` e seletor rápido no Topbar (Green Phase)
  - Integrar carregamento instantâneo via `fetch` assíncrono conectado aos reprodutores de áudio do Svelte.
  - Teste automatizado: `npx tsx tests/test-sample-demos-loader.ts`
  - Documentação: `documentation/ui_design_system/audio_demos_loader.md`
- [x] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Dicionário Semântico e Modo Criativo vs Expert (TDD)
- [x] Task 2.1: Escrever testes automatizados que falham para a tradução de termos e reatividade do modo (`tests/test-creative-expert-mode.ts`) (Red Phase) [failing tests created]
  - Testar mapeamento bijetivo entre termos científicos e termos musicais leigos com suporte a fallback.
  - Teste automatizado: `npx tsx tests/test-creative-expert-mode.ts`
  - Documentação: `documentation/ui_design_system/creative_vs_expert_mode.md`
- [x] Task 2.2: Implementar `src/lib/ui/ui_glossary.ts` e integrar o alternador `[Criativo / Expert]` em `GlobalTopbar.svelte` (Green Phase)
  - Vincular textos reativos através de Runes do Svelte 5 com persistência no `localStorage`.
  - Teste automatizado: `npx tsx tests/test-creative-expert-mode.ts`
  - Documentação: `documentation/ui_design_system/creative_vs_expert_mode.md`
- [x] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Player A/B Comparativo e Modal de Onboarding de Boas-Vindas (TDD)
- [~] Task 3.1: Escrever testes automatizados que falham para comutação A/B e estado do modal (`tests/test-ab-player-onboarding.ts`) (Red Phase)
  - Testar alternância síncrona de reprodução A/B sem estalos de clique e persistência de fechamento do modal.
  - Teste automatizado: `npx tsx tests/test-ab-player-onboarding.ts`
  - Documentação: `documentation/ui_design_system/ab_player_and_onboarding.md`
- [ ] Task 3.2: Implementar componentes `AbComparePlayer.svelte` e `WelcomeModal.svelte` (Green Phase)
  - Criar interface compacta do player com métricas leigas de compressão e modal acessível com suporte a tecla `ESC`.
  - Teste automatizado: `npx tsx tests/test-ab-player-onboarding.ts`
  - Documentação: `documentation/ui_design_system/ab_player_and_onboarding.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Integração Completa, Verificação de Tipos e Handoff (Meta-Task Obrigatória)
- [ ] Task 4.1: Integrar componentes na interface principal e verificar build
  - Testar montagem na árvore principal do `App.svelte` e verificar conformidade com `svelte-check` e `vite build`.
  - Teste automatizado: `npm run check && npm run build`
  - Documentação: `documentation/ui_design_system/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Acessibilidade e Handoff de Usabilidade (Meta-Task Obrigatória)
  - Documentar mapeamento de cores de alto contraste, suporte a leitores de tela e atalhos simplificados.
  - Documentação: `documentation/ui_design_system/ACCESSIBILITY_AND_ONBOARDING_HANDOFF.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

# Implementation Plan - Session Sharing via Compact Reconstruction Format, Offline PWA & Structural Vector Export

Este plano detalha as fases, tarefas e testes automatizados para a implementação do compartilhamento portátil de sessões por formato próprio compacto, execução 100% offline via PWA e exportação vetorial SVG.

---

## Fase 1: Formato Próprio Compacto de Reconstrução e Serialização de URL (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para serialização e desserialização do pacote compacto (`tests/test-session-sharing-compact.ts`) (Red Phase) [failing tests created]
  - Testar roundtrip exato de presets de todas as ferramentas, validação de integridade de versão e codificação de query string.
  - Teste automatizado: `npx tsx tests/test-session-sharing-compact.ts`
  - Documentação: `documentation/sharing/compact_reconstruction_format.md`
- [x] Task 1.2: Implementar módulo `src/lib/sharing/session_serializer.ts` (Green Phase)
  - Criar funções `encodeSpectralPackage(pkg)`, `decodeSpectralPackage(encoded)` e gerador de URL hash com query string.
  - Teste automatizado: `npx tsx tests/test-session-sharing-compact.ts`
  - Documentação: `documentation/sharing/compact_reconstruction_format.md`
- [x] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Botão de Compartilhar, Toast de Feedback e Restauração de Rota (TDD)
- [~] Task 2.1: Integrar botão `[🔗 Compartilhar]` em `GlobalTopbar.svelte` com notificação visual (Toast) e escuta no `App.svelte` (Green Phase)
  - Permitir copiar o link de compartilhamento para a área de transferência e restaurar o estado na inicialização ao detectar hash com parâmetros.
  - Testes automatizados: `npm run check` e `npm run test:integration`
  - Documentação: `documentation/sharing/url_sharing_interaction.md`
- [ ] Task 2.2: Testar restauração de estado e ressíntese analítica sem dependência do áudio original
  - Validar que a abertura de um link restaura a ferramenta correta e os controles sem requisição de arquivos PCM.
  - Teste automatizado: `npx tsx tests/test-session-sharing-compact.ts`
  - Documentação: `documentation/sharing/url_sharing_interaction.md`
- [ ] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Manifesto PWA Offline e Service Worker
- [ ] Task 3.1: Criar manifesto `public/manifest.webmanifest` e Service Worker `public/sw.js` com cache Stale-While-Revalidate
  - Configurar cache prioritário de binários WebAssembly (`core_wasm_bg.wasm`), amostras e assets para funcionamento 100% offline.
  - Documentação: `documentation/sharing/pwa_offline_architecture.md`
- [ ] Task 3.2: Registrar Service Worker no `index.html` e verificar compatibilidade
  - Integrar registro condicional e meta tags para PWA (`theme-color`, `mobile-web-app-capable`).
  - Documentação: `documentation/sharing/pwa_offline_architecture.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Gerador de Exportação Vetorial SVG e Handoff (Meta-Task Obrigatória)
- [ ] Task 4.1: Implementar módulo `src/lib/sharing/svg_exporter.ts` para exportação vetorial de cristas e contornos
  - Desenhar curvas analíticas de Taylor e trajetórias de cristas como `<path>` e `<polygon>` em SVG escalável.
  - Testes automatizados: `npm run test:integration` e `npm run build`
  - Documentação: `documentation/sharing/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Portabilidade e Handoff (Meta-Task Obrigatória)
  - Documentar estratégias de sanitização de links e cache invalidation do Service Worker.
  - Documentação: `documentation/sharing/SESSION_PORTABILITY_HANDOFF.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

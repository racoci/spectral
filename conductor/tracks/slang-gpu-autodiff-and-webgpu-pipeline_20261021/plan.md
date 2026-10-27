# Implementation Plan - Slang GPU Shaders, Automatic Differentiation & WebGPU Compute Pipeline

Este plano detalha as fases, tarefas e testes automatizados para introduzir a compilação de shaders Slang com diferenciação automática direta na GPU e o pipeline WebGPU com fallback gracioso para WebGL 2.

---

## Fase 1: Módulos Slang e Diferenciação Automática de Jatos (TDD)
- [x] Task 1.1: Escrever testes automatizados que falham para o shader Slang e compilação para WGSL (`tests/test-slang-webgpu-pipeline.ts`) (Red Phase) [failing tests created]
  - Testar sintaxe Slang diferenciável, cálculo de gradiente e Hessiana via auto-diff e validade do WGSL gerado.
  - Teste automatizado: `npx tsx tests/test-slang-webgpu-pipeline.ts`
  - Documentação: `documentation/scalar_field_ide/slang_autodiff_shaders.md`
- [~] Task 1.2: Implementar `src/shaders/taylor_field.slang` e transcrição para WGSL com funções `evaluateTaylorJet`, `computeGradient` e `computeHessian` (Green Phase)
  - Teste automatizado: `npx tsx tests/test-slang-webgpu-pipeline.ts`
  - Documentação: `documentation/scalar_field_ide/slang_autodiff_shaders.md`
- [ ] Task 1.3: Verificação da Fase 1 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 2: Renderizador WebGPU e Compute Pipeline de Brushes (TDD)
- [ ] Task 2.1: Escrever testes automatizados que falham para o pipeline WebGPU e fallback gracioso para WebGL 2 (Red Phase)
  - Testar detecção de suporte a WebGPU (`navigator.gpu`), criação de buffers uniformes e fallback sem quebra para WebGL 2.
  - Teste automatizado: `npx tsx tests/test-slang-webgpu-pipeline.ts`
  - Documentação: `documentation/scalar_field_ide/webgpu_compute_pipeline.md`
- [ ] Task 2.2: Implementar classe `WebGpuFieldRenderer` em `src/lib/scalar_field/WebGpuFieldRenderer.ts` (Green Phase)
  - Desenvolver pipeline compute para brushes e pipeline render para curvas de nível antialiased.
  - Teste automatizado: `npx tsx tests/test-slang-webgpu-pipeline.ts`
  - Documentação: `documentation/scalar_field_ide/webgpu_compute_pipeline.md`
- [ ] Task 2.3: Verificação da Fase 2 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 3: Integração no ScalarFieldStudio e Fallback Dinâmico
- [ ] Task 3.1: Conectar `WebGpuFieldRenderer` ao `ScalarFieldStudio.svelte` com indicador de backend gráfico ativo (WebGPU / WebGL 2)
  - Permitir alternância transparente ou seleção automática de hardware com melhor performance.
  - Testes automatizados: `npm run check` e `npm run test:integration`
  - Documentação: `documentation/scalar_field_ide/webgpu_ui_integration.md`
- [ ] Task 3.2: Verificar estabilidade visual e conformidade em 60/120 FPS
  - Validar renderização idêntica entre o backend WebGPU e o backend WebGL 2.
  - Testes automatizados: `npm run check` e `npm run build`
  - Documentação: `documentation/scalar_field_ide/webgpu_ui_integration.md`
- [ ] Task 3.3: Verificação da Fase 3 e Checkpoint (Referente ao `workflow.md`)

---

## Fase 4: Validação Global, Verificação de Tipos e Handoff (Meta-Task Obrigatória)
- [ ] Task 4.1: Integrar nova suíte ao `package.json`, executar verificação global e build de produção
  - Testes automatizados: `npm run test:integration` e `npm run build`
  - Documentação: `documentation/scalar_field_ide/README.md`
- [ ] Task 4.2: Mitigação de Riscos de Portabilidade Gráfica e Handoff (Meta-Task Obrigatória)
  - Documentar estratégias de compatibilidade de drivers e pipeline de compilação Slang offline/online.
  - Documentação: `documentation/scalar_field_ide/SLANG_WEBGPU_ARCHITECTURE_HANDOFF.md`
- [ ] Task 4.3: Verificação Final da Fase 4 e Checkpoint (Referente ao `workflow.md`)

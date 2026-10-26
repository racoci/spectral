# Handoff de Acessibilidade e Usabilidade (Onboarding & Modo Criativo)

## 1. Visão Geral
Este documento formaliza as diretrizes de design, padrões de acessibilidade (WCAG 2.1 AA) e a arquitetura semântica introduzida para democratizar o uso do Spectral para usuários não-especialistas em matemática ou processamento de sinais.

## 2. Arquitetura das Melhorias
1. **Demos de 1 Clique (`src/lib/ui/demo_samples.ts`)**:
   - Disponibiliza 3 amostras canônicas curadas em `public/demos/`:
     - 🗣️ `voice.wav`: Formantes acústicos e curvas contínuas de pitch.
     - 🎹 `synth.wav`: Harmônicos limpos e modulação aditiva.
     - 🎺 `car-horn.wav`: Polifonia densa e batimentos acústicos.
   - O carregamento sob demanda (`loadDemoSampleBytes`) garante zero impacto de banda no load inicial.
2. **Alternador Global [Criativo 🎨 / Expert 🔬] (`src/lib/ui/ui_glossary.ts`)**:
   - Mapeia conceitos complexos para metáforas intuitivas:
     - Geometria Holomorfa $\rightarrow$ Nitidez Quântica do Som
     - Reatribuição Auger-Flandrin $\rightarrow$ Foco Infinito de Harmônicos
     - Decomposição STN $\rightarrow$ Fatiador de Som (Corpo, Estalo e Ar)
     - Dead-Zone RDO $\rightarrow$ Economia Máxima de Tamanho
   - Persistência imediata no `localStorage`.
3. **Player A/B Comparativo (`src/lib/ui/AbComparePlayer.svelte`)**:
   - Comuta instantaneamente entre áudio de referência original (A) e áudio processado/comprimido (B).
   - Apresenta métricas em linguagem leiga: razão de redução ($34.4\times$) e economia percentual ($97.1\%$).
4. **Modal de Boas-Vindas (`src/lib/ui/WelcomeModal.svelte`)**:
   - Diálogo com foco acessível (`role="dialog"`, `aria-modal="true"`).
   - Fechamento com a tecla `Escape` ou clique no backdrop.
   - Opção de dispensa permanente gravada em `localStorage`.

## 3. Conformidade com Padrões e Verificação
- **Suítes de Testes Unitários e de Integração**:
  - `tests/test-sample-demos-loader.ts` (Validação de integridade RIFF das demos)
  - `tests/test-creative-expert-mode.ts` (Validação bijetiva do glossário semântico)
  - `tests/test-ab-player-onboarding.ts` (Cálculo de métricas A/B e persistência de storage)
- Todas as 28 suítes de teste de integração passam com 100% de sucesso via `npm run test:integration`.

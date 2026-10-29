#!/usr/bin/env node
/**
 * Orquestrador Central de Testes de Integração em TypeScript.
 * Executa todas as 32 suítes de teste unitário e de integração em processos isolados.
 */

import { spawnSync } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

interface TestSuite {
  title: string;
  file: string;
}

const TS_TEST_SUITES: TestSuite[] = [
  { title: 'Validação de Áudio WAV e Cabeçalho RIFF', file: 'validate-wav.ts' },
  { title: 'Estrutura do Espectrograma e Propriedades Anti-Ruído', file: 'spectrogram-structure.test.ts' },
  { title: 'DDSP & TreeNN Síntese Diferenciável (WASM & Rust)', file: 'test-ddsp-suite.ts' },
  { title: 'Geometria Holomorfa e Condições de Cauchy-Riemann', file: 'test-holomorphic-field-suite.ts' },
  { title: 'STN Synth (Senoides, Transientes e Ruído)', file: 'test-stn-synth-suite.ts' },
  { title: 'Síntese DX7 e Parser SysEx (Lean 4 AST)', file: 'test-dx7-audio-e2e.ts' },
  { title: 'Protocolo de Widgets Semânticos', file: 'test-semantic-widget-protocol.ts' },
  { title: 'Ciclo de Vida do CodeMirror 6 e Svelte 5', file: 'test-codemirror-svelte-lifecycle.ts' },
  { title: 'Sincronização Bidirecional Texto <-> Widgets', file: 'test-bidirectional-sync.ts' },
  { title: 'Síntese Polifônica e Motor de Audição FM', file: 'test-fm-audio-synthesis.ts' },
  { title: 'Modelo Matemático de Campos Escalares e Jatos de Taylor', file: 'test-scalar-field-math.ts' },
  { title: 'Codec e Roundtrip de Campos Escalares em Lean 4', file: 'test-scalar-field-lean-codec.ts' },
  { title: 'Shaders WebGL 2 e Renderizador de Campo Escalar', file: 'test-scalar-field-webgl.ts' },
  { title: 'Interação com Alças Polares e Hit-Testing', file: 'test-polar-handles-interaction.ts' },
  { title: 'Operadores de Brush e Filtro Inercial', file: 'test-brush-operators.ts' },
  { title: 'Regra da Cadeia Cinemática ao Longo da Trajetória', file: 'test-kinematic-chain-rule.ts' },
  { title: 'Motor de Áudio em Modo Duplo (Órbita vs DDSP)', file: 'test-dual-mode-audio-synthesis.ts' },
  { title: 'Componentes Atômicos de UI e Geometria', file: 'test-ui-atomic-components.ts' },
  { title: 'Navegação Global e Atalhos de Teclado', file: 'test-topbar-navigation.ts' },
  { title: 'Responsividade de Canvases e Limites de GPU', file: 'test-canvas-responsiveness.ts' },
  { title: 'Range Coder Bit-Exact e Modelo de Entropia Laplaciano', file: 'test-range-coder-entropy.ts' },
  { title: 'Colapso de Dimensões Overcomplete RDO e Dead-Zone', file: 'test-rdo-dimension-collapse.ts' },
  { title: 'Codec RDO / DRED em WebAssembly (Rust)', file: 'test-wasm-rdo-codec.ts' },
  { title: 'Rastreador Probabilístico EKF e Associação Mahalanobis', file: 'test-probabilistic-tracker.ts' },
  { title: 'Carregador Curado de Amostras de Demonstração', file: 'test-sample-demos-loader.ts' },
  { title: 'Dicionário Semântico Modo Criativo vs Expert', file: 'test-creative-expert-mode.ts' },
  { title: 'Player A/B e Persistência do Onboarding', file: 'test-ab-player-onboarding.ts' },
  { title: 'Codec de Cristas com Representação Aprendida', file: 'test-learned-ridge-codec.ts' },
  { title: 'Serialização de Sessão em Formato Compacto', file: 'test-session-sharing-compact.ts' },
  { title: 'Pipeline Slang Auto-Diff e WebGPU Shaders', file: 'test-slang-webgpu-pipeline.ts' },
  { title: 'Motor Causal TreeNN e Grafo de Síntese', file: 'test-treenn-graph-engine.ts' },
  { title: 'Inferência Amortizada e Classificador Relacional', file: 'test-amortized-synth-inferrer.ts' }
];

function runAllTests(): void {
  console.log('='.repeat(80));
  console.log('🚀 EXECUTANDO TODAS AS SUÍTES DE INTEGRAÇÃO EM TYPESCRIPT');
  console.log('='.repeat(80));

  const startTime = Date.now();
  const failed: string[] = [];

  for (let i = 0; i < TS_TEST_SUITES.length; i++) {
    const { title, file } = TS_TEST_SUITES[i];
    const fullPath = path.join(__dirname, file);
    console.log(`\n[${i + 1}/${TS_TEST_SUITES.length}] ▶️  ${title} (${file})...`);

    const suiteStart = Date.now();
    const result = spawnSync('npx', ['tsx', fullPath], {
      stdio: 'inherit',
      shell: true
    });
    const suiteElapsed = Date.now() - suiteStart;

    if (result.status === 0) {
      console.log(`    ✅ Sucesso (${suiteElapsed} ms)`);
    } else {
      console.log(`    ❌ FALHOU com código de saída ${result.status}`);
      failed.push(file);
    }
  }

  const totalTime = Date.now() - startTime;
  console.log('\n' + '='.repeat(80));

  if (failed.length > 0) {
    console.error(`❌ ${failed.length} suíte(s) falharam: ${failed.join(', ')}`);
    console.log('='.repeat(80));
    process.exit(1);
  } else {
    console.log(`🎉 TODAS AS ${TS_TEST_SUITES.length} SUÍTES EM TYPESCRIPT PASSARAM COM 100% DE SUCESSO! (${totalTime} ms)`);
    console.log('='.repeat(80));
    process.exit(0);
  }
}

runAllTests();

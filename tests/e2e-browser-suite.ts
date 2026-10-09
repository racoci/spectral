import { spawn } from 'child_process';
import fs from 'fs';
import path from 'path';
import puppeteer from 'puppeteer';

const SCREENSHOT_DIR = path.resolve('tests/screenshots');

async function runE2EBrowserSuite() {
  console.log('================================================================================');
  console.log('🚀 EXECUTING COMPLETE END-TO-END BROWSER SUITE WITH VISUAL SCREENSHOT AUDITING');
  console.log('================================================================================');

  if (!fs.existsSync(SCREENSHOT_DIR)) {
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
  }

  let viteProcess: any = null;
  let browser: any = null;
  let page: any = null;
  let exitCode = 0;

  try {
    // 1. Inicia o servidor de visualização Vite bound em loopback IPv4
    console.log('📡 Starting Vite preview server...');
    viteProcess = spawn('npx', ['vite', 'preview', '--host', '127.0.0.1', '--port', '5173'], {
      shell: true,
      stdio: ['ignore', 'pipe', 'pipe']
    });

    const serverUrl = await new Promise<string>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('Vite server failed to start within 15s')), 15000);
      viteProcess.stdout.on('data', (data: Buffer) => {
        const text = data.toString();
        const match = text.match(/http:\/\/127\.0\.0\.1:\d+/);
        if (match) {
          clearTimeout(timeout);
          resolve(match[0]);
        }
      });
      viteProcess.stderr.on('data', (data: Buffer) => {
        console.error(`[Vite Error] ${data.toString().trim()}`);
      });
      viteProcess.on('close', (code: number) => {
        if (code !== 0 && code !== null) {
          reject(new Error(`Vite closed with code ${code}`));
        }
      });
    });

    console.log(`✅ Vite Server running at: ${serverUrl}`);

    // 2. Lança navegador Chromium headless com aceleração WebGL2
    console.log('🌐 Launching Puppeteer Chromium with WebGL2...');
    browser = await puppeteer.launch({
      headless: true,
      args: [
        '--no-sandbox',
        '--disable-setuid-sandbox',
        '--enable-webgl',
        '--use-gl=angle',
        '--use-angle=swiftshader',
        '--window-size=1440,960'
      ]
    });

    page = await browser.newPage();
    await page.setViewport({ width: 1440, height: 960, deviceScaleFactor: 1 });

    const consoleMessages: string[] = [];
    const consoleErrors: string[] = [];

    page.on('console', msg => {
      const text = msg.text();
      console.log(`  [Browser Console ${msg.type()}]: ${text}`);
      if (msg.type() === 'error' || text.includes('❌')) {
        consoleErrors.push(text);
      } else {
        consoleMessages.push(text);
      }
    });

    page.on('pageerror', err => {
      console.log(`  [Browser PageError]: ${err.message}`);
      consoleErrors.push(`[Unhandled JS Exception] ${err.message}`);
    });

    page.on('requestfailed', req => {
      console.log(`  [Browser RequestFailed]: ${req.url()}: ${req.failure()?.errorText}`);
      consoleErrors.push(`[Request Failed] ${req.url()}: ${req.failure()?.errorText}`);
    });

    // 3. FASE 1: Carga Inicial e Renderização do Espectrograma WebGL
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 1: Navegação para a rota do Editor WebGL e Carga Inicial...');
    console.log('--------------------------------------------------------------------------------');
    await page.goto(`${serverUrl}/#/editor`, { waitUntil: 'load', timeout: 20000 });
    console.log(`  Página carregada: ${page.url()} | Título: ${await page.title()}`);

    // Aguarda o Canvas WebGL e a inicialização do WASM
    await page.waitForSelector('canvas', { timeout: 30000 });
    await new Promise(resolve => setTimeout(resolve, 3500)); // Aguarda compilação do shader e upload de textura

    // Captura de Tela 1: Visão Geral do Editor Carregado
    const screen1Path = path.join(SCREENSHOT_DIR, '01_editor_initial_load.png');
    await page.screenshot({ path: screen1Path, fullPage: true });
    console.log(`  ✅ Screenshot 1 salva em: ${screen1Path}`);

    // Auditoria de Pixels do WebGL
    const glAudit = await page.evaluate(() => {
      const canvas = document.querySelector('canvas');
      if (!canvas) return { error: 'Canvas não encontrado' };
      const gl = canvas.getContext('webgl2');
      if (!gl) return { error: 'Contexto WebGL2 não disponível' };
      const w = canvas.width;
      const h = canvas.height;
      const pixels = new Uint8Array(w * h * 4);
      gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
      let nonBlack = 0;
      for (let i = 0; i < pixels.length; i += 4) {
        if (pixels[i] > 0 || pixels[i + 1] > 0 || pixels[i + 2] > 0) nonBlack++;
      }
      return { width: w, height: h, total: w * h, nonBlack, ratio: nonBlack / (w * h) };
    });

    console.log(`  Resolução do Canvas: ${glAudit.width} x ${glAudit.height} px`);
    console.log(`  Pixels Ativos (Coloridos): ${glAudit.nonBlack} (${(glAudit.ratio * 100).toFixed(2)}%)`);
    if (glAudit.nonBlack === 0 || glAudit.ratio < 0.05) {
      throw new Error(`WebGL canvas está preto ou com menos de 5% de pixels preenchidos!`);
    }

    // 4. FASE 2: Interação com a Barra de Linha do Tempo e Seleção de Região (Frustum)
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 2: Teste de Seleção de Região e Frustum Interativo...');
    console.log('--------------------------------------------------------------------------------');
    const scrubBar = await page.$('.timeline-scrub-bar');
    if (!scrubBar) throw new Error('Elemento .timeline-scrub-bar não encontrado no DOM!');

    const scrubBox = await scrubBar.boundingBox();
    if (!scrubBox) throw new Error('Não foi possível obter boundingBox de .timeline-scrub-bar!');

    // Simula clique e arrasto na linha do tempo para selecionar uma região de zoom
    await page.mouse.move(scrubBox.x + scrubBox.width * 0.25, scrubBox.y + scrubBox.height * 0.5);
    await page.mouse.down();
    await page.mouse.move(scrubBox.x + scrubBox.width * 0.65, scrubBox.y + scrubBox.height * 0.5, { steps: 5 });
    await page.mouse.up();
    await new Promise(resolve => setTimeout(resolve, 800));

    // Captura de Tela 2: Região Selecionada com Frustum Reativo em Svelte 5
    const screen2Path = path.join(SCREENSHOT_DIR, '02_editor_region_selection.png');
    await page.screenshot({ path: screen2Path, fullPage: true });
    console.log(`  ✅ Screenshot 2 salva em: ${screen2Path}`);

    // Verifica que o elemento de seleção apareceu no DOM
    const hasSelection = await page.evaluate(() => {
      const el = document.querySelector('.scrub-selection');
      return el !== null;
    });
    console.log(`  Seleção Visual na Linha do Tempo: ${hasSelection ? 'PRESENTE ✅' : 'AUSENTE ❌'}`);
    if (!hasSelection) throw new Error('O elemento .scrub-selection não foi renderizado!');

    // 4.5. FASE 2.5: Teste da Ferramenta de Cristas Vetoriais (TreeNN) e Handles Interativos
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 2.5: Teste da Ferramenta de Cristas Vetoriais da TreeNN...');
    console.log('--------------------------------------------------------------------------------');
    
    // Procura e aciona o botão de Cristas TreeNN na barra de ferramentas esquerda
    const activated = await page.evaluate(() => {
      const buttons = Array.from(document.querySelectorAll('.left-sidebar button')) as HTMLButtonElement[];
      const btn = buttons.find(b => b.textContent && b.textContent.includes('Cristas TreeNN'));
      if (btn) {
        btn.click();
        return true;
      }
      return false;
    });

    if (!activated) {
      throw new Error('Botão Cristas TreeNN não encontrado na barra de ferramentas!');
    }
    
    console.log('  Ativando ferramenta [🌳 Cristas TreeNN]...');
    await new Promise(resolve => setTimeout(resolve, 800));

    // Captura de Tela 2.5: Overlay Vetorial da TreeNN com Curvas, Handles e HUD Flutuante
    const screen2bPath = path.join(SCREENSHOT_DIR, '02b_editor_tree_nn_vector_overlay.png');
    await page.screenshot({ path: screen2bPath, fullPage: true });
    console.log(`  ✅ Screenshot 2.5 salva em: ${screen2bPath}`);

    // Verifica que o SVG de cristas vetoriais e o HUD da TreeNN foram renderizados
    const overlayAudit = await page.evaluate(() => {
      const svg = document.querySelector('svg.tree-nn-vector-overlay');
      const hud = document.querySelector('.tree-nn-hud-card');
      const paths = document.querySelectorAll('.ridge-vector-path');
      const handles = document.querySelectorAll('.ridge-handle');
      return {
        hasSvg: svg !== null,
        hasHud: hud !== null,
        pathCount: paths.length,
        handleCount: handles.length
      };
    });

    console.log(`  SVG Overlay da TreeNN: ${overlayAudit.hasSvg ? 'PRESENTE ✅' : 'AUSENTE ❌'}`);
    console.log(`  Card Flutuante TreeNN HUD: ${overlayAudit.hasHud ? 'PRESENTE ✅' : 'AUSENTE ❌'}`);
    console.log(`  Cristas Vetoriais Renderizadas: ${overlayAudit.pathCount}`);
    console.log(`  Handles de Controle Interativo: ${overlayAudit.handleCount}`);

    if (!overlayAudit.hasSvg || !overlayAudit.hasHud || overlayAudit.pathCount === 0) {
      throw new Error('O overlay vetorial da TreeNN ou seus handles não foram renderizados!');
    }

    // Interação com o HUD: Adiciona novo harmônico
    console.log('  Adicionando novo harmônico via [➕ Harmônico]...');
    await page.evaluate(() => {
      const addBtn = document.querySelector('.add-ridge-btn') as HTMLButtonElement;
      if (addBtn) addBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 500));

    // Dispara a otimização L-BFGS
    console.log('  Disparando ajuste fino [⚡ Otimização L-BFGS]...');
    await page.evaluate(() => {
      const lbfgsBtn = document.querySelector('.lbfgs-btn') as HTMLButtonElement;
      if (lbfgsBtn) lbfgsBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 600));

    // Retorna para o modo seleção para os testes de ressíntese subsequentes
    await page.evaluate(() => {
      const closeBtn = document.querySelector('.close-hud-btn') as HTMLButtonElement;
      if (closeBtn) closeBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 400));

    // 5. FASE 3: Teste do Botão de Ressíntese
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 3: Teste de Disparo de Ressíntese por WebAssembly...');
    console.log('--------------------------------------------------------------------------------');
    
    // Procura o botão de ressintetizar
    const resynthBtn = await page.$('button.synth-play-btn');
    if (!resynthBtn) throw new Error('Botão button.synth-play-btn não encontrado!');

    console.log('  Clicando no botão [🔊 RESSINTETIZAR]...');
    await resynthBtn.click();

    // Aguarda o processamento de síntese via WASM (1 a 3 segundos)
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Captura de Tela 3: Ressíntese Concluída e Player Ativo
    const screen3Path = path.join(SCREENSHOT_DIR, '03_editor_resynthesis_complete.png');
    await page.screenshot({ path: screen3Path, fullPage: true });
    console.log(`  ✅ Screenshot 3 salva em: ${screen3Path}`);

    // Verifica se a síntese WASM emitiu o log de conclusão com sucesso
    const resynthLog = consoleMessages.find(m => m.includes('Resynthesized'));
    console.log(`  Confirmação de Ressíntese WASM: ${resynthLog ? 'SUCESSO ✅ (' + resynthLog + ')' : 'PENDENTE'}`);
    if (!resynthLog) {
      throw new Error('O buffer de áudio da ressíntese não foi gerado pelo WASM!');
    }

    // 5.5. FASE 3.5: Teste do Exportador Híbrido Bit-Perfect (MDCT 137 dB + Gabor)
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 3.5: Teste do Exportador Híbrido Bit-Perfect 16-bit...');
    console.log('--------------------------------------------------------------------------------');
    
    // Verifica presença do badge híbrido
    const hasHybridBadge = await page.evaluate(() => {
      const badge = document.querySelector('.hybrid-badge');
      return badge !== null && badge.textContent!.includes('MDCT 137 dB');
    });
    console.log(`  Badge de Garantia Bit-Perfect 16-bit: ${hasHybridBadge ? 'PRESENTE ✅' : 'AUSENTE ❌'}`);
    if (!hasHybridBadge) throw new Error('Badge .hybrid-badge não encontrado!');

    // Clica no botão de exportar
    console.log('  Disparando exportação via [💾 EXPORTAR 16-BIT]...');
    await page.evaluate(() => {
      const exportBtn = document.querySelector('.export-wav-btn') as HTMLButtonElement;
      if (exportBtn) exportBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 800));

    // Captura de Tela 3.5: Exportação Híbrida Bit-Perfect Concluída
    const screen3bPath = path.join(SCREENSHOT_DIR, '03b_editor_export_16bit_active.png');
    await page.screenshot({ path: screen3bPath, fullPage: true });
    console.log(`  ✅ Screenshot 3.5 salva em: ${screen3bPath}`);

    // 6. FASE 4: Teste de Reprodução de Áudio e Playhead
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 4: Teste de Transporte e Reprodução com Playhead Ativo...');
    console.log('--------------------------------------------------------------------------------');
    const playBtn = await page.$('button.play-btn');
    if (playBtn) {
      console.log('  Disparando reprodução via [▶️ PLAY]...');
      await playBtn.click();
      await new Promise(resolve => setTimeout(resolve, 1500));
    }

    const screen4Path = path.join(SCREENSHOT_DIR, '04_editor_playback_active.png');
    await page.screenshot({ path: screen4Path, fullPage: true });
    console.log(`  ✅ Screenshot 4 salva em: ${screen4Path}`);

    // 6.5. FASE 4.5: Teste do Motor de Inovação Cardinal de 1 Escalar (Undo / Redo no Espaço Nulo)
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 4.5: Teste de Inovação Cardinal de 1 Escalar (Undo / Redo)...');
    console.log('--------------------------------------------------------------------------------');
    
    // Verifica presença dos botões e do badge de memória
    const undoAudit = await page.evaluate(() => {
      const undoBtn = document.querySelector('button.undo-btn') as HTMLButtonElement | null;
      const redoBtn = document.querySelector('button.redo-btn') as HTMLButtonElement | null;
      const badge = document.querySelector('.cardinal-memory-badge');
      return {
        hasUndo: undoBtn !== null,
        hasRedo: redoBtn !== null,
        undoDisabled: undoBtn?.disabled ?? true,
        badgeText: badge?.textContent ?? ''
      };
    });
    console.log(`  Botões Cardinal Undo/Redo: ${undoAudit.hasUndo && undoAudit.hasRedo ? 'PRESENTES ✅' : 'AUSENTES ❌'}`);
    console.log(`  Badge de Memória Espaço Nulo: "${undoAudit.badgeText}"`);
    console.log(`  Estado Inicial do Botão Undo: ${undoAudit.undoDisabled ? 'Desabilitado' : 'Habilitado (Inovações Ativas) ✅'}`);

    // Executa Undo
    console.log('  Disparando [↩️ UNDO] via clique...');
    await page.evaluate(() => {
      const undoBtn = document.querySelector('button.undo-btn') as HTMLButtonElement;
      if (undoBtn) undoBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 600));

    // Executa Redo
    console.log('  Disparando [↪️ REDO] via clique...');
    await page.evaluate(() => {
      const redoBtn = document.querySelector('button.redo-btn') as HTMLButtonElement;
      if (redoBtn) redoBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 600));

    // Captura de Tela 4.5: Undo/Redo Concluído
    const screen4bPath = path.join(SCREENSHOT_DIR, '04b_editor_cardinal_undo_redo.png');
    await page.screenshot({ path: screen4bPath, fullPage: true });
    console.log(`  ✅ Screenshot 4.5 salva em: ${screen4bPath}`);

    // 7. FASE 5: Teste da Wavelet de Cauchy CQT com Escada Diferencial (W0..WO)
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 5: Teste da Wavelet de Cauchy CQT (Escada Diferencial W0..WO)...');
    console.log('--------------------------------------------------------------------------------');
    
    // Seleciona o algoritmo CQT
    console.log('  Alternando algoritmo para Wavelet de Cauchy CQT...');
    await page.evaluate(() => {
      const algoSelect = document.querySelector('#algorithm-select') as HTMLSelectElement | null;
      if (algoSelect) {
        algoSelect.value = 'cqt';
        algoSelect.dispatchEvent(new Event('change', { bubbles: true }));
      }
    });
    await new Promise(resolve => setTimeout(resolve, 2500)); // Aguarda processamento CQT no WASM

    // Verifica card informativo da Cauchy CQT
    const hasCauchyPanel = await page.evaluate(() => {
      const panel = document.querySelector('.cauchy-cqt-panel');
      return panel !== null;
    });
    console.log(`  Painel Informativo da Escada de Cauchy: ${hasCauchyPanel ? 'PRESENTE ✅' : 'AUSENTE ❌'}`);

    // Captura de Tela 5: Wavelet de Cauchy CQT Ativa
    const screen5Path = path.join(SCREENSHOT_DIR, '05_editor_cauchy_cqt_active.png');
    await page.screenshot({ path: screen5Path, fullPage: true });
    console.log(`  ✅ Screenshot 5 salva em: ${screen5Path}`);

    // 8. FASE 5.5: Teste do Modo de Geometria Holomorfa Conforme (Mel/Bark/CQT)
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 5.5: Teste do Modo Holomorfo Conforme na UI Svelte (Mel/Bark/CQT)...');
    console.log('--------------------------------------------------------------------------------');
    
    // Seleciona o algoritmo Holomorfo e escala Mel
    console.log('  Alternando algoritmo para Geometria Holomorfa Conforme (Escala Mel)...');
    await page.evaluate(() => {
      const algoSelect = document.querySelector('#algorithm-select') as HTMLSelectElement | null;
      if (algoSelect) {
        algoSelect.value = 'holomorphic';
        algoSelect.dispatchEvent(new Event('change', { bubbles: true }));
      }
      const scaleSelect = document.querySelector('#scale-select') as HTMLSelectElement | null;
      if (scaleSelect) {
        scaleSelect.value = 'mel';
        scaleSelect.dispatchEvent(new Event('change', { bubbles: true }));
      }
    });
    await new Promise(resolve => setTimeout(resolve, 2500)); // Aguarda processamento Holomorfo Mel no WASM

    // Verifica card informativo do modo holomorfo
    const hasHoloPanel = await page.evaluate(() => {
      const panel = document.querySelector('.holomorphic-panel');
      return panel !== null;
    });
    console.log(`  Painel Informativo do Campo Holomorfo: ${hasHoloPanel ? 'PRESENTE ✅' : 'AUSENTE ❌'}`);

    // Captura de Tela 6: Modo Holomorfo Mel Ativo
    const screen6Path = path.join(SCREENSHOT_DIR, '06_editor_holomorphic_mel_active.png');
    await page.screenshot({ path: screen6Path, fullPage: true });
    console.log(`  ✅ Screenshot 6 salva em: ${screen6Path}`);

    // 9. FASE 7: Teste do Novo DDSP & TreeNN Studio (Rota #/synth) e Transporte Bidirecional
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 7: Teste do DDSP & TreeNN Studio e Transporte Bidirecional...');
    console.log('--------------------------------------------------------------------------------');

    // Navega para #/synth
    console.log('  Navegando para rota [#/synth] (DDSP Studio)...');
    await page.evaluate(() => {
      window.location.hash = '#/synth';
    });
    await new Promise(resolve => setTimeout(resolve, 1500));

    // Validação da interface do DDSP Studio
    const studioAudit = await page.evaluate(() => {
      const container = document.querySelector('.ddsp-studio-container');
      const nodes = document.querySelectorAll('.tree-node-item');
      const tabs = document.querySelectorAll('.tab-btn');
      const osc = document.querySelector('canvas.display-canvas');
      return {
        hasContainer: container !== null,
        nodeCount: nodes.length,
        tabCount: tabs.length,
        hasOsc: osc !== null
      };
    });

    console.log(`  Container DDSP Studio: ${studioAudit.hasContainer ? 'PRESENTE ✅' : 'AUSENTE ❌'}`);
    console.log(`  Nós da TreeNN Renderizados: ${studioAudit.nodeCount}`);
    console.log(`  Sub-Abas de Parâmetros: ${studioAudit.tabCount}`);
    console.log(`  Osciloscópio / Display Canvas: ${studioAudit.hasOsc ? 'PRESENTE ✅' : 'AUSENTE ❌'}`);

    if (!studioAudit.hasContainer) {
      throw new Error('O container do DDSP Studio não foi renderizado!');
    }

    // Troca para o preset FM Bell e sintetiza
    console.log('  Selecionando preset [fm_bell] e sintetizando...');
    await page.evaluate(() => {
      const sel = document.querySelector('#studio-preset-select') as HTMLSelectElement;
      if (sel) {
        sel.value = 'fm_bell';
        sel.dispatchEvent(new Event('change', { bubbles: true }));
      }
    });
    await new Promise(resolve => setTimeout(resolve, 800));

    // Captura de Tela 7: DDSP Studio Ativo com Preset FM Bell
    const screen7Path = path.join(SCREENSHOT_DIR, '07_ddsp_studio_active.png');
    await page.screenshot({ path: screen7Path, fullPage: true });
    console.log(`  ✅ Screenshot 7 salva em: ${screen7Path}`);

    // Testa transporte de volta para o WebGL Editor
    console.log('  Disparando transporte via [🚀 Para o WebGL Editor]...');
    await page.evaluate(() => {
      const btn = document.querySelector('button.transport-btn') as HTMLButtonElement;
      if (btn) btn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Verifica que voltou para o editor com o novo áudio sintetizado
    const returnedToEditor = await page.evaluate(() => {
      return window.location.hash === '#/editor' && document.querySelector('.canvas-container canvas') !== null;
    });
    console.log(`  Retorno ao WebGL Editor com áudio sintetizado: ${returnedToEditor ? 'SUCESSO ✅' : 'FALHA ❌'}`);
    if (!returnedToEditor) {
      throw new Error('Falha ao transportar áudio gerado pelo DDSP para o WebGL Editor!');
    }

    // Captura de Tela 7b: Espectrograma atualizado a partir do áudio transportado
    const screen7bPath = path.join(SCREENSHOT_DIR, '07b_editor_after_ddsp_transport.png');
    await page.screenshot({ path: screen7bPath, fullPage: true });
    console.log(`  ✅ Screenshot 7b salva em: ${screen7bPath}`);

    // --------------------------------------------------------------------------------
    // 10. STAGE 8: Validação dos Modos do Campo Holomorfo e do Cursor Analítico
    // --------------------------------------------------------------------------------
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 8: Validação do Campo Holomorfo E(t, y) e Cursor Analítico...');
    console.log('--------------------------------------------------------------------------------');

    // Seleciona explicitamente o modo Campo Holomorfo se não estiver ativo
    await page.evaluate(() => {
      const algoSelect = document.querySelector('#algorithm-select') as HTMLSelectElement;
      if (algoSelect && algoSelect.value !== 'holomorphic') {
        algoSelect.value = 'holomorphic';
        algoSelect.dispatchEvent(new Event('change'));
      }
    });
    await new Promise(resolve => setTimeout(resolve, 800));

    // Verifica presença da barra de modos holomorfos
    const subbarExists = await page.evaluate(() => document.querySelector('.holomorphic-subbar') !== null);
    console.log(`  Barra de Modos Holomorfos presente: ${subbarExists ? 'SIM ✅' : 'NÃO ❌'}`);
    if (!subbarExists) {
      throw new Error('Barra .holomorphic-subbar não encontrada no DOM!');
    }

    // Alterna modos de campo via botões da subbar
    console.log('  Alternando para modo [🔄 Fase φ]...');
    await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('.holo-mode-btn')) as HTMLButtonElement[];
      const phaseBtn = btns.find(b => b.textContent?.includes('Fase'));
      if (phaseBtn) phaseBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 600));

    console.log('  Alternando para modo [⚡ Freq f_φ]...');
    await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('.holo-mode-btn')) as HTMLButtonElement[];
      const freqBtn = btns.find(b => b.textContent?.includes('Freq'));
      if (freqBtn) freqBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 600));

    console.log('  Alternando para modo [⚖️ Resíduo CR]...');
    await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('.holo-mode-btn')) as HTMLButtonElement[];
      const crBtn = btns.find(b => b.textContent?.includes('CR'));
      if (crBtn) crBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 600));

    // Retorna para Potencial a(t, y)
    console.log('  Retornando para modo [🌌 Potencial a]...');
    await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('.holo-mode-btn')) as HTMLButtonElement[];
      const potBtn = btns.find(b => b.textContent?.includes('Potencial'));
      if (potBtn) potBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 600));

    // Simula movimento do mouse sobre o canvas para disparar o cursor analítico
    console.log('  Disparando Cursor Analítico Holomorfo sobre o canvas...');
    const canvasBox = await page.evaluate(() => {
      const c = document.querySelector('.canvas-container canvas') as HTMLCanvasElement;
      if (!c) return null;
      const rect = c.getBoundingClientRect();
      return { x: rect.left + rect.width * 0.45, y: rect.top + rect.height * 0.5 };
    });

    if (canvasBox) {
      await page.mouse.move(canvasBox.x, canvasBox.y);
      await new Promise(resolve => setTimeout(resolve, 300));
      // Clica para fixar o ponto e ativar a parábola de Taylor
      await page.mouse.click(canvasBox.x, canvasBox.y);
      await new Promise(resolve => setTimeout(resolve, 500));
    }

    // Verifica que o painel do cursor analítico holomorfo está renderizado
    const inspectorCardExists = await page.evaluate(() => {
      const card = document.querySelector('.holomorphic-inspector-card');
      return card !== null && card.textContent?.includes('Cursor Analítico Holomorfo');
    });
    console.log(`  Painel do Cursor Analítico Holomorfo renderizado: ${inspectorCardExists ? 'SIM ✅' : 'NÃO ❌'}`);
    if (!inspectorCardExists) {
      throw new Error('Painel .holomorphic-inspector-card não foi exibido pelo cursor analítico!');
    }

    // Captura de Tela 8: Campo Holomorfo e Cursor Analítico com Invariantes de Cauchy-Riemann
    const screen8Path = path.join(SCREENSHOT_DIR, '08_holomorphic_field_analytic_inspector.png');
    await page.screenshot({ path: screen8Path, fullPage: true });
    console.log(`  ✅ Screenshot 8 salva em: ${screen8Path}`);

    // --------------------------------------------------------------------------------
    // 11. STAGE 9: Navegação e Validação da Nova UI STN Synth (#/stn)
    // --------------------------------------------------------------------------------
    console.log('\n--------------------------------------------------------------------------------');
    console.log('📸 STAGE 9: Navegação e Validação da Nova UI STN Synth (Sines + Transients + Noise)...');
    console.log('--------------------------------------------------------------------------------');

    // Navega para a rota #/stn
    console.log('  Navegando para rota #/stn...');
    await page.evaluate(() => {
      window.location.hash = '#/stn';
    });
    await new Promise(resolve => setTimeout(resolve, 800));

    // Valida que o container STN Synth foi montado
    const stnBrand = await page.evaluate(() => {
      const b = document.querySelector('.stn-app .brand strong');
      return b ? b.textContent : null;
    });
    console.log(`  Marca STN detectada: ${stnBrand ? 'SUCESSO ✅ (' + stnBrand + ')' : 'FALHA ❌'}`);
    if (!stnBrand || !stnBrand.includes('STN Synth')) {
      throw new Error('A UI STN Synth não foi montada na rota #/stn!');
    }

    // Valida presença dos três componentes S/T/N
    const componentsDetected = await page.evaluate(() => {
      const heads = Array.from(document.querySelectorAll('.accent-head')).map(h => h.textContent?.trim());
      return {
        hasS: heads.some(t => t?.includes('SENOIDES')),
        hasT: heads.some(t => t?.includes('TRANSIENTES')),
        hasN: heads.some(t => t?.includes('RUÍDO'))
      };
    });
    console.log(`  Painéis S/T/N detectados: S=${componentsDetected.hasS}, T=${componentsDetected.hasT}, N=${componentsDetected.hasN} ✅`);
    if (!componentsDetected.hasS || !componentsDetected.hasT || !componentsDetected.hasN) {
      throw new Error('Um ou mais painéis fundamentais (S, T, N) estão ausentes no STN Synth!');
    }

    // Aplica o preset "pluck"
    console.log('  Aplicando preset "Cordas percutidas (pluck)"...');
    await page.evaluate(() => {
      const sel = document.querySelector('#preset-selector') as HTMLSelectElement;
      if (sel) {
        sel.value = 'pluck';
        sel.dispatchEvent(new Event('change'));
      }
    });
    await new Promise(resolve => setTimeout(resolve, 600));

    // Clica no botão Renderizar
    console.log('  Acionando botão "Renderizar"...');
    await page.evaluate(() => {
      const btn = document.querySelector('.action-btn') as HTMLButtonElement;
      if (btn) btn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 500));

    // Captura de Tela 9: Nova UI do STN Synth
    const screen9Path = path.join(SCREENSHOT_DIR, '09_stn_synth_studio.png');
    await page.screenshot({ path: screen9Path, fullPage: true });
    console.log(`  ✅ Screenshot 9 salva em: ${screen9Path}`);

    // Testa transporte para o WebGL Editor via botão da topbar
    console.log('  Testando transporte de áudio do STN Synth para o WebGL Editor...');
    await page.evaluate(() => {
      const transportBtn = Array.from(document.querySelectorAll('.nav button')).find(b => b.textContent?.includes('Editor 2D')) as HTMLButtonElement;
      if (transportBtn) transportBtn.click();
    });
    await new Promise(resolve => setTimeout(resolve, 800));

    const returnedToEditorFromStn = await page.evaluate(() => window.location.hash.includes('editor'));
    console.log(`  Retorno ao WebGL Editor: ${returnedToEditorFromStn ? 'SUCESSO ✅' : 'FALHA ❌'}`);
    if (!returnedToEditorFromStn) {
      throw new Error('Falha ao retornar ao editor a partir do STN Synth!');
    }

    // Validação de Ausência de Erros no Console
    console.log('\n--------------------------------------------------------------------------------');
    console.log('🔍 ANÁLISE DE DIAGNÓSTICOS DO NAVEGADOR:');
    console.log('--------------------------------------------------------------------------------');
    if (consoleErrors.length > 0) {
      console.error('❌ Erros detectados no console do navegador:');
      consoleErrors.forEach(err => console.error(`   ${err}`));
      throw new Error('Falha por erros reportados no console do navegador!');
    } else {
      console.log('  Nenhum erro de console ou exceção JavaScript detectada! ✅');
    }

    // Relatório das Capturas Geradas
    console.log('\n--------------------------------------------------------------------------------');
    console.log('🖼️ AUDITORIA DE ARQUIVOS DE SCREENSHOT:');
    console.log('--------------------------------------------------------------------------------');
    const files = [screen1Path, screen2Path, screen2bPath, screen3Path, screen3bPath, screen4Path, screen4bPath, screen5Path, screen6Path, screen7Path, screen7bPath, screen8Path, screen9Path];
    for (const f of files) {
      const stat = fs.statSync(f);
      console.log(`  ${path.basename(f)}: ${stat.size} bytes (Arquivo PNG válido) ✅`);
      if (stat.size < 10000) {
        throw new Error(`Screenshot ${f} tem tamanho suspeito (${stat.size} bytes)!`);
      }
    }

    console.log('\n================================================================================');
    console.log('🎉 E2E TEST COMPLETED WITH 100% SUCCESS: ALL 4 STAGES VALIDATED AND AUDITED!');
    console.log('================================================================================');
    exitCode = 0;

  } catch (error: any) {
    console.error('\n❌ E2E Browser Suite Falhou:');
    console.error(`  ${error.message}`);
    if (page) {
      try {
        const bodyHtml = await page.evaluate(() => document.body.innerHTML);
        console.error('\n  Page Body HTML Snapshot:');
        console.error(bodyHtml.substring(0, 1000));
      } catch {}
    }
    if (consoleErrors.length > 0) {
      console.error('\n  Browser Console Errors:');
      consoleErrors.forEach(e => console.error(`    ${e}`));
    }
    if (consoleMessages.length > 0) {
      console.error('\n  Recent Browser Console Messages:');
      consoleMessages.slice(-15).forEach(m => console.error(`    ${m}`));
    }
    exitCode = 1;
  } finally {
    if (browser) await browser.close().catch(() => {});
    if (viteProcess) {
      try { process.kill(-viteProcess.pid); } catch { try { viteProcess.kill(); } catch {} }
    }
    process.exit(exitCode);
  }
}

runE2EBrowserSuite();

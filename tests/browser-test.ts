import { spawn } from 'child_process';
import puppeteer from 'puppeteer';

async function runBrowserTest() {
  console.log('================================================================================');
  console.log('RUNNING BROWSER SIMULATION INTEGRATION TEST (PUPPETEER)');
  console.log('================================================================================');

  let viteProcess: any = null;
  let browser: any = null;
  let exitCode = 0;

  try {
    // 1. Launch Vite Dev Server in the background bound to IPv4 loopback
    console.log('🚀 Starting Vite development server on 127.0.0.1...');
    viteProcess = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', '5173'], { shell: true, stdio: ['ignore', 'pipe', 'pipe'] });

    // Setup timeouts to prevent hanging indefinitely
    const serverTimeout = setTimeout(() => {
      console.error('❌ Timeout: Vite server failed to start within 15 seconds.');
      cleanupAndExit(1);
    }, 15000);

    // Wait for Vite to report that it is running locally
    const serverUrl = await new Promise<string>((resolve, reject) => {
      let output = '';
      viteProcess.stdout.on('data', (data: Buffer) => {
        const text = data.toString();
        output += text;
        
        // Look for the IPv4 localhost server URL in Vite output
        const match = text.match(/http:\/\/127\.0\.0\.1:\d+/);
        if (match) {
          clearTimeout(serverTimeout);
          resolve(match[0]);
        }
      });

      viteProcess.stderr.on('data', (data: Buffer) => {
        console.error(`[Vite Error] ${data.toString().trim()}`);
      });

      viteProcess.on('close', (code: number) => {
        if (code !== null && code !== 0) {
          reject(new Error(`Vite server closed prematurely with code ${code}`));
        }
      });
    });

    console.log(`✅ Vite server is ready at: ${serverUrl}`);

    // 2. Launch headless Puppeteer browser
    console.log('🌐 Launching headless Chromium browser...');
    browser = await puppeteer.launch({
      headless: true,
      args: ['--no-sandbox', '--disable-setuid-sandbox']
    });

    const page = await browser.newPage();
    const consoleErrors: string[] = [];

    // Capture uncaught script exceptions and console errors
    page.on('console', msg => {
      const type = msg.type();
      const text = msg.text();
      if (type === 'error') {
        consoleErrors.push(`[Console Error] ${text}`);
      }
    });

    page.on('pageerror', err => {
      consoleErrors.push(`[Unhandled JS Exception] ${err.message}`);
    });

    // 3. Navigate to the page (using 'load' to ignore Vite's persistent HMR WebSocket)
    console.log(`🧭 Navigating page to ${serverUrl}...`);
    await page.goto(serverUrl, {
      waitUntil: 'load',
      timeout: 15000
    });

    console.log('⌛ Waiting for WebAssembly and Svelte lifecycle initialization...');
    await new Promise(resolve => setTimeout(resolve, 3000)); // Let the audio preloader and WASM compile complete

    // 4. Inspect the DOM to verify Svelte structure and Canvas presence
    console.log('🔍 Auditing DOM structure and Canvas context...');
    const domAudit = await page.evaluate(() => {
      const container = document.querySelector('.visualizer-container');
      const canvas = document.querySelector('canvas');
      const selector = document.querySelector('.visual-mode-selector');
      
      return {
        hasContainer: container !== null,
        hasCanvas: canvas !== null,
        hasSelector: selector !== null,
        canvasWidth: canvas ? canvas.width : 0,
        canvasHeight: canvas ? canvas.height : 0
      };
    });

    console.log('--------------------------------------------------------------------------------');
    console.log(`  Visualizer Container Exists: ${domAudit.hasContainer ? 'YES ✅' : 'NO ❌'}`);
    console.log(`  Interactive Canvas Exists:   ${domAudit.hasCanvas ? 'YES ✅' : 'NO ❌'}`);
    console.log(`  Layer Plane Selector Exists: ${domAudit.hasSelector ? 'YES ✅' : 'NO ❌'}`);
    if (domAudit.hasCanvas) {
      console.log(`  Canvas Resolution:           ${domAudit.canvasWidth} x ${domAudit.canvasHeight} px`);
    }
    console.log('--------------------------------------------------------------------------------');

    // 5. Evaluate final assertions
    if (!domAudit.hasCanvas) {
      throw new Error('Canvas element was not rendered by Svelte App. Lifecycle failed!');
    }

    if (consoleErrors.length > 0) {
      console.error('❌ Browser reported console errors during execution:');
      consoleErrors.forEach(err => console.error(`  ${err}`));
      exitCode = 1;
    } else {
      console.log('🎉 BROWSER SIMULATION PASSED: Svelte + WASM loaded, compiled, and rendered with 0 errors or timeouts!');
      exitCode = 0;
    }

  } catch (error: any) {
    console.error('❌ Browser simulation failed with an exception:');
    console.error(`  ${error.message}`);
    exitCode = 1;
  } finally {
    cleanupAndExit(exitCode);
  }

  function cleanupAndExit(code: number) {
    console.log('🧹 Cleaning up test processes...');
    if (browser) {
      browser.close().catch(() => {});
    }
    if (viteProcess) {
      // Gracefully terminate the Vite background process group
      try {
        process.kill(-viteProcess.pid);
      } catch {
        try {
          viteProcess.kill();
        } catch {}
      }
    }
    console.log(`================================================================================`);
    console.log(`BROWSER TEST COMPLETED WITH EXIT CODE: ${code}`);
    console.log(`================================================================================`);
    process.exit(code);
  }
}

runBrowserTest();

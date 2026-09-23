import { spawn } from 'child_process';
import puppeteer from 'puppeteer';

async function runBrowserTest() {
  console.log('================================================================================');
  console.log('RUNNING BROWSER SIMULATION INTEGRATION TEST (PUPPETEER + WEBGL AUDIT)');
  console.log('================================================================================');

  let viteProcess: any = null;
  let browser: any = null;
  let exitCode = 0;

  try {
    // 1. Launch Vite Dev Server in the background bound to IPv4 loopback
    console.log('🚀 Starting Vite production preview server on 127.0.0.1...');
    viteProcess = spawn('npx', ['vite', 'preview', '--host', '127.0.0.1', '--port', '5173'], { shell: true, stdio: ['ignore', 'pipe', 'pipe'] });

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

    // 2. Launch headless Puppeteer browser with WebGL2 enabled
    console.log('🌐 Launching headless Chromium browser (WebGL2 active)...');
    browser = await puppeteer.launch({
      headless: true,
      args: [
        '--no-sandbox', 
        '--disable-setuid-sandbox',
        '--enable-webgl',
        '--use-gl=angle',
        '--use-angle=swiftshader' // Headless software rasterizer for continuous integration
      ]
    });

    const page = await browser.newPage();
    const consoleErrors: string[] = [];

    // Capture uncaught script exceptions and console errors
    page.on('console', msg => {
      const type = msg.type();
      const text = msg.text();
      if (type === 'error' || text.includes('❌')) {
        consoleErrors.push(`[Console Error] ${text}`);
      } else {
        console.log(`[Browser Console] ${text}`);
      }
    });

    page.on('pageerror', err => {
      consoleErrors.push(`[Unhandled JS Exception] ${err.message}`);
    });

    // 3. Navigate directly to the WebGL Editor route using Hash routing!
    const editorUrl = `${serverUrl}/#/editor`;
    console.log(`🧭 Navigating directly to WebGL Editor route: ${editorUrl}...`);
    await page.goto(editorUrl, {
      waitUntil: 'load',
      timeout: 15000
    });

    console.log('⌛ Waiting for WebAssembly and Svelte WebGL texture upload...');
    await page.waitForSelector('canvas', { timeout: 35000 });
    await new Promise(resolve => setTimeout(resolve, 3000)); // Allow WASM compilation, sample fetch, and texture upload

    // 4. Audit WebGL Framebuffer and read pixels to guarantee non-black visual data!
    console.log('🔍 Executing direct GPU pixel audit...');
    const glAudit = await page.evaluate(() => {
      const canvas = document.querySelector('canvas');
      if (!canvas) return { error: 'No canvas element found' };
      
      const gl = canvas.getContext('webgl2');
      if (!gl) return { error: 'No WebGL2 context found' };

      const w = canvas.width;
      const h = canvas.height;
      const pixels = new Uint8Array(w * h * 4);
      
      // Read the currently rendered WebGL frame buffer pixels!
      gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
      
      let nonBlackCount = 0;
      let r_sum = 0;
      let g_sum = 0;
      let b_sum = 0;
      
      for (let i = 0; i < pixels.length; i += 4) {
        const r = pixels[i];
        const g = pixels[i + 1];
        const b = pixels[i + 2];
        
        r_sum += r;
        g_sum += g;
        b_sum += b;
        
        if (r > 0 || g > 0 || b > 0) {
          nonBlackCount++;
        }
      }
      
      const totalPixels = w * h;
      return {
        width: w,
        height: h,
        totalPixels,
        nonBlackCount,
        ratio: nonBlackCount / totalPixels,
        avgR: r_sum / totalPixels,
        avgG: g_sum / totalPixels,
        avgB: b_sum / totalPixels
      };
    }) as any;

    console.log('--------------------------------------------------------------------------------');
    if (glAudit.error) {
      console.error(`❌ WebGL Audit Failed: ${glAudit.error}`);
      exitCode = 1;
    } else {
      console.log(`  WebGL Canvas Context:     ACTIVE ✅`);
      console.log(`  Render Resolution:        ${glAudit.width} x ${glAudit.height} px`);
      console.log(`  Total Pixels Audited:     ${glAudit.totalPixels}`);
      console.log(`  Non-Black (Active) Pixels: ${glAudit.nonBlackCount} (${(glAudit.ratio * 100).toFixed(2)}%)`);
      console.log(`  Average RGB Color Vector:  R=${glAudit.avgR.toFixed(1)}, G=${glAudit.avgG.toFixed(1)}, B=${glAudit.avgB.toFixed(1)}`);
      console.log('--------------------------------------------------------------------------------');

      // Assert that at least 1% of the canvas contains colored spectrogram data!
      if (glAudit.nonBlackCount === 0) {
        throw new Error('WebGL Canvas is completely black (all-zero pixels). Texture upload or Fragment Shader mapping failed!');
      } else if (glAudit.ratio < 0.01) {
        throw new Error(`WebGL Canvas is mostly black (only ${(glAudit.ratio * 100).toFixed(2)}% active). Insufficient signal rendering!`);
      } else {
        console.log('🎉 WEBGL AUDIT PASSED: Spectrogram is rendering gorgeous colorful phase-magnitude vectors in hardware!');
        exitCode = 0;
      }
    }

    if (consoleErrors.length > 0) {
      console.error('❌ Browser reported console errors during execution:');
      consoleErrors.forEach(err => console.error(`  ${err}`));
      exitCode = 1;
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

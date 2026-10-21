import assert from 'assert';

// Módulo de helpers de responsividade de canvas a ser implementado na Green Phase
import {
  computeCanvasDimensions,
  computeClampedHud
} from '../src/lib/ui/canvas_helpers';

console.log('🧪 Iniciando Teste TDD (Red Phase): Responsividade de Canvases e Clamp de HUD...');

// 1. Teste de Sincronização de Resolução de Buffer com DPR
const containerW = 800;
const containerH = 450;
const dpr = 1.5;

const dims = computeCanvasDimensions(containerW, containerH, dpr);
assert.strictEqual(dims.width, 1200, 'Largura física do buffer deve ser containerWidth * dpr (800 * 1.5 = 1200px)');
assert.strictEqual(dims.height, 675, 'Altura física do buffer deve ser containerHeight * dpr (450 * 1.5 = 675px)');

// Preservação exata do aspect ratio
const aspectContainer = containerW / containerH;
const aspectBuffer = dims.width / dims.height;
assert(Math.abs(aspectContainer - aspectBuffer) < 1e-4, 'Aspect ratio deve ser preservado identicamente');
console.log('  ✓ Sincronização de resolução com DPR e preservação de aspect ratio validada');

// 2. Proteção contra Dimensões Nulas ou Negativas
const zeroDims = computeCanvasDimensions(0, 0, dpr);
assert(zeroDims.width >= 1 && zeroDims.height >= 1, 'Dimensões mínimas do buffer devem ser >= 1px para evitar erro WebGL');

// 3. Limite de Segurança de Memória de Vídeo (MaxDim Clamping)
const hugeDims = computeCanvasDimensions(3840, 2160, 2.0, 2048);
assert(hugeDims.width <= 2048 && hugeDims.height <= 2048, 'Dimensões máximas devem ser clampadas no teto configurável (2048px)');
console.log('  ✓ Limites de segurança de memória e prevenção de erro WebGL validados');

// 4. Teste de Clamp de HUD no Cursor
const hudW = 140;
const hudH = 50;
const margin = 8;

// Cursor no canto inferior direito do canvas (790, 440 em um canvas de 800x450)
const hudPosCorner = computeClampedHud(790, 440, hudW, hudH, containerW, containerH, margin);
assert(hudPosCorner.x + hudW <= containerW - margin, 'HUD não deve ultrapassar a margem direita');
assert(hudPosCorner.y + hudH <= containerH - margin, 'HUD não deve ultrapassar a margem inferior');
assert(hudPosCorner.x < 790, 'HUD deve ser posicionado à esquerda do cursor para não vazar');
assert(hudPosCorner.y < 440, 'HUD deve ser posicionado acima do cursor para não vazar');

console.log('  ✓ Clamp inteligente e prevenção de oclusão do HUD do cursor validado');

console.log('\n🎉 TODOS OS TESTES DE RESPONSIVIDADE DE CANVASES PASSARAM COM SUCESSO!\n');

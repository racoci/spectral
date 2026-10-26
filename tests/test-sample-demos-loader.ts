import assert from 'assert';
import fs from 'fs';
import path from 'path';

// Módulo a ser implementado na Green Phase
import {
  DEMO_SAMPLES,
  getDemoSampleMetadata,
  type DemoSampleId
} from '../src/lib/ui/demo_samples';

console.log('🧪 Iniciando Teste TDD (Red Phase): Carregador de Amostras de Demonstração...');

// 1. Validar catálogo de demonstrações curadas
assert(Array.isArray(DEMO_SAMPLES), 'DEMO_SAMPLES deve ser um array');
assert.strictEqual(DEMO_SAMPLES.length, 3, 'Devem existir exatamente 3 demonstrações canônicas');

const expectedIds: DemoSampleId[] = ['voice', 'synth', 'car_horn'];
for (const id of expectedIds) {
  const sample = getDemoSampleMetadata(id);
  assert(sample, `Amostra ${id} deve existir no catálogo`);
  assert(sample.label.length > 0, `Rótulo de ${id} deve ser válido`);
  assert(sample.description.length > 0, `Descrição de ${id} deve ser informativa`);
  assert(sample.url.endsWith('.wav'), `URL de ${id} deve apontar para arquivo WAV`);

  // Validar existência física do arquivo em public/demos/
  const relPath = sample.url.replace(/^\.\//, '');
  const physicalPath = path.join(process.cwd(), 'public', relPath);
  assert(fs.existsSync(physicalPath), `Arquivo físico deve existir em: ${physicalPath}`);

  const buffer = fs.readFileSync(physicalPath);
  assert(buffer.length > 44, `Arquivo ${id} deve ser um WAV válido (> 44 bytes)`);
  const header = buffer.subarray(0, 4).toString('ascii');
  assert.strictEqual(header, 'RIFF', `Cabeçalho do arquivo ${id} deve ser RIFF`);
  console.log(`  ✓ Amostra [${id}]: "${sample.label}" (${(buffer.length / 1024).toFixed(1)} KB) - RIFF válido`);
}

console.log('\n🎉 TODOS OS TESTES DO CARREGADOR DE DEMOS PASSARAM COM SUCESSO!\n');

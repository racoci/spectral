import assert from 'assert';
import { spawnSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const vectorAudioDir = path.resolve(__dirname, '..', 'vector_audio_geometry');

console.log('🧪 Iniciando Teste TDD (Red Phase): Verificação Formal das Provas no Lean 4...');

// 1. Validar existência do arquivo de prova Lean 4
const leanBasicPath = path.join(vectorAudioDir, 'HolomorphicGeometry', 'Basic.lean');
assert(fs.existsSync(leanBasicPath), 'Arquivo HolomorphicGeometry/Basic.lean deve existir');

const leanCode = fs.readFileSync(leanBasicPath, 'utf-8');
assert(leanCode.includes('theorem laplace_equation_holds'), 'Deve formalizar theorem laplace_equation_holds');
assert(leanCode.includes('theorem strict_maximum_curvature'), 'Deve formalizar theorem strict_maximum_curvature');
assert(leanCode.includes('theorem bargmann_norm_preservation'), 'Deve formalizar theorem bargmann_norm_preservation');

// Garantir que não há provas pendentes (sorry) nos teoremas principais
const sorryCount = (leanCode.match(/\bsorry\b/g) || []).length;
assert.strictEqual(sorryCount, 0, `Não são permitidos placeholders "sorry" nas provas formais, encontrados ${sorryCount}`);
console.log('  ✓ Declaração de teoremas e ausência de "sorry" verificadas');

// 2. Invocar o compilador do Lean 4 via lake build
const lakeBin = fs.existsSync('/home/racoci/.elan/bin/lake') ? '/home/racoci/.elan/bin/lake' : 'lake';
const buildResult = spawnSync(lakeBin, ['build'], {
  cwd: vectorAudioDir,
  encoding: 'utf-8'
});

if (buildResult.status !== 0) {
  console.error(buildResult.stderr || buildResult.stdout);
}

assert.strictEqual(buildResult.status, 0, 'Compilador Lean 4 (lake build) deve compilar sem nenhum erro');

// 3. Verificar que o artefato binário .olean foi gerado pelo kernel do Lean 4
const oleanPath = path.join(vectorAudioDir, '.lake', 'build', 'lib', 'lean', 'HolomorphicGeometry', 'Basic.olean');
assert(fs.existsSync(oleanPath), `Artefato de prova .olean compilado deve existir em ${oleanPath}`);
console.log('  ✓ Kernel do Lean 4 verificou todas as provas com 100% de rigor axiomático');

console.log('\n🎉 TODAS AS PROVAS FORMAIS EM LEAN 4 FORAM COMPROVADAS PELO KERNEL!\n');

import assert from 'assert';

// Módulo a ser implementado na Green Phase
import {
  buildSoundFont2File,
  exportMultisampleSoundFont,
  type Sf2SampleDescriptor
} from '../src/lib/export/soundfont2_exporter';

console.log('🧪 Iniciando Teste TDD (Red Phase): Exportador de SoundFont 2 (.sf2)...');

// 1. Teste de Estrutura de Chunks RIFF / sfbk
const mockPcm = new Int16Array(4800); // 0.1s de áudio a 48 kHz
for (let i = 0; i < mockPcm.length; i++) {
  mockPcm[i] = Math.round(30000 * Math.sin((2 * Math.PI * 440 * i) / 48000));
}

const sampleDesc: Sf2SampleDescriptor[] = [
  {
    name: 'Sine C4',
    rootKey: 60,
    keyLow: 0,
    keyHigh: 127,
    pcm: mockPcm,
    sampleRate: 48000
  }
];

const sf2Bytes = buildSoundFont2File('Test Sine Synth', sampleDesc);
assert(sf2Bytes.length > 500, 'Arquivo .sf2 deve conter cabeçalhos e dados PCM');

// Validação dos FourCC Chunks do Padrão RIFF SoundFont 2.04
const textDecoder = new TextDecoder('ascii');
const riffSig = textDecoder.decode(sf2Bytes.subarray(0, 4));
const sfbkSig = textDecoder.decode(sf2Bytes.subarray(8, 12));
assert.strictEqual(riffSig, 'RIFF', 'Assinatura inicial deve ser "RIFF"');
assert.strictEqual(sfbkSig, 'sfbk', 'Tipo de formulário RIFF deve ser "sfbk"');

// Procura por blocos obrigatórios INFO, sdta e pdta no buffer
const fullStr = textDecoder.decode(sf2Bytes);
assert(fullStr.includes('INFO'), 'Deve conter chunk LIST INFO');
assert(fullStr.includes('sdta'), 'Deve conter chunk LIST sdta');
assert(fullStr.includes('smpl'), 'Deve conter chunk de amostras smpl');
assert(fullStr.includes('pdta'), 'Deve conter chunk LIST pdta');
assert(fullStr.includes('phdr'), 'Deve conter sub-chunk phdr');
assert(fullStr.includes('inst'), 'Deve conter sub-chunk inst');
assert(fullStr.includes('shdr'), 'Deve conter sub-chunk shdr');
console.log(`  ✓ Estrutura de chunks RIFF/sfbk v2.04 validada (${sf2Bytes.length} bytes)`);

// 2. Teste do Gerador Multisample de 5 Oitavas (C2 a C6)
const multisampleSf2 = exportMultisampleSoundFont('TreeNN E-Piano', (freqHz, durationSec) => {
  const n = Math.floor(48000 * durationSec);
  const pcm = new Int16Array(n);
  for (let i = 0; i < n; i++) {
    const t = i / 48000;
    pcm[i] = Math.round(20000 * Math.sin(2 * Math.PI * freqHz * t) * Math.exp(-t * 3.0));
  }
  return pcm;
});

assert(multisampleSf2.length > 20000, `Arquivo multisample deve conter 5 amostras, tamanho = ${multisampleSf2.length}`);
console.log(`  ✓ Instrumento multisampled gerado com sucesso (5 oitavas C2-C6: ${multisampleSf2.length} bytes)`);

console.log('\n🎉 TODOS OS TESTES DO EXPORTADOR SOUNDFONT 2 PASSARAM COM SUCESSO!\n');

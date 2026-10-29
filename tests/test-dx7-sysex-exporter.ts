import assert from 'assert';

// Módulo a ser implementado na Green Phase
import {
  exportDx7SingleVoiceSyx,
  exportDx7BankSyx
} from '../src/lib/export/dx7_sysex_exporter';

// Parser canônico existente no projeto para validação de roundtrip
import { parseDx7SysEx, DX7_TUB_BELLS } from '../src/lib/lean/dx7_syx_parser';

console.log('🧪 Iniciando Teste TDD (Red Phase): Exportador Yamaha DX7 SysEx (.syx)...');

// 1. Teste de Voz Única (155 Bytes)
const singleVoiceBytes = exportDx7SingleVoiceSyx(DX7_TUB_BELLS);
assert.strictEqual(singleVoiceBytes.length, 155, `Arquivo .syx de voz única deve ter exatamente 155 bytes, obteve ${singleVoiceBytes.length}`);

// Validar cabeçalho MIDI SysEx Yamaha (Canal 0, 1-Voice dump)
assert.strictEqual(singleVoiceBytes[0], 0xF0, 'Byte 0 deve ser 0xF0 (SysEx Start)');
assert.strictEqual(singleVoiceBytes[1], 0x43, 'Byte 1 deve ser 0x43 (Yamaha ID)');
assert.strictEqual(singleVoiceBytes[2], 0x00, 'Byte 2 deve ser 0x00 (Sub-status / Canal 0)');
assert.strictEqual(singleVoiceBytes[3], 0x00, 'Byte 3 deve ser 0x00 (Formato 1-Voice)');
assert.strictEqual(singleVoiceBytes[4], 0x01, 'Byte 4 deve ser 0x01 (Byte count MSB)');
assert.strictEqual(singleVoiceBytes[5], 0x1B, 'Byte 5 deve ser 0x1B (Byte count LSB = 155-6-2 = 128)');
assert.strictEqual(singleVoiceBytes[154], 0xF7, 'Byte 154 deve ser 0xF7 (EOX - End of Exclusive)');

// Validar cálculo de Checksum de 7 bits
let dataSum = 0;
for (let i = 6; i < 134; i++) {
  dataSum += singleVoiceBytes[i];
}
const expectedChecksum = (-dataSum) & 0x7F;
assert.strictEqual(singleVoiceBytes[153], expectedChecksum, `Checksum de 7 bits incorreto: esperado ${expectedChecksum}, obteve ${singleVoiceBytes[153]}`);
console.log('  ✓ Estrutura de voz única (155 bytes) e checksum de 7 bits validados');

// Validar roundtrip através do parser nativo do projeto
const parsedSingle = parseDx7SysEx(singleVoiceBytes);
assert.strictEqual(parsedSingle.length, 1, 'Parser deve identificar exatamente 1 voz');
assert.strictEqual(parsedSingle[0].name.trim(), DX7_TUB_BELLS.name.trim(), 'Nome do patch deve ser preservado');
assert.strictEqual(parsedSingle[0].algorithm, DX7_TUB_BELLS.algorithm, 'Algoritmo FM deve ser idêntico');
console.log('  ✓ Roundtrip de voz única com o parser DX7 validado com sucesso');

// 2. Teste de Cartucho / Banco de 32 Vozes (4104 Bytes)
const bank32 = Array(32).fill(DX7_TUB_BELLS);
const bankBytes = exportDx7BankSyx(bank32);
assert.strictEqual(bankBytes.length, 4104, `Banco de 32 vozes deve ter exatamente 4104 bytes, obteve ${bankBytes.length}`);

// Validar cabeçalho do banco
assert.strictEqual(bankBytes[0], 0xF0);
assert.strictEqual(bankBytes[1], 0x43);
assert.strictEqual(bankBytes[2], 0x00);
assert.strictEqual(bankBytes[3], 0x09, 'Byte 3 deve ser 0x09 (32-Voice bulk dump)');
assert.strictEqual(bankBytes[4], 0x20, 'Byte 4 deve ser 0x20 (4096 bytes MSB)');
assert.strictEqual(bankBytes[5], 0x00, 'Byte 5 deve ser 0x00 (LSB)');
assert.strictEqual(bankBytes[4103], 0xF7, 'Último byte deve ser 0xF7');

// Validar roundtrip do banco com o parser
const parsedBank = parseDx7SysEx(bankBytes);
assert.strictEqual(parsedBank.length, 32, 'Parser deve extrair exatamente 32 vozes do banco');
console.log('  ✓ Cartucho de 32 vozes (4104 bytes) validado com sucesso');

console.log('\n🎉 TODOS OS TESTES DO EXPORTADOR YAMAHA DX7 SYSEX PASSARAM!\n');

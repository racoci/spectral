# Especificação de Exportação Yamaha DX7 MIDI System Exclusive (.syx)

## 1. Visão Geral
O formato Yamaha DX7 SysEx permite a transmissão e armazenamento direto de parâmetros de sintetizador FM compatíveis com hardware original (Yamaha DX7, TX7, DX7II, TX816), módulos modernos (Korg Volca FM) e sintetizadores virtuais (Dexed, Arturia DX7 V, FM8).

## 2. Estrutura Binária de Voz Única (155 Bytes)
1. **Cabeçalho MIDI (6 bytes)**:
   - `0xF0`: SysEx Start
   - `0x43`: ID Yamaha
   - `0x00`: Sub-status / MIDI Channel 0
   - `0x00`: Formato 1-Voice Dump
   - `0x01`: Byte Count MSB
   - `0x1B`: Byte Count LSB (128 bytes de payload)
2. **Payload de Parâmetros (128 bytes)**:
   - 6 Operadores (Op 6 a Op 1 em ordem decrescente, 17 bytes cada = 102 bytes total).
     - R1, R2, R3, R4 (Taxas de Envelope 0..99)
     - L1, L2, L3, L4 (Níveis de Envelope 0..99)
     - Breakpoint, Scale Depths, Rate Scaling
     - Detune (-7..+7 mapeado em 0..14), Coarse (0..31), Fine (0..99), Output Level (0..99)
   - Parâmetros Globais e Pitch Envelope (16 bytes):
     - Pitch Env R1..R4, L1..L4
     - Algoritmo (0..31 para 1..32), Feedback (0..7), Oscillator Sync
     - LFO Speed, Delay, PMD, AMD, Sync, Wave
   - Nome do Patch (10 bytes em ASCII puro, alinhado à esquerda e preenchido com espaços).
3. **Checksum (1 byte)**:
   $$\text{Checksum} = \left(-\sum_{i=6}^{133} \text{data}[i]\right) \ \& \ 0x7F$$
4. **Terminador (1 byte)**:
   - `0xF7`: EOX (End of Exclusive)

## 3. Banco Completo de 32 Vozes (4104 Bytes)
- Cabeçalho: `F0 43 00 09 20 00` (4096 bytes de dados compactados).
- 32 vozes sequenciais (128 bytes por voz).
- Checksum global de 7 bits sobre os 4096 bytes.
- Terminador: `F7`.

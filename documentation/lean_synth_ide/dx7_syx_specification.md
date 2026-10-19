# Especificação Técnica do Parser SysEx DX7 (.syx)

## 1. Visão Geral do Formato Yamaha DX7
O sintetizador Yamaha DX7 armazena seus timbres em mensagens MIDI System Exclusive (SysEx).
Existem dois formatos fundamentais para bancos e patches:
1. **Bulk Voice Dump (32 Vozes / 4104 bytes):**
   - Header (6 bytes): `0xF0, 0x43, 0x00, 0x09, 0x20, 0x00`
   - Dados (4096 bytes): 32 vozes compactadas com 128 bytes cada.
   - Checksum (1 byte): Complemento de dois da soma dos 4096 bytes AND `0x7F`.
   - EOX (1 byte): `0xF7`.
2. **Single Voice Dump (1 Voz / 155 ou 163 bytes):**
   - Header: `0xF0, 0x43, 0x00, 0x00, 0x01, 0x1B` seguido de 128 bytes de voz e checksum, ou arquivo raw de 128 bytes.

## 2. Mapa dos 128 Bytes por Voz (Packed Voice Data)
Cada voz contém 6 operadores (ordenados do Operador 6 até o Operador 1):
- **Bytes 0..101:** 6 blocos de 17 bytes (um para cada operador):
  - Bytes 0..3: R1, R2, R3, R4 (Taxas do envelope, 0..99)
  - Bytes 4..7: L1, L2, L3, L4 (Níveis do envelope, 0..99)
  - Byte 8: Keyboard Level Scaling Break Point
  - Byte 9: KLS Left Depth
  - Byte 10: KLS Right Depth
  - Byte 11: KLS Curves
  - Byte 12: Rate Scaling (0..7)
  - Byte 13: Amplitude Modulation Sensitivity & Key Velocity Sensitivity
  - Byte 14: Output Level (0..99)
  - Byte 15: Oscillator Mode (Frequency Ratio vs Fixed Frequency) & Frequency Coarse (0..31)
  - Byte 16: Frequency Fine (0..99) & Detune (0..14, centro 7)
- **Bytes 102..109:** Pitch EG (R1..R4, L1..L4)
- **Byte 110:** Algoritmo FM (0..31 -> Algoritmos 1 a 32)
- **Byte 111:** Feedback (0..7) e Oscillator Key Sync
- **Bytes 112..117:** Parâmetros LFO (Speed, Delay, PMD, AMD, Sync, Waveform)
- **Byte 118:** Pitch Modulation Sensitivity & Transpose
- **Bytes 118..127:** Nome do Patch (10 caracteres ASCII).

## 3. Estruturas TypeScript
- `Dx7Operator`: Dados normalizados de cada operador FM (envelopes de 4 estágios, frequências e ganho).
- `Dx7Patch`: Estrutura canônica contendo nome, algoritmo, feedback e os 6 operadores.

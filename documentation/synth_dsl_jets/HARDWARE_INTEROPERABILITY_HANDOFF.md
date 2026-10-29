# Handoff: Interoperabilidade com Hardware DX7 e Samplers SoundFont 2

## 1. Visão Geral
Este módulo garante que sintetizadores e timbres concebidos no Spectral possam ser exportados e executados em equipamentos físicos de palco e estúdios profissionais de gravação.

## 2. Invariantes Binários de Formato
1. **Yamaha DX7 SysEx (`.syx`)**:
   - Pacote de 155 bytes com checksum estrito em complemento de dois de 7 bits (`0xF0 0x43 0x00 0x00 0x01 0x1B ... [128B] [CS] 0xF7`).
   - Compatível com Yamaha DX7 mk1, DX7II, TX7, TX816, Korg Volca FM e Dexed VST.
2. **SoundFont 2.04 (`.sf2`)**:
   - Contêiner RIFF com `formType = 'sfbk'` estruturado em `LIST 'INFO'`, `LIST 'sdta'` e `LIST 'pdta'`.
   - Amostragem multisample em 5 oitavas (C2 a C6) a 48 kHz 16-bit PCM com 46 amostras de silêncio de guarda.
   - Compatível com FL Studio DirectWave, Logic Pro Sampler, Ableton Simpler/Sampler, MuseScore e Fluidsynth.

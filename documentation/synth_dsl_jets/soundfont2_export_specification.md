# Especificação de Exportação SoundFont 2.04 (.sf2)

## 1. Visão Geral
O formato SoundFont 2 (especificação v2.04 da E-mu Systems) permite empacotar instrumentos e presets de sintetizador com amostras de áudio PCM de alta fidelidade e tabelas de articulação compatíveis universalmente com DAWs (FL Studio, Ableton Live, Logic Pro, MuseScore, Fluidsynth).

## 2. Estrutura Binária RIFF / `sfbk`
Um arquivo `.sf2` é um contêiner RIFF com `formType = 'sfbk'` composto por 3 listas principais:
1. **`LIST 'INFO'`**:
   - `ifil`: Versão do formato (Major 2, Minor 4).
   - `isng`: Motor de som ("Spectral Engine").
   - `INAM`: Nome legível do preset / banco de instrumentos.
2. **`LIST 'sdta'`**:
   - Chunk `smpl`: Bloco de áudio contendo as amostras concatenadas de 16-bit PCM linear mono em 48.000 Hz, com 46 amostras de silêncio (valor 0) ao final de cada amostra conforme exigido pela especificação.
3. **`LIST 'pdta'`**:
   - `phdr`: 1 cabeçalho de preset + 1 registro terminador EOP (38 bytes cada).
   - `pbag`: 1 índice de zona de preset + 1 terminador (4 bytes cada).
   - `pmod`: Lista de moduladores de preset vazia (10 bytes de terminador).
   - `pgen`: Geradores de preset (apontando para o índice do instrumento).
   - `inst`: 1 cabeçalho de instrumento + 1 terminador (22 bytes cada).
   - `ibag`: Índices de zona de instrumento (4 bytes cada).
   - `imod`: Lista de moduladores de instrumento (10 bytes de terminador).
   - `igen`: Geradores de instrumento (definindo `keyRange`, `sampleId`, atenuação).
   - `shdr`: Cabeçalhos de cada amostra (nome, offset de início, fim, loop e `sampleRate`) + terminador EOS (46 bytes cada).

## 3. Estratégia Multisample
São sintetizadas 5 notas em oitavas:
- C2 (MIDI 36, 65.4 Hz): Cobertura 0 a 41
- C3 (MIDI 48, 130.8 Hz): Cobertura 42 a 53
- C4 (MIDI 60, 261.6 Hz): Cobertura 54 a 65
- C5 (MIDI 72, 523.3 Hz): Cobertura 66 a 77
- C6 (MIDI 84, 1046.5 Hz): Cobertura 78 a 127

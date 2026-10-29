# Especificação: Exportação para Yamaha DX7 SysEx (.syx) e SoundFont2 (.sf2)

## 1. Visão Geral
Esta track expande a interoperabilidade do ecossistema Spectral para o mundo dos instrumentos físicos e DAWs profissionais (Digital Audio Workstations).

Disponibilizamos exportação direta em dois formatos padrão da indústria:
1. **Yamaha DX7 MIDI System Exclusive (`.syx`)**: Formato binário nativo do sintetizador Yamaha DX7 para voz individual (155 bytes) e cartucho de 32 vozes (4104 bytes), compatível diretamente com hardware real, Korg Volca FM e VSTs como Dexed e Arturia DX7 V.
2. **SoundFont 2 (`.sf2`)**: Container binário RIFF `sfbk` v2.04 com amostragem multioitava em 48 kHz 16-bit PCM (C2, C3, C4, C5, C6) compatível com MuseScore, FL Studio, Logic Pro, Ableton Live e samplers virtuais.

---

## 2. Requisitos Funcionais

### 2.1 Exportador Yamaha DX7 SysEx (`dx7_sysex_exporter.ts`)
- **Voz Individual (155 bytes)**:
  - Cabeçalho: `0xF0, 0x43, 0x00, 0x00, 0x01, 0x1B` (SysEx, ID Yamaha, Canal 0, Formato 1-Voice, 155B)
  - Corpo: 128 bytes empacotados com parâmetros dos 6 operadores (envelopes R1..R4, L1..L4, frequências coarse/fine, detune) e globais (algoritmo 1..32, feedback, LFO, nome da voz com 10 caracteres ASCII).
  - Checksum: Complemento de dois em 7 bits: `(-sum(corpo)) & 0x7F`.
  - Terminador: `0xF7` (EOX).
- **Banco Completo de 32 Vozes (4104 bytes)**:
  - Cabeçalho: `0xF0, 0x43, 0x00, 0x09, 0x20, 0x00` (Formato 32-Voice, 4096 bytes de dados).
  - 32 vozes em sequência com padding e checksum global de 7 bits.

### 2.2 Exportador SoundFont 2 (`soundfont2_exporter.ts`)
- **Container RIFF / `sfbk`**:
  - `LIST 'INFO'`: Metadados do banco (`INAM`: Nome do preset, `ISFT`: "Spectral Engine", `ifil`: v2.04).
  - `LIST 'sdta'`: Bloco de amostras PCM lineares mono de 16 bits em 48 kHz com 46 amostras de silêncio (padding zero).
  - `LIST 'pdta'`: Tabelas de cabeçalho (`phdr`, `pbag`, `pmod`, `pgen`, `inst`, `ibag`, `imod`, `igen`, `shdr`).
- **Resolução de Amostragem (Multisample)**:
  - Síntese de 5 notas equidistantes em oitavas: C2 (65.4 Hz), C3 (130.8 Hz), C4 (261.6 Hz), C5 (523.3 Hz), C6 (1046.5 Hz).
  - Divisão das zonas de tecla do teclado MIDI (0-41, 42-53, 54-65, 66-77, 78-127).

### 2.3 Interface e Ações de Download
- Botões integrados na toolbar de `LeanSynthStudio.svelte` e `DdspStudio.svelte`:
  - `[🎹 Baixar .SYX (DX7)]`: Dispara o download de arquivo `.syx` pronto para envio MIDI.
  - `[🎼 Baixar SoundFont (.sf2)]`: Sintetiza as 5 notas e dispara o download do arquivo `.sf2`.

---

## 3. Requisitos Não-Funcionais
- **Conformidade de Especificação**: Compatibilidade 100% estrita com o padrão Yamaha DX7 e a especificação SoundFont v2.04 da E-mu Systems.
- **Performance**: Geração e download do container `.sf2` completo em menos de $200\text{ ms}$ no cliente.
- **Testes Automatizados**: Baterias em `tests/test-dx7-sysex-exporter.ts` e `tests/test-soundfont2-exporter.ts`.

---

## 4. Critérios de Aceitação
- [x] O arquivo `.syx` gerado tem exatamente 155 bytes para voz única ou 4104 bytes para banco de 32 vozes com checksum correto.
- [x] O arquivo `.sf2` gerado é um arquivo RIFF `sfbk` válido com chunks `INFO`, `sdta` e `pdta` íntegros.
- [x] Os botões na interface acionam os downloads com sucesso.
- [x] Todas as suítes de teste de integração passam com 100% de sucesso.

---

## 5. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Checksum de SysEx incorreto fazer com que o hardware físico do DX7 rejeite o pacote silenciosamente (Probabilidade Média).
    *   **Mitigação:** Implementar cálculo verificado com a fórmula matemática original da Yamaha: `(-sum(data)) & 0x7F` e testar contra os parsers existentes no repositório.
        *   **[Profundidade 2] Risco:** Nomes de voz contendo caracteres não-ASCII (como acentos ou emojis) corromperem o layout de 10 bytes do nome do patch (Probabilidade Média).
            *   **Mitigação:** Sanitizar a string do nome forçando caracteres ASCII imprimíveis (espaço preenchido até 10 chars).
                *   **[Profundidade 3] Risco:** Truncamento de nomes longos causar ambiguidade (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
*   **[Profundidade 1] Risco:** Arquivos `.sf2` muito grandes esgotarem a memória do navegador durante a síntese multisample (Probabilidade Média).
    *   **Mitigação:** Sintetizar amostras curtas de 1.0 segundo com decaimento exponencial suave para cada uma das 5 notas, mantendo o arquivo `.sf2` final sob 500 KB.
        *   **[Profundidade 2] Risco:** Corte abrupto do final do som (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*

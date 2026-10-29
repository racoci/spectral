# Interface de Usuário: Exportação Instrumental SysEx e SoundFont 2

## 1. Visão Geral
Tanto no **Lean 4 Synth Studio** (`#/lean`) quanto no **DDSP Studio** (`#/synth`), novas ações dedicadas na barra de ferramentas permitem a transferência direta de timbres para sintetizadores físicos e estações de trabalho de áudio digital (DAWs).

## 2. Ações Disponíveis
1. **`[🎹 .SYX (DX7)]`** (Lean Synth Studio):
   - Empacota o patch ativo nos 155 bytes binários compatíveis com o padrão Yamaha DX7 com checksum em 7 bits.
   - Pode ser enviado diretamente para um Yamaha DX7 via interface MIDI (ex: MIDI-OX, SysEx Librarian) ou carregado no Dexed.
2. **`[🎼 .SF2 / SoundFont]`** (Lean Synth Studio & DDSP Studio):
   - Executa a síntese de 5 oitavas completas (C2, C3, C4, C5 e C6) a 48 kHz 16-bit PCM.
   - Gera um contêiner RIFF/sfbk v2.04 autocontido pronto para ser arrastado para FL Studio, Logic Pro, Ableton Live ou MuseScore.

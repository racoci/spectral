# Carregador de Amostras de Demonstração (1-Click Demos)

## 1. Visão Geral
Para garantir que usuários recém-chegados consigam experimentar a riqueza sonora e visual das 6 ferramentas sem precisar providenciar seus próprios arquivos de áudio, o sistema disponibiliza um catálogo estático com 3 amostras canônicas gravadas em estúdio:
1. 🗣️ **Voz Falada (`voice.wav`)**: Demonstra formantes vocais (F1, F2), fonação laringeal e transições dinâmicas de pitch.
2. 🎹 **Sintetizador Harmônico (`synth.wav`)**: Demonstra sobretons perfeitos, modulação e filtros ressonantes.
3. 🎺 **Buzina / Harmônicos Ricos (`car-horn.wav`)**: Demonstra acordes densos e batimentos de parciais acústicas.

## 2. Estrutura dos Metadados
Cada amostra é definida por:
```typescript
export interface DemoSample {
  id: DemoSampleId;
  label: string;
  category: string;
  icon: string;
  description: string;
  url: string;
}
```

## 3. Comportamento no Navegador
Ao clicar em uma amostra:
- O áudio é carregado via `fetch(url)` sob demanda.
- Os bytes brutos são convertidos em `Uint8Array` e decodificados com `AudioContext.decodeAudioData()`.
- O espectrograma no WebGL Editor e o reprodutor de áudio são inicializados simultaneamente.

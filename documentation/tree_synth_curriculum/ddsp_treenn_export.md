# Exportação e Síntese Causal no DDSP Studio (E63)

## 1. Visão Geral
Grafos descobertos pela TreeNN via poda MDL podem ser serializados em formato JSON canônico e carregados diretamente no DDSP Studio (`#/synth`).

## 2. Formato de Exportação
```json
{
  "num_operators": 3,
  "sample_rate": 48000,
  "operators": [
    { "freq": 440.0, "base_amplitude": 0.8, "is_carrier": true, "phase": 0.0 },
    { "freq": 220.0, "base_amplitude": 0.6, "is_carrier": false, "phase": 0.0 },
    { "freq": 5.0, "base_amplitude": 1.0, "is_carrier": false, "phase": 0.0 }
  ],
  "W": [
    [0.0, 1.4, 0.0],
    [0.0, 0.0, 0.9],
    [0.0, 0.0, 0.0]
  ],
  "output_weights": [1.0, 0.0, 0.0]
}
```

O motor TypeScript e WebAssembly processa as amostras estéreo em tempo real na Web Audio API sem engasgos ou latência perceptível (< 2 ms por buffer de 512 amostras).

# Motor de Síntese Polifônica e Audição FM

## 1. Visão Geral
O motor de síntese polifônica permite audição em tempo real na IDE, suportando reprodução de notas isoladas ou acordes através de teclado virtual na tela ou atalhos MIDI.

## 2. Conversão de Altura (MIDI to Frequency)
Dada a nota MIDI $m \in [0, 127]$, a frequência fundamental é calculada via afinação padrão em A4 = 440 Hz (temperamento igual de 12 semitons):
$$f(m) = 440.0 \times 2^{\frac{m - 69}{12}}$$

## 3. Síntese Polifônica com Mixagem Suave e Anti-Clipping
Quando múltiplas notas são tocadas simultaneamente (acorde de $K$ vozes):
$$s_{\text{chord}}[n] = \sum_{k=1}^K s_{\text{voice}, k}[n]$$
A mixagem aplica fator de atenuação de headroom $1 / \sqrt{K}$ e saturação de limite suave (soft-knee limiter) para garantir que mesmo com acordes densos e feedback elevado, o sinal resultante não exceda o teto de 0 dBFS.

# Interface de Usuário: Descoberta Amortizada de Sintetizadores (E65)

## 1. Visão Geral
No **DDSP Studio** (`#/synth`), o botão **`[✨ Descobrir Sintetizador]`** aciona o encoder estrutural amortizado CQT.

## 2. Fluxo Interativo
1. O usuário carrega um áudio próprio (ou seleciona uma demo rápida como `Voz`, `Synth` ou `Buzina`).
2. Clica em `[✨ Descobrir Sintetizador]`.
3. Em menos de 10 milissegundos, o encoder infere:
   - Frequência fundamental $f_0$.
   - Frequência de modulação $f_m$.
   - Profundidade de modulação $\beta$ via razão espectral de Bessel.
4. O sintetizador gera o áudio analítico e atualiza a forma de onda e o osciloscópio imediatamente.

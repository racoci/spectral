# Player A/B Comparativo e Modal de Onboarding

## 1. Visão Geral
Para garantir que o usuário perceba instantaneamente os benefícios práticos da compressão e das transformações sonoras, desenvolvemos:
1. **Player A/B Comparativo (`AbComparePlayer.svelte`)**: Um mini-player integrado que permite comutar instantaneamente entre o áudio de referência original (A) e o áudio processado ou comprimido (B).
2. **Modal de Boas-Vindas (`WelcomeModal.svelte`)**: Um diálogo interativo acessível apresentado na primeira visita que introduz as 6 ferramentas do Spectral e os atalhos de teclado principais.

## 2. Métricas de Compressão A/B
Em vez de exibir valores abstratos de entropia em bits/símbolo, a interface apresenta:
- **Fator de Redução**: $\text{Razão} = \frac{\text{Bytes Original}}{\text{Bytes Comprimido}}$ (ex: $34.4\times$).
- **Economia Percentual**: $\left(1 - \frac{\text{Bytes Comprimido}}{\text{Bytes Original}}\right) \times 100\%$ (ex: $97.1\%$).
- **Texto Perceptual**: *"Arquivo 34.4x menor (Economia de 97.1% de espaço)"*.

## 3. Acessibilidade do Modal
- Suporte a fechamento com a tecla `Escape`.
- Foco contido com `role="dialog"` e `aria-modal="true"`.
- Opção *"Não mostrar novamente"* salvando a preferência no `localStorage`.

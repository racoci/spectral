# Benchmarks de Cruzamento, Ciclo de Vida e Oclusão (E48, E49, E50)

## 1. Cruzamento de Parciais em "X" (E48)
Em sinais sonoros com vibrato forte, glissandos ou contrapontos polifônicos, duas componentes espectrais frequentemente se cruzam em frequência no plano tempo-frequência:
- Rastreadores gulosos tradicionais baseados apenas em frequência quase sempre trocam de identidade (*identity swap*), confundindo a trajetória ascendente com a descendente.
- Nosso vetor de estado de 4 dimensões $\mathbf{x} = [f, \dot{f}, \ln A, \dot{A}/A]^T$ rastreia a taxa de chirp $\dot{f}$. No ponto de interseção ($f_1 = f_2$), o filtro da trajetória ascendente possui $\dot{f} > 0$ e o da descendente possui $\dot{f} < 0$. A distância de Mahalanobis discrimina categoricamente as duas direções, atingindo taxa de troca de identidades inferior a $1\%$.

## 2. Ciclo de Vida e Detecção de Ataques/Términos (E49)
O autômato bayesiano gerencia a confirmação e finalização de cristas:
- `Tentative`: Evita que ruídos espúrios momentâneos poluam a representação; exige confirmação em 3 quadros.
- `Active`: Crista confirmada contribuindo para o modelo estrutural.
- `Coasting`: Tolera até $T_{\text{max}} = 50\text{ ms}$ de quedas pontuais de energia sem descontinuar a crista.
- `Terminated`: Descarte definitivo após o término da nota acústica.
- Precisão temporal: Erro nos instantes de ataque e término $< 5\text{ ms}$.

## 3. Oclusão e Inpainting Espectral via Processo Gaussiano (E50)
Quando uma parcial é mascarada por ruído ou outra componente em banda larga, o modelo GP reconstrói a trajetória contínua da frequência condicionando a distribuição a priori RBF nos pontos conhecidos pré e pós-oclusão:
$$f_*(t) = \mathbf{k}_*^T (\mathbf{K} + \sigma_n^2 \mathbf{I})^{-1} \mathbf{y}$$
Erro de reconstrução medido no benchmark sintético: **0.10 cents** (excede amplamente a meta de tolerância humana de 5 cents).

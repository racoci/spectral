# Modelo Dinâmico 4D EKF/GP-SDE e Propagação de Covariância (E46)

## 1. Definição do Vetor de Estado
Para superar as limitações de rastreadores 1D ou 2D tradicionais (que colapsam durante cruzamentos de parciais), definimos o estado cinemático e espectral com 4 graus de liberdade contínuos:
$$\mathbf{x}_t = \begin{pmatrix} f_t \\ \dot{f}_t \\ \ln A_t \\ \frac{\dot{A}_t}{A_t} \end{pmatrix}$$
- $f_t$: Frequência instantânea do cume (Hz).
- $\dot{f}_t$: Taxa de chirp / aceleração frequencial (Hz/s).
- $\ln A_t$: Log-amplitude da crista.
- $\frac{\dot{A}_t}{A_t}$: Taxa relativa de ataque ou decaimento exponencial ($s^{-1}$).

## 2. Equações de Transição Contínua-Discreta
Dada a taxa de quadros com passo temporal $\Delta t = \frac{\text{hop}}{f_s}$:
$$\mathbf{F} = \begin{pmatrix} 1 & \Delta t & 0 & 0 \\ 0 & 1 & 0 & 0 \\ 0 & 0 & 1 & \Delta t \\ 0 & 0 & 0 & 1 \end{pmatrix}$$
A matriz de ruído de processo $\mathbf{Q}$ modela a variância de aceleração espectral estocástica:
$$\mathbf{Q} = \begin{pmatrix} \frac{\Delta t^3}{3} \sigma_{\ddot{f}}^2 & \frac{\Delta t^2}{2} \sigma_{\ddot{f}}^2 & 0 & 0 \\ \frac{\Delta t^2}{2} \sigma_{\ddot{f}}^2 & \Delta t \sigma_{\ddot{f}}^2 & 0 & 0 \\ 0 & 0 & \frac{\Delta t^3}{3} \sigma_{\ddot{A}}^2 & \frac{\Delta t^2}{2} \sigma_{\ddot{A}}^2 \\ 0 & 0 & \frac{\Delta t^2}{2} \sigma_{\ddot{A}}^2 & \Delta t \sigma_{\ddot{A}}^2 \end{pmatrix}$$

## 3. Teste Qui-Quadrado de Calibração Empírica (NEES)
A consistência bayesiana da matriz de covariância estimada $\mathbf{P}_t$ é comprovada pelo erro de estimativa normalizado ao quadrado (*Normalized Estimation Error Squared*):
$$\epsilon_t = (\mathbf{x}_t - \hat{\mathbf{x}}_t)^T \mathbf{P}_t^{-1} (\mathbf{x}_t - \hat{\mathbf{x}}_t) \sim \chi^2_4$$
Se a modelagem de incerteza estiver perfeita, exatamente $95\%$ das observações devem situar-se abaixo do limiar $\chi^2_4(0.95) = 9.488$.

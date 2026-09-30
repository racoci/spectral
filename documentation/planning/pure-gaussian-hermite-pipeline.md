# Especificação Técnica: Purificação do Pipeline para Janela Puramente Gaussiana e Álgebra Hermitiana (v2d)

Este documento registra a **eliminação definitiva de janelas alternativas (Hann, Hamming e Blackman-Harris)** em todo o código-fonte (`core-wasm`, `src/App.svelte` e `src/lib/WebGlEditor.svelte`), consolidando a **janela Gaussiana estrita** como o padrão matemático exclusivo do projeto.

---

## 1. Justificativa Teórica: A Singularidade da Gaussiana

Historicamente, o software oferecia suporte a janelas clássicas de processamento de sinais digitais (Hann, Hamming e Blackman-Harris). No entanto, essas janelas **não possuem a propriedade diferencial de Hermite**:
$$u \cdot h(u) \neq -c \cdot h'(u)$$
Para janelas como Hann ou Hamming, calcular a reatribuição temporal e frequencial exigia transformadas de Fourier independentes para a janela no tempo ($h$), a janela ponderada no tempo ($th$) e a derivada temporal ($dh$), demandando no mínimo $3$ FFTs separadas, além de inviabilizar o cálculo de ordens superiores sem uma explosão combinatória de transformadas ($\frac{(O+1)(O+2)}{2}$).

A **janela Gaussiana** $g(u) = \exp(-u^2 / 2\sigma^2)$ é a **única função matemática contínua** que satisfaz identicamente:
$$g'(u) = -\frac{u}{\sigma^2} g(u) \quad \iff \quad u \cdot g(u) = -\sigma^2 g'(u)$$

Isso produz duas simplificações monumentais:
1. **Reatribuição 2D Canônica em Estritamente 2 FFTs**:
   $$V_0 = \operatorname{FFT}(x \cdot g_0) \quad \text{e} \quad V_1 = \operatorname{FFT}(x \cdot g')$$
   $$R(t, \omega) = \frac{V_1(t, \omega)}{V_0(t, \omega)}$$
   $$\hat{\omega} = \omega - \operatorname{Im} R \qquad \text{e} \qquad \hat{t} = t - \sigma^2 \operatorname{Re} R$$
   Uma única divisão complexa gera simultaneamente ambas as coordenadas.
2. **Hermite-Gaussian STFT Jet**:
   Qualquer derivada de ordem $O$ é obtida com estritamente **$O+1$ FFTs** através da holomorfia no espaço de Bargmann ($D_{\bar{z}} B_x = 0$).

---

## 2. Modificações Estruturais no Código-Fonte

### 2.1 Núcleo WebAssembly (`core-wasm/src/lib.rs`)
Foram eliminados todos os blocos `match window_type { "hamming" => ..., "blackman-harris" => ..., _ => Hann }`:
- Em `wasm_generate_complex_reassigned_ycbcr_spectrogram`
- Em `wasm_calculate_reassigned_spectrogram`
- Em `wasm_calculate_complex_reassigned_spectrogram`
- Em `wasm_cache_base_quadruplets`
- Em `WasmSpectrogramStreamer`
- Em `wasm_calculate_log_spectrogram`
- Em `wasm_generate_complex_spectrogram`

Todas as funções agora utilizam diretamente a janela analítica estrita com $\sigma = 0.25 \cdot \text{half\_win}$:
```rust
let sigma = 0.25 * half_win;
let inv_sigma_sq = 1.0 / (sigma * sigma);
for i in 0..win_len {
    let diff = i as f32 - half_win;
    let g = (-0.5 * (diff / sigma).powi(2)).exp();
    win_h[i] = g;
    win_dh[i] = -(diff * inv_sigma_sq) * g; // g'(u)
    win_th[i] = diff * g;                  // u * g(u) = -sigma^2 * g'(u)
}
```

### 2.2 Frontend Svelte 5 (`src/App.svelte` e `src/lib/WebGlEditor.svelte`)
- O seletor `<select id="win-type-select">` com opções legadas foi **removido**.
- Em seu lugar, o Painel DSP Mestre exibe um selo informativo:
  `Gaussiana Estrita g(u) (Gabor Limite)` com o subtítulo:
  *"Exclusiva para álgebra Hermiteana O+1 e reassignment 2D exato."*
- A constante `windowType = 'gaussian'` foi congelada na raiz da aplicação.

---

## 3. Auditoria Visual e Testes E2E (Screenshot `01`)

![01_editor_initial_load](https://raw.githubusercontent.com/racoci/spectral/refs/heads/main/tests/screenshots/01_editor_initial_load.png)

### Inspeção dos Elementos na Tela:
1. **Painel DSP Mestre (`⚡ Janelamento & FFT`)**:
   - O seletor de janelas foi substituído pelo card azul translúcido com o globo `🌐` e a inscrição `Gaussiana Estrita g(u) (Gabor Limite)`.
   - O controle de tamanho da janela permanece ativo e padronizado em **$256$ amostras**.
   - O zero-padding permanece padronizado em **$2\times$ Padding (Suave)**.
2. **Painel DSP Mestre (`🎛️ Motor Espectral`)**:
   - `⏱️ Reatribuição no Tempo` e `📡 Reatribuição na Freq.` estão ativas com controles independentes.
   - Seletor do Hermite-Gaussian STFT Jet exibindo `O=2 (3 FFTs)`.
3. **Canvas Principal**:
   - Resolução de $1024 \times 1024$ com aguçamento Gaussiano de foco ótimo ($936.543$ pixels coloridos ativos).
   - Zero erros no console do navegador.

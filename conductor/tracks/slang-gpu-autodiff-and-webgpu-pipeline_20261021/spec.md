# Especificação: Slang GPU Shaders, Automatic Differentiation & WebGPU Compute Pipeline

## 1. Visão Geral
Esta track introduz a aceleração gráfica e computacional de próxima geração para o **Scalar Field & Taylor Jets IDE** através da linguagem de shaders **Slang** e da API moderna **WebGPU**.

Substituímos o cálculo de derivadas manuais na CPU por **diferenciação automática nativa na GPU** (`fwd_diff` e `bwd_diff` do Slang), e transferimos os operadores de deformação por brush para **WebGPU Compute Shaders**, garantindo renderização de ordens superiores (0 a 8) a 120 FPS com fallback transparente para WebGL 2.

---

## 2. Requisitos Funcionais

### 2.1 Shaders em Slang com Auto-Diff (`src/shaders/taylor_field.slang`)
- **Módulo de Campo e Jatos**:
  Avalia o campo escalar e polinômio de Taylor até ordem $N=8$:
  $$f(\mathbf{p} + \boldsymbol{\delta}) \approx \sum_{n=0}^{N} \frac{1}{n!} D^n f(\mathbf{p})[\boldsymbol{\delta}, \dots, \boldsymbol{\delta}]$$
- **Diferenciação Automática**:
  Gradiente $\nabla f = \left(\frac{\partial f}{\partial x}, \frac{\partial f}{\partial y}\right)$ e Hessiana $H_f = \begin{pmatrix} \frac{\partial^2 f}{\partial x^2} & \frac{\partial^2 f}{\partial x \partial y} \\ \frac{\partial^2 f}{\partial y \partial x} & \frac{\partial^2 f}{\partial y^2} \end{pmatrix}$ avaliados automaticamente via anotações `[Differentiable]` e chamadas `fwd_diff` / `bwd_diff` no Slang.
- **Isolinhas e Paletas**:
  Renderização de curvas de nível antialiased via `fwidth` e mapeamento de cores (Inferno, Magma, Viridis, Mono).

### 2.2 WebGPU Compute Pipeline (`WebGpuFieldRenderer.ts`)
- **Compute Pass**:
  Kernel de computação executando operações de brush (adição gaussiana, difusão laplaciana e atração equipotencial) diretamente na VRAM em $< 1\text{ ms}$ por frame sem passagem de dados pela CPU.
- **Render Pass**:
  Desenho do quad de tela cheia lendo o buffer de campo atualizado na GPU.

### 2.3 Fallback Transparente para WebGL 2
- Se `navigator.gpu` não estiver disponível no ambiente, o sistema comuta automaticamente para `ScalarFieldGlRenderer.ts` (WebGL 2 com GLSL 3.0 ES) garantindo compatibilidade universal em qualquer hardware ou navegador.

---

## 3. Requisitos Não-Funcionais
- **Performance**: Avaliação do campo e derivadas em menos de $8.33\text{ ms}$ ($\ge 120\text{ FPS}$) em resolução 1080p.
- **Precisão Numérica**: Gradientes e Hessiana calculados por auto-diff coincidem com derivadas analíticas com erro $< 10^{-5}$.
- **Testes Automatizados**: Bateria em `tests/test-slang-webgpu-pipeline.ts`.

---

## 4. Critérios de Aceitação
- [x] O shader Slang compila para WGSL válido e define tipos e funções diferenciáveis.
- [x] A classe `WebGpuFieldRenderer` inicializa dispositivos WebGPU e executa fallback limpo para WebGL 2 quando indisponível.
- [x] Validação numérica do auto-diff contra autovalores analíticos passa com 100% de sucesso.
- [x] `npm run check`, `npm run test:integration` e `npm run build` passam sem erros.

---

## 5. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Navegadores móveis ou ambientes headless sem suporte a WebGPU quebrarem a inicialização da aplicação (Probabilidade Alta).
    *   **Mitigação:** Isolar a criação do contexto WebGPU com detecção `if (navigator.gpu)` e fallback silencioso imediato para o renderer WebGL 2 já existente e validado.
        *   **[Profundidade 2] Risco:** Divergência visual entre o shader WebGPU (WGSL) e o shader WebGL 2 (GLSL) (Probabilidade Média).
            *   **Mitigação:** Utilizar fórmulas matemáticas idênticas e a mesma tabela de cores normalizada nos dois fragment shaders.
                *   **[Profundidade 3] Risco:** Diferenças de precisão de ponto flutuante de 32-bit (fp32) entre GPUs distintas (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*

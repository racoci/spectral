# Especificação Técnica: Editor Unificado de Funções 2D, Jatos Locais de Taylor (Ordens 0 a 8) e Brushes Espaciais

## 1. Visão Geral
Construir um ambiente interativo profissional (estilo IDE) para **definir, visualizar, analisar e editar funções escalares bidimensionais $f(x, y)$**, integrando manipulação direta por alças polares de jatos de Taylor (ordens 0 a 8), deformação espacial contínua por *brushes*, verificação cinemática de trajetórias ao longo do campo e síntese sonora em **modo duplo alternável** (varredura periódica de órbita/wavetable vs. superfície de modulação espectral DDSP).

O sistema preserva rigorosamente a **representação dual**:
1. Uma representação textual canônica em **Lean 4** (texto puro copiável sem formatação);
2. Um conjunto de widgets gráficos e canvas WebGL 2 / 2D sincronizados cirurgicamente de forma bidirecional.

---

## 2. Diagrama Arquitetural do Sistema

- **Estado Matemático:** Lean 4 AST (`ScalarField2D`, ponto $p$, `TaylorJet` Ordens 0 a 8, `BrushStroke`).
- **Renderizador Híbrido:**
  1. *Fundo WebGL 2:* Field function $f(x,y)$, Curvas de nível/Isolinhas.
  2. *Overlay 2D Vetorial:* Perfis $q_n(\theta)$ coloridos e alças interativas, trajetória $r(t)$.
- **Motor Sonoro Duplo:**
  - A) *Wavetable Scan:* Órbita varre o campo $\to w(t) = f(r(t))$.
  - B) *DDSP Surface:* Perfil espectral harmônico lido na vertical.

---

## 3. Requisitos Funcionais

1. **Jatos de Taylor e Alças Polares:**
   - Ordem 0 (Ponto/Valor Central), 1 (Gradiente vetorial), 2 (Hessiana, autovalores/curvatura principal).
   - Ordens 3 a 8 parametrizadas por $n+1$ amostras direcionais controlando polinômios reconstruídos via inversão pré-condicionada.
2. **Sistema Híbrido WebGL + 2D:**
   - O Shader WebGL calcula pixels e isolinhas (via $df/dx$) em 60 FPS com paletas (Magma, Inferno). Overlay 2D exibe geometria de hit-testing preciso com captura de cursor (`setPointerCapture`).
3. **Brushes e Derivadas Cinemáticas:**
   - Operadores: Add, Sub, Smooth, Snap equipotencial.
   - Avaliação da regra da cadeia: $\frac{d}{dt}f(r(t)) = \nabla f(r(t)) \cdot r'(t)$. O cursor suavizado requer duas trilhas separadas (uma para dados raw cinemáticos, outra para catmull-rom visual).
4. **Modo Sonoro Duplo:** Alternador UI WebAudio processando wavetables de síntese 2D.
5. **HUD Diagnóstico no Cursor:** $x,y$, valor escalar, vetor gradiente $\nabla f$ e classificação morfológica (pico, sela, vale).

---

## 4. Critérios de Aceitação e Testes Automatizados (TDD)
- **Teste 1:** Roundtrip emissor Lean 4 de campos contínuos preservando jatos de Taylor (ordens 0..8).
- **Teste 2:** Preservação de invariantes da Hessiana (Traço $\lambda_1 + \lambda_2$) sob rotação da alça.
- **Teste 3:** Identidade Matemática Cinemática: erro relativo $< 2\%$ entre derivada temporal empírica de $f(r(t))$ e a analítica espacial $\nabla f \cdot r'$.
- **Teste 4:** Sincronização e localização cirúrgica bidirecional perfeita via `view.dispatch`.
- **Teste 5:** Conversão sonora não clipa em transições (Modo A/B).

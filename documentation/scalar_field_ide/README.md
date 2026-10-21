# Scalar Field & Taylor Jets IDE: Editor Unificado de Funções 2D e Brushes

## 1. Visão Geral
O **Scalar Field & Taylor Jets IDE** é um ambiente interativo profissional para definição, exploração, edição e audição de campos escalares contínuos $f(x, y)$ sob representação dual:
1. **Editor de Código Puro (CodeMirror 6):** Exibe a representação textual canônica em **Lean 4** (`ScalarField2D`), com cópia de texto simples com 1 clique (`clipboard`).
2. **Renderizador Híbrido WebGL 2 / 2D:**
   - Camada GPU em WebGL 2 renderizando o campo em 60 FPS com isolinhas antialiased (`fwidth`).
   - Camada Overlay 2D desenhando perfis angulares polares coloridos $q_n(\theta)$ de ordens 0 a 8 e alças interativas arrastáveis com captura de ponteiro.
3. **Brushes Espaciais:**
   - Operadores de deformação contínua (*Add, Sub, Smooth via Laplaciano e Equipotential Snap*).
   - Filtro inercial com amortecimento $p_{t+1} = p_t + \alpha(m_t - p_t)$.
4. **Validação Cinemática da Regra da Cadeia:**
   - Visualizador de perfil 1D $q(t)$ ao longo do traço com verificação empírica vs analítica de $\frac{d}{dt}f(r(t)) = \nabla f \cdot r'$ e $\frac{d^2}{dt^2}f(r(t)) = r'^T H_f r' + \nabla f \cdot r''$.
5. **Síntese Sonora em Modo Duplo:**
   - *Modo A (Wavetable Orbit Scan):* Órbita periódica $w(t) = f(r(t))$ permitindo "ouvir" a geometria e curvatura do campo.
   - *Modo B (Superfície Espectral DDSP):* Modulação aditiva harmônica em tempo real.
   - Exportador WAV de 16-bit 48kHz.

## 2. Estrutura de Arquivos da Implementação
- `src/lib/ScalarFieldStudio.svelte`: Componente principal do estúdio com split redimensionável e controles.
- `src/lib/scalar_field/taylor_jet_math.ts`: Matemática de jatos locais de Taylor (Ordens 0 a 8), autovalores da Hessiana e funções base.
- `src/lib/scalar_field/scalar_field_codec.ts`: Emissor, parser e localizador cirúrgico de limites `[from, to]` para código Lean 4.
- `src/lib/scalar_field/ScalarFieldGlRenderer.ts`: Renderizador WebGL 2 com GLSL 3.0 ES, paletas e isolinhas.
- `src/lib/scalar_field/PolarHandlesOverlay.ts`: Geometria, bijeção tela-mundo e hit-testing de alças polares.
- `src/lib/scalar_field/BrushEngine.ts`: Filtro inercial de ponteiro e operadores de deformação.
- `src/lib/scalar_field/kinematic_chain_analyzer.ts`: Analisador cinemático e verificação diferencial da regra da cadeia.
- `src/lib/scalar_field/KinematicProfileViewer.svelte`: Gráfico de perfil 1D de $f(r(t))$ e velocidade.
- `src/lib/scalar_field/ScalarFieldAudioEngine.ts`: Motor sonoro em modo duplo (Órbita e DDSP Espectral).

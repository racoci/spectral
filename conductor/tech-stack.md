# Tech Stack

## Programming Languages
- **Python 3.10+**: Core scientific research, curriculum laboratory, benchmarking, PyTorch, NumPy, SciPy.
- **Rust / WASM**: Performance-critical DSP engine, Cauchy wavelets, holomorphic transforms (`core-wasm`).
- **TypeScript / Svelte 5**: Interactive visualizer, front-end with Runes reactivity, and semantic widgets.
- **Lean 4 (DSL / AST)**: Formal authoring language for synthesizer graphs, typed structures, and dependent parameter constraints.

## Core Libraries
- **NumPy & SciPy**: Linear algebra, signal processing, filter design, optimization.
- **PyTorch**: Deep learning and invertible neural models (Normalizing Flows).
- **Concurrent.futures**: Multi-threaded / multi-process parallel evaluation.
- **CodeMirror 6**: Extensible code editor core (`@codemirror/view`, `@codemirror/state`) with custom `SemanticBlockWidget` decorations.
- **WebGL 2.0 (GLSL 3.0 ES)**: Real-time high-performance 2D scalar field shader rendering with screen-space derivative antialiased isolines (`fwidth`).
- **HTML5 2D Canvas**: Precision vector overlays for interactive polar handles $q_n(\theta)$ (orders 0 to 8) and kinematic trajectory visualizations.
- **Web Audio API**: Real-time polyphonic 6-operator FM audio engine, wavetable orbit scan synthesis, DDSP spectral surface synthesis, and 16-bit PCM WAV encoding.
- **14-bit rANS Entropy Coder**: High-speed, carry-free Asymmetric Numeral Systems arithmetic engine with geometric Laplacian prior models (Opus 1.5 DRED).
- **RDO Overcomplete Analysis Core**: Lagrangean rate-distortion optimizer with dead-zone quantization and Pareto frontier analysis.
- **Bayesian Ridge Tracker & GP-SDE Core**: 4D Extended Kalman Filter ($f, \dot{f}, \ln A, \dot{A}/A$), Mahalanobis distance metric with chi-squared gating ($\chi^2_2=9.21$), Hungarian optimal bipartite assignment, and Gaussian Process spectral inpainting for occlusions.
- **Learned Ridge Trajectory Codec & Surprise Gate Core**: Orthonormal learned basis dictionary ($\mathbf{\Phi} \in \mathbb{R}^{L \times K}$), uniform innovation quantizer, 14-bit rANS entropy stream packaging, and Bayesian surprise gating ($\Delta G > \lambda$) with adaptive lookahead.
- **PWA & Compact Session Sharing Core**: Stale-While-Revalidate service worker (`sw.js`), standalone manifest (`manifest.webmanifest`), URL-safe Base64 compact state serializer, and standalone scalable vector graphics (`svg_exporter.ts`).
- **Svelte 5 Atomic UI Architecture**: Modular component library (`src/lib/ui/`) with reactive runes, pointer capture rotary knobs, responsive splitters, and WCAG AA contrast compliance.
- **Puppeteer Headless E2E**: Automated multi-resolution visual testing and screenshot auditing across desktop (1440x960) and laptop (1280x720) viewports.
- **Slang & WebGPU Shaders**: Cross-platform shader language with automatic differentiation for higher-order differential jets (`fwd_diff`/`bwd_diff`) and WebGPU compute pipelines with transparent WebGL 2 fallback.
- **TreeNN & Neural-Causal Synthesis Core**: Fully connected coupling matrix $\mathbf{W} \in \mathbb{R}^{N \times N}$, smooth $\tanh$ feedback stabilization, MDL structural edge pruner, and isomorphic real-time Web Audio graph engine.

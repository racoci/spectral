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
- **Slang (Roadmap / GPU Handoff)**: Cross-platform shader language with automatic differentiation for higher-order differential jets and WebGPU compute pipelines.

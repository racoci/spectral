# Módulo: Adaptadores de Fronteira WebAssembly (`webassembly_bindings`)

Este módulo implementa a camada de apresentação e interface FFI (*Foreign Function Interface*) com o JavaScript e a aplicação Web (Svelte/Vite).

---

## 📂 Adaptadores

| Arquivo | Exportações WebAssembly (`#[wasm_bindgen]`) |
| :--- | :--- |
| `spectrogram_generation_bindings.rs` | `wasm_new_architecture_generate_holomorphic_spectrogram`<br>`wasm_new_architecture_generate_short_time_fourier_transform`<br>`wasm_new_architecture_generate_constant_q_transform`<br>`wasm_new_architecture_generate_higher_order_derivatives`<br>`wasm_new_architecture_generate_sliding_differential_jet` |
| `audio_resynthesis_bindings.rs` | `wasm_new_architecture_synthesize_hybrid_spectrogram_to_wav` |

---

## 🛡️ Regra Arquitetural Absoluta

- **Nenhuma lógica matemática ou alocação interna é permitida neste módulo**.
- Todas as funções se limitam a:
  1. Desempacotar fatias de bytes primitivas (`&[u8]`) e parâmetros primitivos (`usize`, `f32`, `&str`).
  2. Construir o objeto de requisição do caso de uso correspondente.
  3. Executar o caso de uso (`UseCase::execute(request)`).
  4. Retornar o vetor de bytes resultante para o JavaScript.

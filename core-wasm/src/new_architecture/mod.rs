//! Nova Arquitetura Limpa e Modular do Motor WebAssembly
//!
//! Separação estrita de responsabilidades:
//! - domain_mathematics: Matemática pura e sem efeitos colaterais
//! - application_use_cases: Orquestração de casos de uso sem acoplamento a bindings
//! - infrastructure: Drivers de FFT, parsers de áudio e gerenciamento de estado
//! - webassembly_bindings: Adaptadores de fronteira FFI estritos

pub mod domain_mathematics;
pub mod infrastructure;
pub mod application_use_cases;
pub mod webassembly_bindings;

pub use webassembly_bindings::*;

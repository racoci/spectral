# Plano de Arquitetura: Audio2Image

## 1. Objetivo
Criar uma aplicação web estática (hospedada no GitHub Pages) capaz de converter arquivos de áudio em imagens bidimensionais e vice-versa de forma **100% reversível e sem perdas (lossless)**. A interface funcionará inicialmente como um visualizador, mas sua arquitetura deve suportar a evolução para um editor de imagens que, na prática, manipula o áudio resultante.

## 2. Decisões Arquiteturais e Tech Stack

Baseado nos requisitos de alta performance, hospedagem estática e precisão matemática bit-a-bit:

*   **Motor de Conversão (Matemática Lossless): Rust + WebAssembly (Wasm)**
    *   **Justificativa:** GPUs podem apresentar não-determinismo matemático (variações de ponto flutuante entre Nvidia/AMD/Apple) que destroem a garantia "bit-perfect". Utilizando Rust compilado para WebAssembly, garantimos execução isolada, veloz e matematicamente exata em qualquer navegador.
    *   **Função:** Ler o buffer do áudio (Float32 ou PCM 16-bit) -> Transformar (ex: Integer Wavelets ou Raw Byte Packing) -> Gerar um Array de Pixels (RGBA). E vice-versa.
*   **Renderização e Edição: HTML5 Canvas / WebGL**
    *   **Justificativa:** Para renderizar espectrogramas enormes ou imagens que representam minutos de áudio, o DOM nativo é lento. WebGL permite renderizar o buffer gerado pelo Rust instantaneamente, dar zoom, aplicar pan e capturar pinceladas/edições na imagem de forma fluida a 60 FPS.
*   **Interface Web (Frontend): Svelte + Vite + TypeScript**
    *   **Justificativa:** O Svelte remove a sobrecarga do Virtual DOM e compila o código para Javascript nativo leve e reativo. Excelente para ferramentas intensivas de CPU. O Vite permite compilação ultrarrápida e integração nativa perfeita com Wasm.
*   **Deploy: GitHub Pages (GitHub Actions)**
    *   **Justificativa:** O pipeline de build criará ativos puramente estáticos (`.html`, `.js`, `.wasm`) que não precisam de backend em tempo de execução, rodando 100% no navegador do cliente (zero custos de servidor).

## 3. Etapas de Implementação

### Fase 1: Setup do Projeto e Infraestrutura Base
1.  Inicializar um monorepo/workspace Svelte + Vite.
2.  Configurar o pacote Rust (`wasm-pack`) dentro do projeto para interagir com o Svelte.
3.  Configurar o fluxo de CI/CD do GitHub Actions para compilar o Rust/Svelte e realizar o deploy estático no `gh-pages`.

### Fase 2: Baseline de Conversão Reversível (Prova de Conceito)
1.  Implementar o algoritmo "Naive Raw Byte Packing" em Rust: ler os bytes do arquivo `.wav` e mapeá-los diretamente num Uint8Array de RGBA de imagem.
2.  Implementar a via reversa (Uint8Array de volta para WAV).
3.  Implementar os testes unitários no Rust garantindo que `decode(encode(audio)) == audio`.
4.  No Svelte, carregar um arquivo de áudio, passar pro Wasm, exibir a imagem resultante via WebGL/Canvas e permitir download do áudio reconstruído.

### Fase 3: Algoritmos Avançados (Visual + Lossless)
1.  Substituir o algoritmo ingênuo por uma **Integer Wavelet Transform (ex: CDF 5/3)** em Rust, mapeando as frequências (Aproximação e Detalhes) no espaço 2D da imagem.
2.  Implementar quantização exata, mapeando os coeficientes da Wavelet em canais de 16-bits.
3.  Testes de estresse da garantia Symmetrical Cross-Validation para assegurar perda zero no domínio do tempo e da frequência.

### Fase 4: O Editor Interativo
1.  Configurar interações no Canvas/WebGL (Zoom/Pan).
2.  Ferramentas básicas de edição ("pincel", borracha, seleções).
3.  Qualquer edição feita no Canvas atualizará o Array de pixels, que será reenviado para o Rust realizar a transformação inversa e tocar o áudio alterado via Web Audio API.

## 4. Validação e Testes
*   **Testes de Identidade:** Todos os pipelines de conversão de áudio para matriz 2D em Rust serão fortemente testados por propriedades e testes de unidade para garantir que a saída invertida seja 100% idêntica aos bytes originais (Symmetrical Handshake).
*   **Compilação Estática:** Validação contínua na CI para verificar se os artefatos Vite rodam em contextos de arquivo estático (`file://` ou `gh-pages` URL path).

# Mitigação de Riscos de Desempenho e Handoff para Compilador Slang na GPU

## 1. Contexto e Motivação
A presente implementação do **Scalar Field IDE 2D** valida com sucesso o modelo de jatos locais de Taylor (Ordens 0 a 8), deformação por brushes com filtragem inercial, verificação cinemática em 1D e síntese sonora em modo duplo (Órbita e DDSP Espectral).

Para manter a renderização a 60 FPS com centenas de deformações ativas ou avançar para campos 3D volumétricos e SDFs com ray-marching e diferenciação automática reversa, é mandatório preparar o pipeline para compilação de shaders via **Slang**.

## 2. Por que Slang?
- **Diferenciação Automática Nativa:** Slang fornece suporte de primeira classe para `fwd_diff` e `bwd_diff` direto em shaders, eliminando a necessidade de calcular derivadas analíticas manualmente em código TypeScript.
- **Portabilidade para WebGPU e SPIR-V:** O compilador Slang pode gerar código WGSL para a Web e SPIR-V para Vulkan/Metal nativo a partir do mesmo código-fonte.
- **Estruturas Genéricas:** Permite parametrizar tensores simétricos de ordem $N$ em tipos uniformes compartilhados.

## 3. Protocolo de Handoff
1. O Document Model em Lean 4 (`ScalarField2D`) e seus tipos em TypeScript (`src/lib/scalar_field/taylor_jet_math.ts`) definem o contrato de dados imutável.
2. O shader WebGL 2 atual (`ScalarFieldGlRenderer.ts`) atua como referência canônica funcional da avaliação do campo.
3. Para a próxima fase de aceleração:
   - Compilar os módulos de campo e operadores de brush em módulos `.slang`.
   - Gerar kernels compute via WebGPU para atualizar grades tridimensionais sem cópia de memória pela CPU.

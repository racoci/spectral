# Shaders Slang com Diferenciação Automática Nativa na GPU (Auto-Diff)

## 1. Visão Geral
Para campos escalares 2D e ordens de Taylor superiores (ordens 0 a 8), o cálculo de derivadas analíticas na CPU e a transferência contínua de matrizes tensoriais para a GPU introduzem gargalos de barramento.

A linguagem de shaders **Slang** resolve este problema permitindo definir a função escalar com o modificador `[Differentiable]` e invocando `fwd_diff` e `bwd_diff` diretamente na GPU para derivar:
1. Gradiente: $\nabla f(x, y) = \left(\frac{\partial f}{\partial x}, \frac{\partial f}{\partial y}\right)$.
2. Hessiana: $H_f(x, y) = \begin{pmatrix} \frac{\partial^2 f}{\partial x^2} & \frac{\partial^2 f}{\partial x \partial y} \\ \frac{\partial^2 f}{\partial y \partial x} & \frac{\partial^2 f}{\partial y^2} \end{pmatrix}$.

## 2. Estrutura Diferenciável em Slang
```slang
struct TaylorJetParams {
    float2 center;
    float scale;
    int order;
    float coeffs[9];
};

[Differentiable]
float evaluateScalarField(float2 p, TaylorJetParams params) {
    float2 delta = (p - params.center) * params.scale;
    float r = length(delta);
    float val = 0.0;
    // Expansão de Taylor de ordens 0 a 8
    for (int n = 0; n <= params.order; ++n) {
        val += params.coeffs[n] * pow(r, float(n));
    }
    return val;
}

[Differentiable]
float2 computeGradient(float2 p, TaylorJetParams params) {
    DifferentialPair<float2> dp = diffPair(p);
    bwd_diff(evaluateScalarField)(dp, params, 1.0);
    return dp.d;
}
```

## 3. Compilação para WGSL e WebGPU
O compilador do Slang gera código **WGSL** com kernels de renderização contínua e computação sem perdas de precisão em ponto flutuante de 32 bits (fp32).

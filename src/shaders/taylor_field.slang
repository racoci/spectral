// Slang Shader Module for Scalar Field 2D & Higher-Order Taylor Jets
module TaylorField;

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
    // Expansão analítica de Taylor ordens 0 a 8
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

struct HessianMatrix {
    float2 row0;
    float2 row1;
};

[Differentiable]
HessianMatrix computeHessian(float2 p, TaylorJetParams params) {
    // Avaliação automática da segunda derivada via fwd_diff sobre o gradiente
    DifferentialPair<float2> dp_x = diffPair(p, float2(1.0, 0.0));
    DifferentialPair<float2> dp_y = diffPair(p, float2(0.0, 1.0));
    float2 d_grad_x = fwd_diff(computeGradient)(dp_x, params).d;
    float2 d_grad_y = fwd_diff(computeGradient)(dp_y, params).d;

    HessianMatrix H;
    H.row0 = d_grad_x;
    H.row1 = d_grad_y;
    return H;
}

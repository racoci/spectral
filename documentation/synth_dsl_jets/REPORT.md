# Synthesizer parameter DSL and analytic jet benchmark

A symbolic named DAG plus a PyTorch-autodifferentiable Taylor-jet engine was implemented.

The DSL contains explicit parameters for pitch, detune, vibrato, tremolo, ADSR, duration, velocity, inharmonicity, harmonic amplitudes/phases and a log-frequency Gaussian-RBF spectral envelope. Arpeggiated note events are generated separately and preserved as ground truth.

Jet order: 4
Representative note t0: 0.530000000 s
Jet outputs: ['f0', 'phase', 'envelope', 'logA_1', 'logA_2', 'logA_3', 'logA_4']
Jacobian shape: [35, 31]
Full Jacobian finite-difference check: max abs 5.384412e-03, mean abs 4.541884e-05
Maximum independent SymPy jet verification error: 1.455192e-11

The jet engine stores coefficient k as f^(k)(t0)/k!, so multiplying by k! gives the ordinary derivative. It propagates derivatives analytically through addition, multiplication, division, exp, log, sin, cos, sqrt and erf.

The parameter Jacobian is obtained by PyTorch reverse/forward automatic differentiation through the analytic jet algebra. Thus no finite-difference time derivatives are used.

Nonlocal effects such as delay, chorus and reverb are retained as synthesis-graph operations but are not represented as local scalar parameter jets.

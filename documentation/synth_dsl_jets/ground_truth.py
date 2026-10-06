from __future__ import annotations

from dataclasses import asdict
from pathlib import Path
import json, math, sys
import numpy as np
import torch
import sympy as sp
from scipy.io import wavfile

sys.path.insert(0, str(Path(__file__).resolve().parent))
from synthdsl import Note, Timbre, Arpeggiator, build_note_dsl, render_demo
from taylor_jet import Jet, jet_derivatives


def note_jet_numeric(note: Note, timbre: Timbre, t0: float, order: int, theta: torch.Tensor):
    """Evaluate the same analytic graph as the SymPy DSL using Taylor jets."""
    # theta order is defined by PARAM_NAMES below.
    p = {k: theta[i] for i, k in enumerate(PARAM_NAMES)}
    tau = Jet.variable(float(t0 - note.onset), order)
    # scalar parameters as constant jets
    def C(name): return Jet.constant(p[name], order)

    f_static = C('f_nom') * (C('detune_ratio'))
    q = C('vibrato_depth_rad')
    vr = C('vibrato_rate')
    vp = C('vibrato_phase')
    vib_arg = (tau * (2*math.pi*vr) + vp)
    f0 = f_static * (1 + q * vib_arg.sin())
    phase = C('phase0') + (2*math.pi) * f_static * tau + (f_static*q/vr) * (vp.cos() - vib_arg.cos())

    attack=C('attack'); decay=C('decay'); sustain=C('sustain'); release=C('release'); duration=C('duration')
    sharp = 4.0 / math.sqrt(2.0)
    ka=(tau/attack*sharp).erf(); kd=((tau-attack)/decay*sharp).erf(); kr=((tau-duration)/release*sharp).erf()
    smooth=lambda x: (x+1)*0.5
    env_base = smooth(ka) - (1-sustain)*smooth(kd) - sustain*smooth(kr)
    trem = 1 + C('tremolo_depth') * (tau*(2*math.pi*C('tremolo_rate')) + C('tremolo_phase')).sin()
    env = env_base * trem

    outputs = {
        'f0': f0,
        'phase': phase,
        'envelope': env,
    }
    centers = np.log2(np.asarray(timbre.spectral_x_hz, float))
    sigma = float(timbre.spectral_sigma_oct)
    S = [C(f'S_{j}') for j in range(len(centers))]
    for k, _h in enumerate(timbre.harmonic_amplitudes[:4], 1):
        fk = f0 * math.sqrt(1.0 + 0.0) * (k * (Jet.constant(1.0,order))) * (1 + C('inharmonicity')*(k*k)).sqrt()
        u = fk.log() / math.log(2.0)
        weights=[((u-c)/sigma * -1.0).pow(2) for c in centers]  # exp(-z^2)
        weights=[(w*0.5).neg().exp() if False else None for w in weights]
        # rebuild cleanly: exp(-0.5*((u-c)/sigma)^2)
        weights=[]
        for c in centers:
            z=(u-c)/sigma
            weights.append((-0.5*(z*z)).exp())
        den=weights[0]
        num=S[0]*weights[0]
        for j in range(1,len(weights)):
            den=den+weights[j]; num=num+S[j]*weights[j]
        Sk=num/den
        logA=C('velocity').log() + env.log() + C(f'H_{k}').log() + (math.log(10)/20.0)*Sk
        outputs[f'logA_{k}']=logA
        outputs[f'f_{k}']=fk
        outputs[f'phase_{k}']=k*phase + C(f'psi_{k}')
    return {name: jet_derivatives(j) for name,j in outputs.items()}


PARAM_NAMES = [
    'f_nom','detune_ratio','vibrato_depth_rad','vibrato_rate','vibrato_phase',
    'tremolo_depth','tremolo_rate','tremolo_phase','attack','decay','sustain',
    'release','duration','velocity','phase0','inharmonicity',
    'H_1','H_2','H_3','H_4','psi_1','psi_2','psi_3','psi_4','S_0','S_1','S_2','S_3','S_4','S_5','S_6'
]


def build_parameter_vector(note: Note, timbre: Timbre):
    vals = [
        440*2**((note.midi-69)/12),
        2**(note.detune_cents/1200),
        math.log(2)/1200*note.vibrato_depth_cents,
        note.vibrato_rate,
        note.vibrato_phase,
        note.tremolo_depth,
        note.tremolo_rate,
        note.tremolo_phase,
        note.attack,
        note.decay,
        note.sustain,
        note.release,
        note.duration,
        note.velocity,
        0.0,
        timbre.inharmonicity,
    ]
    vals += list(timbre.harmonic_amplitudes[:4])
    vals += list(timbre.harmonic_phases[:4])
    vals += list(timbre.spectral_db[:7])
    return torch.tensor(vals[:len(PARAM_NAMES)],dtype=torch.float64,requires_grad=True)


def jacobian_of_jet(note,timbre,t0,order,output_names):
    base=build_parameter_vector(note,timbre)
    def fn(theta):
        y=note_jet_numeric(note,timbre,t0,order,theta)
        return torch.cat([y[name] for name in output_names])
    y=fn(base)
    J=torch.autograd.functional.jacobian(fn,base,create_graph=False,vectorize=True)
    return y.detach().numpy(),J.detach().numpy()


def main(outdir='/mnt/data/synth_dsl_jets'):
    out=Path(outdir); out.mkdir(parents=True,exist_ok=True)
    audio,timbre,arp_notes,voice2=render_demo(sr=12000,duration=4.0,seed=7)
    wavfile.write(out/'demo.wav',12000,(np.clip(audio,-1,1)*32767).astype(np.int16))

    rep=arp_notes[3]
    dsl,_,_=build_note_dsl(rep,timbre)
    dsl.export_json(out/'demo_dsl.json')

    t0=rep.onset+0.17
    order=4
    output_names=['f0','phase','envelope','logA_1','logA_2','logA_3','logA_4']
    y,J=jacobian_of_jet(rep,timbre,t0,order,output_names)

    # Evaluate jets over a short interval for plotting/downstream CQT comparison.
    tt=np.linspace(t0-0.05,t0+0.25,120)
    theta=build_parameter_vector(rep,timbre)
    series={}
    for t in tt:
        ys=note_jet_numeric(rep,timbre,float(t),order,theta)
        for name in output_names:
            series.setdefault(name,[]).append(ys[name].detach().numpy())
    np.savez_compressed(out/'analytic_jets.npz',t=tt,**{k:np.stack(v) for k,v in series.items()})
    np.savez_compressed(out/'jet_jacobian.npz',values=y,jacobian=J)

    # Optional first-order analytic check against direct SymPy differentiation.
    sym_out=['f0','phase','envelope']
    sym_wrt=['f_nom','detune_cents','vibrato_depth_cents','vibrato_rate','vibrato_phase','tremolo_depth','tremolo_rate','attack','decay','sustain','release','duration','velocity','inharmonicity']
    # The compact Taylor graph uses detune_ratio and vibrato_depth_rad; verify equivalent low-order values separately.
    rows=[]
    for i,name in enumerate(output_names):
        block=y[i*(order+1):(i+1)*(order+1)]
        rows.append({
            'output':name,
            'value':float(block[0]),
            'd1':float(block[1]),
            'd2':float(block[2]),
            'd3':float(block[3]),
            'd4':float(block[4]),
        })
    import csv
    with open(out/'jet_values.csv','w',newline='',encoding='utf-8') as f:
        w=csv.DictWriter(f,fieldnames=rows[0].keys()); w.writeheader(); w.writerows(rows)

    with open(out/'ground_truth.json','w',encoding='utf-8') as f:
        json.dump({
            'sample_rate':12000,
            'duration':4.0,
            'timbre':asdict(timbre),
            'arpeggio_notes':[asdict(n) for n in arp_notes],
            'voice2_notes':[asdict(n) for n in voice2],
            'representative_note':asdict(rep),
            'jet_order':order,
            'jet_outputs':output_names,
            'jacobian_parameter_names':PARAM_NAMES,
            't0':t0,
            'jacobian_shape':list(J.shape),
            'jacobian_max_abs':float(np.max(np.abs(J))),
        },f,indent=2)

    # Independent symbolic check for the low-dimensional local fields.
    checks=[]
    sympy_targets=['f0','phase','envelope']
    sub={dsl.params[k]: dsl.values[k] for k in dsl.params}; sub[dsl.t]=t0
    theta0=build_parameter_vector(rep,timbre)
    local=note_jet_numeric(rep,timbre,t0,order,theta0)
    for name in sympy_targets:
        e=dsl.expanded(name)
        for k in range(order+1):
            val=float(sp.N(sp.diff(e,dsl.t,k).subs(sub)))
            jval=float(local[name][k].detach())
            checks.append({'output':name,'order':k,'sympy':val,'taylor_jet':jval,'abs_error':abs(val-jval)})
    import csv
    with open(out/'jet_symbolic_verification.csv','w',newline='',encoding='utf-8') as f:
        w=csv.DictWriter(f,fieldnames=checks[0].keys()); w.writeheader(); w.writerows(checks)
    max_sym_err=max(q['abs_error'] for q in checks)

    # Independent finite-difference check of the full parameter Jacobian.
    base_np=theta0.detach().numpy()
    def fn_np(theta_np):
        tt=torch.tensor(theta_np,dtype=torch.float64,requires_grad=True)
        yy=note_jet_numeric(rep,timbre,t0,order,tt)
        return np.concatenate([yy[n].detach().numpy() for n in output_names])
    fd=np.zeros_like(J)
    for col in range(base_np.size):
        step=1e-6*max(1.0,abs(base_np[col]))
        vp=base_np.copy(); vm=base_np.copy(); vp[col]+=step; vm[col]-=step
        fd[:,col]=(fn_np(vp)-fn_np(vm))/(2*step)
    jac_fd_max=float(np.max(np.abs(J-fd)))
    jac_fd_mean=float(np.mean(np.abs(J-fd)))
    np.savez_compressed(out/'jacobian_fd_check.npz',analytic=J,finite_difference=fd)
    (out/'jacobian_fd_report.txt').write_text(
        f'max_abs_error={jac_fd_max:.9e}\nmean_abs_error={jac_fd_mean:.9e}\n',encoding='utf-8')

    # Graph summary.
    graph={
        'nodes':{
            'f_static':'f_nom * detune_ratio',
            'f0':'f_static * (1 + vibrato_depth_rad * sin(2*pi*vibrato_rate*tau + vibrato_phase))',
            'phase':'phase0 + 2*pi*f_static*tau + f_static*vibrato_depth_rad/vibrato_rate * (cos(vibrato_phase) - cos(2*pi*vibrato_rate*tau + vibrato_phase))',
            'envelope':'Gaussian-CDF attack/decay/release transitions multiplied by (1 + tremolo)',
            'harmonic_frequency':'k * f0 * sqrt(1 + inharmonicity*k^2)',
            'spectral_envelope':'Gaussian-RBF interpolation of S_j in log2(f)',
            'logA_k':'log(velocity) + log(envelope) + log(H_k) + ln(10)/20 * S(f_k)',
            'harmonic_phase':'k * phase + psi_k',
            'audio_note':'sum_k exp(logA_k) * sin(harmonic_phase_k) + masked/fractal noise',
        },
        'edges':[
            ['vibrato','f0'],['f0','phase'],['phase','harmonic_phase'],['f0','harmonic_frequency'],
            ['harmonic_frequency','spectral_envelope'],['spectral_envelope','logA_k'],['envelope','logA_k'],
            ['arpeggiator','note_events'],['note_events','f_nom'],['tremolo','envelope'],
        ],
        'nonlocal_effects':['delay','chorus','reverb']
    }
    (out/'graph_spec.json').write_text(json.dumps(graph,indent=2),encoding='utf-8')

    report=f'''# Synthesizer parameter DSL and analytic jet benchmark\n\nA symbolic named DAG plus a PyTorch-autodifferentiable Taylor-jet engine was implemented.\n\nThe DSL contains explicit parameters for pitch, detune, vibrato, tremolo, ADSR, duration, velocity, inharmonicity, harmonic amplitudes/phases and a log-frequency Gaussian-RBF spectral envelope. Arpeggiated note events are generated separately and preserved as ground truth.\n\nJet order: {order}\nRepresentative note t0: {t0:.9f} s\nJet outputs: {output_names}\nJacobian shape: {list(J.shape)}\nFull Jacobian finite-difference check: max abs {jac_fd_max:.6e}, mean abs {jac_fd_mean:.6e}\nMaximum independent SymPy jet verification error: {max_sym_err:.6e}\n\nThe jet engine stores coefficient k as f^(k)(t0)/k!, so multiplying by k! gives the ordinary derivative. It propagates derivatives analytically through addition, multiplication, division, exp, log, sin, cos, sqrt and erf.\n\nThe parameter Jacobian is obtained by PyTorch reverse/forward automatic differentiation through the analytic jet algebra. Thus no finite-difference time derivatives are used.\n\nNonlocal effects such as delay, chorus and reverb are retained as synthesis-graph operations but are not represented as local scalar parameter jets.\n'''
    (out/'REPORT.md').write_text(report,encoding='utf-8')

if __name__=='__main__':
    main()

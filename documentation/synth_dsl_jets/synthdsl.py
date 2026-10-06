from __future__ import annotations

from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Dict, List, Sequence, Optional, Any, Tuple
import json
import math
import numpy as np
import sympy as sp
from scipy import signal
from scipy.io import wavfile

Array = np.ndarray


# =============================================================================
# Symbolic DAG DSL
# =============================================================================

class SynthDSL:
    """Named symbolic DAG for audio-parameter trajectories.

    Expressions are kept as named nodes rather than recursively substituted.
    Jets and Jacobians are generated only for requested outputs.
    """
    def __init__(self):
        self.t = sp.Symbol('t', real=True)
        self.params: Dict[str, sp.Symbol] = {}
        self.values: Dict[str, float] = {}
        self.expr: Dict[str, sp.Expr] = {}
        self.meta: Dict[str, Dict[str, Any]] = {}

    def parameter(self, name: str, value: float, unit: str = '', learnable: bool = True):
        s = sp.Symbol(name, real=True)
        self.params[name] = s
        self.values[name] = float(value)
        self.meta[name] = {'unit': unit, 'learnable': learnable, 'kind': 'parameter'}
        return s

    def derived(self, name: str, expression: sp.Expr, unit: str = '', kind: str = 'derived'):
        self.expr[name] = sp.sympify(expression)
        self.meta[name] = {'unit': unit, 'kind': kind}
        return self.expr[name]

    def get(self, name: str | sp.Expr):
        if isinstance(name, sp.Expr):
            return name
        if name in self.expr:
            return self.expr[name]
        if name in self.params:
            return self.params[name]
        raise KeyError(name)

    def expanded(self, name: str | sp.Expr) -> sp.Expr:
        """Recursively expand named derived nodes only when requested."""
        e = self.get(name)
        repl = {self.params[k]: self.values[k] for k in []}  # no-op; symbolic stays symbolic
        changed = True
        while changed:
            changed = False
            atoms = list(e.atoms(sp.Symbol))
            for a in atoms:
                n = a.name
                if n in self.expr:
                    e2 = e.xreplace({a: self.expr[n]})
                    if e2 != e:
                        e = e2
                        changed = True
        return e

    def numeric_subs(self, overrides: Optional[Dict[str, float]] = None):
        d = {self.params[k]: v for k, v in self.values.items()}
        if overrides:
            d.update({self.params[k]: float(v) for k, v in overrides.items() if k in self.params})
        return d

    def time_jet(self, name: str, order: int, expand: bool = True) -> List[sp.Expr]:
        e = self.expanded(name) if expand else self.get(name)
        return [sp.diff(e, self.t, k) for k in range(order + 1)]

    def jacobian(self, outputs: Sequence[str], wrt: Optional[Sequence[str]] = None, expand: bool = True):
        es = [self.expanded(o) if expand else self.get(o) for o in outputs]
        if wrt is None:
            names = sorted({s.name for e in es for s in e.free_symbols if s != self.t})
            # Only registered parameters are eligible by default.
            wrt = [n for n in names if n in self.params]
        vs = [self.params[n] for n in wrt]
        return sp.Matrix(es).jacobian(vs), list(wrt)

    def jet_jacobian(self, outputs: Sequence[str], time_order: int, wrt: Optional[Sequence[str]] = None, expand: bool = True):
        es = [self.expanded(o) if expand else self.get(o) for o in outputs]
        if wrt is None:
            names = sorted({s.name for e in es for s in e.free_symbols if s != self.t})
            wrt = [n for n in names if n in self.params]
        vs = [self.params[n] for n in wrt]
        rows = []
        for e in es:
            for k in range(time_order + 1):
                de = sp.diff(e, self.t, k)
                rows.append([sp.diff(de, v) for v in vs])
        return sp.Matrix(rows), list(wrt)

    def evaluate(self, expression: str | sp.Expr, t: Array | float, overrides: Optional[Dict[str, float]] = None):
        e = self.expanded(expression)
        fn = sp.lambdify([self.t, *self.params.values()], e, modules=['numpy'])
        vals = [self.values[k] if overrides is None or k not in overrides else overrides[k] for k in self.params]
        return np.asarray(fn(t, *vals), dtype=float)

    def export_json(self, path: str | Path):
        p = Path(path)
        payload = {
            'time_symbol': str(self.t),
            'parameters': {
                k: {'symbol': str(v), 'value': self.values[k], **self.meta.get(k, {})}
                for k, v in self.params.items()
            },
            'derived': {
                k: {'expression': str(v), **self.meta.get(k, {})}
                for k, v in self.expr.items()
            }
        }
        p.write_text(json.dumps(payload, indent=2), encoding='utf-8')


# =============================================================================
# Mathematical primitives
# =============================================================================

def midi_to_hz(m):
    return sp.Float(440) * sp.Pow(2, (sp.sympify(m) - 69) / 12)


def cents_ratio(cents):
    return sp.exp(sp.log(2) * sp.sympify(cents) / 1200)


def lfo_sine(t, freq, phase=0):
    return sp.sin(2 * sp.pi * freq * t + phase)


def gaussian_rbf(x, centers, values, sigma):
    weights = [sp.exp(-((x-c)**2) / (2*sigma**2)) for c in centers]
    den = sum(weights)
    return sum(v*w for v, w in zip(values, weights)) / den


def gaussian_step(u, sharpness=4.0):
    """Smooth step obtained by integrating a Gaussian density."""
    return sp.Rational(1, 2) * (1 + sp.erf(sharpness * u / sp.sqrt(2)))


def smooth_adsr(tau, attack, decay, sustain, release, duration):
    """C-infinity Gaussian-smoothed ADSR.

    The envelope is represented as a sum of three Gaussian-CDF transitions:
      attack: +1
      decay:  -(1-sustain)
      release: -sustain
    This is differentiable to arbitrary order and therefore well suited to jets.
    """
    ka = gaussian_step(tau / attack, 4.0)
    kd = gaussian_step((tau - attack) / decay, 4.0)
    kr = gaussian_step((tau - duration) / release, 4.0)
    return ka - (1 - sustain) * kd - sustain * kr


# =============================================================================
# Events and synthesis configuration
# =============================================================================

@dataclass
class Note:
    onset: float
    duration: float
    midi: float
    velocity: float = 1.0
    attack: float = 0.015
    decay: float = 0.12
    sustain: float = 0.72
    release: float = 0.18
    detune_cents: float = 0.0
    pitch_bend_cents: float = 0.0
    vibrato_depth_cents: float = 0.0
    vibrato_rate: float = 5.0
    vibrato_phase: float = 0.0
    tremolo_depth: float = 0.0
    tremolo_rate: float = 4.0
    tremolo_phase: float = 0.0


@dataclass
class Timbre:
    harmonic_amplitudes: List[float]
    harmonic_phases: List[float]
    inharmonicity: float
    spectral_x_hz: List[float]
    spectral_db: List[float]
    spectral_sigma_oct: float = 0.30


@dataclass
class ArpStep:
    index: int
    octave: int = 0
    velocity: float = 1.0
    gate: float = 0.9
    transpose: int = 0
    probability: float = 1.0
    ratchet: int = 1


@dataclass
class Arpeggiator:
    notes: List[int]
    rate: float
    direction: str = 'up_down'
    octaves: int = 2
    gate: float = 0.8
    swing: float = 0.0
    seed: int = 0
    steps: Optional[List[ArpStep]] = None

    def events(self, start: float, end: float, base_midi: int, velocity: float = 1.0):
        rng = np.random.default_rng(self.seed)
        seq = [s.index for s in self.steps] if self.steps else list(range(len(self.notes)))
        if not seq:
            return []
        if self.direction == 'down':
            seq = seq[::-1]
        elif self.direction == 'up_down':
            seq = seq + seq[-2:0:-1] if len(seq) > 2 else seq + seq[::-1]
        elif self.direction == 'down_up':
            q = seq[::-1]
            seq = q + q[-2:0:-1] if len(q) > 2 else q + q[::-1]
        out = []
        t = start; i = 0
        while t < end - 1e-12:
            idx = seq[i % len(seq)]
            step = self.steps[i % len(self.steps)] if self.steps else None
            dt = self.rate * (1 + self.swing if i % 2 else 1 - self.swing)
            octave = (i // len(seq)) % max(self.octaves, 1)
            if self.direction in ('down', 'down_up'):
                octave = self.octaves - 1 - octave
            prob = 1.0 if step is None else step.probability
            gate = self.gate if step is None else step.gate
            vel = velocity if step is None else velocity * step.velocity
            transpose = 0 if step is None else step.transpose
            ratchet = 1 if step is None else max(1, step.ratchet)
            if rng.random() <= prob:
                sub = dt / ratchet
                for r in range(ratchet):
                    on = t + r*sub
                    off = min(t + gate*dt, end)
                    if on < end:
                        out.append(Note(on, max(off-on, 1e-4), base_midi+self.notes[idx]+12*octave+transpose, vel))
            t += dt; i += 1
        return out


# =============================================================================
# DSL construction
# =============================================================================

def build_note_dsl(note: Note, timbre: Timbre) -> Tuple[SynthDSL, List[str], List[str]]:
    d = SynthDSL()
    t = d.t
    tau = t - note.onset

    # Free parameters of the note.
    d.parameter('f_nom', float(440*2**((note.midi-69)/12)), 'Hz')
    d.parameter('detune_cents', note.detune_cents, 'cents')
    d.parameter('pitch_bend_cents', note.pitch_bend_cents, 'cents')
    d.parameter('vibrato_depth_cents', note.vibrato_depth_cents, 'cents')
    d.parameter('vibrato_rate', note.vibrato_rate, 'Hz')
    d.parameter('vibrato_phase', note.vibrato_phase, 'rad')
    d.parameter('tremolo_depth', note.tremolo_depth, 'linear')
    d.parameter('tremolo_rate', note.tremolo_rate, 'Hz')
    d.parameter('tremolo_phase', note.tremolo_phase, 'rad')
    d.parameter('attack', note.attack, 's')
    d.parameter('decay', note.decay, 's')
    d.parameter('sustain', note.sustain, 'linear')
    d.parameter('release', note.release, 's')
    d.parameter('duration', note.duration, 's')
    d.parameter('velocity', note.velocity, 'linear')
    d.parameter('phase0', 0.0, 'rad')
    d.parameter('inharmonicity', timbre.inharmonicity, '1')

    # Harmonic and spectral-envelope control parameters are explicit learnable DOF.
    for k, h in enumerate(timbre.harmonic_amplitudes, 1):
        d.parameter(f'H_{k}', h, 'relative harmonic amplitude')
        phase = timbre.harmonic_phases[k-1] if k-1 < len(timbre.harmonic_phases) else 0.0
        d.parameter(f'psi_{k}', phase, 'rad')
    for j, db in enumerate(timbre.spectral_db):
        d.parameter(f'S_{j}', db, 'dB')

    # Pitch trajectory. Vibrato is represented as a fractional pitch-ratio modulation.
    static_ratio = cents_ratio(d.params['detune_cents'] + d.params['pitch_bend_cents'])
    vib = sp.Float(math.log(2)/1200) * d.params['vibrato_depth_cents']
    vib_phase = d.params['vibrato_phase']
    vib_rate = d.params['vibrato_rate']
    # Small-vibrato exponential approximation: f = f_static * (1 + q sin(...)).
    f_static = d.params['f_nom'] * static_ratio
    d.derived('f0', f_static * (1 + vib * sp.sin(2*sp.pi*vib_rate*tau + vib_phase)), 'Hz', 'pitch')

    # Analytically integrable phase for the small-vibrato approximation.
    phase_static = d.params['phase0'] + 2*sp.pi*f_static*tau
    phase_vib = (f_static * vib / vib_rate) * (sp.cos(vib_phase) - sp.cos(2*sp.pi*vib_rate*tau + vib_phase))
    d.derived('phase', phase_static + phase_vib, 'rad', 'phase')

    env = smooth_adsr(tau, d.params['attack'], d.params['decay'], d.params['sustain'], d.params['release'], d.params['duration'])
    trem = 1 + d.params['tremolo_depth'] * sp.sin(2*sp.pi*d.params['tremolo_rate']*tau + d.params['tremolo_phase'])
    d.derived('envelope', env * trem, 'linear', 'envelope')

    centers = [math.log2(f) for f in timbre.spectral_x_hz]
    u_sigma = float(timbre.spectral_sigma_oct)
    S_symbols = [d.params[f'S_{j}'] for j in range(len(centers))]

    outputs = ['f0', 'phase', 'envelope']
    for k in range(1, len(timbre.harmonic_amplitudes)+1):
        fk = k * d.expr['f0'] * sp.sqrt(1 + d.params['inharmonicity'] * k*k)
        u = sp.log(fk, 2)
        Sk = gaussian_rbf(u, centers, S_symbols, u_sigma)
        Ek = sp.exp(sp.log(10) * Sk / 20)
        Ak = d.params['velocity'] * d.expr['envelope'] * d.params[f'H_{k}'] * Ek
        phik = k*d.expr['phase'] + d.params[f'psi_{k}']
        d.derived(f'f_{k}', fk, 'Hz', 'harmonic_frequency')
        d.derived(f'S_{k}_interp', Sk, 'dB', 'spectral_envelope')
        d.derived(f'E_{k}', Ek, 'linear', 'spectral_envelope_gain')
        logAk = sp.log(d.params['velocity']) + sp.log(d.expr['envelope']) + sp.log(d.params[f'H_{k}']) + sp.log(10)*Sk/20
        d.derived(f'A_{k}', Ak, 'linear', 'harmonic_amplitude')
        d.derived(f'logA_{k}', logAk, 'natural log amplitude', 'harmonic_log_amplitude')
        d.derived(f'phase_{k}', phik, 'rad', 'harmonic_phase')
        outputs += [f'f_{k}', f'logA_{k}', f'phase_{k}']
    free = list(d.params.keys())
    return d, outputs, free


# =============================================================================
# Numerical audio renderer (kept separate from symbolic ground truth)
# =============================================================================

def fractal_noise(n, sr, alpha, knee, rng):
    w = rng.standard_normal(n)
    X = np.fft.rfft(w)
    f = np.fft.rfftfreq(n, 1/sr)
    H = np.maximum(f, 1.0)**(alpha/2)
    H /= np.sqrt(1 + (f/max(knee, 1.0))**2)
    X *= H
    y = np.fft.irfft(X, n)
    return y/(np.std(y)+1e-12)


def gaussian_env_num(u, points, sigma):
    p = np.asarray(points, float)
    xs = np.array([q[0] for q in p]); ys = np.array([q[1] for q in p])
    W = np.exp(-.5*((u[:,None]-xs[None,:])/sigma)**2)
    return (W@ys)/np.maximum(W.sum(axis=1),1e-12)


def adsr_num(tau, note: Note):
    a=max(note.attack,1e-6); d=max(note.decay,1e-6); r=max(note.release,1e-6)
    y=np.zeros_like(tau)
    ua=np.clip(tau/a,0,1); y[tau<a]=3*ua[tau<a]**2-2*ua[tau<a]**3
    ud=np.clip((tau-a)/d,0,1); ii=(tau>=a)&(tau<a+d); y[ii]=1+(note.sustain-1)*(3*ud[ii]**2-2*ud[ii]**3)
    ii=(tau>=a+d)&(tau<note.duration); y[ii]=note.sustain
    ur=np.clip((tau-note.duration)/r,0,1); ii=tau>=note.duration; y[ii]=note.sustain*(1-(3*ur[ii]**2-2*ur[ii]**3))
    return y


def render_note(note: Note, timbre: Timbre, sr: int, duration: float, rng: np.random.Generator):
    n=int(duration*sr); t=np.arange(n)/sr; tau=t-note.onset
    f_static=440*2**((note.midi-69)/12)*2**((note.detune_cents+note.pitch_bend_cents)/1200)
    q=math.log(2)/1200*note.vibrato_depth_cents
    f=f_static*(1+q*np.sin(2*np.pi*note.vibrato_rate*tau+note.vibrato_phase))
    phase=2*np.pi*f_static*tau + f_static*q/max(note.vibrato_rate,1e-6)*(np.cos(note.vibrato_phase)-np.cos(2*np.pi*note.vibrato_rate*tau+note.vibrato_phase))
    env=adsr_num(tau,note)*(1+note.tremolo_depth*np.sin(2*np.pi*note.tremolo_rate*tau+note.tremolo_phase))
    out=np.zeros(n)
    centers=np.log2(np.asarray(timbre.spectral_x_hz))
    uu_sigma=timbre.spectral_sigma_oct
    for k,h in enumerate(timbre.harmonic_amplitudes,1):
        fk=k*f*np.sqrt(1+timbre.inharmonicity*k*k)
        S=gaussian_env_num(np.log2(np.maximum(fk,1e-6)), list(zip(centers,timbre.spectral_db)), uu_sigma)
        E=10**(S/20)
        psi=timbre.harmonic_phases[k-1] if k-1<len(timbre.harmonic_phases) else 0
        out += note.velocity*env*h*E*np.sin(k*phase+psi)
    return out


def render_demo(sr=12000, duration=4.0, seed=7):
    timbre=Timbre(
        harmonic_amplitudes=[1,.63,.34,.18,.09,.045,.02,.01],
        harmonic_phases=[0,.2,-.15,.4,-.1,.05,.3,-.25],
        inharmonicity=1.2e-4,
        spectral_x_hz=[70,180,400,900,1800,3600,7200],
        spectral_db=[2,2,1,-1,-4,-9,-15],
        spectral_sigma_oct=.30,
    )
    arp=Arpeggiator([0,4,7,11],1/8,'up_down',2,.82,.12,seed)
    notes=arp.events(0,duration,57,.85)
    voice2=[Note(0,1,48,.62,.03,.18,.65,.25,vibrato_depth_cents=8,vibrato_rate=4.8),
            Note(1,1,50,.58,.05,.18,.62,.22),
            Note(2,1,52,.65,.025,.12,.70,.25),
            Note(3,.9,55,.60,.04,.15,.68,.30)]
    rng=np.random.default_rng(seed)
    x=np.zeros(int(duration*sr))
    for note in notes: x += render_note(note,timbre,sr,duration,rng)
    for note in voice2: x += .6*render_note(note,timbre,sr,duration,rng)
    # deterministic fractal noise mixture
    x = .96*x + .04*fractal_noise(len(x),sr,-.65,2200,rng)
    x = np.tanh(1.4*x)/np.tanh(1.4)
    x /= max(np.max(np.abs(x)),1e-9)
    return x,timbre,notes,voice2


def write_wav(path,x,sr):
    y=np.clip(x,-1,1)
    wavfile.write(path,sr,(y*32767).astype(np.int16))


def finite_difference_jacobian(dsl: SynthDSL, outputs: Sequence[str], wrt: Sequence[str], t0: float, eps: float=1e-6):
    vals=dsl.values.copy()
    exprs=[dsl.expanded(o) for o in outputs]
    def evaluate(vs):
        sub={dsl.params[k]:vs[k] for k in dsl.params}
        sub[dsl.t]=t0
        return np.array([float(sp.N(e.subs(sub))) for e in exprs])
    J=np.zeros((len(outputs),len(wrt)))
    for j,n in enumerate(wrt):
        vp=vals.copy(); vm=vals.copy(); vp[n]+=eps; vm[n]-=eps
        J[:,j]=(evaluate(vp)-evaluate(vm))/(2*eps)
    return J


def verify_jacobian(dsl: SynthDSL, outputs: Sequence[str], wrt: Sequence[str], t0: float):
    Jsym,_=dsl.jacobian(outputs,wrt)
    sub=dsl.numeric_subs(); sub[dsl.t]=t0
    Jexact=np.array([[float(sp.N(Jsym[i,j].subs(sub))) for j in range(Jsym.cols)] for i in range(Jsym.rows)])
    Jfd=finite_difference_jacobian(dsl,outputs,wrt,t0)
    return Jexact,Jfd,float(np.max(np.abs(Jexact-Jfd)))


def save_demo(outdir='/mnt/data/synth_dsl_jets'):
    out=Path(outdir); out.mkdir(parents=True,exist_ok=True)
    audio,timbre,arp_notes,voice2=render_demo()
    write_wav(out/'demo.wav',audio,12000)
    representative=arp_notes[3]
    dsl,outputs,free=build_note_dsl(representative,timbre)
    dsl.export_json(out/'demo_dsl.json')

    # Keep the analytical benchmark modest enough to run while retaining the full parameter graph.
    jet_outputs=['f0','phase','envelope','logA_1','logA_2','logA_3','logA_4']
    jet_order=4
    jets={name:[str(e) for e in dsl.time_jet(name,jet_order)] for name in jet_outputs}

    # Evaluate Jacobian at a regular interior point to avoid Piecewise boundary ambiguity.
    t0=representative.onset+0.17
    # Selected parameter set focused on the physically informative branch.
    wrt=['f_nom','detune_cents','pitch_bend_cents','vibrato_depth_cents','vibrato_rate','tremolo_depth','tremolo_rate','attack','decay','sustain','velocity','inharmonicity','H_1','H_2','H_3','H_4','S_0','S_1','S_2','S_3']
    Jexact,Jfd,Jerr=verify_jacobian(dsl,jet_outputs,wrt,t0)

    tt=np.linspace(representative.onset+0.05,representative.onset+0.45,500)
    jet_numeric={}
    for name in jet_outputs:
        es=dsl.time_jet(name,jet_order)
        fun=sp.lambdify([dsl.t,*dsl.params.values()],es,modules=['numpy'])
        vals=[dsl.values[k] for k in dsl.params]
        arr=np.asarray(fun(tt,*vals),float)
        jet_numeric[name]=arr
    np.savez_compressed(out/'analytic_jets.npz',t=tt,**jet_numeric)
    np.savez_compressed(out/'jacobian_check.npz',exact=Jexact,finite_difference=Jfd)

    truth={
        'sample_rate':12000,
        'duration':4.0,
        'timbre':asdict(timbre),
        'arpeggio_note_count':len(arp_notes),
        'arpeggio_notes':[asdict(n) for n in arp_notes],
        'second_voice_notes':[asdict(n) for n in voice2],
        'representative_note':asdict(representative),
        'jet_outputs':jet_outputs,
        'jet_order':jet_order,
        'jacobian_parameters':wrt,
        'jacobian_max_abs_error':Jerr,
        'nonlocal_effects':{
            'delay':'represented numerically; constant delay can be treated analytically as t -> t-tau',
            'chorus':'represented numerically; time-varying delay requires chain-rule jets',
            'reverb':'represented as convolution; not a local pointwise jet unless impulse response is explicitly parameterized',
        }
    }
    (out/'ground_truth.json').write_text(json.dumps(truth,indent=2),encoding='utf-8')
    report=f"""# Synth DSL + analytic jet benchmark\n\nA symbolic DAG DSL was implemented with named parameter nodes and derived nodes.\n\nSample rate: 12000 Hz\nDuration: 4 s\nCQT target: 60 bins/oct (used by the downstream analyzer; this package contains the synth-side analytic ground truth).\n\nRepresentative note: {representative}\n\nJet outputs: {jet_outputs}\nJet order: {jet_order}\n\nAnalytic Jacobian max absolute error against centered finite differences: {Jerr:.6e}\n\nThe DSL exports:\n- demo_dsl.json: symbolic parameter/derived graph\n- ground_truth.json: events, timbre and benchmark metadata\n- analytic_jets.npz: evaluated time jets up to order {jet_order}\n- jacobian_check.npz: analytic and finite-difference Jacobians\n- demo.wav: rendered multi-voice/arpeggio audio\n\nImportant modeling boundary:\nlocal generators (pitch, envelope, harmonic amplitudes, spectral envelope, modulation) have analytic pointwise jets. Delay, chorus and reverb are nonlocal operators; they remain in the synthesis graph but are not incorrectly treated as ordinary local scalar parameter functions.\n"""
    (out/'REPORT.md').write_text(report,encoding='utf-8')
    # CSV-like compact parameter list.
    rows=['name,value,unit,kind']
    for n,s in dsl.params.items():
        m=dsl.meta[n]; rows.append(f"{n},{dsl.values[n]},{m.get('unit','')},{m.get('kind','parameter')}")
    for n,e in dsl.expr.items():
        rows.append(f"{n},\"{str(e).replace(chr(34),chr(34)*2)}\",{dsl.meta[n].get('unit','')},{dsl.meta[n].get('kind','derived')}")
    (out/'parameter_graph.csv').write_text('\n'.join(rows),encoding='utf-8')

if __name__=='__main__':
    save_demo()

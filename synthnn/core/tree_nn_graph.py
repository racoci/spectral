"""
Motor de Síntese Causal com Matriz de Acoplamento Não-Linear TreeNN (E60-E61).
"""

from typing import List, Dict, Any, Optional
import numpy as np

class OperatorConfig:
    def __init__(self, freq: float, base_amplitude: float = 1.0, is_carrier: bool = False, phase: float = 0.0):
        self.freq = float(freq)
        self.base_amplitude = float(base_amplitude)
        self.is_carrier = bool(is_carrier)
        self.phase = float(phase)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "freq": self.freq,
            "base_amplitude": self.base_amplitude,
            "is_carrier": self.is_carrier,
            "phase": self.phase
        }

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "OperatorConfig":
        return cls(
            freq=d["freq"],
            base_amplitude=d.get("base_amplitude", 1.0),
            is_carrier=d.get("is_carrier", False),
            phase=d.get("phase", 0.0)
        )


class TreeNNCausalGraph:
    """Grafo neural-causal de síntese com matriz de modulação cruzada W e saturação suave."""
    def __init__(
        self,
        ops: List[OperatorConfig],
        W: np.ndarray,
        output_weights: Optional[np.ndarray] = None,
        sample_rate: int = 48000
    ):
        self.ops = ops
        self.num_ops = len(ops)
        self.W = np.array(W, dtype=np.float64)
        self.sample_rate = int(sample_rate)

        if output_weights is not None:
            self.output_weights = np.array(output_weights, dtype=np.float64)
        else:
            # Default: emite apenas nós configurados como portadoras
            self.output_weights = np.array([1.0 if op.is_carrier else 0.0 for op in ops], dtype=np.float64)

    def synthesize(self, duration: float = 1.0) -> np.ndarray:
        num_samples = int(self.sample_rate * duration)
        out = np.zeros(num_samples, dtype=np.float64)

        # Buffer de estado anterior de cada operador: y_{prev} in R^N
        y_prev = np.zeros(self.num_ops, dtype=np.float64)
        y_curr = np.zeros(self.num_ops, dtype=np.float64)

        # Fases acumuladas
        phase_inc = np.array([2.0 * np.pi * op.freq / self.sample_rate for op in self.ops], dtype=np.float64)
        current_phases = np.array([op.phase for op in self.ops], dtype=np.float64)
        amps = np.array([op.base_amplitude for op in self.ops], dtype=np.float64)

        for n in range(num_samples):
            # 1. Modulação cruzada não-linear via tanh
            # theta = W @ tanh(y_prev)
            sat_y = np.tanh(y_prev)
            theta = self.W @ sat_y

            # 2. Avaliação de cada oscilador com modulação de fase
            total_phases = current_phases + theta
            y_curr = amps * np.sin(total_phases)

            # 3. Combinação linear master
            out[n] = float(np.dot(self.output_weights, y_curr))

            # Atualiza estados e fases
            y_prev[:] = y_curr[:]
            current_phases += phase_inc

        return out

    def to_dict(self) -> Dict[str, Any]:
        return {
            "num_operators": self.num_ops,
            "sample_rate": self.sample_rate,
            "operators": [op.to_dict() for op in self.ops],
            "W": self.W.tolist(),
            "output_weights": self.output_weights.tolist()
        }

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "TreeNNCausalGraph":
        ops = [OperatorConfig.from_dict(op_dict) for op_dict in d["operators"]]
        W = np.array(d["W"], dtype=np.float64)
        output_weights = np.array(d["output_weights"], dtype=np.float64)
        sample_rate = d.get("sample_rate", 48000)
        return cls(ops=ops, W=W, output_weights=output_weights, sample_rate=sample_rate)

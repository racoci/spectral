"""Differentiable oscillatory tree neural network module and experiments.

Each node computes:
    theta_v(t) = 2*pi*exp(log_f_v)*t + psi_v + sum_e beta_e sin(theta_child(t))
and emits:
    u_v(t) = c_re * cos(theta_v(t)) + c_im * sin(theta_v(t)) (or pure phase modulation).

Provides:
1. TreeNode and TreeNN architectures with modulation-only option.
2. Hierarchical 4-stage optimization solving the non-convex local minima challenge.
3. Comparative experiments on synthetic nested signals and real speech (voice.wav).
"""
import math
from typing import Optional, Sequence, List, Tuple, Dict
import torch
from torch import nn

DTYPE = torch.float64


class TreeNode(nn.Module):
    """Oscillatory neuron with recursive frequency modulation."""
    def __init__(self, depth: int, init_freq: float, emit: bool = True, emit_children: bool = True):
        super().__init__()
        if depth < 0:
            raise ValueError("depth must be >= 0")
        if init_freq <= 0:
            raise ValueError("init_freq must be > 0")

        self.depth = depth
        self.emit = emit
        self.log_freq = nn.Parameter(torch.tensor(math.log(init_freq), dtype=DTYPE))
        self.phase = nn.Parameter(torch.tensor(0.0, dtype=DTYPE))

        if depth > 0:
            self.beta = nn.Parameter(torch.tensor(0.1, dtype=DTYPE))
        else:
            self.register_parameter("beta", None)

        if emit:
            self.c_re = nn.Parameter(torch.tensor(0.1, dtype=DTYPE))
            self.c_im = nn.Parameter(torch.tensor(0.1, dtype=DTYPE))
        else:
            self.register_parameter("c_re", None)
            self.register_parameter("c_im", None)

        self.child: Optional[TreeNode] = (
            TreeNode(depth - 1, max(init_freq / 2.0, 1.0), emit=emit_children, emit_children=emit_children)
            if depth > 0 else None
        )

    def theta(self, t: torch.Tensor) -> torch.Tensor:
        theta = 2.0 * math.pi * torch.exp(self.log_freq) * t + self.phase
        if self.child is not None and self.beta is not None:
            theta = theta + self.beta * torch.sin(self.child.theta(t))
        return theta

    def forward(self, t: torch.Tensor) -> torch.Tensor:
        theta = self.theta(t)
        if not self.emit or self.c_re is None or self.c_im is None:
            return torch.zeros_like(t)
        return self.c_re * torch.cos(theta) + self.c_im * torch.sin(theta)

    def coefficient_count(self) -> int:
        n = 2  # log_freq, phase
        if self.beta is not None:
            n += 1
        if self.emit:
            n += 2
        return n + (self.child.coefficient_count() if self.child is not None else 0)


class TreeNN(nn.Module):
    """The oscillatory tree network as a differentiable computational graph."""
    def __init__(self, depth: int, root_freqs: Sequence[float], emit_internal: bool = True):
        super().__init__()
        if not root_freqs:
            raise ValueError("root_freqs must not be empty")

        self.bias = nn.Parameter(torch.tensor(0.0, dtype=DTYPE))
        self.slope = nn.Parameter(torch.tensor(0.0, dtype=DTYPE))
        self.roots = nn.ModuleList([
            TreeNode(depth, float(f), emit=True, emit_children=emit_internal) for f in root_freqs
        ])

    def forward(self, t: torch.Tensor) -> torch.Tensor:
        y = self.bias + self.slope * t
        for root in self.roots:
            y = y + root(t)
        return y

    @property
    def coefficient_count(self) -> int:
        return 2 + sum(root.coefficient_count() for root in self.roots)

    def node_count(self) -> int:
        def count(n):
            return 1 + (count(n.child) if n.child is not None else 0)
        return sum(count(r) for r in self.roots)

    def grow_root(self, init_freq: float, depth: int = 0, emit: bool = True) -> None:
        self.roots.append(TreeNode(depth, init_freq, emit=emit))

    def prune_root(self, index: int) -> None:
        if not 0 <= index < len(self.roots):
            raise IndexError(index)
        del self.roots[index]


def train_hierarchical(
    model: TreeNN,
    t: torch.Tensor,
    y: torch.Tensor,
    lr: float = 0.05,
    steps_per_stage: int = 400,
) -> Dict[str, float]:
    """Hierarchical 4-stage training overcoming non-convex landscape:
    Stage 1: Freeze children and beta. Learn bias, slope, root carriers, and linear amplitudes.
    Stage 2: Unfreeze child frequencies and phases.
    Stage 3: Unfreeze modulation couplings beta.
    Stage 4: Joint fine-tuning of all parameters simultaneously.
    """
    loss_fn = nn.MSELoss()

    # Collect parameter groups
    carrier_params = [model.bias, model.slope]
    child_freq_params = []
    beta_params = []

    for root in model.roots:
        carrier_params.extend([root.log_freq, root.phase])
        if root.c_re is not None:
            carrier_params.extend([root.c_re, root.c_im])

        node = root.child
        while node is not None:
            child_freq_params.extend([node.log_freq, node.phase])
            if node.c_re is not None:
                carrier_params.extend([node.c_re, node.c_im])
            node = node.child

        # Collect all betas
        def collect_betas(n):
            if n.beta is not None:
                beta_params.append(n.beta)
            if n.child is not None:
                collect_betas(n.child)
        collect_betas(root)

    # Helper to set requires_grad
    def set_grad(params, flag: bool):
        for p in params:
            p.requires_grad = flag

    # --- STAGE 1: Learn Carriers ---
    set_grad(carrier_params, True)
    set_grad(child_freq_params, False)
    set_grad(beta_params, False)
    opt1 = torch.optim.LBFGS(carrier_params, lr=0.1, max_iter=80, line_search_fn="strong_wolfe")
    def closure1():
        opt1.zero_grad()
        loss = loss_fn(model(t), y)
        loss.backward()
        return loss
    opt1.step(closure1)

    loss_stage1 = loss_fn(model(t), y).item()

    # --- STAGE 2: Unfreeze Child Frequencies ---
    set_grad(child_freq_params, True)
    opt2 = torch.optim.LBFGS(carrier_params + child_freq_params, lr=0.1, max_iter=80, line_search_fn="strong_wolfe")
    def closure2():
        opt2.zero_grad()
        loss = loss_fn(model(t), y)
        loss.backward()
        return loss
    opt2.step(closure2)

    loss_stage2 = loss_fn(model(t), y).item()

    # --- STAGE 3: Unfreeze Modulation Betas ---
    set_grad(beta_params, True)
    opt3 = torch.optim.LBFGS(carrier_params + child_freq_params + beta_params, lr=0.1, max_iter=100, line_search_fn="strong_wolfe")
    def closure3():
        opt3.zero_grad()
        loss = loss_fn(model(t), y)
        loss.backward()
        return loss
    opt3.step(closure3)

    loss_stage3 = loss_fn(model(t), y).item()

    # --- STAGE 4: Joint Fine-Tuning of All Parameters ---
    opt4 = torch.optim.LBFGS(model.parameters(), lr=0.08, max_iter=120, line_search_fn="strong_wolfe")
    def closure4():
        opt4.zero_grad()
        loss = loss_fn(model(t), y)
        loss.backward()
        return loss
    opt4.step(closure4)

    final_loss = loss_fn(model(t), y).item()
    return {
        "rmse_stage1": math.sqrt(loss_stage1),
        "rmse_stage2": math.sqrt(loss_stage2),
        "rmse_stage3": math.sqrt(loss_stage3),
        "rmse_final": math.sqrt(final_loss),
    }


def train_naive_joint(
    model: TreeNN,
    t: torch.Tensor,
    y: torch.Tensor,
    lr: float = 0.05,
    total_steps: int = 1600,
) -> float:
    """Naive joint optimization (gets trapped in local minima)."""
    loss_fn = nn.MSELoss()
    opt = torch.optim.Adam(model.parameters(), lr=lr)
    for _ in range(total_steps):
        opt.zero_grad()
        loss = loss_fn(model(t), y)
        loss.backward()
        opt.step()
    return math.sqrt(loss_fn(model(t), y).item())

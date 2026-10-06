from __future__ import annotations
from dataclasses import dataclass
import math
import torch


def stack_scalars(xs):
    return torch.stack(list(xs))


@dataclass
class Jet:
    """Truncated Taylor series around t0.

    coeff[k] = f^(k)(t0) / k!.
    The implementation avoids in-place tensor writes so PyTorch autograd can
    differentiate the complete jet with respect to model parameters.
    """
    coeff: torch.Tensor

    @property
    def order(self):
        return self.coeff.shape[0] - 1

    @staticmethod
    def constant(x, order):
        x = torch.as_tensor(x, dtype=torch.float64) if not isinstance(x, torch.Tensor) else x
        z = [torch.zeros_like(x) for _ in range(order)]
        return Jet(torch.stack([x, *z]))

    @staticmethod
    def variable(x0, order):
        x0 = torch.as_tensor(x0, dtype=torch.float64) if not isinstance(x0, torch.Tensor) else x0
        z = [torch.zeros_like(x0) for _ in range(order)]
        coeff = [x0]
        if order >= 1:
            coeff.append(torch.ones_like(x0))
            coeff.extend(z[1:])
        return Jet(torch.stack(coeff))

    def _coerce(self, other):
        return other if isinstance(other, Jet) else Jet.constant(other, self.order)

    def __add__(self, other):
        o=self._coerce(other); return Jet(self.coeff+o.coeff)
    __radd__=__add__

    def __neg__(self): return Jet(-self.coeff)
    def __sub__(self, other): return self + (-self._coerce(other))
    def __rsub__(self, other): return self._coerce(other)-self

    def __mul__(self, other):
        o=self._coerce(other); n=self.order
        vals=[]
        for k in range(n+1):
            terms=[self.coeff[j]*o.coeff[k-j] for j in range(k+1)]
            vals.append(sum(terms))
        return Jet(torch.stack(vals))
    __rmul__=__mul__

    def reciprocal(self):
        n=self.order
        b=[1/self.coeff[0]]
        for k in range(1,n+1):
            terms=[self.coeff[j]*b[k-j] for j in range(1,k+1)]
            b.append(-sum(terms)/self.coeff[0])
        return Jet(torch.stack(b))

    def __truediv__(self, other): return self*self._coerce(other).reciprocal()
    def __rtruediv__(self, other): return self._coerce(other)/self

    def exp(self):
        n=self.order; b=[torch.exp(self.coeff[0])]
        for k in range(1,n+1):
            terms=[j*self.coeff[j]*b[k-j] for j in range(1,k+1)]
            b.append(sum(terms)/k)
        return Jet(torch.stack(b))

    def log(self):
        n=self.order; b=[torch.log(self.coeff[0])]
        # a*b' = a'. Degree k-1 gives
        # k*a0*b_k + sum_{j=1}^{k-1} (k-j) a_j b_{k-j} = k*a_k.
        for k in range(1,n+1):
            terms=[(k-j)*self.coeff[j]*b[k-j] for j in range(1,k)]
            b.append((k*self.coeff[k]-sum(terms))/(k*self.coeff[0]))
        return Jet(torch.stack(b))

    def sin(self):
        n=self.order
        ss=[torch.sin(self.coeff[0])]
        cc=[torch.cos(self.coeff[0])]
        for k in range(1,n+1):
            ss.append(sum(j*self.coeff[j]*cc[k-j] for j in range(1,k+1))/k)
            cc.append(-sum(j*self.coeff[j]*ss[k-j] for j in range(1,k+1))/k)
        return Jet(torch.stack(ss))

    def cos(self):
        n=self.order
        ss=[torch.sin(self.coeff[0])]
        cc=[torch.cos(self.coeff[0])]
        for k in range(1,n+1):
            ss.append(sum(j*self.coeff[j]*cc[k-j] for j in range(1,k+1))/k)
            cc.append(-sum(j*self.coeff[j]*ss[k-j] for j in range(1,k+1))/k)
        return Jet(torch.stack(cc))

    def sqrt(self): return (self.log()*0.5).exp()
    def pow(self, exponent): return (self.log()*exponent).exp()

    def erf(self):
        # y' = 2/sqrt(pi) * exp(-x^2) * x'.
        n=self.order
        g=(-(self*self)).exp()
        out=[torch.erf(self.coeff[0])]
        c=2/math.sqrt(math.pi)
        # For n >= 1: n*y_n = c * sum_{j=0}^{n-1} g_j * (n-j) * x_{n-j}.
        for ndeg in range(1,n+1):
            prod=sum(g.coeff[j]*(ndeg-j)*self.coeff[ndeg-j] for j in range(ndeg))
            out.append(c*prod/ndeg)
        return Jet(torch.stack(out))


def jet_derivatives(j: Jet):
    fac=torch.tensor([math.factorial(k) for k in range(j.order+1)],dtype=j.coeff.dtype,device=j.coeff.device)
    return j.coeff*fac

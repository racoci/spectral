from __future__ import annotations

import math
import numpy as np
from scipy.integrate import cumulative_trapezoid
from scipy.optimize import nnls

# Given a monotone perceptual scale y(f) and target local resolution sigma_y(y):
#
#   eta'(y) = -1/(2*pi*sigma_y(y)^2*f'(y))
#   Phi'(f) = 2*pi*eta(y(f))
#
# so eta and Phi are determined by one quadrature each.

def build_eta(y, fprime, sigma_y, eta_top=1e-4):
    deta_dy = -1.0/(2.0*np.pi*np.asarray(sigma_y)**2*np.asarray(fprime))
    eta_rev = cumulative_trapezoid(deta_dy[::-1], y[::-1], initial=0.0)
    return eta_top + eta_rev[::-1]

def build_filters(fgrid, y, fy, eta, fs):
    df = fgrid[1] - fgrid[0]
    eta_f = np.interp(fgrid, fy, eta)
    phi = cumulative_trapezoid(2.0*np.pi*eta_f, fgrid, initial=0.0)
    G = np.empty((len(y), len(fgrid)))
    for j, fj in enumerate(fy):
        phi_j = np.interp(fj, fgrid, phi)
        # Centered logarithm avoids catastrophic cancellation and removes
        # only a y-dependent factor. Subsequent L2 normalization restores
        # exactly the normalized window shape.
        logG = phi - phi_j - 2.0*np.pi*(fgrid-fj)*eta[j]
        gj = np.exp(np.clip(logG, -745.0, 0.0))
        norm = np.sqrt(np.sum(gj**2)*df)
        G[j] = gj/norm
    return G

def solve_tight_weights(G):
    A = (np.abs(G)**2).T
    rho, residual = nnls(A, np.ones(A.shape[0]))
    H = A@rho
    scale = H.mean()
    if scale > 0:
        rho /= scale
        H /= scale
    return rho, H, residual

def analysis_coefficients(x, fs, fgrid, band_mask, G, rho):
    N=len(x)
    X=np.fft.fft(x)
    Xpos=np.zeros(N,dtype=complex)
    Xpos[0]=X[0]
    Xpos[1:N//2]=2.0*X[1:N//2]
    Xpos[N//2]=X[N//2]
    out=np.empty((G.shape[0],N),dtype=complex)
    for j in range(G.shape[0]):
        filt=np.zeros(N,float); filt[band_mask]=G[j]
        out[j]=np.fft.ifft(Xpos*filt)*np.sqrt(max(rho[j],1e-12))
    return out

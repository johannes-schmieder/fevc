"""Iterative fallback for the firm system: block preconditioned conjugate gradients on the grounded firm
Laplacian (all right-hand sides advanced together), with a Jacobi or an incomplete-factor preconditioner."""
import numpy as np, time
from scipy.sparse.linalg import spilu
from akm_design import Design


class IterDesign(Design):
    def __init__(self, worker, firm, mass, stay, tol=1e-9, maxit=5000, precond='jacobi', verbose=False):
        super().__init__(worker, firm, mass, stay)
        self.tol, self.maxit, self.precond, self.verbose = tol, maxit, precond, verbose
        self.iters = []; self._pre = None
        self.Lg = self.L[:-1, :-1].tocsr()

    def _prec(self):
        if self._pre is None:
            if self.precond == 'jacobi':
                d = 1.0 / self.Lg.diagonal(); self._pre = lambda R: (d * R.T).T
            else:                                                   # incomplete LU as an (unsymmetric) approximate inverse
                ilu = spilu(self.Lg.tocsc(), drop_tol=1e-5, fill_factor=20, permc_spec='MMD_AT_PLUS_A', diag_pivot_thresh=0.0,
                            options=dict(SymmetricMode=True))
                self._pre = lambda R: ilu.solve(np.ascontiguousarray(R))
        return self._pre

    def _pcg(self, T):
        one = T.ndim == 1; B = T[:, None] if one else T
        M = self._prec(); X = np.zeros_like(B); R = B.copy(); Z = M(R); Pd = Z.copy()
        rz = (R * Z).sum(0); bn = np.linalg.norm(B, axis=0); bn[bn == 0] = 1.0
        for it in range(self.maxit):
            AP = self.Lg @ Pd; den = (Pd * AP).sum(0); den[den == 0] = 1.0
            a = rz / den; X += Pd * a; R -= AP * a
            rel = np.linalg.norm(R, axis=0) / bn
            if rel.max() < self.tol: break
            Z = M(R); rz_new = (R * Z).sum(0)                      # flexible variant is not needed for a fixed preconditioner
            beta = rz_new / np.where(rz == 0, 1.0, rz); Pd = Z + Pd * beta; rz = rz_new
        self.iters.append(it + 1)
        if self.verbose: print(f'pcg {B.shape[1]} rhs, {it + 1} iterations, max rel residual {rel.max():.2e}', flush=True)
        return X[:, 0] if one else X

    def solve(self, rw, rf):
        t = rf - self.DWF.T @ (rw.T / self.nw).T
        psi = np.zeros_like(rf); psi[:-1] = self._pcg(np.ascontiguousarray(t[:-1]))
        alpha = ((rw - self.DWF @ psi).T / self.nw).T
        return alpha, psi

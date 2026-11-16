"""Synthetic worker-firm designs for the automatic-inference prototype.

Rows are the independent units of FEVC's pooled fixed-offset match design:
one row per mover match (regression mass F = spell length) and one row per
physical stayer observation (F = 1).  The scaled row is sqrt(F) * (e_worker +
e_firm) and the scaled outcome is sqrt(F) * (match mean outcome).
"""

from __future__ import annotations

import numpy as np
import scipy.sparse as sp
from scipy.sparse.csgraph import connected_components


def simulate_rows(
    n_firms, n_movers, n_stayers, rng, max_spell=6, sorting=1.0, size_sigma=1.0, matches_p3=0.3
):
    """Return (worker, firm, mass, is_stayer) row arrays before pruning."""
    size = np.exp(size_sigma * rng.standard_normal(n_firms))
    psi_rank = rng.permutation(n_firms) / n_firms  # latent firm type in [0,1)
    w_rank_m = rng.random(n_movers)
    w_rank_s = rng.random(n_stayers)

    def draw_firm(wr, exclude=None):
        p = size * np.exp(-sorting * 4.0 * (psi_rank - wr) ** 2)
        if exclude is not None:
            p[list(exclude)] = 0.0
        return rng.choice(n_firms, p=p / p.sum())

    worker, firm, mass, stay = [], [], [], []
    for i in range(n_movers):
        k = 3 if rng.random() < matches_p3 else 2
        used = set()
        for _ in range(k):
            j = draw_firm(w_rank_m[i], used)
            used.add(j)
            worker.append(i)
            firm.append(j)
            mass.append(1 + min(rng.poisson(1.5), max_spell - 1))
            stay.append(False)
    for s in range(n_stayers):
        j = draw_firm(w_rank_s[s])
        t = rng.integers(2, max_spell + 1)
        for _ in range(t):
            worker.append(n_movers + s)
            firm.append(j)
            mass.append(1)
            stay.append(True)
    lat = dict(psi_rank=psi_rank, size=size, w_rank=np.concatenate([w_rank_m, w_rank_s]))
    return (np.array(worker), np.array(firm), np.array(mass, float), np.array(stay), lat)


def _bridges(u, v, n_nodes):
    """Indices of bridge edges in an undirected multigraph (iterative Tarjan)."""
    m = len(u)
    order = np.argsort(np.concatenate([u, v]), kind="stable")
    ends = np.concatenate([v, u])[order]
    eid = np.concatenate([np.arange(m), np.arange(m)])[order]
    start = np.searchsorted(np.concatenate([u, v])[order], np.arange(n_nodes + 1))
    disc = -np.ones(n_nodes, int)
    low = np.zeros(n_nodes, int)
    out = []
    t = 0
    for root in range(n_nodes):
        if disc[root] >= 0 or start[root] == start[root + 1]:
            continue
        stack = [(root, -1, start[root])]
        disc[root] = low[root] = t
        t += 1
        while stack:
            node, pe, ptr = stack[-1]
            if ptr < start[node + 1]:
                stack[-1] = (node, pe, ptr + 1)
                e = eid[ptr]
                w = ends[ptr]
                if e == pe:
                    continue
                if disc[w] < 0:
                    disc[w] = low[w] = t
                    t += 1
                    stack.append((w, e, start[w]))
                else:
                    low[node] = min(low[node], disc[w])
            else:
                stack.pop()
                if stack:
                    par = stack[-1][0]
                    low[par] = min(low[par], low[node])
                    if low[node] > disc[par]:
                        out.append(pe)
    return np.array(out, int)


def prune(worker, firm, mass, stay):
    """Leave-one-match-out connected set of movers plus eligible stayers."""
    keep = np.ones(len(worker), bool)
    nW = worker.max() + 1
    nF = firm.max() + 1
    while True:
        mv = keep & ~stay
        idx = np.flatnonzero(mv)
        # largest connected component of the bipartite mover graph
        g = sp.coo_matrix(
            (np.ones(len(idx)), (worker[idx], nW + firm[idx])), shape=(nW + nF, nW + nF)
        )
        _, lab = connected_components(g, directed=False)
        counts = np.bincount(lab[worker[idx]], minlength=lab.max() + 1)
        big = counts.argmax()
        drop = idx[lab[worker[idx]] != big]
        br = _bridges(worker[idx], nW + firm[idx], nW + nF)
        drop = np.union1d(drop, idx[br])
        if len(drop) == 0:
            # movers must retain at least two matches
            cnt = np.bincount(worker[idx], minlength=nW)
            lone = idx[cnt[worker[idx]] < 2]
            if len(lone) == 0:
                break
            drop = lone
        keep[drop] = False
    mv = keep & ~stay
    firms_ok = np.zeros(nF, bool)
    firms_ok[firm[mv]] = True
    keep &= ~stay | firms_ok[firm]
    w = worker[keep]
    f = firm[keep]
    _, w = np.unique(w, return_inverse=True)
    fu, f = np.unique(f, return_inverse=True)
    return w, f, mass[keep], stay[keep], keep, fu


class Design:
    """Scaled FE-only design with a worker-eliminated (firm Schur) solver."""

    def __init__(self, worker, firm, mass, stay):
        self.w, self.f, self.F, self.stay = worker, firm, mass.astype(float), stay
        self.n = len(worker)
        self.nW = worker.max() + 1
        self.nJ = firm.max() + 1
        self.sF = np.sqrt(self.F)
        self.omega = self.F / self.F.sum()
        n = self.n
        self.Dw = sp.csr_matrix((np.ones(n), (np.arange(n), worker)), shape=(n, self.nW))
        self.Ff = sp.csr_matrix((np.ones(n), (np.arange(n), firm)), shape=(n, self.nJ))
        self.nw = np.bincount(worker, weights=self.F, minlength=self.nW)
        WF = sp.diags(self.F) @ self.Ff
        self.DWF = (self.Dw.T @ WF).tocsr()  # nW x nJ
        L = (self.Ff.T @ WF) - self.DWF.T @ sp.diags(1.0 / self.nw) @ self.DWF
        self.L = L.tocsc()
        self._lu = None

    # ---- linear algebra -------------------------------------------------
    def factor(self):
        if self._lu is None:
            from scipy.sparse.linalg import splu

            Lg = self.L[:-1, :-1].tocsc()  # ground last firm
            self._lu = splu(Lg)
        return self._lu

    def Xt(self, v):
        """X_c' v -> (worker part, firm part); v may be n or n x k."""
        sv = (self.sF * v.T).T
        return self.Dw.T @ sv, self.Ff.T @ sv

    def X(self, a, p):
        return (self.sF * (a[self.w] + p[self.f]).T).T

    def solve(self, rw, rf):
        """Solve S beta = (rw, rf) with the last firm grounded at zero."""
        lu = self.factor()
        t = rf - self.DWF.T @ (rw.T / self.nw).T
        psi = np.zeros_like(rf)
        psi[:-1] = lu.solve(np.ascontiguousarray(t[:-1]))
        alpha = ((rw - self.DWF @ psi).T / self.nw).T
        return alpha, psi

    # ---- targets --------------------------------------------------------
    def _cen(self, x):
        return x - self.omega @ x

    def A(self, t, a, p):
        """A_t beta for t in {0: worker var, 1: firm var, 2: covariance}."""
        u = (self.omega * self._cen(a[self.w]).T).T
        v = (self.omega * self._cen(p[self.f]).T).T
        if t == 0:
            return self.Dw.T @ u, np.zeros_like(p)
        if t == 1:
            return np.zeros_like(a), self.Ff.T @ v
        return 0.5 * (self.Dw.T @ v), 0.5 * (self.Ff.T @ u)

    def theta(self, a, p):
        u = self._cen(a[self.w])
        v = self._cen(p[self.f])
        om = self.omega
        return np.array(
            [(om * u.T * u.T).T.sum(0), (om * v.T * v.T).T.sum(0), (om * u.T * v.T).T.sum(0)]
        )

    def Ah_t(self, t, r):
        """A_h,t' r for the half-target factor (t in {0 worker, 1 firm})."""
        x = (np.sqrt(self.omega) * r.T).T
        x = x - np.outer(self.omega, x.sum(0)) if x.ndim == 2 else x - self.omega * x.sum()
        if t == 0:
            return self.Dw.T @ x, np.zeros((self.nJ,) + x.shape[1:])
        return np.zeros((self.nW,) + x.shape[1:]), self.Ff.T @ x

    def dense_X(self):
        Xd = np.zeros((self.n, self.nW + self.nJ - 1))
        Xd[np.arange(self.n), self.w] = self.sF
        m = self.f < self.nJ - 1
        Xd[np.flatnonzero(m), self.nW + self.f[m]] = self.sF[m]
        return Xd


def make_design(n_firms, n_movers, n_stayers, seed, **kw):
    rng = np.random.default_rng(seed)
    w, f, F, st, lat = simulate_rows(n_firms, n_movers, n_stayers, rng, **kw)
    w2, f2, F2, st2, keep, fu = prune(w, f, F, st)
    d = Design(w2, f2, F2, st2)
    d.psi_rank = lat["psi_rank"][fu]
    wu = np.unique(w[keep])
    d.w_rank = lat["w_rank"][wu]
    d.size = lat["size"][fu]
    return d


def simulate_rows_fast(
    n_firms, n_movers, n_stayers, rng, max_spell=6, sort_sd=0.25, size_sigma=1.0, matches_p3=0.3
):
    """Vectorised generator for large designs (same row convention)."""
    size = np.exp(size_sigma * rng.standard_normal(n_firms))
    cdf = np.cumsum(size) / size.sum()  # firms are ordered by latent type
    psi_rank = (np.arange(n_firms) + 0.5) / n_firms

    def draw(wr):
        u = np.clip(wr + sort_sd * rng.standard_normal(len(wr)), 0.0, 1.0 - 1e-12)
        return np.minimum(np.searchsorted(cdf, u), n_firms - 1)

    w_rank_m = rng.random(n_movers)
    w_rank_s = rng.random(n_stayers)
    k = np.where(rng.random(n_movers) < matches_p3, 3, 2)
    wm = np.repeat(np.arange(n_movers), k)
    fm = draw(w_rank_m[wm])
    key = wm.astype(np.int64) * n_firms + fm
    _, first = np.unique(key, return_index=True)  # one match per worker-firm pair
    first.sort()
    wm, fm = wm[first], fm[first]
    Fm = 1 + np.minimum(rng.poisson(1.5, len(wm)), max_spell - 1)
    t = rng.integers(2, max_spell + 1, n_stayers)
    ws = np.repeat(n_movers + np.arange(n_stayers), t)
    fs = np.repeat(draw(w_rank_s), t)
    worker = np.concatenate([wm, ws])
    firm = np.concatenate([fm, fs])
    mass = np.concatenate([Fm, np.ones(len(ws))]).astype(float)
    stay = np.concatenate([np.zeros(len(wm), bool), np.ones(len(ws), bool)])
    lat = dict(psi_rank=psi_rank, size=size, w_rank=np.concatenate([w_rank_m, w_rank_s]))
    return worker, firm, mass, stay, lat


def make_design_fast(n_firms, n_movers, n_stayers, seed, **kw):
    rng = np.random.default_rng(seed)
    w, f, F, st, lat = simulate_rows_fast(n_firms, n_movers, n_stayers, rng, **kw)
    w2, f2, F2, st2, keep, fu = prune(w, f, F, st)
    d = Design(w2, f2, F2, st2)
    d.psi_rank = lat["psi_rank"][fu]
    d.w_rank = lat["w_rank"][np.unique(w[keep])]
    d.size = lat["size"][fu]
    return d


def make_design_bottleneck(n_firms, n_movers, n_stayers, seed, n_bridge=3, max_spell=6):
    """Two firm clusters joined by only n_bridge movers (concentrated spectrum)."""
    rng = np.random.default_rng(seed)
    half = n_firms // 2
    grp_w = rng.integers(0, 2, n_movers)
    worker, firm, mass, stay = [], [], [], []
    for i in range(n_movers):
        lo = 0 if grp_w[i] == 0 else half
        if i < n_bridge:
            js = [rng.integers(0, half), half + rng.integers(0, n_firms - half)]
        else:
            k = 3 if rng.random() < 0.3 else 2
            js = lo + rng.choice(half if grp_w[i] == 0 else n_firms - half, size=k, replace=False)
        for j in js:
            worker.append(i)
            firm.append(int(j))
            stay.append(False)
            mass.append(1 + min(rng.poisson(1.5), max_spell - 1))
    for s in range(n_stayers):
        j = rng.integers(0, n_firms)
        t = rng.integers(2, max_spell + 1)
        for _ in range(t):
            worker.append(n_movers + s)
            firm.append(int(j))
            mass.append(1)
            stay.append(True)
    w, f, F, st = np.array(worker), np.array(firm), np.array(mass, float), np.array(stay)
    w2, f2, F2, st2, keep, fu = prune(w, f, F, st)
    d = Design(w2, f2, F2, st2)
    # latent types: the two clusters differ in mean firm effect
    d.psi_rank = np.where(fu < half, 0.25, 0.75) + 0.2 * (rng.random(len(fu)) - 0.5)
    d.w_rank = rng.random(d.nW)
    d.size = np.ones(d.nJ)
    return d

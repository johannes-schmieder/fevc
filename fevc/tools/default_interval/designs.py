"""Synthetic two-block person-year panels for the Monte Carlo of the default interval (NumPy only).

Design: nJ firms in two blocks; workers belong to a block and move within it; n_bridge workers
make one move to the other block (the bottleneck). Six years, unbalanced; pruned to the
leave-one-match-out connected set. Truth: person effects, firm effects with a gap between the
blocks, person-year error variances of one of four kinds (flat, profile, movers, bridge).

This is a port of the prototype's design code (two_block_panel, prune_panel and truth with the
pieces of Panel and the pruning they use). The campaign designs are fixed by their sha256 in the run
manifests, so every random draw, its order and the output format must stay as they are: a change
here changes the designs.
"""

import numpy as np


class Panel:
    """Person-year panel of integer codes; workers with one firm are stayers."""

    def __init__(self, worker, firm, year):
        _, self.w = np.unique(worker, return_inverse=True)
        _, self.f = np.unique(firm, return_inverse=True)
        self.year = np.asarray(year)
        self.nW = self.w.max() + 1
        self.nJ = self.f.max() + 1
        self.n = len(self.w)
        key = self.w.astype(np.int64) * self.nJ + self.f
        uk, self.match = np.unique(key, return_inverse=True)  # match id per person-year
        self.match_w = (uk // self.nJ).astype(np.int64)
        self.match_f = (uk % self.nJ).astype(np.int64)
        self.stay_w = np.bincount(self.match_w, minlength=self.nW) == 1


def _components(u, v, n_nodes):
    """Connected components of an undirected graph; each node is labelled by the smallest node of
    its component. The largest component is then chosen with ties resolved as by scipy's
    connected_components (whose labels follow the smallest node of each component)."""
    lab = np.arange(n_nodes)
    while True:
        m = np.minimum(lab[u], lab[v])
        new = lab.copy()
        np.minimum.at(new, u, m)
        np.minimum.at(new, v, m)
        new = new[new]  # a label is a node of the same component with a smaller index
        if (new == lab).all():
            return lab
        lab = new


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


def _prune_rows(worker, firm, stay):
    """Rows kept in the leave-one-match-out connected set of movers plus eligible stayers."""
    keep = np.ones(len(worker), bool)
    nW = worker.max() + 1
    nF = firm.max() + 1
    while True:
        idx = np.flatnonzero(keep & ~stay)
        # largest connected component of the bipartite mover graph
        lab = _components(worker[idx], nW + firm[idx], nW + nF)
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
    return keep


def prune_panel(w, f, yr):
    """Person-year panel pruned to the leave-one-match-out connected set (movers keep >= 2 matches,
    stayers >= 2 years)."""
    while True:
        P = Panel(w, f, yr)
        st_py = P.stay_w[P.w]
        mv = ~P.stay_w[P.match_w]
        rw = np.r_[P.match_w[mv], P.w[st_py]]
        rf = np.r_[P.match_f[mv], P.f[st_py]]
        rs = np.r_[np.zeros(mv.sum(), bool), np.ones(st_py.sum(), bool)]
        keep = _prune_rows(rw, rf, rs)
        keep_match = np.zeros(len(P.match_w), bool)
        keep_match[np.flatnonzero(mv)[keep[: mv.sum()]]] = True
        keep_py = np.where(st_py, False, keep_match[P.match])
        keep_py[np.flatnonzero(st_py)[keep[mv.sum():]]] = True
        cnt = np.bincount(P.w[keep_py], minlength=P.nW)
        keep_py &= cnt[P.w] >= 2
        if keep_py.all():
            return w, f, yr
        w, f, yr = w[keep_py], f[keep_py], yr[keep_py]


def two_block_panel(nJ, nW, n_bridge, seed, years=6, p_move=0.10):
    """Returns (Panel, firm block labels 0/1, bridging-worker flags). n_bridge = None: one block (no
    bottleneck)."""
    rng = np.random.default_rng([seed, 17])
    half = nJ // 2
    size = np.exp(rng.standard_normal(nJ))
    blk_f = (np.arange(nJ) >= half).astype(int)
    if n_bridge is None:
        blk_f[:] = 0
    pj = [size * (blk_f == b) / (size * (blk_f == b)).sum() if (blk_f == b).any() else None
          for b in (0, 1)]
    w, f, yr = [], [], []
    for i in range(nW):
        b = 0 if n_bridge is None else int(rng.integers(0, 2))
        bridge = n_bridge is not None and i < n_bridge
        T = int(rng.integers(3 if bridge else 2, years + 1))
        t0 = int(rng.integers(0, years - T + 1))
        j = rng.choice(nJ, p=pj[b])
        tb = t0 + int(rng.integers(1, T)) if bridge else -1
        for t in range(t0, t0 + T):
            if t == tb:
                b = 1 - b
                j = rng.choice(nJ, p=pj[b])
            elif t > t0 and rng.random() < p_move:
                j = rng.choice(nJ, p=pj[b])
            w.append(i)
            f.append(j)
            yr.append(t)
    w, f, yr = prune_panel(np.array(w), np.array(f), np.array(yr))
    P = Panel(w, f, yr)
    uf = np.unique(f)
    uw = np.unique(w)
    return P, blk_f[uf], (uw < (n_bridge or 0))


def truth(P, blk, cell, seed, bridge=None):
    """Effects and person-year error variances of a cell (deterministic given the seed)."""
    rng = np.random.default_rng([seed, 23])
    psi = 0.5 * cell['gap'] * (2 * blk - 1.0) + 0.12 * rng.standard_normal(P.nJ)
    fw = np.zeros(P.nW, int)
    fw[P.w] = P.f  # a firm of each worker (the last listed)
    alpha = 0.30 * rng.standard_normal(P.nW) + 0.5 * (psi[fw] - psi.mean())
    a = alpha - np.average(alpha[P.w])
    at = a / np.sqrt(np.average(a[P.w] ** 2))
    v = np.full(P.n, 0.04)
    kind = cell['variance']
    if kind == 'flat':
        pass
    elif kind == 'profile':  # quadratic in the person effect, worker factors
        lamw = rng.gamma(1 / 1.0 ** 2, 1.0 ** 2, P.nW)
        v = v * ((1 + 0.5 * at ** 2) / (1 + 0.5 * np.average(at[P.w] ** 2)) * lamw)[P.w]
    elif kind == 'movers':  # movers steeper than stayers, and three times the level
        st = P.stay_w
        g = np.where(st, 0.5, 1.5)
        fac = 1 + g * at ** 2
        for m in (st, ~st):
            fac[m] /= np.average(fac[P.w][m[P.w]])
        v = v * (fac * np.where(st, 1.0, 3.0))[P.w]
    elif kind == 'bridge':  # the bridging workers alone are three times as noisy
        v = v * np.where(bridge, 3.0, 1.0)[P.w]
    else:
        raise ValueError(kind)
    return alpha, psi, v


_CONTROL_DEFAULTS = dict(alpha=0.4, psi=0.8, noise=0.25, coef=0.3, second=False, hetero=0.0)


def _control_spec(ctrl, name):
    """The options of an object-valued control, with defaults; anything else is rejected."""
    bad = set(ctrl) - set(_CONTROL_DEFAULTS)
    if bad:
        raise ValueError(f"cell {name}: unknown control keys {sorted(bad)}")
    spec = dict(_CONTROL_DEFAULTS, **ctrl)
    for k, v in spec.items():
        if k == 'second':
            ok = isinstance(v, bool)
        else:
            ok = isinstance(v, (int, float)) and not isinstance(v, bool) and np.isfinite(v)
        if not ok:
            raise ValueError(f"cell {name}: control key {k!r} has the invalid value {v!r}")
    return spec


def write_design(cell, path):
    """Write the design CSV of a cell and return its description for the manifest.

    Columns: worker, firm, year, alpha, psi, sd (the person-year error standard deviation); with
    "control": true also x and xb, a control fixed with the design and its contribution to the
    outcome. "control" may instead be an object of options (alpha, psi, noise, coef, second,
    hetero; see the README), which adds the description key control_realised, and with second
    a further control x2 (xb is then the contribution of both).
    collapsed_rows counts the rows of the collapsed design (one per mover match, one per stayer
    person-year) and mover_rows the mover matches.
    """
    P, blk, bridge = two_block_panel(cell['nJ'], cell['nW'], cell['n_bridge'], cell['design_seed'])
    alpha, psi, v_py = truth(P, blk, cell, cell['design_seed'], bridge)
    cols = [P.w + 1, P.f + 1, P.year, alpha[P.w], psi[P.f], np.sqrt(v_py)]
    header = 'worker,firm,year,alpha,psi,sd'
    fmt = ['%d', '%d', '%d', '%.17g', '%.17g', '%.17g']
    ctrl = cell.get('control')
    spec = _control_spec(ctrl, cell['name']) if isinstance(ctrl, dict) else None
    realised = None
    if ctrl is not None and ctrl is not False and (spec is not None or ctrl):
        spec = spec or _CONTROL_DEFAULTS
        # A control fixed with the design, correlated with both effects and varying within every
        # worker and match; its contribution to the outcome (default coefficient 0.3) travels as
        # its own column. The estimand is unchanged.
        e = np.random.default_rng([cell['design_seed'], 77]).standard_normal(P.n)
        x = spec['alpha'] * alpha[P.w] + spec['psi'] * psi[P.f] + spec['noise'] * e
        xb = spec['coef'] * x
        cols += [x, xb]
        header += ',x,xb'
        fmt += ['%.17g', '%.17g']
        if spec['second']:
            # A second control that trends with the calendar year and is unrelated to the effects.
            x2 = (0.3 * (P.year - P.year.mean())
                  + 0.5 * np.random.default_rng([cell['design_seed'], 78]).standard_normal(P.n))
            xb = xb + 0.2 * x2
            cols[-1] = xb
            cols.append(x2)
            header += ',x2'
            fmt.append('%.17g')
        if spec['hetero']:
            # The error variance moves with the control within every cell; its mean factor is one.
            h = spec['hetero']
            cols[5] = cols[5] * np.exp(h * e - h ** 2)
        if isinstance(ctrl, dict):
            eff = alpha[P.w] + psi[P.f]
            mean_x = np.bincount(P.match, x) / np.bincount(P.match)
            realised = dict(
                corr_x_effects=float(np.corrcoef(x, eff)[0, 1]),
                within_match_share=float(np.sum((x - mean_x[P.match]) ** 2)
                                         / np.sum((x - x.mean()) ** 2)),
                index_sd=float(np.std(xb)), effects_sd=float(np.std(eff)),
                controls=2 if spec['second'] else 1)
            if spec['hetero']:
                vf = np.exp(2 * h * e - 2 * h ** 2)
                realised.update(variance_factor_p10=float(np.percentile(vf, 10)),
                                variance_factor_p90=float(np.percentile(vf, 90)))
    np.savetxt(path, np.column_stack(cols), delimiter=',', header=header, comments='', fmt=fmt)
    mover_rows = int((~P.stay_w[P.match_w]).sum())
    info = dict(person_years=int(P.n), workers=int(P.nW), firms=int(P.nJ),
                bridge_workers=int(bridge.sum()),
                collapsed_rows=mover_rows + int(P.stay_w[P.w].sum()), mover_rows=mover_rows)
    if realised is not None:
        info['control_realised'] = realised
    return info

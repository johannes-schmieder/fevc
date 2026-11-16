"""Simulation of the q = 1 interval on synthetic two-block designs (prototype; Section 7 of the note).

Design: nJ firms in two blocks; workers belong to a block and move within it; n_bridge workers make one move to
the other block (the bottleneck). Six years, unbalanced; pruned to the leave-one-match-out connected set.
Truth: person effects, firm effects with a gap between the blocks, error variances by scenario.
For each draw: theta-hat, the normal interval with the note's variance estimate, and the q = 1 interval, each with the
model for movers ('dq') and with movers' true variances ('or').

Usage: lp_q1sim.py cell <out-root> <manifest> <cell-index> <task-index>     (one task of one cell; atomic JSON)
       lp_q1sim.py design <manifest> <cell-index>                            (print the realised design only)"""
import sys, os, json, time, zlib, hashlib, numpy as np
from lp6 import Panel, build_design, aggregate, T34
from akm_design import prune
from lp_q1 import LPQ, interval, q1_interval

COMP = ('person', 'firm', 'cov', 'sum')
SOURCES = ('lp_q1sim.py', 'lp_q1.py', 'lp7.py', 'lp6.py', 'akm_design.py', 'sparse_auto.py', 'dense_mc.py')


def source_hashes():
    here = os.path.dirname(os.path.abspath(__file__))
    return {f: hashlib.sha256(open(os.path.join(here, f), 'rb').read()).hexdigest() for f in SOURCES}


def prune_panel(w, f, yr):
    """Person-year panel pruned to the leave-one-match-out connected set (movers keep >= 2 matches, stayers >= 2 years)."""
    while True:
        P = Panel(w, f, yr); st_py = P.stay_w[P.w]; mv = ~P.stay_w[P.match_w]
        Fm = np.bincount(P.match, minlength=len(P.match_w)).astype(float)
        rw = np.r_[P.match_w[mv], P.w[st_py]]; rf = np.r_[P.match_f[mv], P.f[st_py]]
        rF = np.r_[Fm[mv], np.ones(st_py.sum())]; rs = np.r_[np.zeros(mv.sum(), bool), np.ones(st_py.sum(), bool)]
        _, _, _, _, keep, _ = prune(rw, rf, rF, rs)
        keep_match = np.zeros(len(P.match_w), bool); keep_match[np.flatnonzero(mv)[keep[:mv.sum()]]] = True
        keep_py = np.where(st_py, False, keep_match[P.match]); keep_py[np.flatnonzero(st_py)[keep[mv.sum():]]] = True
        cnt = np.bincount(P.w[keep_py], minlength=P.nW); keep_py &= cnt[P.w] >= 2
        if keep_py.all(): return w, f, yr
        w, f, yr = w[keep_py], f[keep_py], yr[keep_py]


def two_block_panel(nJ, nW, n_bridge, seed, years=6, p_move=0.10):
    """Returns (Panel, firm block labels 0/1, bridging-worker flags). n_bridge = None: one block (no bottleneck)."""
    rng = np.random.default_rng([seed, 17]); half = nJ // 2
    size = np.exp(rng.standard_normal(nJ)); blk_f = (np.arange(nJ) >= half).astype(int)
    if n_bridge is None: blk_f[:] = 0
    pj = [size * (blk_f == b) / (size * (blk_f == b)).sum() if (blk_f == b).any() else None for b in (0, 1)]
    w, f, yr = [], [], []
    for i in range(nW):
        b = 0 if n_bridge is None else int(rng.integers(0, 2)); bridge = n_bridge is not None and i < n_bridge
        T = int(rng.integers(3 if bridge else 2, years + 1)); t0 = int(rng.integers(0, years - T + 1)); j = rng.choice(nJ, p=pj[b])
        tb = t0 + int(rng.integers(1, T)) if bridge else -1
        for t in range(t0, t0 + T):
            if t == tb: b = 1 - b; j = rng.choice(nJ, p=pj[b])
            elif t > t0 and rng.random() < p_move: j = rng.choice(nJ, p=pj[b])
            w.append(i); f.append(j); yr.append(t)
    w, f, yr = prune_panel(np.array(w), np.array(f), np.array(yr))
    P = Panel(w, f, yr); uf = np.unique(f); uw = np.unique(w)
    return P, blk_f[uf], (uw < (n_bridge or 0))


def truth(P, blk, cell, seed, bridge=None):
    """Effects and person-year error variances of a cell (deterministic given the seed)."""
    rng = np.random.default_rng([seed, 23]); psi = 0.5 * cell['gap'] * (2 * blk - 1.0) + 0.12 * rng.standard_normal(P.nJ)
    fw = np.zeros(P.nW, int); fw[P.w] = P.f                                        # a firm of each worker (the last listed)
    alpha = 0.30 * rng.standard_normal(P.nW) + 0.5 * (psi[fw] - psi.mean())
    a = alpha - np.average(alpha[P.w]); at = a / np.sqrt(np.average(a[P.w] ** 2)); v = np.full(P.n, 0.04)
    kind = cell['variance']
    if kind == 'flat': pass
    elif kind == 'profile':                                                       # quadratic in the person effect, worker factors
        lamw = rng.gamma(1 / 1.0 ** 2, 1.0 ** 2, P.nW); v = v * ((1 + 0.5 * at ** 2) / (1 + 0.5 * np.average(at[P.w] ** 2)) * lamw)[P.w]
    elif kind == 'movers':                                                        # movers steeper than stayers, and three times the level
        st = P.stay_w; g = np.where(st, 0.5, 1.5); fac = 1 + g * at ** 2
        for m in (st, ~st): fac[m] /= np.average(fac[P.w][m[P.w]])
        v = v * (fac * np.where(st, 1.0, 3.0))[P.w]
    elif kind == 'bridge': v = v * np.where(bridge, 3.0, 1.0)[P.w]                # the bridging workers alone are three times as noisy
    else: raise ValueError(kind)
    return alpha, psi, v


def setup(cell, probes=1600):
    P, blk, bridge = two_block_panel(cell['nJ'], cell['nW'], cell['n_bridge'], cell['design_seed'])
    D, info = build_design(P); E = LPQ(D, probes=probes, seed=cell['design_seed'] + 5)
    alpha, psi, v_py = truth(P, blk, cell, cell['design_seed'], bridge); mu = D.X(alpha, psi); v_rows = aggregate(info, D.n, v_py * info['py_w'])
    return P, D, info, E, mu, v_py, v_rows, alpha, psi, blk, bridge


def describe(cell, E, D, mu, v_rows, alpha, psi, lead, bridge):
    """Realised regime of a cell: eigenvalue, weights, true covariance matrix, curvature, variance of theta-hat."""
    th3 = E.theory(mu, v_rows, draws=512); out = dict(rows=int(D.n), mover_rows=int((~D.stay).sum()), firms=int(D.nJ), workers=int(D.nW), bridge_workers=int(bridge.sum()), comp={})
    theta = T34 @ D.theta(alpha, psi)
    for c in range(4):
        lam, w1, _, steps = lead[c]; tr = E.q1_truth(mu, v_rows, c, lam, w1); S = np.array(tr['Sigma']); V = float(th3['V4'][c])
        rho = S[0, 1] / np.sqrt(S[0, 0] * S[1, 1]); kap = 2 * abs(lam) * S[0, 0] / (np.sqrt(S[1, 1]) * np.sqrt(1 - rho ** 2))
        out['comp'][COMP[c]] = dict(theta=float(theta[c]), V=V, lam1=float(lam), power_steps=int(steps), max_w2=float((w1 ** 2).max()),
                                    w2_movers=float((w1[~D.stay] ** 2).sum()), w2_bridge=float((w1[bridge[D.w]] ** 2).sum()), b1=tr['b1'], Sigma=S.tolist(), rho=float(rho), kappa=float(kap),
                                    lam_sq_Sbb_sq_over_V=float(2 * lam ** 2 * S[0, 0] ** 2 / V), b1_over_sd=float(tr['b1'] / np.sqrt(S[0, 0])))
    return out


def errors(kind, rng, n):
    """Standardised person-year errors: normal, or skewed (centred exponential: skewness 2, kurtosis 9)."""
    if kind == 'normal': return rng.standard_normal(n)
    if kind == 'skew': return rng.exponential(1.0, n) - 1.0
    raise ValueError(kind)


def run_task(oroot, mfile, ci, ti):
    man = json.load(open(mfile)); cell = man['cells'][ci]; odir = f"{oroot}/{cell['name']}"; rfile = f'{odir}/task-{ti:03d}.json'
    if os.path.exists(rfile): raise SystemExit(f'{rfile} exists; refusing to overwrite')
    if not 0 <= ti < man['tasks']: raise SystemExit('task index outside the manifest')
    src = source_hashes()
    if 'sources' in man and man['sources'] != src: raise SystemExit('source files differ from the manifest: ' + ', '.join(f for f in src if man['sources'].get(f) != src[f]))
    key = zlib.crc32(cell['name'].encode()); ekind = cell.get('errors', 'normal')
    os.makedirs(odir, exist_ok=True); t0 = time.time()
    P, D, info, E, mu, v_py, v_rows, alpha, psi, blk, bridge = setup(cell, man.get('probes', 1600))
    lead = [E.leading(c) for c in range(4)]; theta = T34 @ D.theta(alpha, psi)
    if ti == 0:
        d = describe(cell, E, D, mu, v_rows, alpha, psi, lead, bridge); tmp = f'{odir}/design.json.tmp'; json.dump(d, open(tmp, 'w')); os.replace(tmp, f'{odir}/design.json')
    KEYS = ('theta', 'V_dq', 'V_or', 'b1', 'theta_r', 'S_dq', 'S_or', 'bbfit_dq', 'bbfit_or', 'ci_dq', 'ci_or', 'cilo_dq', 'cifit_dq', 'ci_true')
    St = [E.q1_truth(mu, v_rows, c, lead[c][0], lead[c][1])['Sigma'] for c in range(4)]          # true covariance matrices (same in every task)
    rec = {k: [] for k in KEYS}; fails = 0
    for r in range(man['reps_per_task']):
        rng = np.random.default_rng([man['stream'], key, ti, r]); y = mu + aggregate(info, D.n, np.sqrt(v_py) * errors(ekind, rng, P.n))
        try:
            o = E.estimate_final(y, draws=man.get('draws', 16), dseed=7000 * ti + r, vtrue=v_rows)
            row = dict(theta=o['theta'].tolist(), V_dq=np.diag(T34 @ o['dq'] @ T34.T).tolist(), V_or=np.diag(T34 @ o['or'] @ T34.T).tolist(),
                       **{k: [] for k in KEYS[3:]})
            for c in range(4):
                lam, w1, _, _ = lead[c]; q = E.q1(y, c, lam, w1, draws=man.get('draws', 16), dseed=7000 * ti + r + 31 * c, vtrue=v_rows)
                row['b1'].append(q['b1']); row['theta_r'].append(q['theta_r'])
                for nm in ('dq', 'or'):
                    S = q[nm]; bf = q[nm + '_bbfit']; row['S_' + nm].append(S); row['bbfit_' + nm].append(bf)
                    iv = q1_interval(lam, q['b1'], q['theta_r'], S, bf); row['ci_' + nm].append(None if iv is None else list(iv))
                S = q['dq']; bf = q['dq_bbfit']                                                    # comparisons: leave-out alone, fitted alone, true matrix
                for kind, M in (('cilo_dq', S), ('cifit_dq', [[bf, S[0][1]], [S[0][1], S[1][1]]]), ('ci_true', St[c])):
                    iv = interval(lam, q['b1'], q['theta_r'], M); row[kind].append(None if iv is None else list(iv))
        except Exception as ex:
            fails += 1; print('rep failed', r, repr(ex), flush=True); continue
        for k in rec: rec[k].append(row[k])
    out = dict(schema='LP-Q1-TASK-V3', run=man['run'], stream=man['stream'], sources=src, cell=ci, name=cell['name'], task=ti, reps_requested=man['reps_per_task'], reps_done=len(rec['theta']), failures=fails,
               theta_true=theta.tolist(), lam1=[float(l[0]) for l in lead], Sigma_true=St, seconds=time.time() - t0, **rec)
    tmp = rfile + '.tmp'; json.dump(out, open(tmp, 'w')); os.replace(tmp, rfile); print('wrote', rfile, f'{out["seconds"]:.0f}s', flush=True)


if __name__ == '__main__':
    if sys.argv[1] == 'cell': run_task(sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5]))
    elif sys.argv[1] == 'design':
        man = json.load(open(sys.argv[2])); cell = man['cells'][int(sys.argv[3])]; P, D, info, E, mu, v_py, v_rows, alpha, psi, blk, bridge = setup(cell, man.get('probes', 1600))
        print(json.dumps(describe(cell, E, D, mu, v_rows, alpha, psi, [E.leading(c) for c in range(4)], bridge), indent=1))
    else: raise SystemExit('unknown mode')

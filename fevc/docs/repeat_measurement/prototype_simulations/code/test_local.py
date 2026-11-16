"""Local check of lp.py on a small synthetic unbalanced panel: exact diagonals from the dense design, normal
and skewed errors; mean of each variance estimate against the theoretical variance."""
import sys, time, numpy as np
from lp import Panel, build_design, aggregate, LP, synth_panel, T34
rng = np.random.default_rng(1)
P = synth_panel(nJ=int(sys.argv[1]) if len(sys.argv) > 1 else 40, nW=int(sys.argv[2]) if len(sys.argv) > 2 else 700, seed=3, minT=int(sys.argv[5]) if len(sys.argv) > 5 else 2)
D, info = build_design(P)
print('person-years', P.n, 'workers', P.nW, 'stayers', int(P.stay_w.sum()), 'firms', P.nJ, 'rows', D.n, 'mover rows', int((~D.stay).sum()))
print('stayer years distribution', np.bincount(P.T_w[P.stay_w])[2:])
# exact diagonals (dense)
X = D.dense_X(); S = X.T @ X; Sinv = np.linalg.inv(S)
Pd = np.einsum('ij,jk,ik->i', X, Sinv, X)
def targ(t):
    # A_t as matrix on (worker, firm[:-1]) coordinates via the design's A operator applied to unit vectors
    k = D.nW + D.nJ - 1; M = np.zeros((k, k))
    I = np.eye(k)
    for j in range(k):
        a = I[:D.nW, j]; p = np.r_[I[D.nW:, j], 0.0]
        ra, rp = D.A(t, a, p); M[:, j] = np.r_[ra, rp[:-1]]
    return M
Bd = np.zeros((3, D.n))
for t in range(3):
    At = targ(t); U = X @ Sinv; Bd[t] = np.einsum('ij,jk,ik->i', U, At, U)
E = LP(D, exact_diag=(Pd, Bd))
st = D.stay
print('max |exact P - 1/R| on stayer rows', np.abs(Pd[st] - 1 / E.rows_w[D.w[st]]).max(), ' max |B_firm|, |B_cov| on stayer rows', np.abs(Bd[1][st]).max(), np.abs(Bd[2][st]).max())
alpha = 0.3 * rng.standard_normal(D.nW); psi = 0.15 * rng.standard_normal(D.nJ) + 0.3 * 0  # effects
mu_py = alpha[P.w] + psi[P.f]
mu = D.X(alpha, psi)
# variances: worker-specific for stayers, match-specific for movers (person-year level, constant within match)
vw = 0.04 * np.exp(0.6 * rng.standard_normal(P.nW)); vm = 0.06 * np.exp(0.5 * rng.standard_normal(len(P.match_w)))
v_py = np.where(P.stay_w[P.w], vw[P.w], vm[P.match])
v_rows = aggregate(info, D.n, v_py * info['py_w'])            # (1/F) sum v = v for constant v within match
kind = sys.argv[3] if len(sys.argv) > 3 else 'gauss'; reps = int(sys.argv[4]) if len(sys.argv) > 4 else 400
th = E.theory(mu, v_rows, draws=512)
sgn = np.sign(th['m'][0] + th['m'][1] + 2 * th['m'][2])       # sign of m for the total, by row
sgn_py = np.ones(P.n); sgn_py[info['valid']] = sgn[info['py_row'][info['valid']]]
def draw():
    if kind == 'gauss': u = rng.standard_normal(P.n)
    elif kind == 'skew':                                       # gamma shape 2 (skewness 1.41, kurtosis 6), stayers signed against m
        u = (rng.gamma(2.0, 1.0, P.n) - 2.0) / np.sqrt(2.0)
        u = np.where(P.stay_w[P.w], -sgn_py * u, np.where(rng.random(P.n) < 0.5, u, -u))
    elif kind == 'fat':
        u = (rng.gamma(0.4, 1.0, P.n) - 0.4) / np.sqrt(0.4); u = np.where(rng.random(P.n) < 0.5, u, -u)
    return aggregate(info, D.n, np.sqrt(v_py) * u)
names = None; acc = {}; ths = []; t0 = time.time()
for r in range(reps):
    y = mu + draw()
    o = E.estimate(y, draws=8, dseed=r, vtrue=v_rows)
    ths.append(o['theta'])
    for k, V in o.items():
        if isinstance(V, np.ndarray) and V.shape == (3, 3): acc.setdefault(k, []).append(np.diag(T34 @ V @ T34.T))
ths = np.array(ths)
print(f'{kind}: {reps} reps in {time.time() - t0:.0f}s; MC var / theory', np.round(ths.var(0, ddof=1) / th['V4'], 3), ' signal share', np.round(th['signal4'] / th['V4'], 2))
print('bias of theta-hat in SD:', np.round((ths.mean(0) - T34 @ D.theta(alpha, psi)) / ths.std(0), 3))
for k, v in acc.items():
    v = np.array(v); print(f'  {k:12s} mean V-hat / theory {np.round(v.mean(0) / th["V4"], 3)}  (se {np.round(v.std(0) / np.sqrt(reps) / th["V4"], 3)})')

"""Numbers and tables of Section 7 (q = 1). Usage: gen_q1.py <run-dir> <manifest> <dev-run-dir> [real_q1.json label]...
<run-dir> holds aggregate.json, validation.json and the task files of the confirmation run. Writes gen/q1_numbers.tex,
gen/tab_q1.tex, gen/tab_q1cells.tex, gen/q1_summary.txt."""
import sys, json, glob, math, numpy as np
run, mfile, dev = sys.argv[1], sys.argv[2], sys.argv[3]; reals = list(zip(sys.argv[4::2], sys.argv[5::2]))
man = json.load(open(mfile)); agg = json.load(open(run + '/aggregate.json'))['cells']; val = json.load(open(run + '/validation.json'))
N = {}; log = []; C3 = ('person', 'firm', 'cov')
def put(k, v): N[k] = v; log.append(f'{k} = {v}')
f3 = lambda x: f'{x:.3f}'; f2 = lambda x: f'{x:.2f}'
def up(x, d): return math.ceil(x * 10 ** d - 1e-9) / 10 ** d
def dn(x, d): return math.floor(x * 10 ** d + 1e-9) / 10 ** d
def pct(x, d=0): return f'{100 * x:.{d}f}\\%'
cells = {c['name']: c for c in man['cells']}
def block(c):
    if c['role'] == 'stress': return 'b3'
    if c.get('errors') == 'skew': return 'skew'
    if c['variance'] == 'bridge': return 'bridge'
    return c['name'].split('_')[0]
B = {}
for n, c in cells.items(): B.setdefault(block(c), []).append(n)
order = ['none', 'b100', 'b30', 'b10', 'skew', 'bridge', 'b3']
label = dict(none='None', b100='$b=100$', b30='$b=30$', b10='$b=10$', skew='$b=30,10$, skewed errors', bridge='$b=30,10$, noisy bridge', b3='$b=3$ (stress test)')
def vals(names, f, comps=C3): return [f(agg[n]['comp'][k], agg[n]) for n in names for k in comps]
reps = {a['reps'] for a in agg.values()}; assert len(reps) == 1; reps = reps.pop(); put('q1.reps', f'{reps:,}')
devagg = json.load(open(dev + '/aggregate.json'))['cells']; put('q1.dev.draws', str({a['reps'] for a in devagg.values()}.pop()))
acc = [n for n, c in cells.items() if c['role'] == 'acceptance']; put('q1.acc.cells', str(len(acc))); put('q1.acc.n', str(3 * len(acc)))
cov = vals(acc, lambda x, a: x['q1_dq']['cover']); put('q1.acc.min', f3(min(cov))); put('q1.acc.max', f3(max(cov))); put('q1.acc.mean', f3(np.mean(cov)))
nw = vals(acc, lambda x, a: x['normal_dq']['cover']); put('q1.normal.worst', f3(min(nw))); put('q1.normal.acc.max', f3(max(nw)))
put('q1.acc.below93', str(sum(v < 0.93 for v in cov))); put('q1.acc.above97', str(sum(v > 0.97 for v in cov)))
sm = [agg[n]['comp']['sum']['normal_dq']['cover'] for n in acc]; put('q1.sum.min', f3(min(sm))); put('q1.sum.max', f3(max(sm)))
put('q1.sum.kappa.max', f3(up(max(agg[n]['comp']['sum']['kappa'] for n in agg), 3)))
put('q1.fail.reps', str(sum(a['failures'] for a in agg.values()))); put('q1.draws.total', f'{sum(a["reps"] for a in agg.values()):,}')
put('q1.noint.acc', str(sum(vals(acc, lambda x, a: x['q1_dq']['none'])))); put('q1.mcse', f3(math.sqrt(0.95 * 0.05 / reps)))
put('q1.design.rows', f"{round(np.mean([a['design']['rows'] for a in agg.values()]), -2):,.0f}")
put('q1.design.movers', f"{min(a['design']['mover_rows'] for a in agg.values()):,} to {max(a['design']['mover_rows'] for a in agg.values()):,}")
wb = vals([n for n in agg if cells[n]['n_bridge']], lambda x, a: x['w2_bridge']); put('q1.design.w2bridge', f'{pct(dn(min(wb), 2))} to {pct(up(max(wb), 2))}')
for b in order:
    ns = B[b]; mw = vals(ns, lambda x, a: x['max_w2']); put(f'q1.maxw2.{b}', f3(max(mw)) if max(mw) - min(mw) < 5e-4 else f'{f3(min(mw))} to {f3(max(mw))}')
    kp = vals(ns, lambda x, a: x['kappa']); put(f'q1.kappa.{b}.min', f2(dn(min(kp), 2))); put(f'q1.kappa.{b}.max', f2(up(max(kp), 2)) if max(kp) < 10 else f'{up(max(kp), 0):.0f}')
    for key, fld in (('normal', 'normal_dq'), ('q1', 'q1_dq'), ('lo', 'q1_dqlo'), ('fit', 'q1_dqfit'), ('true', 'q1_true'), ('or', 'q1_or')):
        v = vals(ns, lambda x, a: x[fld]['cover']); put(f'q1.{key}.{b}.min', f3(min(v))); put(f'q1.{key}.{b}.max', f3(max(v))); put(f'q1.{key}.{b}.mean', f3(np.mean(v)))
    ln = vals(ns, lambda x, a: x['q1_dq']['len_mean'] / x['normal_dq']['len_mean']); put(f'q1.len.{b}.min', f2(min(ln))); put(f'q1.len.{b}.max', f2(max(ln))); put(f'q1.len.{b}.mean', f2(np.mean(ln)))
    lt = vals(ns, lambda x, a: x['q1_dq']['len_mean'] / x['q1_true']['len_mean']); put(f'q1.lentrue.{b}.min', f2(min(lt))); put(f'q1.lentrue.{b}.max', f2(max(lt))); put(f'q1.lentrue.{b}.mean', f2(np.mean(lt)))
    ll = vals(ns, lambda x, a: x['q1_dq']['len_mean'] / x['q1_dqlo']['len_mean'] if x['q1_dqlo']['len_mean'] == x['q1_dqlo']['len_mean'] else np.nan); put(f'q1.lenlo.{b}.mean', f2(np.nanmean(ll))); put(f'q1.lenlo.{b}.max', f2(np.nanmax(ll)))
    fl = vals(ns, lambda x, a: x['q1_dq']['fitted_larger'] / a['reps']); put(f'q1.fitlarger.{b}.min', pct(min(fl))); put(f'q1.fitlarger.{b}.max', pct(max(fl)))
    npz = vals(ns, lambda x, a: x['Sigma_dq']['bb_leaveout_nonpos'] / a['reps']); put(f'q1.lononpos.{b}.max', pct(max(npz), 1))
    lon = vals(ns, lambda x, a: x['q1_dqlo']['none'] / a['reps']); put(f'q1.lonone.{b}.max', pct(max(lon), 1))
    no = vals(ns, lambda x, a: x['q1_dq']['none'] / a['reps']); put(f'q1.noint.{b}.max', pct(max(no), 1))
    st = vals(ns, lambda x, a: x['power_steps']); put(f'q1.steps.{b}', f'{min(st)} to {max(st)}' if min(st) != max(st) else str(st[0]))
    for key, f in (('bb', lambda x, a: x['Sigma_dq']['bb_leaveout']), ('bbfit', lambda x, a: x['Sigma_dq']['bb_fitted']), ('rr', lambda x, a: x['Sigma_dq']['rr']), ('br', lambda x, a: x['Sigma_dq']['br_bias']),
                   ('bbcv', lambda x, a: x['Sigma_dq']['bb_leaveout_cv']), ('b1sd', lambda x, a: abs(x['b1_over_sd']))):
        v = vals(ns, f); put(f'q1.cal.{key}.{b}.min', f2(min(v))); put(f'q1.cal.{key}.{b}.max', f2(max(v)))
    sn = [agg[n]['comp']['sum']['normal_dq']['cover'] for n in ns]; put(f'q1.sumnormal.{b}.min', f3(min(sn))); put(f'q1.sumnormal.{b}.max', f3(max(sn)))
stb = vals([n for n in agg if cells[n]['n_bridge']], lambda x, a: x['power_steps']); put('q1.steps.bottleneck', f'{min(stb)} to {max(stb)}')
put('q1.verdict', 'All tolerances were met.' if val['passed'] else f"Not all tolerances were met: {len(val['failures'])} of the checks failed."); put('q1.nfail', str(len(val['failures']))); log.extend('FAILURE ' + f for f in val['failures'])
# the failed length tolerance, and other details quoted in the text
sys.path.insert(0, '../../longpanel/code')
from lp_q1 import am_critical
none = [n for n in acc if cells[n]['n_bridge'] is None]
lr = sorted(((agg[n]['comp'][k]['q1_dq']['len_mean'] / agg[n]['comp'][k]['normal_dq']['len_mean'], n, k) for n in none for k in C3), reverse=True)
put('q1.fail.len', f'{100 * (lr[0][0] - 1):.1f}\\%'); xf = agg[lr[0][1]]['comp'][lr[0][2]]; put('q1.fail.comp', lr[0][2]); put('q1.fail.cell', lr[0][1].replace('_', '-'))
put('q1.fail.kappa', f2(xf['kappa'])); put('q1.fail.maxw2', f2(xf['max_w2'])); put('q1.fail.z', f2(am_critical(xf['kappa']))); put('q1.fail.zpct', f'{100 * (am_critical(xf["kappa"]) / 1.959964 - 1):.1f}\\%')
put('q1.fail.truelen', f'{100 * (xf["q1_true"]["len_mean"] / xf["normal_dq"]["len_mean"] - 1):.0f}\\%'); put('q1.fail.cover', f3(xf['q1_dq']['cover'])); put('q1.fail.normalcover', f3(xf['normal_dq']['cover']))
put('q1.none.len.others', f'{lr[-1][0]:.2f} and {lr[1][0]:.2f}'); put('q1.none.n', str(len(lr)))
tr = vals(acc, lambda x, a: x['q1_true']['cover']); put('q1.true.acc.min', f3(min(tr))); put('q1.true.acc.max', f3(max(tr)))
se = lambda names: vals(names, lambda x, a: x['normal_dq']['ratio'])
v = se(B['bridge']); put('q1.se.bridge', f'{f2(dn(min(v), 2))} to {f2(up(max(v), 2))}')
pg = [n for n in acc if cells[n]['variance'] == 'profile' and cells[n]['gap'] > 0]; v = se(pg); put('q1.se.profilegap', f'{f2(dn(min(v), 2))} to {f2(up(max(v), 2))}'); put('q1.se.profilegap.n', str(len(pg)))
v = vals(pg, lambda x, a: x['Sigma_dq']['bb_fitted']); put('q1.bbfit.profilegap', f'{pct(dn(1 - max(v), 2))} to {pct(up(1 - min(v), 2))}')
v = vals(B['bridge'], lambda x, a: x['Sigma_dq']['bb_fitted']); put('q1.bbfit.bridge', f'{f2(min(v))} to {f2(max(v))}')
other = [n for n in acc if n not in pg and n not in B['bridge']]; v = se(other); put('q1.se.other', f'{f2(dn(min(v), 2))} to {f2(up(max(v), 2))}')
v = [agg[n]['comp']['sum']['normal_dq']['ratio'] for n in acc]; put('q1.se.sum', f'{f2(dn(min(v), 2))} to {f2(up(max(v), 2))}')
v = vals(B['b10'] + [n for n in B['skew'] + B['bridge'] if n.startswith('b10')], lambda x, a: x['q1_dqlo']['none'] / a['reps']); put('q1.lonone.b10all', f'{pct(dn(min(v), 2))} to {pct(up(max(v), 2))}')
v = vals(B['b10'] + [n for n in B['skew'] + B['bridge'] if n.startswith('b10')], lambda x, a: x['q1_dqlo']['cover']); put('q1.lo.b10all', f'{f3(min(v))} to {f3(max(v))}')
ge30 = [n for n in acc if not n.startswith('b10')]; v = vals(ge30, lambda x, a: x['q1_dqlo']['cover']); put('q1.lo.ge30', f'{f3(min(v))} to {f3(max(v))}')
v = vals(acc, lambda x, a: x['Sigma_dq']['bb_leaveout']); put('q1.cal.bb.acc', f'{f2(min(v))} to {f2(max(v))}')
v = vals(acc, lambda x, a: x['Sigma_dq']['rr']); put('q1.cal.rr.acc', f'{f2(min(v))} to {f2(max(v))}')
v = vals(acc, lambda x, a: abs(x['Sigma_dq']['br_bias'])); put('q1.cal.br.acc', f2(up(max(v), 2)))
v = vals(acc, lambda x, a: x['mc_var_over_V']); put('q1.mcvar.acc', f'{f2(min(v))} to {f2(max(v))}')
for b in ('b100', 'b30', 'b10'):
    names = [n for n in acc if n.startswith(b + '_')]
    v = vals(names, lambda x, a: x['q1_dq']['len_mean'] / x['q1_true']['len_mean']); put(f'q1.lentrue.all.{b}', f'{f2(np.mean(v))}'); put(f'q1.lentrue.all.{b}.max', f2(max(v)))
    v = vals(names, lambda x, a: x['Sigma_dq']['bb_leaveout_cv']); put(f'q1.bbcv.all.{b}', f'{f2(min(v))} to {f2(max(v))}')
put('q1.lentrue.acc.max', f2(max(vals(acc, lambda x, a: x['q1_dq']['len_mean'] / x['q1_true']['len_mean']))))
put('q1.len.acc.max', f2(max(vals(acc, lambda x, a: x['q1_dq']['len_mean'] / x['normal_dq']['len_mean']))))
# normal interval by curvature
bins = [(0, 0.2), (0.2, 0.6), (0.6, 2), (2, 6), (6, 1e9)]
for lo, hi in bins:
    v = [(x['normal_dq']['cover'], x['q1_dq']['cover']) for n in acc for x in (agg[n]['comp'][k] for k in C3) if lo <= x['kappa'] < hi]
    tag = f'{lo}'.replace('.', 'p'); put(f'q1.bykappa.{tag}.n', str(len(v)))
    if v: put(f'q1.bykappa.{tag}.normal', f'{f3(min(a for a, _ in v))} to {f3(max(a for a, _ in v))}'); put(f'q1.bykappa.{tag}.q1', f'{f3(min(b for _, b in v))} to {f3(max(b for _, b in v))}')
# screening curvature from the task files
scr = {}
for n in agg:
    recs = [json.load(open(p)) for p in sorted(glob.glob(f'{run}/{n}/task-*.json'))]; lam = recs[0]['lam1']
    for c, k in enumerate(C3):
        kt = [2 * abs(lam[c]) * max(r['S_dq'][i][c][0][0], r['bbfit_dq'][i][c]) / math.sqrt(r['V_dq'][i][c]) for r in recs for i in range(r['reps_done']) if r['V_dq'][i][c] > 0]
        kh = [r['ci_dq'][i][c][2] for r in recs for i in range(r['reps_done']) if r['ci_dq'][i][c] is not None]
        scr[(n, k)] = (float(np.median(kt)), float(np.median(kh)), agg[n]['comp'][k]['kappa'], float(np.mean(np.array(kt) < 0.1)), agg[n]['comp'][k]['normal_dq']['cover'])
rat = [v[0] / v[2] for v in scr.values()]; put('q1.screen.ratio.min', f2(min(rat))); put('q1.screen.ratio.max', f2(max(rat)))
rat2 = [v[1] / v[2] for v in scr.values()]; put('q1.kappahat.ratio.min', f2(min(rat2))); put('q1.kappahat.ratio.max', f2(max(rat2)))
low = [v for (n, k), v in scr.items() if cells[n]['role'] == 'acceptance' and v[0] < 0.1]; high = [v for (n, k), v in scr.items() if cells[n]['role'] == 'acceptance' and v[0] >= 0.1]
put('q1.screen.low.n', str(len(low))); put('q1.screen.low.normal.min', f3(min(v[4] for v in low)) if low else '--'); put('q1.screen.high.n', str(len(high))); put('q1.screen.high.normal.min', f3(min(v[4] for v in high)) if high else '--')
# correlation of b1-hat and the remainder: truth, and the leave-out matrix draw by draw
put('q1.rho.true.max', f2(up(max(abs(agg[n]['comp'][k]['corr']) for n in agg for k in C3), 2)))
worst = 0.0
for n in agg:
    recs = [json.load(open(p_)) for p_ in sorted(glob.glob(f'{run}/{n}/task-*.json'))]
    for c in range(3):
        S = np.array([r['S_dq'][i][c] for r in recs for i in range(r['reps_done'])]); ok = (S[:, 0, 0] > 0) & (S[:, 1, 1] > 0)
        with np.errstate(divide='ignore', invalid='ignore'): r2 = np.where(ok, S[:, 0, 1] ** 2 / (S[:, 0, 0] * S[:, 1, 1]), 0.0)
        worst = max(worst, float((r2 > 2 / 3).mean()))
put('q1.rholo.above', pct(worst, 1))
bp = json.load(open('../../longpanel/results/q1_conf1/benchmark_precision.json'))
put('q1.bench.srr', pct(up(max(max(v['Srr_rel_sd']) for v in bp.values()), 3), 1)); put('q1.bench.v', pct(up(max(max(v['V_rel_sd'][:3]) for v in bp.values()), 2), 0))
# ---- draw-level statistics (recomputed from the task files) ----
ZQ = 1.959963984540054
def load(n):
    recs = [json.load(open(p_)) for p_ in sorted(glob.glob(f'{run}/{n}/task-*.json'))]
    g = lambda key: [x for r in recs for x in r[key]]
    th = np.array(g('theta')); V = np.array(g('V_dq')); out = dict(theta=th, V=V, truth=np.array(recs[0]['theta_true']))
    for key in ('ci_dq', 'ci_true', 'cilo_dq'):
        ci = g(key); out[key] = np.array([[(q[0], q[1]) if q is not None else (np.nan, np.nan) for q in row[:3]] for row in ci])
    return out
DR = {n: load(n) for n in agg}
LRN = {}; LRT = {}; VNEG = {}; NCOND = {}
for n, d in DR.items():
    for c, k in enumerate(C3):
        okn = d['V'][:, c] > 0; se = np.sqrt(np.where(okn, d['V'][:, c], np.nan)); nlen = 2 * ZQ * se
        q = d['ci_dq'][:, c]; okq = ~np.isnan(q[:, 0]); both = okn & okq
        LRN[(n, k)] = float((q[both, 1] - q[both, 0]).mean() / nlen[both].mean())
        t = d['ci_true'][:, c]; bt = okq & ~np.isnan(t[:, 0]); LRT[(n, k)] = float((q[bt, 1] - q[bt, 0]).mean() / (t[bt, 1] - t[bt, 0]).mean())
        VNEG[(n, k)] = float((~okn).mean())
        cov = (np.abs(d['theta'][:, c] - d['truth'][c]) <= ZQ * se); NCOND[(n, k)] = float(cov[okn].mean())
accc = [(n, k) for n in acc for k in C3]
put('q1.vneg.max', pct(up(max(VNEG[x] for x in accc), 3), 1)); put('q1.vneg.cases', str(sum(VNEG[x] > 0 for x in accc))); put('q1.vneg.cases10', str(sum(VNEG[x] > 0.10 for x in accc)))
worst = min(accc, key=lambda x: agg[x[0]]['comp'][x[1]]['normal_dq']['cover']); put('q1.vneg.worst', pct(VNEG[worst], 1)); put('q1.normal.worst.cond', f3(NCOND[worst]))
put('q1.normal.cond.min', f3(dn(min(NCOND[x] for x in accc), 3)))
put('q1.vneg.sum', str(int(sum((DR[n]['V'][:, 3] <= 0).sum() for n in agg))))
put('q1.vneg.lowkappa', pct(max(VNEG[x] for x in accc if agg[x[0]]['comp'][x[1]]['kappa'] < 0.6), 1))
for b in order:
    v = [LRN[(n, k)] for n in B[b] for k in C3]; N[f'q1.len.{b}.mean'] = f2(np.mean(v)); N[f'q1.len.{b}.max'] = f2(up(max(v), 2)); N[f'q1.len.{b}.min'] = f2(dn(min(v), 2))
    v = [LRT[(n, k)] for n in B[b] for k in C3]; N[f'q1.lentrue.{b}.mean'] = f2(np.mean(v)); N[f'q1.lentrue.{b}.max'] = f2(up(max(v), 2))
put('q1.lentrue.acc.max', f2(up(max(LRT[x] for x in accc), 2))); put('q1.len.acc.max', f2(up(max(LRN[x] for x in accc), 2)))
for b in ('b100', 'b30', 'b10'):
    v = [LRT[(n, k)] for n in B[b] for k in C3]; N[f'q1.lentrue.all.{b}'] = f2(np.mean(v))
# ranges quoted in the text: three decimals, rounded outwards
r3 = lambda v, d=3: f'{dn(min(v), d):.{d}f} to {up(max(v), d):.{d}f}'
none9 = sorted(LRN[(n, k)] for n in none for k in C3)
put('q1.none.len.others', r3(none9[:-1]).replace(' to ', ' and '))
put('q1.se.sum', r3([agg[n]['comp']['sum']['normal_dq']['ratio'] for n in agg])); put('q1.se.bridge', r3(vals(B['bridge'], lambda x, a: x['normal_dq']['ratio'])))
put('q1.cal.bb.acc', r3(vals(acc, lambda x, a: x['Sigma_dq']['bb_leaveout']))); put('q1.cal.rr.acc', r3(vals(acc, lambda x, a: x['Sigma_dq']['rr'])))
put('q1.cal.br.acc', f3(up(max(vals(acc, lambda x, a: abs(x['Sigma_dq']['br_bias']))), 3)))
for b in ('b100', 'b10'):
    names = [n for n in acc if n.startswith(b + '_')]; put(f'q1.bbcv.all.{b}', r3(vals(names, lambda x, a: x['Sigma_dq']['bb_leaveout_cv'])))
put('q1.bbfit.bridge', r3(vals(B['bridge'], lambda x, a: x['Sigma_dq']['bb_fitted'])))
wbv = vals([n for n in agg if cells[n]['n_bridge']], lambda x, a: x['w2_bridge']); put('q1.design.w2bridge', f'{100 * dn(min(wbv), 3):.1f}\\% to {100 * up(max(wbv), 3):.1f}\\%')
put('q1.screen.ratio.min', f3(dn(min(v[0] / v[2] for v in scr.values()), 3))); put('q1.screen.ratio.max', f3(up(max(v[0] / v[2] for v in scr.values()), 3)))
g0 = [abs(agg[n]['comp'][k]['b1_over_sd']) for n in acc if cells[n]['n_bridge'] and cells[n]['gap'] == 0 for k in C3]; g3 = [abs(agg[n]['comp'][k]['b1_over_sd']) for n in acc if cells[n]['n_bridge'] and cells[n]['gap'] > 0 for k in C3]
put('q1.b1sd.gap0', f'{dn(min(g0), 1):.1f} to {up(max(g0), 1):.1f}'); put('q1.b1sd.gap3', f'{dn(min(g3), 1):.1f} to {up(max(g3), 0):.0f}')
for b in ('b100', 'b30', 'b10'): put(f'q1.neff.{b}', f"{1 / max(vals(B[b], lambda x, a: x['max_w2'])):.0f}")
kap = lambda b: vals(B[b], lambda x, a: x['kappa'])
put('q1.kappa.bridge.max', f"{max(vals(B['bridge'], lambda x, a: x['kappa'])):.0f}")
kf_ = lambda v: f'{v:.2f}' if v < 9.995 else f'{v:.0f}'
for b in order: N[f'q1.kappa.{b}.min'] = kf_(min(kap(b))); N[f'q1.kappa.{b}.max'] = kf_(max(kap(b)))
# summary table
rows = [r'\begin{tabular}{@{}l c c c c c c c c@{}}', r'\toprule', r' & & & \multicolumn{4}{c}{Coverage (smallest to largest)} & \multicolumn{2}{c}{Length of $q=1$} \\ \cmidrule(lr){4-7}\cmidrule(l){8-9}',
        r'Bottleneck & $\max_iw_{i1}^2$ & $\kappa$ & Normal & $q=1$ & Leave-out & True $\Sigma$ & /normal & /true $\Sigma$ \\', r'\midrule']
for b in order:
    if b == 'skew': rows.append(r'\addlinespace')
    r_ = lambda key: f"{N[f'q1.{key}.{b}.min'][1:]}--{N[f'q1.{key}.{b}.max'][1:]}".replace('--.000', '--1.00') if N[f'q1.{key}.{b}.max'] != '1.000' else f"{N[f'q1.{key}.{b}.min'][1:]}--1.00"
    mw = N[f'q1.maxw2.{b}'].replace('0.', '.').replace(' to ', '--')
    rows.append(f"{label[b]} & {mw} & {N[f'q1.kappa.{b}.min']}--{N[f'q1.kappa.{b}.max']} & {r_('normal')} & {r_('q1')} & {r_('lo')} & {r_('true')} & {N[f'q1.len.{b}.mean']} & {N[f'q1.lentrue.{b}.mean']} \\\\")
rows += [r'\bottomrule', r'\end{tabular}']; open('gen/tab_q1.tex', 'w').write('\n'.join(rows) + '\n')
# per-cell table
vl = dict(flat='flat', profile='profile', movers='movers', bridge='bridge')
rows = [r'\begin{tabular}{@{}l l l l ccc ccc ccc c c c@{}}', r'\toprule',
        r' & & & & \multicolumn{3}{c}{$\kappa$} & \multicolumn{3}{c}{Normal} & \multicolumn{3}{c}{$q=1$} & Sum & Fitted & Length \\ \cmidrule(lr){5-7}\cmidrule(lr){8-10}\cmidrule(lr){11-13}',
        r'$b$ & Gap & Variances & Errors & $\alpha$ & $\psi$ & cov & $\alpha$ & $\psi$ & cov & $\alpha$ & $\psi$ & cov & normal & larger & ratio \\', r'\midrule']
for n, c in cells.items():
    a = agg[n]; x = a['comp']; kf = lambda v: f'{v:.2f}' if v < 10 else f'{v:.0f}'
    if n in ('none_flat', 'b10_g3_profile_skew', 'b3_g0_profile', 'b30_g0_flat', 'b100_g0_flat'): rows.append(r'\addlinespace')
    rows.append(' & '.join(['--' if c['n_bridge'] is None else n.split('_')[0][1:], f"{c['gap']:.1f}", vl[c['variance']], 'skewed' if c.get('errors') == 'skew' else 'normal']
                           + [kf(x[k]['kappa']) for k in C3] + [f"{x[k]['normal_dq']['cover']:.3f}"[1:] for k in C3] + [f"{x[k]['q1_dq']['cover']:.3f}".replace('0.', '.').replace('1.000', '1.00') for k in C3]
                           + [f"{x['sum']['normal_dq']['cover']:.3f}"[1:], f"{math.floor(100 * np.mean([x[k]['q1_dq']['fitted_larger'] for k in C3]) / a['reps'] + 0.5 + 1e-9):.0f}\\%", f"{np.mean([LRN[(n, k)] for k in C3]):.2f}"]) + r' \\')
rows += [r'\bottomrule', r'\end{tabular}']; open('gen/tab_q1cells.tex', 'w').write('\n'.join(rows) + '\n')
# real designs
for path, lab in reals:
    r = json.load(open(path)); put(f'q1.real.{lab}.rows', f"{r['rows']:,}"); put(f'q1.real.{lab}.seconds', f"{r['total_seconds']:.0f}"); put(f'q1.real.{lab}.solves.se', str(r['solves_standard_error']))
    for k in ('person', 'firm', 'cov', 'sum'):
        x = r['comp'][k]; p = f'q1.real.{lab}.{k}.'
        put(p + 'share', pct(x['eigen_share'], 1) if x['eigen_share'] < 0.1 else pct(x['eigen_share'])); put(p + 'maxw2', f"{x['max_w2']:.3f}" if x['max_w2'] >= 0.0005 else f"{x['max_w2']:.1e}"); put(p + 'steps', str(x['power_steps'])); put(p + 'converged', str(x['converged']))
        put(p + 'neff', f"{x['inverse_sum_w4']:,.0f}"); put(p + 'half', f"{x['rows_for_half_of_w2']:,}"); put(p + 'kscreen', f"{x['kappa_screen']:.3f}"); put(p + 'solves', str(x['solves_q1'])); put(p + 'b1se', f"{x['b1_over_se']:.1f}")
        put(p + 'lo.nonpos', str(x['Sigma_leave_out'][0][0] <= 0)); put(p + 'bbratio', f"{x['Sigma_leave_out'][0][0] / x['Sbb_fitted']:.2f}")
        if x['q1'] is not None:
            put(p + 'kappa', f"{x['q1']['kappa']:.3f}"); put(p + 'z', f"{x['q1']['z']:.3f}"); put(p + 'larger', x['q1']['larger'])
            put(p + 'q1', f"[{x['q1']['lo']:.5f}, {x['q1']['hi']:.5f}]"); put(p + 'normal', f"[{x['normal']['lo']:.5f}, {x['normal']['hi']:.5f}]")
            put(p + 'lenratio', f"{(x['q1']['hi'] - x['q1']['lo']) / (x['normal']['hi'] - x['normal']['lo']):.3f}")
if reals:
    R = {lab: json.load(open(path)) for path, lab in reals}; names = dict(six='Six years', half='Half of the workers', y3='Three years'); cn = dict(person=r'$\sigma^2_\alpha$', firm=r'$\sigma^2_\psi$', cov=r'$\sigma_{\alpha,\psi}$')
    allx = [(lab, k, R[lab]['comp'][k]) for lab in R for k in C3]
    rng = lambda f, fmt: f'{fmt(min(f(x) for _, _, x in allx))} to {fmt(max(f(x) for _, _, x in allx))}'
    sh = [x['eigen_share'] for _, _, x in allx]; put('q1.real.share', f'{100 * dn(min(sh), 4):.2f}\\% to {100 * up(max(sh), 4):.2f}\\%'); put('q1.real.kappa', rng(lambda x: x['q1']['kappa'], f2)); put('q1.real.maxw2', rng(lambda x: x['max_w2'], f3))
    lr_ = [(x['q1']['hi'] - x['q1']['lo']) / (x['normal']['hi'] - x['normal']['lo']) for _, _, x in allx]; put('q1.real.lenratio', r3(lr_)); put('q1.real.half', rng(lambda x: x['rows_for_half_of_w2'], lambda v: f'{v:,}'))
    put('q1.real.steps', rng(lambda x: x['power_steps'], str)); put('q1.real.bbratio', rng(lambda x: x['Sigma_leave_out'][0][0] / x['Sbb_fitted'], f2)); put('q1.real.fitted.n', str(sum(x['q1']['larger'] == 'fitted' for _, _, x in allx))); put('q1.real.n', str(len(allx)))
    put('q1.real.unconverged', str(sum(not x['converged'] for _, _, x in allx))); put('q1.real.power.seconds', rng(lambda x: x['seconds_power'], lambda v: f'{v:.0f}')); ms = max(R[lab]['comp']['sum']['eigen_share'] for lab in R); assert ms < 1e-5; put('q1.real.sum.share', 'below 0.001\\%')
    rows = [r'\begin{tabular}{@{}l l c c c c c c c@{}}', r'\toprule', r'Design & & $\lambda_1^2/\sum_\ell\lambda_\ell^2$ & $\max_iw_{i1}^2$ & Steps & $\hat\kappa$ & Normal interval & $q=1$ interval & Ratio \\', r'\midrule']
    for lab in ('six', 'half', 'y3'):
        if lab not in R: continue
        for j, k in enumerate(C3):
            x = R[lab]['comp'][k]; q = x['q1']; n_ = x['normal']
            rows.append(f"{names[lab] if j == 0 else ''} & {cn[k]} & {100 * x['eigen_share']:.1f}\\% & {x['max_w2']:.3f} & {x['power_steps']} & {q['kappa']:.2f} & [{n_['lo']:.4f}, {n_['hi']:.4f}] & [{q['lo']:.4f}, {q['hi']:.4f}] & {(q['hi'] - q['lo']) / (n_['hi'] - n_['lo']):.3f} \\\\")
        if lab != 'y3': rows.append(r'\addlinespace')
    rows += [r'\bottomrule', r'\end{tabular}']; open('gen/tab_q1real.tex', 'w').write('\n'.join(rows) + '\n')
open('gen/q1_numbers.tex', 'w').write('\n'.join(f'\\setres{{{k}}}{{{v}}}' for k, v in N.items()) + '\n')
open('gen/q1_summary.txt', 'w').write('\n'.join(log) + '\n\nscreening (cell, comp): median kappa-tilde, median kappa-hat, true kappa, share of draws kappa-tilde < 0.1, normal coverage\n'
                                      + '\n'.join(f'{n} {k}: {v[0]:.3f} {v[1]:.3f} {v[2]:.3f} {v[3]:.2f} {v[4]:.3f}' for (n, k), v in scr.items()) + '\n')
print('wrote', len(N), 'numbers;', 'validation passed' if val['passed'] else 'validation FAILED')

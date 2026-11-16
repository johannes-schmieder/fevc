"""Generate every number, table and figure coordinate of the note from the result files, for the chosen movers' model.
Usage: gen2.py <model> <real7.json> <controls_lcs3.json> <mc-v6 aggregate> <mc-v4 aggregate> <mc-v5 aggregate> <check6.json> <mc-v1 aggregate>
mc-v6 holds the candidates of round 6 (same data draws as rounds 4 and 5); the oracle ("movers' true variances")
comes from rounds 4 (scenarios 1-9) and 5 (scenario 10).  Writes gen/*.tex and gen/summary.txt, gen/detail.txt."""
import sys, json, numpy as np
name = sys.argv[1]; real = json.load(open(sys.argv[2])); ctrl = json.load(open(sys.argv[3]))
v6 = json.load(open(sys.argv[4]))['cells']; v4 = json.load(open(sys.argv[5]))['cells']; v5 = json.load(open(sys.argv[6]))['cells']
chk = json.load(open(sys.argv[7])); old = json.load(open(sys.argv[8]))['cells']
N = {}; log = []
def put(k, v): N[k] = v; log.append(f'{k} = {v}')
pct = lambda x, d=0: f'{100 * x:.{d}f}\\%'
import math
cpct = lambda x, d=0: f'{math.ceil(100 * x * 10 ** d - 1e-9) / 10 ** d:.{d}f}\\%'   # rounded up, for "at most" statements
cdec = lambda x, d: f'{math.ceil(x * 10 ** d - 1e-9) / 10 ** d:.{d}f}'
spct = lambda x: f'{x:+.0%}'.replace('%', '\\%')
f2 = lambda x: f'{x:.2f}'
comp = ('alpha', 'psi', 'cov', 'sum'); assert real['model'] == name
# ---------------- real data
th = np.array(real['theta']); runs = real['runs']; se = np.array(runs[0]['se'])
for c, k in enumerate(comp):
    put(f'real.theta.{k}', f'{th[c]:.4f}'); put(f'real.se.{k}', f'{se[c] * 1e4:.2f}'); put(f'real.relse.{k}', pct(se[c] / th[c], 2)); put(f'real.plugin.{k}', f'{real["plugin"][c]:.4f}')
put('real.rel_se_sum', pct(se[3] / th[3], 2)); put('real.relse.min', pct((se / th).min(), 2)); put('real.relse.max', pct((se / th).max(), 2))
allse = np.array([r['se'] for r in runs]); put('real.run_maxdiff', pct(np.abs(allse / allse[0] - 1).max(), 1))
put('real.se_solves', f'{runs[1]["solves"]}'); put('real.se_seconds', f'{runs[1]["seconds"]:.0f}'); put('real.factor_minutes', f'{(runs[0]["seconds"] - runs[1]["seconds"]) / 60:.0f}')
put('real.probe_solves', f'{3 * real["probes"]:,}'.replace(',', '{,}'))
md = real['models']; sp = real['stayer_profile']
put('real.floor.scale', pct(md['scale_floored'], 2)); put('real.floor.profile', pct(md.get('stayer_profile_floored', 0.0), 2)); put('real.floor.final', pct(md[name + '_floored'], 2))
put('real.load.c0', f2(md['load'][0])); put('real.load.c1', f2(md['load'][1])); put('real.profile_r2', f'{sp["r2"]:.3f}')
put('real.capped.stayers', pct(sp['share_capped_person'], 1)); put('real.capped.movers', pct(real['share_capped_mover_rows']['person'], 1))
put('real.capped.first_term.sum', pct(real['first_term_capped_rows'][3])); put('real.capped.first_term.max', pct(max(real['first_term_capped_rows'])))
for j, nm in enumerate(('one', 'a', 'aa', 'p', 'pp', 'ap')): put(f'real.coef.{nm}', f2(sp['coef_relative'][j]))
sh = real['share_first_term']
for c, k in enumerate(comp):
    put(f'real.share.movers.{k}', pct(sh['movers'][c])); put(f'real.share.stayers.{k}', pct(1 - sh['movers'][c]))
    put(f'real.first_over_V.{k}', pct(real['first_term_over_V'][c])); put(f'real.adjust_over_V.{k}', pct(real['adjust_over_V'][c], 2))
put('real.adjust_max', cpct(max(abs(x) for x in real['adjust_over_V']), 2))
dg = real['diagnostics']
put('real.max_stayer_share', pct(max(dg['max_stayer_share']), 1)); put('real.max_mover_share', pct(max(dg['max_mover_row_share']), 1))
put('real.d2.min', f'{min(dg["D2"]):.4f}'); put('real.d2.max', f'{max(dg["D2"]):.4f}')
reff = (np.array(dg['double_sum_over_V']) / 2 / np.array(dg['D2'])) ** 2
put('real.reff.min', f'{round(reff.min(), -2):,.0f}'.replace(',', '{,}')); put('real.reff.max', f'{round(reff.max(), -2):,.0f}'.replace(',', '{,}'))
put('real.lambda.min', f'{min(dg["lambda2_over_V"]):.4f}'); put('real.lambda.max', f'{max(dg["lambda2_over_V"]):.4f}')
for k in ('rows', 'mover_rows', 'stayer_rows', 'person_years', 'firms', 'workers', 'stayers', 'movers'): put(f'real.n.{k}', f'{real[k]:,}'.replace(',', '{,}'))
put('real.share.stayer_rows', pct(real['stayer_rows'] / real['rows'])); put('real.share.stayer_py', pct(real['stayer_rows'] / real['person_years']))
wm = real['within_match_overall']; put('real.within_match_mean', f'{wm["mean"]:.3f}'); put('real.stayers_mean', f'{wm["stayers_mean"]:.3f}')
put('real.within_match_matches', f'{wm["matches"]:,}'.replace(',', '{,}'))
ml = real['movers_by_length']; lo = [ml[k]['leave_out_product'] for k in sorted(ml, key=int)]
put('real.lo.len1', f'{lo[0]:.3f}'); put('real.lo.longer.min', f'{min(lo[1:]):.3f}'); put('real.lo.longer.max', f'{max(lo[1:]):.3f}')
rows = [('Variance of person effects, $\\sigma^2_\\alpha$', 0), ('Variance of firm effects, $\\sigma^2_\\psi$', 1), ('Covariance, $\\sigma_{\\alpha,\\psi}$', 2), ('Variance of the sum, $\\sigma^2_{\\alpha+\\psi}$', 3)]
open('gen/tab_real.tex', 'w').write('\n'.join(f'{lab} & {real["plugin"][c]:.4f} & {th[c]:.4f} & {se[c] * 1e4:.2f} & {100 * se[c] / th[c]:.2f} & {100 * real["first_term_over_V"][c]:.0f} & {100 * sh["movers"][c]:.0f} \\\\' for lab, c in rows) + '\n')
# ---------------- profile figure
P = real['profiles']; out = []
coords = lambda x, y: ' '.join(f'({a:.3f},{b:.3f})' for a, b in zip(x, y))
for lab in ('alpha', 'psi'):
    s = P['stayers_by_' + lab]; m = P['movers_by_' + lab]
    rs = np.array(s['sample_variance']) / wm['stayers_mean']; rf = np.array(s['fitted_profile']) / wm['stayers_mean']; rm = np.array(m['within_match_variance']) / wm['mean']
    out += [f'\\def\\prof{lab}stay{{{coords(s["mean_index"], rs)}}}', f'\\def\\prof{lab}fit{{{coords(s["mean_index"], rf)}}}', f'\\def\\prof{lab}move{{{coords(m["mean_index_matches"], rm)}}}']
    for tag, r in (('stay', rs), ('fit', rf), ('move', rm)):
        put(f'real.prof.{lab}.{tag}.lo', f2(r[0])); put(f'real.prof.{lab}.{tag}.hi', f2(r[-1])); put(f'real.prof.{lab}.{tag}.mid', f2(r[3:7].mean()))
for lab in ('alpha', 'psi'):
    m = P['movers_by_' + lab]
    out += [f'\\def\\fig{lab}lo{{{coords(m["mean_index_rows"], 100 * np.array(m["leave_out_product"]))}}}', f'\\def\\fig{lab}fit{{{coords(m["mean_index_rows"], 100 * np.array(m["fitted"]))}}}']
open('gen/fig_profile.tex', 'w').write('\n'.join(out) + '\n')
# ---------------- movers' rows by decile: data and simulations with known truth
mb = P['movers_by_alpha']; cs = chk['scenarios']; row = lambda lab, v: lab + ' & ' + ' & '.join(f'{100 * x:.1f}' for x in v) + ' \\\\'
L = ['\\multicolumn{11}{@{}l}{\\emph{Veneto panel}} \\\\', row('\\quad leave-out estimate', mb['leave_out_product']), row('\\quad fitted by the model', mb['fitted'])]
for nm, title in (('N1', 'Simulated data, scenario 1 (no profile)'), ('N6', "Simulated data, scenario 6 (a step profile shared by stayers and movers)"), ('N10', "Simulated data, scenario 10 (movers follow the decile profile of the data)")):
    if nm in cs:
        L += ['\\addlinespace', '\\multicolumn{11}{@{}l}{\\emph{' + title + '}} \\\\', row('\\quad leave-out estimate', cs[nm]['movers_leave_out']), row('\\quad true variance', cs[nm]['movers_true']), row('\\quad fitted by the model', cs[nm]['movers_fitted_' + name])]
        l_, t_, f_ = (np.array(cs[nm][k]) for k in ('movers_leave_out', 'movers_true', 'movers_fitted_' + name))
        put(f'dec.{nm}.gap.max', pct(np.abs(l_ / t_ - 1).max())); put(f'dec.{nm}.fit.maxgap', pct(np.abs(f_ / t_ - 1).max()))
        put(f'dec.{nm}.fit.bottom', spct(f_[0] / t_[0] - 1)); put(f'dec.{nm}.fit.mid', spct(f_[3:7].mean() / t_[3:7].mean() - 1)); put(f'dec.{nm}.fit.top', spct(f_[9] / t_[9] - 1))
open('gen/tab_deciles.tex', 'w').write('\n'.join(L) + '\n')
lo_, ft_ = np.array(mb['leave_out_product']), np.array(mb['fitted']); nr = np.array(mb['n_rows'], float); mean = (lo_ * nr).sum() / nr.sum()
put('dec.data.lo.bottom', f'{lo_[0] / mean:.1f}'); put('dec.data.lo.mid', f2(lo_[3:7].mean() / mean)); put('dec.data.lo.top', f'{lo_[9] / mean:.1f}')
put('dec.data.fit.bottom', spct(ft_[0] / lo_[0] - 1)); put('dec.data.fit.mid', spct(ft_[3:7].mean() / lo_[3:7].mean() - 1)); put('dec.data.fit.top', spct(ft_[9] / lo_[9] - 1)); put('dec.data.fit.maxgap', pct(np.abs(ft_ / lo_ - 1).max()))
put('dec.reps', str(chk['reps']))
gap = ft_ / lo_ - 1
put('dec.data.over.min', pct(gap[1:4].min())); put('dec.data.over.max', pct(gap[1:4].max()))
put('dec.data.under.min', pct(-gap[6:9].max())); put('dec.data.under.max', pct(-gap[6:9].min())); put('dec.data.top', spct(gap[9])); put('dec.data.bottom', spct(gap[0]))
put('dec.data.d1.lo', f'{100 * lo_[0]:.1f}'); put('dec.data.d1.fit', f'{100 * ft_[0]:.1f}')
put('dec.rows.min', pct(nr.min() / nr.sum())); put('dec.rows.max', pct(nr.max() / nr.sum()))
tab = ('N1', 'N6', 'N10')
put('dec.sim.gap.max', pct(max(np.abs(np.array(cs[k]['movers_leave_out']) / np.array(cs[k]['movers_true']) - 1).max() for k in tab)))
put('dec.sd.max', pct(np.sqrt(chk['reps'] / (chk['reps'] - 1)) * max((np.array(cs[k]['sd_movers_leave_out']) / np.array(cs[k]['movers_leave_out'])).max() for k in tab)))  # sample SD (the file stores ddof=0)
put('real.load.pibar', f2((1 - md['load'][0]) / md['load'][1]))
# ---------------- controls
cc = ctrl['components']
put('ctrl.sum', pct(cc['all:sum'][name + '_se_ratio'] - 1)); put('ctrl.others', pct(max(abs(cc[f'all:{k}'][name + '_se_ratio'] - 1) for k in ('worker', 'firm', 'cov')), 1)); put('ctrl.n.py', f'{ctrl["sample"]["person_years"] / 1e6:.2f}')
# ---------------- simulation
order = ['N1', 'N2', 'N3', 'N4', 'N5', 'N6', 'N7', 'N8', 'N9', 'N10']
labels = {'N1': 'No profile', 'N2': 'Quadratic profile, worker factors', 'N3': 'Same, fat tails', 'N4': 'Quadratic profile, skew against wage', 'N5': 'Quadratic profile, skew with wage',
          'N6': 'Profile of the data (deciles)', 'N7': 'Log-linear in the wage level', 'N8': 'Movers steeper than stayers', 'N9': 'Movers flat, stayers steep', 'N10': "Movers' own profile in the data"}
cell = {k: v6[f'{k}__default'] for k in order if f'{k}__default' in v6}
orc = {k: (v4.get(f'{k}__default') or v5.get(f'{k}__default')) for k in order}
L1, L2, L3 = [], [], []
for i, k in enumerate(order):
    if k not in cell: continue
    e = cell[k]['est'][name]; o_ = orc[k]['est']['or'] if orc[k] else None; dash = '--- & --- & --- & ---'
    if k == 'N8': L1.append('\\midrule'); L2.append('\\midrule')
    L1.append(f'{i + 1} & {labels[k]} & ' + ' & '.join(f2(x) for x in e['ratio']) + ' & & ' + (' & '.join(f2(x) for x in o_['ratio']) if o_ else dash) + ' \\\\')
    L2.append(f'{i + 1} & {labels[k]} & ' + ' & '.join(f'{x:.3f}' for x in e['coverage_true']) + ' & & ' + (' & '.join(f'{x:.3f}' for x in o_['coverage_true']) if o_ else dash) + ' \\\\')
    r = cell[k]['regime']; s_, m_ = r['ratio_by_decile_stayers'], r['ratio_by_decile_movers']
    L3.append(f"{i + 1} & {s_[0]:.2f} & {s_[4]:.2f} & {s_[9]:.2f} & {m_[0]:.2f} & {m_[4]:.2f} & {m_[9]:.2f} & {100 * r['mover_share_first_term'][3]:.0f}\\% \\\\")
open('gen/tab_sim.tex', 'w').write('\n'.join(L1) + '\n'); open('gen/tab_simcov.tex', 'w').write('\n'.join(L2) + '\n'); open('gen/tab_regime.tex', 'w').write('\n'.join(L3) + '\n')
def rg(keys, est, field, src=None):
    v = np.array([(src or cell)[k]['est'][est][field] for k in keys if k in (src or cell) and (src or cell)[k]]); return v.min(), v.max()
groups = dict(all=order, a4=order[:7], norm=['N1', 'N2', 'N6', 'N7'], shape=['N3', 'N4', 'N5'], viol=order[7:])
for tag, keys in groups.items():
    keys = [k for k in keys if k in cell]
    if not keys: continue
    lo, hi = rg(keys, name, 'ratio'); put(f'sim.{tag}.min', f2(lo)); put(f'sim.{tag}.max', f2(hi))
    lo, hi = rg(keys, name, 'coverage_true'); put(f'sim.{tag}.cov.min', f'{lo:.3f}'); put(f'sim.{tag}.cov.max', f'{hi:.3f}')
    lo, hi = rg(keys, 'or', 'ratio', orc); put(f'sim.{tag}.or.min', f2(lo)); put(f'sim.{tag}.or.max', f2(hi))
for k in cell:
    for c, cn in enumerate(comp):
        put(f'sim.short.{k}.{cn}', pct(1 - cell[k]['est'][name]['ratio'][c])); put(f'sim.{k}.{cn}', f2(cell[k]['est'][name]['ratio'][c])); put(f'sim.{k}.cover.{cn}', f2(cell[k]['est'][name]['coverage_true'][c]))
        if orc[k]: put(f'sim.{k}.or.{cn}', f2(orc[k]['est']['or']['ratio'][c]))
if cell:
    R_ = np.array([cell[k]['est'][name]['ratio'] for k in cell]); C_ = np.array([cell[k]['est'][name]['coverage_true'] for k in cell]); ks = list(cell)
    for c, cn in enumerate(comp): put(f'sim.comp.{cn}.min', f2(R_[:, c].min())); put(f'sim.comp.{cn}.max', f2(R_[:, c].max()))
    i, j = np.unravel_index(R_.argmin(), R_.shape); put('sim.argmin', f'{ks[i]} {comp[j]} {R_[i, j]:.3f}')
    i, j = np.unravel_index(R_.argmax(), R_.shape); put('sim.argmax', f'{ks[i]} {comp[j]} {R_[i, j]:.3f}')
    put('sim.over.max', pct(R_.max() - 1)); put('sim.under.max', pct(1 - R_.min()))
    put('sim.ratio.mcse.max', cdec(max((0.5 * np.array(cell[k]['est'][name]['mean_var_se']) / np.array(cell[k]['est'][name]['ratio'])).max() for k in cell), 3))
    put('sim.n.cells', str(R_.size)); put('sim.n.within5', str(int((np.abs(R_ - 1) <= 0.05).sum()))); put('sim.n.below95', str(int((R_ < 0.95).sum()))); put('sim.n.above105', str(int((R_ > 1.05).sum())))
    put('sim.cov.n.below93', str(int((C_ < 0.93).sum()))); put('sim.cov.n.below92', str(int((C_ < 0.92).sum())))
    i, j = np.unravel_index(C_.argmin(), C_.shape); put('sim.cov.argmin', f'{ks[i]} {comp[j]} {C_[i, j]:.3f}')
    put('sim.reps', str(min(cell[k]['reps'] for k in cell))); put('sim.failures', str(sum(cell[k]['failures'] for k in cell))); put('sim.cells', str(len(cell)))
    b = np.array([cell[k]['bias_over_sd'] for k in cell]); put('sim.bias.max', f'{np.abs(b).max():.1f}'); put('sim.bias1600', f'{np.abs(b[:, 3]).max():.1f}')  # variance of the sum, like sim.bias200
    fl = [cell[k]['diag'].get(f'cells:{name}_floored', [0, 0, 0])[2] for k in cell]; put('sim.floor.final.max', pct(max(fl), 2)); put('sim.floor.profile.max', pct(max(cell[k]['diag'].get('cells:stayer_profile_floored', [0, 0, 0])[2] for k in cell), 2))
    fs = [cell[k]['diag'].get('cells:scale_floored', [0, 0, 0])[2] for k in cell]; put('sim.floor.scale.max', pct(max(fs), 1))
    mc = np.array([cell[k]['mc_var_over_theory'] for k in cell]); put('sim.mcvar.min', f2(mc.min())); put('sim.mcvar.max', f2(mc.max()))
b = np.array([old[k]['bias_over_sd'] for k in old if old[k]['independent'] and k.endswith('__default') and 'AR' not in k and 'RW' not in k]); put('sim.bias200', f'{np.abs(b[:, 3]).max():.1f}')
# ---------------- oracle error of the model at fixed characteristics (lp_bdelta.py), optional 9th argument
import os
bd = {}
if len(sys.argv) > 9:
    for k in order:
        f = f'{sys.argv[9]}/{k}__default/bdelta.json'
        if os.path.exists(f): bd[k] = json.load(open(f))
L4 = []
for i, k in enumerate(order):
    if k not in bd: continue
    x = bd[k]['models'][name]; ld = bd[k]['fit']['load']
    if k == 'N8': L4.append('\\midrule')
    sim = ' & '.join(f2(v) for v in cell[k]['est'][name]['ratio']) if k in cell else '--- & --- & --- & ---'
    L4.append(f'{i + 1} & {labels[k]} & ' + ' & '.join(f2(v) for v in x['se_ratio']) + ' & & ' + sim + ' \\\\')
    for c, cn in enumerate(comp):
        put(f'sim.bd.{k}.{cn}', f2(x['se_ratio'][c])); put(f'sim.bdV.{k}.{cn}', spct(x['b_over_V'][c])); put(f'sim.bdquad.{k}.{cn}', f"{100 * x['quadratic_over_V'][c]:+.1f}\\%")
    put(f'sim.bd.load.{k}', f2(ld[1] * bd[k]['fit']['load_centre'])); put(f'sim.bd.neg.{k}', pct(bd[k]['fit'][name + '_negative'], 2))
open('gen/tab_bdelta.tex', 'w').write('\n'.join(L4) + '\n')
L3b = []
for i, k in enumerate(order):
    if k not in cell: continue
    r = cell[k]['regime']; s_, m_ = r['ratio_by_decile_stayers'], r['ratio_by_decile_movers']
    x = bd[k]['models'][name] if k in bd else None
    extra = (f"{100 * x['linear_over_V'][3]:+.1f}\\% & {100 * x['quadratic_over_V'][3]:+.1f}\\%" if x else '--- & ---')
    L3b.append(f"{i + 1} & {s_[0]:.2f} & {s_[4]:.2f} & {s_[9]:.2f} & {m_[0]:.2f} & {m_[4]:.2f} & {m_[9]:.2f} & {100 * r['mover_share_first_term'][3]:.0f}\\% & {extra} \\\\")
open('gen/tab_regime.tex', 'w').write('\n'.join(L3b) + '\n')
if bd:
    v = np.array([bd[k]['models'][name]['se_ratio'] for k in bd]); put('sim.bd.min', f2(v.min())); put('sim.bd.max', f2(v.max())); put('sim.bd.cells', str(len(bd)))
    dif = np.array([np.array(cell[k]['est'][name]['ratio']) - np.array(bd[k]['models'][name]['se_ratio']) for k in bd if k in cell])
    if len(dif): put('sim.bd.diff.min', f'{dif.min():+.2f}'); put('sim.bd.diff.max', f'{dif.max():+.2f}'); put('sim.bd.diff.absmax', f2(np.abs(dif).max()))
    put('sim.bd.draws', str(min(bd[k]['draws'] for k in bd)))
    ne = [k for k in bd if k not in ('N1', 'N9')]
    vs = np.array([bd[k]['models'][name]['se_ratio'][3] for k in ne]); put('sim.bd.sum.min', f2(vs.min())); put('sim.bd.sum.max', f2(vs.max()))
    put('sim.bd.sum.short.min', pct(1 - vs.max())); put('sim.bd.sum.short.max', pct(1 - vs.min()))
    va = np.array([bd[k]['models'][name]['se_ratio'][:3] for k in ne]); put('sim.bd.other.min', f2(va.min())); put('sim.bd.other.max', f2(va.max()))
    ex = np.array([bd[k]['models'][name]['se_ratio'] for k in ('N1', 'N9') if k in bd]); put('sim.bd.exact.maxdev', f'{np.abs(ex - 1).max():.3f}')
    ds = np.array([cell[k]['est'][name]['ratio'][3] - bd[k]['models'][name]['se_ratio'][3] for k in bd if k in cell]); put('sim.bd.diff.sum.min', f'{ds.min():+.2f}'); put('sim.bd.diff.sum.max', f'{ds.max():+.2f}')
    mix = [k for k in ('N2', 'N3', 'N6', 'N10') if k in bd]; qm = np.array([bd[k]['models'][name]['quadratic_over_V'][3] for k in mix])
    put('sim.bdquad.mix.min', pct(-qm.max())); put('sim.bdquad.mix.max', pct(-qm.min()))
    qo = np.array([bd[k]['models'][name]['quadratic_over_V'] for k in bd if k not in mix]); put('sim.bdquad.other.max', pct(np.abs(qo).max(), 1))
for k in cell:
    if orc[k]:
        for c, cn in enumerate(comp): put(f'sim.{k}.orcover.{cn}', f2(orc[k]['est']['or']['coverage_true'][c]))
orv = np.array([orc[k]['est']['or']['ratio'] for k in cell if orc[k] and k not in ('N4', 'N5')]); put('sim.or.noskew.min', f2(orv.min())); put('sim.or.noskew.max', f2(orv.max()))
orc_ = np.array([orc[k]['est']['or']['coverage_true'] for k in cell if orc[k]]); put('sim.or.cov.min', f'{orc_.min():.3f}'); put('sim.or.cov.max', f'{orc_.max():.3f}')
if bd and cell:
    Lm = np.array([bd[k]['models'][name]['se_ratio'] for k in order]); Rm = np.array([cell[k]['est'][name]['ratio'] for k in order])
    closer = np.abs(Rm - 1) < np.abs(Lm - 1)
    for c, cn in enumerate(comp): put(f'sim.bd.closer.{cn}', str(int(closer[:, c].sum())))
    put('sim.bd.closer.all', str(int(closer.sum()))); put('sim.bd.above.all', str(int((Rm > Lm).sum()))); put('sim.bd.above.sum', str(int((Rm > Lm)[:, 3].sum())))
cap = {}
if len(sys.argv) > 10:
    for k in order:
        for fn in (f'bdelta_cap_{name}.json', 'bdelta_cap.json'):
            f = f'{sys.argv[10]}/{k}__default/{fn}'
            if os.path.exists(f): cap[k] = json.load(open(f)); assert cap[k].get('model', 'twl') == name; break
for k, r in cap.items():
    put(f'sim.cap.{k}.rows3', pct(r['share_rows_beyond']['3'], 1)); put(f'sim.cap.{k}.first3', pct(r['first_term_share_beyond']['3']))
    put(f'sim.cap.{k}.first3.whole', pct(r['first_term_share_beyond']['3'] * cell[k]['regime']['mover_share_first_term'][3]))
    for cv, tag in (('3.0', '3'), ('4.0', '4'), ('6.0', '6'), ('1000000.0', 'none')):
        for c, cn in enumerate(comp): put(f'sim.cap.{k}.{tag}.{cn}', f2(r['caps'][cv]['se_ratio'][c]))
# ---------------- clean single-model run on the data (lp_real8.py), optional 11th argument
if len(sys.argv) > 11 and os.path.exists(sys.argv[11]):
    r8 = json.load(open(sys.argv[11])); a8 = r8['runs'][0]
    put('real8.solves.fit', str(a8['solves_fit'])); put('real8.solves.draws', str(a8['solves_draws']))
    put('real8.seconds.fit', f"{np.mean([x['seconds_fit'] for x in r8['runs']]):.0f}"); put('real8.seconds.draws', f"{np.mean([x['seconds_draws'] for x in r8['runs']]):.0f}")
    ev = np.array([x['eigenvalues'] for x in r8['runs']]); put('real8.eig.ratio', f'{100 * (ev[:, 0] / ev[:, -1]).min():.1f}\\%'); put('real8.eig.allpos', str(bool((ev > 0).all())))
    assert r8['model'] == name
    d8 = np.array([x['bound_same_draws'] for x in r8['runs']]); put('real8.d2.min', f'{d8.min():.4f}'); put('real8.d2.max', f'{d8.max():.4f}')
    re8 = np.array([x['reff_same_draws'] for x in r8['runs']]); put('real8.reff.min', f'{round(re8.min(), -2):,.0f}'.replace(',', '{,}')); put('real8.reff.max', f'{round(re8.max(), -2):,.0f}'.replace(',', '{,}'))
    s8 = np.array([x['se'] for x in r8['runs']]); put('real8.se.maxdiff7', pct(np.abs(s8 / se - 1).max(), 1))
    alls = np.vstack([allse, s8]); put('real.run_maxdiff', cpct(max(np.abs(alls / alls[i] - 1).max() for i in range(len(alls))), 1))  # all runs of this model on the data
    for c, k in enumerate(comp): put(f'real8.se.{k}', f'{s8[0][c] * 1e4:.2f}')
# ---------------- validation campaign val-v1 (lp_val7.py receipts), optional 12th argument: directory with
# <run>/validation.json and, for the two new designs, <run>/panel_descriptives.json
val = {}
if len(sys.argv) > 12:
    for rid in ('six', 'y3', 'half'):
        f = f'{sys.argv[12]}/{rid}/validation.json'
        if os.path.exists(f): val[rid] = json.load(open(f))
vlab = {'six': 'Six years, fresh draws', 'y3': 'Three-year window', 'half': 'Half of the workers'}
LV, LC = [], []
for rid, v in val.items():
    Rv = np.array([c['ratio'] for c in v['cells']]); Cv = np.array([c['coverage'] for c in v['cells']]); Ro = np.array([c['ratio_or'] for c in v['cells']])
    put(f'val.{rid}.passed', 'passed' if v['passed'] else 'failed'); put(f'val.{rid}.nfail', str(len(v['failures']))); put(f'val.{rid}.cells', str(Rv.size))
    put(f'val.{rid}.ratio.min', f2(Rv.min())); put(f'val.{rid}.ratio.max', f2(Rv.max())); put(f'val.{rid}.cover.min', f'{Cv.min():.3f}'); put(f'val.{rid}.cover.max', f'{Cv.max():.3f}')
    put(f'val.{rid}.cover.mean', f'{Cv.mean():.3f}'); put(f'val.{rid}.or.min', f2(Ro.min())); put(f'val.{rid}.or.max', f2(Ro.max()))
    put(f'val.{rid}.nonpositive', str(sum(c['nonpositive'] for c in v['cells']))); put(f'val.{rid}.failed_draws', str(sum(c['failures'] for c in v['cells'])))
    put(f'val.{rid}.draws', str(sum(c['reps'] for c in v['cells']))); put(f'val.{rid}.floor.max', pct(max(max(c['max_final_floored'], c['max_scale_floored']) for c in v['cells']), 2))
    put(f'val.{rid}.eig.min', f'{100 * min(c["eig_ratio_min"] for c in v["cells"]):.1f}\\%')
    for c, cn in enumerate(comp): put(f'val.{rid}.{cn}.min', f2(Rv[:, c].min())); put(f'val.{rid}.{cn}.max', f2(Rv[:, c].max()))
    LV.append(f"{vlab[rid]} & {len(v['cells'])} & " + ' & '.join(f'{Rv[:, c].min():.2f}--{Rv[:, c].max():.2f}' for c in range(4)) + f" & {Cv.min():.3f}--{Cv.max():.3f} & {'yes' if v['passed'] else 'no'} \\\\")
    LC.append('\\addlinespace\n\\multicolumn{10}{@{}l}{\\emph{' + vlab[rid] + '}} \\\\')
    for c in v['cells']:
        k = c['cell'].split('__')[0]; LC.append(f"\\quad {order.index(k) + 1} & {labels[k]} & " + ' & '.join(f2(x) for x in c['ratio']) + ' & ' + ' & '.join(f'{x:.3f}' for x in c['coverage']) + ' \\\\')
        for j, cn in enumerate(comp): put(f'val.{rid}.{k}.{cn}', f2(c['ratio'][j])); put(f'val.{rid}.{k}.{cn}.d3', f"{c['ratio'][j]:.3f}")
if len(sys.argv) > 12:
    for rid in ('y3', 'half'):
        d = f'{sys.argv[12]}/{rid}/panel_descriptives.json'
        if os.path.exists(d):
            dd = json.load(open(d)); fm = lambda x: f'{x:,}'.replace(',', '{,}')
            for kk in ('person_years', 'workers', 'firms', 'stayers', 'movers', 'mover_matches', 'stayer_rows'): put(f'val.{rid}.n.{kk}', fm(dd[kk]))
            put(f'val.{rid}.n.rows', fm(dd['mover_matches'] + dd['stayer_rows'])); put(f'val.{rid}.share.stayer2', pct(dd['stayer_years_hist'].get('2', 0) / dd['stayers']))
    a = json.load(open(sys.argv[12] + '/manifest_val-v1-six.json'))['acceptance']
    put('val.acc.ratio_min', f2(a['ratio_min'])); put('val.acc.ratio_max', f2(a['ratio_max'])); put('val.acc.coverage_min', f2(a['coverage_min']))
    put('val.acc.mean_coverage_min', f2(a['mean_coverage_min'])); put('val.acc.max_floored', pct(a['max_floored']))
open('gen/tab_val.tex', 'w').write('\n'.join(LV) + '\n'); open('gen/tab_val_cells.tex', 'w').write('\n'.join(LC) + '\n')
if val:
    allR = np.concatenate([np.array([c['ratio'] for c in v['cells']]).ravel() for v in val.values()]); allC = np.concatenate([np.array([c['coverage'] for c in v['cells']]).ravel() for v in val.values()])
    put('val.all.ratio.min', f2(allR.min())); put('val.all.ratio.max', f2(allR.max())); put('val.all.cover.min', f'{allC.min():.3f}'); put('val.all.cover.max', f'{allC.max():.3f}'); put('val.all.cells', str(allR.size))
    put('val.all.draws', f"{sum(sum(c['reps'] for c in v['cells']) for v in val.values()):,}".replace(',', '{,}')); put('val.all.failed_draws', str(sum(sum(c['failures'] for c in v['cells']) for v in val.values())))
    put('val.all.nonpositive', str(sum(sum(c['nonpositive'] for c in v['cells']) for v in val.values()))); put('val.all.eig.min', f"{100 * min(min(c['eig_ratio_min'] for c in v['cells']) for v in val.values()):.1f}\\%")
    put('val.all.floor.max', pct(max(max(max(c['max_final_floored'], c['max_scale_floored']) for c in v['cells']) for v in val.values()), 2))
    put('val.all.n.below95', str(int((allR < 0.95).sum()))); put('val.all.n.above110', str(int((allR > 1.10).sum()))); put('val.all.n.above115', str(int((allR > 1.15).sum())))
    put('val.all.over.max', pct(allR.max() - 1)); put('val.all.cover.n.below93', str(int((allC < 0.93).sum())))
    if 'six' in val and cell:
        dd = np.array([np.array(cell[c['cell'].split('__')[0]]['est'][name]['ratio']) - np.array(c['ratio']) for c in val['six']['cells']])
        put('val.six.devdiff.max', cdec(np.abs(dd).max(), 3)); put('val.six.devdiff.mean', f'{dd.mean():+.4f}')
    put('val.all.passed', str(sum(v['passed'] for v in val.values()))); put('val.all.nfail', str(sum(len(v['failures']) for v in val.values())))
    assert all(v['acceptance'] == a for v in val.values())
open('gen/numbers.tex', 'w').write('\n'.join(f'\\setres{{{k}}}{{{v}}}' for k, v in N.items()) + '\n'); open('gen/summary.txt', 'w').write('\n'.join(log) + '\n')
with open('gen/detail.txt', 'w') as fh:
    for k, a in cell.items():
        fh.write(f"{k} reps {a['reps']} failed {a['failures']} bias/sd {[round(x, 2) for x in a['bias_over_sd']]} mc var/theory {[round(x, 2) for x in a['mc_var_over_theory']]}\n")
        for e, x in a['est'].items(): fh.write(f"  {e:4s} ratio {[round(v, 3) for v in x['ratio']]} cov_true {[round(v, 3) for v in x['coverage_true']]} se_cv {[round(v, 3) for v in x['se_cv']]}\n")
        if orc[k]: fh.write(f"  or   ratio {[round(v, 3) for v in orc[k]['est']['or']['ratio']]} cov_true {[round(v, 3) for v in orc[k]['est']['or']['coverage_true']]}\n")
        fh.write(f"  diag { {kk: [round(v, 4) for v in vv] for kk, vv in a['diag'].items()} }\n")
print('generated', len(N), 'numbers for model', name)

"""Generate every number, table and figure coordinate of the note from the result files.
Usage: gen.py <real4.json> <controls_lcs2.json> <mc aggregate.json> <mc-v1 aggregate.json>
Writes gen/numbers.tex, gen/tab_real.tex, gen/tab_sim.tex, gen/tab_simcov.tex, gen/fig_profile.tex, gen/summary.txt."""
import sys, json, numpy as np
real = json.load(open(sys.argv[1])); ctrl = json.load(open(sys.argv[2])); agg = json.load(open(sys.argv[3]))['cells']
old = json.load(open(sys.argv[4]))['cells'] if len(sys.argv) > 4 else {}
N = {}; log = []
def put(k, v): N[k] = v; log.append(f'{k} = {v}')
pct = lambda x, d=0: f'{100 * x:.{d}f}\\%'
comp = ('alpha', 'psi', 'cov', 'sum')
# ---------------- real data
th = np.array(real['theta']); runs = real['runs']; se = np.array(runs[0]['se']); V = np.array(runs[0]['V'])
for c, k in enumerate(comp):
    put(f'real.theta.{k}', f'{th[c]:.4f}'); put(f'real.se.{k}', f'{se[c] * 1e4:.2f}'); put(f'real.relse.{k}', pct(se[c] / th[c], 2))
    put(f'real.plugin.{k}', f'{real["plugin"][c]:.4f}')
put('real.rel_se_sum', pct(se[3] / th[3], 2))
put('real.relse.min', pct((se / th).min(), 2)); put('real.relse.max', pct((se / th).max(), 2))
allse = np.array([r['se'] for r in runs]); put('real.run_maxdiff', pct(np.abs(allse / allse[0] - 1).max(), 1))
put('real.run_maxdiff16', pct(np.abs(allse[1] / allse[0] - 1).max(), 1))
put('real.se_solves', f'{runs[1]["solves"]}'); put('real.se_seconds', f'{runs[1]["seconds"]:.0f}')
put('real.factor_minutes', f'{(runs[0]["seconds"] - runs[1]["seconds"]) / 60:.0f}')
put('real.probe_solves', f'{3 * real["probes"]:,}'.replace(',', '{,}'))
tr = real['transfer']
put('real.profile_floored', pct(tr['profile_floored'], 2)); put('real.variance_floored', pct(tr['variance_floored'], 2))
put('real.profile_r2', f'{tr["r2"]:.3f}'); put('real.profile_min', f'{tr["profile_min"]:.2f}'); put('real.profile_max', f'{tr["profile_max"]:.2f}')
for j, nm in enumerate(('one', 'a', 'aa', 'p', 'pp', 'ap')): put(f'real.coef.{nm}', f'{tr["coef"][j] * 1e3:.2f}')
sh = real['share_first_term']
for c, k in enumerate(comp):
    put(f'real.share.movers.{k}', pct(sh['movers'][c])); put(f'real.share.stayers.{k}', pct(1 - sh['movers'][c]))
    put(f'real.first_over_V.{k}', pct(real['first_term_over_V'][c])); put(f'real.adjust_over_V.{k}', pct(real['adjust_over_V'][c], 2))
put('real.adjust_max', pct(max(abs(x) for x in real['adjust_over_V']), 2))
dg = real['diagnostics']
put('real.max_stayer_share', pct(max(dg['max_stayer_share']), 1)); put('real.max_mover_share', pct(max(dg['max_mover_row_share']), 1))
put('real.lambda.min', f'{min(dg["lambda2_over_V"]):.4f}'); put('real.lambda.max', f'{max(dg["lambda2_over_V"]):.4f}')
put('real.double_sum.min', pct(min(dg['double_sum_over_V']))); put('real.double_sum.max', pct(max(dg['double_sum_over_V'])))
for k in ('rows', 'mover_rows', 'stayer_rows', 'person_years', 'firms', 'workers', 'stayers', 'movers'):
    put(f'real.n.{k}', f'{real[k]:,}'.replace(',', '{,}'))
put('real.share.stayer_rows', pct(real['stayer_rows'] / real['rows'])); put('real.share.stayer_py', pct(real['stayer_rows'] / real['person_years']))
wm = real['within_match_overall']; put('real.within_match_mean', f'{wm["mean"]:.3f}'); put('real.stayers_mean', f'{wm["stayers_mean"]:.3f}')
ml = real['movers_by_length']; lo = [ml[k]['leave_out_product'] for k in sorted(ml, key=int)]
put('real.lo.len1', f'{lo[0]:.3f}'); put('real.lo.longer.min', f'{min(lo[1:]):.3f}'); put('real.lo.longer.max', f'{max(lo[1:]):.3f}')
put('real.lo.overall', f'{sum(ml[k]["leave_out_product"] * ml[k]["rows"] for k in ml) / sum(ml[k]["rows"] for k in ml):.3f}')
put('real.within_match_matches', f'{wm["matches"]:,}'.replace(',', '{,}'))
rows = [('Variance of person effects, $\\sigma^2_\\alpha$', 0), ('Variance of firm effects, $\\sigma^2_\\psi$', 1),
        ('Covariance, $\\sigma_{\\alpha,\\psi}$', 2), ('Variance of the sum, $\\sigma^2_{\\alpha+\\psi}$', 3)]
open('gen/tab_real.tex', 'w').write('\n'.join(
    f'{lab} & {real["plugin"][c]:.4f} & {th[c]:.4f} & {se[c] * 1e4:.2f} & {100 * se[c] / th[c]:.2f} & {100 * real["first_term_over_V"][c]:.0f} & {100 * sh["movers"][c]:.0f} \\\\' for lab, c in rows) + '\n')
# ---------------- profile figure (coordinates: mean standardised effect in the decile, variance relative to the group mean)
P = real['profiles']; out = []
def coords(x, y): return ' '.join(f'({a:.3f},{b:.3f})' for a, b in zip(x, y))
for lab in ('alpha', 'psi'):
    s = P['stayers_by_' + lab]; m = P['movers_by_' + lab]
    out.append(f'\\def\\prof{lab}stay{{{coords(s["mean_index"], np.array(s["sample_variance"]) / wm["stayers_mean"])}}}')
    out.append(f'\\def\\prof{lab}fit{{{coords(s["mean_index"], np.array(s["fitted_profile"]) / wm["stayers_mean"])}}}')
    out.append(f'\\def\\prof{lab}move{{{coords(m["mean_index_matches"], np.array(m["within_match_variance"]) / wm["mean"])}}}')
    rs = np.array(s['sample_variance']) / wm['stayers_mean']; rm = np.array(m['within_match_variance']) / wm['mean']
    put(f'real.prof.{lab}.stay.lo', f'{rs[0]:.2f}'); put(f'real.prof.{lab}.stay.hi', f'{rs[-1]:.2f}'); put(f'real.prof.{lab}.stay.min', f'{rs.min():.2f}')
    put(f'real.prof.{lab}.move.lo', f'{rm[0]:.2f}'); put(f'real.prof.{lab}.move.hi', f'{rm[-1]:.2f}'); put(f'real.prof.{lab}.move.min', f'{rm.min():.2f}')
    put(f'real.prof.{lab}.corr', f'{np.corrcoef(rs, rm)[0, 1]:.2f}')
    put(f'real.prof.{lab}.stay.mid', f'{rs[3:7].mean():.2f}'); put(f'real.prof.{lab}.move.mid', f'{rm[3:7].mean():.2f}')
    rf = np.array(s['fitted_profile']) / wm['stayers_mean']
    put(f'real.prof.{lab}.fit.lo', f'{rf[0]:.2f}'); put(f'real.prof.{lab}.fit.hi', f'{rf[-1]:.2f}'); put(f'real.prof.{lab}.fit.mid', f'{rf[3:7].mean():.2f}')
    put(f'real.prof.{lab}.maxgap', f'{np.abs(rm / rs - 1).max():.2f}')
open('gen/fig_profile.tex', 'w').write('\n'.join(out) + '\n')
# ---------------- movers' rows by decile: data and simulation with known truth
if len(sys.argv) > 5:
    chk = json.load(open(sys.argv[5]))['scenarios']; mb = P['movers_by_alpha']
    row = lambda lab, v: lab + ' & ' + ' & '.join(f'{100 * x:.1f}' for x in v) + ' \\\\'
    L = ['\\multicolumn{11}{@{}l}{\\emph{Veneto panel}} \\\\', row('\\quad leave-out estimate', mb['leave_out_product']), row('\\quad fitted by the transfer model', mb['fitted']),
         '\\addlinespace', '\\multicolumn{11}{@{}l}{\\emph{Simulated data, scenario 1 (no profile)}} \\\\', row('\\quad leave-out estimate', chk['N1']['movers_leave_out']), row('\\quad true variance', chk['N1']['movers_true']),
         '\\addlinespace', '\\multicolumn{11}{@{}l}{\\emph{Simulated data, scenario 6 (movers share the stayers\' profile)}} \\\\', row('\\quad leave-out estimate', chk['N6']['movers_leave_out']), row('\\quad true variance', chk['N6']['movers_true']),
         row('\\quad fitted by the transfer model', chk['N6']['movers_fitted'])]
    open('gen/tab_deciles.tex', 'w').write('\n'.join(L) + '\n')
    lo, ft = np.array(mb['leave_out_product']), np.array(mb['fitted']); nr = np.array(mb['n_rows'], float); mean = (lo * nr).sum() / nr.sum()
    put('dec.data.lo.bottom', f'{lo[0] / mean:.1f}'); put('dec.data.lo.mid', f'{lo[3:7].mean() / mean:.2f}'); put('dec.data.lo.top', f'{lo[9] / mean:.1f}')
    put('dec.data.fit.bottom', f'{ft[0] / lo[0] - 1:+.0%}'.replace('%', '\\%')); put('dec.data.fit.mid', f'{ft[3:7].mean() / lo[3:7].mean() - 1:+.0%}'.replace('%', '\\%')); put('dec.data.fit.top', f'{ft[9] / lo[9] - 1:+.0%}'.replace('%', '\\%'))
    for nm in ('N1', 'N6'):
        l_, t_, f_ = (np.array(chk[nm][k]) for k in ('movers_leave_out', 'movers_true', 'movers_fitted'))
        put(f'dec.{nm}.gap.max', f'{np.abs(l_ / t_ - 1).max():.0%}'.replace('%', '\\%')); put(f'dec.{nm}.gap.bottom', f'{l_[0] / t_[0] - 1:+.0%}'.replace('%', '\\%')); put(f'dec.{nm}.gap.top', f'{l_[9] / t_[9] - 1:+.0%}'.replace('%', '\\%'))
        put(f'dec.{nm}.gap.mid', f'{l_[3:7].mean() / t_[3:7].mean() - 1:+.0%}'.replace('%', '\\%'))
        m_ = chk[nm]['overall'][2]; put(f'dec.{nm}.true.bottom', f'{t_[0] / m_:.1f}'); put(f'dec.{nm}.true.mid', f'{t_[3:7].mean() / m_:.2f}'); put(f'dec.{nm}.true.top', f'{t_[9] / m_:.1f}')
        put(f'dec.{nm}.fit.bottom', f'{f_[0] / t_[0] - 1:+.0%}'.replace('%', '\\%')); put(f'dec.{nm}.fit.mid', f'{f_[3:7].mean() / t_[3:7].mean() - 1:+.0%}'.replace('%', '\\%')); put(f'dec.{nm}.fit.top', f'{f_[9] / t_[9] - 1:+.0%}'.replace('%', '\\%'))
    put('dec.reps', str(json.load(open(sys.argv[5]))['reps']))
# ---------------- controls
cc = ctrl['components']
put('ctrl.sum', pct(cc['all:sum']['transfer_se_ratio'] - 1)); put('ctrl.others', pct(max(abs(cc[f'all:{k}']['transfer_se_ratio'] - 1) for k in ('worker', 'firm', 'cov')), 1))
put('ctrl.sum.year_only', pct(cc['year_only:sum']['transfer_se_ratio'] - 1, 1))
# ---------------- simulation
order = ['N1', 'N2', 'N3', 'N4', 'N5', 'N6', 'N7', 'N8', 'N9', 'N10']
labels = {'N1': 'No profile', 'N2': 'Quadratic profile, worker factors', 'N3': 'Same, fat tails', 'N4': 'Quadratic profile, skew against wage',
          'N5': 'Quadratic profile, skew with wage', 'N6': 'Profile of the data (deciles)', 'N7': 'Log-linear in the wage level',
          'N8': 'Movers steeper than stayers', 'N9': 'Movers flat, stayers steep',
          'N10': "Movers' own profile in the data"}
cells = {k: agg[f'{k}__default'] for k in order if f'{k}__default' in agg}
v5 = json.load(open(sys.argv[6]))['cells'] if len(sys.argv) > 6 else {}
if v5:
    cells['N10'] = v5['N10__default']
    d = [abs(x - y) for k in ('N1', 'N6', 'N8', 'N9') for e in ('tr', 'or') for x, y in zip(v5[f'{k}__default']['est'][e]['ratio'], cells[k]['est'][e]['ratio'])]
    put('sim.v5.reproduce', f'{max(d):.1e}')
    for k in ('N1', 'N6', 'N8', 'N9', 'N10'):
        for c, cn in enumerate(comp): put(f'sim.tl.{k}.{cn}', f"{v5[f'{k}__default']['est']['tl']['ratio'][c]:.2f}")
    for c, cn in enumerate(comp): put(f'sim.N10.or.{cn}', f"{cells['N10']['est']['or']['ratio'][c]:.2f}"); put(f'sim.N10.cov.{cn}', f"{cells['N10']['est']['tr']['coverage_true'][c]:.2f}")
f2 = lambda x: f'{x:.2f}'
L1, L2 = [], []
for i, k in enumerate(order):
    if k not in cells: continue
    a = cells[k]; e = a['est']
    if k == 'N8': L1.append('\\midrule'); L2.append('\\midrule')
    L1.append(f'{i + 1} & {labels[k]} & ' + ' & '.join(f2(x) for x in e['tr']['ratio']) + ' & & ' + ' & '.join(f2(x) for x in e['or']['ratio']) + ' \\\\')
    L2.append(f'{i + 1} & {labels[k]} & ' + ' & '.join(f2(x) for x in e['tr']['coverage_true']) + ' & & ' + (' & '.join(f2(x) for x in e['trx']['ratio']) if 'trx' in e else '--- & --- & --- & ---') + ' \\\\')
open('gen/tab_sim.tex', 'w').write('\n'.join(L1) + '\n'); open('gen/tab_simcov.tex', 'w').write('\n'.join(L2) + '\n')
L3 = []
for i, k in enumerate(order):
    if k not in cells: continue
    r = cells[k]['regime']; s_, m_ = r['ratio_by_decile_stayers'], r['ratio_by_decile_movers']
    L3.append(f"{i + 1} & {s_[0]:.2f} & {s_[4]:.2f} & {s_[9]:.2f} & {m_[0]:.2f} & {m_[4]:.2f} & {m_[9]:.2f} & {100 * r['mover_share_first_term'][3]:.0f}\\% \\\\")
open('gen/tab_regime.tex', 'w').write('\n'.join(L3) + '\n')
ok = [k for k in order[:7] if k in cells]; bad = [k for k in order[7:] if k in cells]
def rng(keys, est, field, comps=range(4)):
    v = np.array([[cells[k]['est'][est][field][c] for c in comps] for k in keys]); return v.min(), v.max()
if ok:
    lo, hi = rng(ok, 'tr', 'ratio'); put('sim.ok.ratio.min', f2(lo)); put('sim.ok.ratio.max', f2(hi))
    lo, hi = rng(ok, 'tr', 'ratio', [3]); put('sim.ok.ratio.sum.min', f2(lo)); put('sim.ok.ratio.sum.max', f2(hi))
    lo, hi = rng(ok, 'or', 'ratio'); put('sim.ok.oracle.min', f2(lo)); put('sim.ok.oracle.max', f2(hi))
    lo, hi = rng(ok, 'tr', 'coverage_true'); put('sim.ok.cov.min', f2(lo)); put('sim.ok.cov.max', f2(hi))
    lo, hi = rng(ok, 'tr', 'coverage_mean'); put('sim.ok.covmean.min', f2(lo)); put('sim.ok.covmean.max', f2(hi))
    d = np.array([[cells[k]['est']['trx']['ratio'][c] - cells[k]['est']['tr']['ratio'][c] for c in range(4)] for k in ok + bad if 'trx' in cells[k]['est']])
    dok = np.array([[cells[k]['est']['trx']['ratio'][c] - cells[k]['est']['tr']['ratio'][c] for c in range(4)] for k in ok]); put('sim.trx_maxdiff_ok', f2(np.abs(dok).max()))
    put('sim.trx_maxdiff', f2(np.abs(d).max())); put('sim.trx_meandiff', f'{d.mean():+.3f}')
    b = np.array([cells[k]['bias_over_sd'] for k in ok + bad]); put('sim.bias1600', f'{np.abs(b).max():.1f}'); put('sim.bias1600.sum', f'{np.abs(b[:, 3]).max():.1f}')
    bw = np.array([cells[k]['bias_weights_over_sd'] for k in ok + bad]); put('sim.biasw1600', f'{np.abs(bw).max():.2f}')
    put('sim.reps', str(min(cells[k]['reps'] for k in cells))); put('sim.failures', str(sum(cells[k]['failures'] for k in cells)))
    put('sim.cells', str(len(cells)))
    r2 = [cells[k]['diag'].get('tr:r2', [np.nan])[0] for k in cells]; fl = [cells[k]['diag'].get('tr:variance_floored', [0, 0, 0])[2] for k in cells]
    put('sim.variance_floored.max', pct(max(fl), 0))
    pf = [cells[k]['diag'].get('tr:profile_floored', [0, 0, 0])[2] for k in cells]; put('sim.profile_floored.max', pct(max(pf), 2))
    ms = np.array([cells[k]['regime']['mover_share_first_term'] for k in cells]); put('sim.mover_share.sum.min', pct(ms[:, 3].min())); put('sim.mover_share.sum.max', pct(ms[:, 3].max()))
    mc = np.array([cells[k]['mc_var_over_theory'] for k in cells]); put('sim.mcvar.min', f2(mc.min())); put('sim.mcvar.max', f2(mc.max()))
    secv = np.array([cells[k]['est']['tr']['se_cv'] for k in cells]); put('sim.se_cv.max', pct(secv.max(), 1))
def rg(keys, est, field, pick=None):
    v = np.array([[cells[k]['est'][est][field][c] for c in range(4)] for k in keys if est in cells[k]['est']])
    if pick is not None: v = v[:, pick]
    return f2(v.min()), f2(v.max())
if ok:
    for tag, keys in (('norm', ['N1', 'N2', 'N6', 'N7']), ('shape', ['N3', 'N4', 'N5']), ('a4', ok)):
        for est in ('tr', 'or', 'trx'):
            lo, hi = rg(keys, est, 'ratio'); put(f'sim.{tag}.{est}.min', lo); put(f'sim.{tag}.{est}.max', hi)
        lo, hi = rg(keys, 'tr', 'coverage_true'); put(f'sim.{tag}.cov.min', lo); put(f'sim.{tag}.cov.max', hi)
    for k in order:
        if k in cells:
            for c, cn in enumerate(comp):
                put(f'sim.{k}.tr.{cn}', f2(cells[k]['est']['tr']['ratio'][c])); put(f'sim.{k}.or.{cn}', f2(cells[k]['est']['or']['ratio'][c]))
                put(f'sim.{k}.cover.{cn}', f2(cells[k]['est']['tr']['coverage_true'][c]))
                if 'trx' in cells[k]['est']: put(f'sim.{k}.trx.{cn}', f2(cells[k]['est']['trx']['ratio'][c]))
    b4 = np.array([cells[k]['bias_over_sd'] for k in cells]); put('sim.bias.max', f'{np.abs(b4).max():.1f}')
for k in bad:
    for c, cn in enumerate(comp): put(f'sim.{k}.{cn}', f2(cells[k]['est']['tr']['ratio'][c]))
if old:
    b = np.array([old[k]['bias_over_sd'] for k in old if old[k]['independent'] and k.endswith('__default') and k.startswith('L') and 'AR' not in k and 'RW' not in k])
    put('sim.bias200', f'{np.abs(b).max():.1f}')
open('gen/numbers.tex', 'w').write('\n'.join(f'\\setres{{{k}}}{{{v}}}' for k, v in N.items()) + '\n')
open('gen/summary.txt', 'w').write('\n'.join(log) + '\n')
# realised regimes and full detail for checking
with open('gen/detail.txt', 'w') as fh:
    for k, a in cells.items():
        r = a['regime']
        fh.write(f"{k} reps {a['reps']} failed {a['failures']}\n  stayers by decile {[round(x, 2) for x in r['ratio_by_decile_stayers']]}\n  movers by decile  {[round(x, 2) for x in r['ratio_by_decile_movers']]}\n"
                 f"  mover share first term {[round(x, 3) for x in r['mover_share_first_term']]} signal share {[round(x, 3) for x in a['signal_share']]}\n"
                 f"  bias/sd MC {[round(x, 2) for x in a['bias_over_sd']]} weights {[round(x, 2) for x in a['bias_weights_over_sd']]} mc var/theory {[round(x, 2) for x in a['mc_var_over_theory']]}\n")
        for e, x in a['est'].items():
            fh.write(f"  {e:4s} ratio {[round(v, 3) for v in x['ratio']]} cov_true {[round(v, 3) for v in x['coverage_true']]} cov_mean {[round(v, 3) for v in x['coverage_mean']]} se_cv {[round(v, 3) for v in x['se_cv']]}\n")
        fh.write(f"  diag {a['diag']}\n")
print('generated', len(N), 'numbers')

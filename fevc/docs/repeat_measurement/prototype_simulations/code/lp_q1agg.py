"""Validate, aggregate and judge the q = 1 simulation.

Usage: lp_q1agg.py <out-root> <manifest> [--allow-partial]
Inventory: every task of every cell exactly once, schema, run, stream, source hashes, replication counts; every problem
and every failed replication is listed. Aggregation: coverage of the normal interval and of the q = 1 interval (rule of
the note 'dq'; fitted variance of b1 throughout 'dqfit'; movers' true variances 'or', 'orfit'), lengths, fallbacks.
Acceptance: the thresholds stored in the manifest, applied to cells with role 'acceptance'; every failing cell is
listed. Writes aggregate.json, aggregate.md, validation.json. Exit 0 passed, 3 failed, 2 incomplete or malformed."""
import sys, os, json, glob, numpy as np

COMP = ('person', 'firm', 'cov', 'sum'); Z = 1.959963984540054


def cov_stats(lo, hi, ok, truth):
    lo = np.where(ok, lo, np.nan); hi = np.where(ok, hi, np.nan)
    with np.errstate(invalid='ignore'):
        c = ok & (lo <= truth) & (truth <= hi)
        return dict(cover=float(c.mean()), miss_low=float((ok & (hi < truth)).mean()), miss_high=float((ok & (lo > truth)).mean()),
                    none=int((~ok).sum()), len_mean=float(np.nanmean(hi - lo)), len_median=float(np.nanmedian(hi - lo)))


def main(oroot, mfile, partial=False):
    man = json.load(open(mfile)); problems = []; agg = {}
    for ci, cell in enumerate(man['cells']):
        cdir = f"{oroot}/{cell['name']}"; expect = [f'{cdir}/task-{t:03d}.json' for t in range(man['tasks'])]; found = sorted(glob.glob(cdir + '/task-*'))
        problems += [f'missing {p}' for p in expect if p not in found] + [f'unexpected {p}' for p in found if p not in expect]
        if not os.path.exists(cdir + '/design.json'): problems.append(f'missing {cdir}/design.json'); continue
        des = json.load(open(cdir + '/design.json')); rec = {}; done = fails = 0; truth = None
        for p in [p for p in expect if p in found]:
            try: r = json.load(open(p))
            except Exception as ex: problems.append(f'unreadable {p}: {ex!r}'); continue
            if r.get('schema') != 'LP-Q1-TASK-V3' or r.get('cell') != ci or r.get('name') != cell['name'] or r.get('task') != int(p[-8:-5]): problems.append(f'wrong schema or identity in {p}'); continue
            if r.get('run') != man['run'] or r.get('stream') != man['stream']: problems.append(f'run or stream differs from the manifest in {p}'); continue
            if 'sources' in man and r.get('sources') != man['sources']: problems.append(f'source hashes differ from the manifest in {p}'); continue
            if r['reps_requested'] != man['reps_per_task'] or r['reps_done'] + r['failures'] != r['reps_requested'] or any(len(r[k]) != r['reps_done'] for k in ('theta', 'ci_dq', 'S_dq')):
                problems.append(f'replication counts inconsistent in {p}'); continue
            if r['failures']: problems.append(f"{r['failures']} failed replications in {p}")
            if truth is not None and not np.allclose(truth, r['theta_true'], rtol=1e-12, atol=0): problems.append(f'true parameters differ between tasks in {p}'); continue
            truth = np.array(r['theta_true']); done += r['reps_done']; fails += r['failures']
            for k in ('theta', 'V_dq', 'V_or', 'b1', 'theta_r', 'S_dq', 'S_or', 'bbfit_dq', 'bbfit_or', 'ci_dq', 'ci_or', 'cilo_dq', 'cifit_dq', 'ci_true'): rec.setdefault(k, []).extend(r[k])
        if done == 0: problems.append(f"no successful replications in {cell['name']}"); continue
        th = np.array(rec['theta']); a = dict(cell=cell, reps=done, failures=fails, design={k: v for k, v in des.items() if k != 'comp'}, comp={})
        for c, nm in enumerate(COMP):
            d = des['comp'][nm]; S = np.array(d['Sigma']); x = dict(theta=float(truth[c]), **{k: d[k] for k in ('V', 'lam1', 'kappa', 'max_w2', 'w2_movers', 'w2_bridge', 'b1_over_sd', 'power_steps')},
                                                                   corr=d['rho'], share=d['lam_sq_Sbb_sq_over_V'], mc_var_over_V=float(th[:, c].var(ddof=1) / d['V']), bias_over_sd=float((th[:, c].mean() - truth[c]) / th[:, c].std(ddof=1)))
            for v in ('dq', 'or'):
                Vh = np.array(rec['V_' + v])[:, c]; se = np.sqrt(np.maximum(Vh, 0)); Sh = np.array([q[c] for q in rec['S_' + v]]); bf = np.array([q[c] for q in rec['bbfit_' + v]])
                x['normal_' + v] = dict(**cov_stats(th[:, c] - Z * se, th[:, c] + Z * se, Vh > 0, truth[c]), ratio=float(np.sqrt(Vh.mean() / d['V'])))
                for tag, key in ((v, 'ci_' + v),) + ((('dqlo', 'cilo_dq'), ('dqfit', 'cifit_dq'), ('true', 'ci_true')) if v == 'dq' else ()):
                    ci = [q[c] for q in rec[key]]; ok = np.array([q is not None for q in ci]); g = lambda j: np.array([q[j] if q is not None else np.nan for q in ci], float)
                    x['q1_' + tag] = dict(**cov_stats(g(0), g(1), ok, truth[c]), kappa_median=float(np.nanmedian(g(2))) if ok.any() else None)
                    if key in ('ci_dq', 'ci_or'): x['q1_' + tag]['fitted_larger'] = int(sum(q is not None and q[4] == 'fitted' for q in ci))
                x['Sigma_' + v] = dict(bb_leaveout=float(Sh[:, 0, 0].mean() / S[0, 0]), bb_leaveout_cv=float(Sh[:, 0, 0].std(ddof=1) / S[0, 0]), bb_leaveout_nonpos=int((Sh[:, 0, 0] <= 0).sum()),
                                       bb_fitted=float(bf.mean() / S[0, 0]), bb_fitted_cv=float(bf.std(ddof=1) / S[0, 0]), rr=float(Sh[:, 1, 1].mean() / S[1, 1]), rr_cv=float(Sh[:, 1, 1].std(ddof=1) / S[1, 1]),
                                       br_bias=float((Sh[:, 0, 1].mean() - S[0, 1]) / np.sqrt(S[0, 0] * S[1, 1])), br_sd=float(Sh[:, 0, 1].std(ddof=1) / np.sqrt(S[0, 0] * S[1, 1])))
            a['comp'][nm] = x
        agg[cell['name']] = a
    for p in problems: print('PROBLEM', p)
    incomplete = [p for p in problems if 'failed replications' not in p]
    if incomplete and not partial: print(f'{len(incomplete)} inventory problems; nothing aggregated'); return 2
    # acceptance
    acc = man.get('acceptance'); fails = []; summary = {}
    if acc:
        cells = [n for n, a in agg.items() if a['cell'].get('role') == 'acceptance']; cov = []
        for n in cells:
            a = agg[n]
            if a['failures'] > acc['failed_replications_max']: fails.append(f"{n}: {a['failures']} failed replications")
            for nm in acc['components']:
                q = a['comp'][nm]['q1_dq']; cov.append(q['cover'])
                if q['cover'] < acc['q1_cover_min']: fails.append(f"{n} {nm}: q1 coverage {q['cover']:.3f} < {acc['q1_cover_min']}")
                if q['none'] / a['reps'] > acc['no_interval_max']: fails.append(f"{n} {nm}: no interval in {q['none']} of {a['reps']} draws")
                if a['cell']['n_bridge'] is None and q['len_mean'] / a['comp'][nm]['normal_dq']['len_mean'] > acc['len_ratio_no_bottleneck_max']:
                    fails.append(f"{n} {nm}: q1 length {q['len_mean'] / a['comp'][nm]['normal_dq']['len_mean']:.3f} times the normal length without a bottleneck")
            s = a['comp']['sum']['normal_dq']
            if s['cover'] < acc['sum_normal_cover_min']: fails.append(f"{n} sum: normal coverage {s['cover']:.3f} < {acc['sum_normal_cover_min']}")
        if cov and np.mean(cov) < acc['q1_cover_mean_min']: fails.append(f"mean q1 coverage {np.mean(cov):.4f} < {acc['q1_cover_mean_min']}")
        if [p for p in problems if 'failed replications' in p] and acc['failed_replications_max'] == 0 and not any('failed replications' in f for f in fails): fails += [p for p in problems if 'failed replications' in p]
        summary = dict(acceptance_cells=len(cells), q1_cover_min=float(np.min(cov)) if cov else None, q1_cover_max=float(np.max(cov)) if cov else None, q1_cover_mean=float(np.mean(cov)) if cov else None)
    def dump(obj, name):
        tmp = f'{oroot}/{name}.tmp'; json.dump(obj, open(tmp, 'w'), indent=1); os.replace(tmp, f'{oroot}/{name}')
    dump(dict(run=man['run'], problems=problems, cells=agg), 'aggregate.json')
    lines = ['| cell | role | reps | comp | kappa | max w^2 | b1/sd | normal: cover | q1: cover | miss lo/hi | fitted larger | none | q1 leave-out: cover (none) | q1 fitted: cover | q1 true matrix: cover | len q1/normal | len q1/true | Sbb lo, fit | Srr |', '|' + '---|' * 19]
    for n, a in agg.items():
        for nm in COMP:
            x = a['comp'][nm]; q = x['q1_dq']
            lines.append(f"| {n} | {a['cell'].get('role', '')} | {a['reps']} | {nm} | {x['kappa']:.2f} | {x['max_w2']:.3f} | {x['b1_over_sd']:.1f} | {x['normal_dq']['cover']:.3f} | {q['cover']:.3f} | {q['miss_low']:.3f}/{q['miss_high']:.3f} | {q['fitted_larger']} | {q['none']} | "
                         f"{x['q1_dqlo']['cover']:.3f} ({x['q1_dqlo']['none']}) | {x['q1_dqfit']['cover']:.3f} | {x['q1_true']['cover']:.3f} | {q['len_mean'] / x['normal_dq']['len_mean']:.2f} | {q['len_mean'] / x['q1_true']['len_mean']:.2f} | {x['Sigma_dq']['bb_leaveout']:.2f}, {x['Sigma_dq']['bb_fitted']:.2f} | {x['Sigma_dq']['rr']:.2f} |")
    open(f'{oroot}/aggregate.md', 'w').write('\n'.join(lines) + '\n')
    if acc:
        ok = not fails and not incomplete; dump(dict(run=man['run'], passed=ok, complete=not incomplete, acceptance=acc, failures=fails, inventory_problems=incomplete, summary=summary), 'validation.json')
        for f in fails: print('FAIL', f)
        print('summary', summary, 'PASSED' if ok else 'FAILED'); return 0 if ok else (2 if incomplete else 3)
    print(f'aggregated {len(agg)} cells; no acceptance block in the manifest'); return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1], sys.argv[2], '--allow-partial' in sys.argv))

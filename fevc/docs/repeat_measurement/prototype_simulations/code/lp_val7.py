"""Apply the acceptance thresholds of a validation manifest to a completed run.
Usage: lp_val7.py <out-root> <manifest>
Requires aggregate.json from lp_agg.py with zero problems (complete inventory).  Enumerates EVERY failing cell,
writes <out-root>/validation.json and exits 0 if all thresholds hold, 3 otherwise, 2 if the run is incomplete."""
import sys, os, json, glob, numpy as np
oroot, mfile = sys.argv[1], sys.argv[2]; man = json.load(open(mfile)); acc = man['acceptance']
afile = oroot + '/aggregate.json'
if not os.path.exists(afile): raise SystemExit(2)
agg = json.load(open(afile))
if agg['problems'] or len(agg['cells']) != len(man['cells']): print('incomplete run:', agg['problems'][:5]); raise SystemExit(2)
comp = ('person', 'firm', 'cov', 'sum'); fails = []; rows = []; covs = []
for c in man['cells']:
    name = f"{c['scenario']}__{c['design']}"; a = agg['cells'][name]; e = a['est']['dq']
    nonpos = 0; eigmin = 9.0; fl = 0.0; scf = 0.0; seen = 0
    for p in sorted(glob.glob(f'{oroot}/{name}/task-*/result.json')):
        r = json.load(open(p)); nonpos += r['nonpositive']['dq']; eigmin = min(eigmin, r['eig_ratio']['dq'][1]); seen += 1
        fl = max(fl, r['diag']['dq:final_floored'][2]); scf = max(scf, r['diag']['dq:scale_floored'][2])
    if seen != c['tasks']: fails.append(f'{name}: {seen} task files, expected {c["tasks"]}')
    if a['reps'] != c['tasks'] * c['reps_per_task'] or a['failures'] > acc['max_failed_draws']: fails.append(f"{name}: {a['reps']} draws, {a['failures']} failed")
    if nonpos > acc['max_nonpositive']: fails.append(f'{name}: {nonpos} draws with a non-positive variance estimate')
    if max(fl, scf) > acc['max_floored']: fails.append(f'{name}: floors bind for up to {max(fl, scf):.4f} of movers rows')
    for j, cn in enumerate(comp):
        if not (acc['ratio_min'] <= e['ratio'][j] <= acc['ratio_max']): fails.append(f"{name} {cn}: ratio {e['ratio'][j]:.3f} outside [{acc['ratio_min']}, {acc['ratio_max']}]")
        if e['coverage_true'][j] < acc['coverage_min']: fails.append(f"{name} {cn}: coverage {e['coverage_true'][j]:.3f} below {acc['coverage_min']}")
    covs += e['coverage_true']
    rows.append(dict(cell=name, reps=a['reps'], failures=a['failures'], ratio=e['ratio'], coverage=e['coverage_true'], ratio_or=a['est']['or']['ratio'],
                     coverage_or=a['est']['or']['coverage_true'], nonpositive=nonpos, eig_ratio_min=eigmin, max_final_floored=fl, max_scale_floored=scf,
                     mc_var_over_theory=a['mc_var_over_theory'], bias_over_sd=a['bias_over_sd'], se_cv=e['se_cv']))
mc = float(np.mean(covs))
if mc < acc['mean_coverage_min']: fails.append(f"mean coverage {mc:.4f} below {acc['mean_coverage_min']}")
R = np.array([r['ratio'] for r in rows]); C = np.array([r['coverage'] for r in rows])
out = dict(schema='LP-VALIDATION-V1', run=os.path.basename(os.path.normpath(oroot)), acceptance=acc, passed=not fails, failures=fails, cells=rows,
           summary=dict(cells=int(R.size), ratio_min=float(R.min()), ratio_max=float(R.max()), coverage_min=float(C.min()), coverage_max=float(C.max()), mean_coverage=mc))
tmp = oroot + '/validation.json.tmp'; json.dump(out, open(tmp, 'w'), indent=1); os.replace(tmp, oroot + '/validation.json')
print(json.dumps(out['summary'])); print('PASSED' if not fails else f'FAILED ({len(fails)}):'); [print('  ', x) for x in fails]
raise SystemExit(0 if not fails else 3)

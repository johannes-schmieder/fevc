"""Validate and aggregate the simulation output.  Usage: lp_agg.py <out-root> <manifest> [--allow-partial]
Checks the complete inventory before aggregating: every (cell, task) present exactly once, schema, cell identity,
replication counts, failures counted; lists every problem (not just the first) and exits non-zero unless
--allow-partial is given.  Writes aggregate.json and aggregate.md in <out-root>."""
import sys, os, json, glob, numpy as np
oroot, mfile = sys.argv[1], sys.argv[2]; partial = '--allow-partial' in sys.argv
man = json.load(open(mfile)); problems = []; agg = {}
EST = tuple(json.load(open(sys.argv[2])).get('estimators', ['shipped', 'model', 'prop_noadj', 'prop_k3', 'propfree', 'propo_noadj', 'propo_k3', 'propofree']))
for ci, c in enumerate(man['cells']):
    name = f"{c['scenario']}__{c['design']}"; cdir = f'{oroot}/{name}'
    found = sorted(glob.glob(cdir + '/task-*/result.json')); expect = [f'{cdir}/task-{t:03d}/result.json' for t in range(c['tasks'])]
    for p in expect:
        if p not in found: problems.append(f'missing {p}')
    for p in found:
        if p not in expect: problems.append(f'unexpected {p}')
    th, V, fails, done, req = [], {e: [] for e in EST}, 0, 0, 0; dgs = {}
    for p in [p for p in expect if p in found]:
        try: r = json.load(open(p))
        except Exception as ex: problems.append(f'unreadable {p}: {ex!r}'); continue
        if r.get('schema') != 'LP-MC-TASK-V1' or r.get('cell') != ci or r.get('scenario') != c['scenario'] or r.get('design') != c['design']:
            problems.append(f'wrong identity or schema in {p}'); continue
        if r['reps_requested'] != c['reps_per_task'] or r['reps_done'] + r['failures'] != r['reps_requested'] or len(r['theta']) != r['reps_done']:
            problems.append(f'replication counts inconsistent in {p}'); continue
        if any(len(r['V'][e]) != r['reps_done'] for e in EST): problems.append(f'estimator rows inconsistent in {p}'); continue
        if r['failures'] > 0: problems.append(f"{r['failures']} failed replications in {p}")
        th += r['theta']; fails += r['failures']; done += r['reps_done']; req += r['reps_requested']
        for k, x in r.get('diag', {}).items(): dgs.setdefault(k, []).append(x)
        for e in EST: V[e] += r['V'][e]
    tfile = cdir + '/theory.json'
    if not os.path.exists(tfile): problems.append(f'missing {tfile}'); theory = None
    else: theory = json.load(open(tfile))
    if done == 0: problems.append(f'no successful replications in {name}')
    if done == 0 or theory is None: continue
    th = np.array(th); sd = th.std(0, ddof=1); mean = th.mean(0); truth = np.array(theory['theta_true'])
    a = dict(scenario=c['scenario'], design=c['design'], reps=done, failures=fails, requested=req, theta_true=truth.tolist(),
             bias_over_sd=((mean - truth) / sd).tolist(), bias_over_theta=((mean - truth) / truth).tolist(), mc_sd=sd.tolist(),
             independent=theory['independent'], V_indep=theory['V_indep'], mc_var_over_theory=(sd ** 2 / np.array(theory['V_indep'])).tolist(),
             bias_exact_over_sd=(np.array(theory.get('bias_exact', [0, 0, 0, 0])) / np.sqrt(np.array(theory['V_indep']))).tolist(),
             bias_exact_over_theta=(np.array(theory.get('bias_exact', [0, 0, 0, 0])) / truth).tolist(), est={})
    a['diag'] = {k: [float(np.mean([x[0] for x in v])), float(np.min([x[1] for x in v])), float(np.max([x[2] for x in v]))] for k, v in dgs.items()}
    a['regime'] = theory.get('regime'); a['signal_share'] = (np.array(theory['signal']) / np.array(theory['V_indep'])).tolist()
    a['bias_weights_over_sd'] = (np.array(theory.get('bias_draws', [0, 0, 0, 0])) / np.sqrt(np.array(theory['V_indep']))).tolist()
    ref = np.array(theory['V_indep']) if theory['independent'] else sd ** 2
    for e in EST:
        v = np.array(V[e]); se = np.sqrt(np.maximum(v, 0))
        a['est'][e] = dict(ratio=np.sqrt(v.mean(0) / ref).tolist(), ratio_mc=(np.sqrt(v.mean(0)) / sd).tolist(),
                           mean_var_se=(v.std(0, ddof=1) / np.sqrt(len(v)) / ref).tolist(),
                           ratio_median=np.sqrt(np.median(v, 0) / ref).tolist(), se_cv=(se.std(0, ddof=1) / se.mean(0)).tolist(),
                           coverage_true=(np.abs(th - truth) <= 1.959964 * se).mean(0).tolist(),
                           coverage_mean=(np.abs(th - mean) <= 1.959964 * se).mean(0).tolist())
    agg[name] = a
for p in problems: print('PROBLEM', p)
if problems and not partial: raise SystemExit(f'{len(problems)} problems; nothing aggregated')
json.dump(dict(problems=problems, cells=agg), open(oroot + '/aggregate.json', 'w'), indent=1)
lines = ['| cell | reps (failed) | estimator | ratio: worker, firm, cov, sum | coverage of truth (sum) | bias of theta-hat / SD (worker, sum) |', '|---|---|---|---|---|---|']
for name, a in agg.items():
    for e in EST:
        x = a['est'][e]
        lines.append(f"| {name} | {a['reps']} ({a['failures']}) | {e} | " + ', '.join(f'{v:.3f}' for v in x['ratio']) + f" | {x['coverage_true'][3]:.3f} | {a['bias_over_sd'][0]:+.2f}, {a['bias_over_sd'][3]:+.2f} |")
open(oroot + '/aggregate.md', 'w').write('\n'.join(lines) + '\n'); print('\n'.join(lines)); print('problems:', len(problems))

"""End-to-end test of the q = 1 simulation harness on a tiny profile, with deliberate failures.
Usage: test_q1agg.py <scratch-dir>   (the directory is created and must not exist)"""
import sys, os, json, shutil, subprocess, copy
PY = sys.executable; here = os.path.dirname(os.path.abspath(__file__)); root = sys.argv[1]; os.makedirs(root)
from lp_q1sim import source_hashes
man = dict(schema='LP-Q1-MANIFEST-V2', run='tiny', stream=7, probes=200, draws=8, reps_per_task=6, tasks=2, sources=source_hashes(),
           cells=[dict(name='t_bridge', nJ=30, nW=2500, n_bridge=8, design_seed=3, gap=0.3, variance='profile', errors='skew', role='acceptance'),
                  dict(name='t_none', nJ=30, nW=2500, n_bridge=None, design_seed=3, gap=0.0, variance='bridge', errors='normal', role='acceptance')],
           acceptance=dict(components=['person', 'firm', 'cov'], q1_cover_min=0.0, q1_cover_mean_min=0.0, no_interval_max=1.0, sum_normal_cover_min=0.0, len_ratio_no_bottleneck_max=10.0, failed_replications_max=0))
mf = f'{root}/manifest.json'; json.dump(man, open(mf, 'w'))
run = lambda *a: subprocess.run([PY, *a], cwd=here, capture_output=True, text=True)
task = lambda out, m, c, t: run('lp_q1sim.py', 'cell', out, m, str(c), str(t))
agg = lambda out, m=mf: run('lp_q1agg.py', out, m)
base = f'{root}/base'
for c in (0, 1):
    for t in (0, 1):
        r = task(base, mf, c, t); assert r.returncode == 0, r.stderr[-2000:]
checks = []
def check(name, cond, r=None):
    checks.append((name, bool(cond))); print('ok  ' if cond else 'FAIL', name, '' if cond or r is None else (r.stdout[-500:] + r.stderr[-500:]))
r = agg(base); check('complete run aggregates and passes loose thresholds (exit 0)', r.returncode == 0, r)
v = json.load(open(f'{base}/validation.json')); a = json.load(open(f'{base}/aggregate.json'))
check('validation receipt: passed, complete, 2 cells', v['passed'] and v['complete'] and v['summary']['acceptance_cells'] == 2)
check('aggregate: 12 replications per cell, 4 components, design recorded', all(c['reps'] == 12 and len(c['comp']) == 4 and c['design']['rows'] > 0 for c in a['cells'].values()))
check('no-bottleneck cell has no bridging workers; bottleneck cell has them', a['cells']['t_none']['design']['bridge_workers'] == 0 and a['cells']['t_bridge']['design']['bridge_workers'] > 0)
r = task(base, mf, 0, 0); check('a task refuses to overwrite its output', r.returncode != 0 and 'refusing' in r.stderr + r.stdout, r)
r = task(base, mf, 0, 2); check('a task index outside the manifest is refused', r.returncode != 0, r)
def variant(name, fn):
    d = f'{root}/{name}'; shutil.copytree(base, d)
    for f in ('aggregate.json', 'aggregate.md', 'validation.json'): os.remove(f'{d}/{f}')
    m = fn(d); return d, (m if isinstance(m, str) and m.endswith('manifest.json') else mf)
def edit(path, fn):
    r = json.load(open(path)); fn(r); json.dump(r, open(path, 'w'))
d, m = variant('missing', lambda d: os.remove(f'{d}/t_bridge/task-001.json')); r = agg(d, m); check('missing task: exit 2, no receipt', r.returncode == 2 and not os.path.exists(f'{d}/validation.json'), r)
d, m = variant('extra', lambda d: shutil.copy(f'{d}/t_none/task-001.json', f'{d}/t_none/task-002.json')); r = agg(d, m); check('unexpected (duplicate) task file: exit 2', r.returncode == 2, r)
d, m = variant('partial', lambda d: open(f'{d}/t_none/task-001.json', 'w').write(open(f'{base}/t_none/task-001.json').read()[:2000])); r = agg(d, m); check('truncated task file: exit 2', r.returncode == 2, r)
d, m = variant('schema', lambda d: edit(f'{d}/t_none/task-000.json', lambda r: r.update(schema='LP-Q1-TASK-V2'))); r = agg(d, m); check('wrong schema: exit 2', r.returncode == 2, r)
d, m = variant('swapped', lambda d: shutil.copy(f'{d}/t_none/task-000.json', f'{d}/t_none/task-001.json')); r = agg(d, m); check('a task file carrying another task index: exit 2', r.returncode == 2, r)
d, m = variant('stream', lambda d: edit(f'{d}/t_bridge/task-000.json', lambda r: r.update(stream=8))); r = agg(d, m); check('stream differs from the manifest: exit 2', r.returncode == 2, r)
def bad_sources(d):
    m2 = copy.deepcopy(man); m2['sources']['lp7.py'] = '0' * 64; p = f'{d}/manifest.json'; json.dump(m2, open(p, 'w')); return p
d, m = variant('sources', bad_sources); r = agg(d, m); check('source hashes differ from the manifest: exit 2', r.returncode == 2, r)
r = task(f'{root}/sources_task', m, 0, 0); check('a task refuses to run on sources other than the manifest', r.returncode != 0 and 'source files differ' in r.stderr + r.stdout, r)
def failed_rep(r):
    for k in ('theta', 'V_dq', 'V_or', 'b1', 'theta_r', 'S_dq', 'S_or', 'bbfit_dq', 'bbfit_or', 'ci_dq', 'ci_or', 'cilo_dq', 'cifit_dq', 'ci_true'): r[k].pop()
    r['reps_done'] -= 1; r['failures'] += 1
d, m = variant('failedrep', lambda d: edit(f'{d}/t_bridge/task-001.json', failed_rep)); r = agg(d, m)
check('one failed replication: counted, exit 3, listed in the receipt', r.returncode == 3 and any('failed replication' in f for f in json.load(open(f'{d}/validation.json'))['failures']), r)
d, m = variant('counts', lambda d: edit(f'{d}/t_bridge/task-001.json', lambda r: r['theta'].pop())); r = agg(d, m); check('replication counts inconsistent: exit 2', r.returncode == 2, r)
def strict(key, val):
    def f(d):
        m2 = copy.deepcopy(man); m2['acceptance'][key] = val; p = f'{d}/manifest.json'; json.dump(m2, open(p, 'w')); return p
    return f
d, m = variant('cover', strict('q1_cover_min', 1.01)); r = agg(d, m); v = json.load(open(f'{d}/validation.json'))
check('coverage below the threshold: exit 3 and all six cell-components listed', r.returncode == 3 and not v['passed'] and sum('q1 coverage' in f for f in v['failures']) == 6, r)
d, m = variant('mean', strict('q1_cover_mean_min', 1.01)); r = agg(d, m); check('mean coverage below the threshold: exit 3', r.returncode == 3 and any('mean q1 coverage' in f for f in json.load(open(f'{d}/validation.json'))['failures']), r)
d, m = variant('sum', strict('sum_normal_cover_min', 1.01)); r = agg(d, m); check('coverage of the sum below the threshold: exit 3, both cells listed', r.returncode == 3 and sum(' sum: ' in f for f in json.load(open(f'{d}/validation.json'))['failures']) == 2, r)
d, m = variant('length', strict('len_ratio_no_bottleneck_max', 0.5)); r = agg(d, m); check('length rule applies to the no-bottleneck cell only: exit 3, three entries', r.returncode == 3 and sum('without a bottleneck' in f for f in json.load(open(f'{d}/validation.json'))['failures']) == 3, r)
def no_interval(r):
    for row in r['ci_dq']: row[0] = None
d, m = variant('none', lambda d: [edit(f'{d}/t_bridge/task-00{t}.json', no_interval) for t in (0, 1)] and strict('no_interval_max', 0.01)(d)); r = agg(d, m)
check('draws without an interval count as not covered and against the limit: exit 3', r.returncode == 3 and any('no interval in 12 of 12' in f for f in json.load(open(f'{d}/validation.json'))['failures'])
      and json.load(open(f'{d}/aggregate.json'))['cells']['t_bridge']['comp']['person']['q1_dq']['cover'] == 0.0, r)
r1 = json.load(open(f'{base}/t_bridge/task-000.json')); r = task(f'{root}/again', mf, 0, 0); r2 = json.load(open(f'{root}/again/t_bridge/task-000.json'))
check('a task is reproducible from the manifest alone', r1['theta'] == r2['theta'] and r1['ci_dq'] == r2['ci_dq'])
r3 = json.load(open(f'{base}/t_bridge/task-001.json')); check('tasks draw different outcomes', r1['theta'][0] != r3['theta'][0])
print(f'{sum(c for _, c in checks)} of {len(checks)} checks passed'); raise SystemExit(0 if all(c for _, c in checks) else 1)

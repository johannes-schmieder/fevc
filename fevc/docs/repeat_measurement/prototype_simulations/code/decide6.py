"""Apply the round-6 decision rule (PLAN.md, fixed before the run) to an mc-v6 aggregate.
usage: decide6.py <aggregate.json> [real floor shares as cand=share ...]"""
import json, sys
a = json.load(open(sys.argv[1]))
real_floor = dict((kv.split('=')[0], float(kv.split('=')[1])) for kv in sys.argv[2:])
cells = a['cells']
print('problems:', a.get('problems'))
keys = sorted(cells, key=lambda k: int(k.split('__')[0][1:]))
cands = ['tw', 'twl', 'tlw', 'dq']
names = ['alpha', 'psi', 'cov', 'sum']
summary = {}
for c in cands:
    print(f'\n== {c}: sqrt(mean Vhat / V), then coverage; reps and failures')
    lo, hi, clo = 9.0, 0.0, 9.0
    for k in keys:
        cell = cells[k]
        if c not in cell['est']:
            print(k, 'MISSING'); continue
        r = cell['est'][c]['ratio']; cv = cell['est'][c]['coverage_true']
        fl = cell['diag'].get(f'cells:{c}_floored', [0, 0, 0])
        print(f"{k.split('__')[0]:>4} " + ' '.join(f'{x:5.3f}' for x in r) + '   ' + ' '.join(f'{x:5.3f}' for x in cv)
              + f"   reps {cell['reps']} fail {cell['failures']} floored mean {fl[0]:.4f} max {fl[2]:.4f}")
        lo = min(lo, min(r)); hi = max(hi, max(r)); clo = min(clo, min(cv))
    summary[c] = (lo, hi, clo)
    print(f'   min ratio {lo:.3f}  max ratio {hi:.3f}  min coverage {clo:.3f}')
print('\nRule: among candidates with max ratio <= 1.10 (and < 0.5% of movers rows floored on the data), the largest'
      ' minimum ratio; within 0.02 of the best, prefer twl.')
ok = {c: v for c, v in summary.items() if v[1] <= 1.10 and real_floor.get(c, 0.0) < 0.005}
for c in cands:
    why = []
    if summary[c][1] > 1.10: why.append(f'max ratio {summary[c][1]:.3f} > 1.10')
    if real_floor.get(c, 0.0) >= 0.005: why.append(f'{100*real_floor[c]:.1f}% floored on the data')
    print(f'{c:>4}: min {summary[c][0]:.3f} max {summary[c][1]:.3f} ' + ('ELIGIBLE' if c in ok else 'excluded: ' + '; '.join(why)))
if ok:
    best = max(ok, key=lambda c: ok[c][0])
    if 'twl' in ok and ok[best][0] - ok['twl'][0] <= 0.02: best = 'twl'
    print('WINNER:', best)
else:
    print('NO ELIGIBLE CANDIDATE')

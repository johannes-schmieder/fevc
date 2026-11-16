"""Table of the end-to-end package runs on the private panels (aggregates only). Usage: e2e_summary.py <results dir>"""
import sys, glob, os, json

COMP = ('worker variance', 'firm variance', 'covariance', 'sum')
rows = []
for f in sorted(glob.glob(sys.argv[1] + '/*__*.txt')):
    d = dict(line.rstrip('\n').split('=', 1) for line in open(f) if '=' in line)
    num = lambda s: float('nan') if s.strip() in ('.', '') else float(s)
    r = dict(label=d['label'], deletion=d['deletion'], person_years=int(d['person_years']), retained=int(d['retained']), status=d['status'],
             seconds_point=float(d['seconds_point']), seconds_auto=float(d['seconds_auto']), threads=int(d['threads']), e_V_posted=int(d['e_V_posted']),
             q1_components=int(float(d['inference_q1_components'])), leading_steps=int(float(d['inference_leading_steps'])), unresolved=int(float(d['inference_leading_unresolved'])),
             sets=int(float(d['inference_repeat_sets'])), offset_pass_solves=num(d['inference_offset_pass_solves']), point_reldif=float(d['point_reldif']), comp=[])
    for j in range(1, 5):
        t = [num(x) for x in d[f'intervals{j}'].split()]; l = [num(x) for x in d[f'leading{j}'].split()]
        r['comp'].append(dict(name=COMP[j - 1], estimate=t[0], se=t[1], normal=[t[2], t[3]], q1=[t[4], t[5]], regime=t[6], reported=[t[7], t[8]], share=t[9], kappa=t[10],
                              critical=t[11], eigen_share=t[12], max_w2=t[13], converged=t[14], q1_status=t[15], steps=l[2], eigenvalue=l[4], residual=l[6]))
    rows.append(r)
json.dump(rows, open(sys.argv[1] + '/summary.json', 'w'), indent=1)
for r in rows:
    print(f"{r['label']:12s} {r['deletion']:11s} person-years {r['person_years']:,} retained {r['retained']:,} sets {r['sets']:,}  point {r['seconds_point']:.0f}s  automatic {r['seconds_auto']:.0f}s "
          f"(+{100 * (r['seconds_auto'] / r['seconds_point'] - 1):.0f}%)  steps {r['leading_steps']} unresolved {r['unresolved']}  q1 components {r['q1_components']}  e(V) {'posted' if r['e_V_posted'] else 'not posted'}  {r['status']}")
    for c in r['comp']:
        kind = {0.0: 'normal', 1.0: 'q = 1'}.get(c['regime'], 'none')
        print(f"    {c['name']:16s} {c['estimate']:.6f}  se {c['se']:.6f}  reported [{c['reported'][0]:.6f}, {c['reported'][1]:.6f}] {kind:7s} q1 [{c['q1'][0]:.6f}, {c['q1'][1]:.6f}]  "
              f"share {100 * c['share']:.2f}%  kappa {c['kappa']:.3f}  max w2 {c['max_w2']:.3f}  steps {c['steps']:.0f}")

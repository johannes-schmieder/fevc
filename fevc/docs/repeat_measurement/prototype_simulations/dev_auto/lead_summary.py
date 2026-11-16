"""Summarise real_lead_<design>.json files: solves needed by Lanczos for the leading direction and how the q = 1
diagnostics from an early-stopped direction compare with the converged one."""
import json, sys
for path in sys.argv[1:]:
    r = json.load(open(path)); print(f"\n{path.split('/')[-1]}: rows {r['rows']:,}  seconds per solve {r['seconds_per_solve']:.2f}  SE solves {r['solves_standard_error']}")
    for c, v in r['comp'].items():
        s8, sf = v['restart8']['steps_to'], v['full']['steps_to']
        print(f"  {c:6s} steps to residual 1e-2/1e-3/1e-4/1e-6: restart-8 {s8['0.01']}/{s8['0.001']}/{s8['0.0001']}/{s8['1e-06']}   no restart {sf['0.01']}/{sf['0.001']}/{sf['0.0001']}/{sf['1e-06']}"
              + (f"   warm start to 1e-3: {v['warm_start_1e-3']['steps']}" if 'warm_start_1e-3' in v else ''))
        ref = v['variant']['tol1e-6']
        for k in ('cap10', 'cap20', 'tol1e-3', 'tol1e-6'):
            x = v['variant'][k]; q = x['q1']
            print(f"     {k:8s} steps {x['steps']:3d} resid {x['resid']:.1e} lam/ref-1 {x['lam'] / ref['lam'] - 1:+.1e} lam2/lam {x['lam2'] / x['lam']:+.3f} max w2 {x['max_w2']:.3f} share {x['variance_share']:.5f} "
                  f"kappa {q['kappa'] if q else float('nan'):.4f} q1 [{q['lo']:.6f}, {q['hi']:.6f}] normal [{x['normal']['lo']:.6f}, {x['normal']['hi']:.6f}]")

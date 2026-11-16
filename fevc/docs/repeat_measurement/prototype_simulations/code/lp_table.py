"""LaTeX rows for the six-year simulation table from aggregate.json.  Usage: lp_table.py <aggregate.json> <out.tex>"""
import sys, json
agg = json.load(open(sys.argv[1]))['cells']
LAB = {'L1__default': ('1', 'normal, working-model variances', 'person-years'),
       'L4__default': ('4', 'normal, variance by wage level and by worker', 'person-years'),
       'L5__default': ('5', 'fat tails, $\\kappa=6$', 'person-years'),
       'L11__default': ('11', 'skew against the wage level', 'person-years'),
       'L12__default': ('12', 'skew with the wage level', 'person-years'),
       'LAR5__default': ('AR', 'stayers AR(1), $\\rho=0.5$', 'person-years'),
       'LAR5__gap1': ('AR', 'stayers AR(1), $\\rho=0.5$', 'two blocks, gap 1'),
       'LAR5__firstlast': ('AR', 'stayers AR(1), $\\rho=0.5$', 'first and last year'),
       'LRW__default': ('RW', 'stayers random walk plus noise', 'person-years'),
       'LRW__gap1': ('RW', 'stayers random walk plus noise', 'two blocks, gap 1'),
       'L1__gap1': ('1', 'normal, working-model variances', 'two blocks, gap 1')}
rows = []
for k, (sc, desc, des) in LAB.items():
    if k not in agg: continue
    a = agg[k]; e = a['est']
    f = lambda est, c: f"{e[est]['ratio'][c]:.2f}"
    rows.append(f"{sc} & {des} & {a['bias_over_sd'][0]:+.1f} & {a['bias_over_sd'][3]:+.1f} & "
                f"{f('shipped', 3)} & {f('model', 3)} & {f('prop_k3', 3)} & {f('propfree', 3)} & {f('propo_k3', 3)} & & "
                f"{f('shipped', 0)} & {f('prop_k3', 0)} & {f('propo_k3', 0)} & {e['prop_k3']['coverage_true'][3]:.2f} \\\\")
open(sys.argv[2], 'w').write('\n'.join(rows) + '\n'); print('\n'.join(rows))
for k, a in agg.items():
    print(k, 'reps', a['reps'], 'failed', a['failures'], 'MC var/theory', [round(x, 2) for x in a['mc_var_over_theory']],
          'bias exact/SD', [round(x, 2) for x in a['bias_exact_over_sd']], 'bias MC/SD', [round(x, 2) for x in a['bias_over_sd']],
          'bias/theta', [round(x, 3) for x in a['bias_over_theta']])

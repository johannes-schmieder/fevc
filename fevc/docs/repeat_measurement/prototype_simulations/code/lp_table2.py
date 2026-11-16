"""LaTeX table and summary numbers for the six-year simulation (rounds 1 and 2).
Usage: lp_table2.py <agg_v1.json> <agg_v2.json or -> <out.tex>"""
import sys, json
a1 = json.load(open(sys.argv[1]))['cells']; a2 = json.load(open(sys.argv[2]))['cells'] if sys.argv[2] != '-' else {}
LAB = [('L1__default', '1', 'normal, working-model variances', 'person-years'),
       ('L5__default', '5', 'fat tails ($\\kappa=6$)', 'person-years'),
       ('L4__default', '4', 'variance by wage level and by worker', 'person-years'),
       ('L11__default', '11', 'skew against wage level; variance by wage level', 'person-years'),
       ('L12__default', '12', 'skew with wage level; variance by wage level', 'person-years'),
       ('L1__gap1', '1', 'normal, working-model variances', 'two blocks, gap 1'),
       ('LAR5__default', 'AR', 'stayers AR(1), $\\rho=0.5$', 'person-years'),
       ('LAR5__gap1', 'AR', '', 'two blocks, gap 1'),
       ('LAR5__firstlast', 'AR', '', 'first and last year'),
       ('LRW__default', 'RW', 'stayers random walk plus noise', 'person-years'),
       ('LRW__gap1', 'RW', '', 'two blocks, gap 1')]
rows = []
for k, sc, desc, des in LAB:
    a = a1[k]; e = a['est']; h = a2.get(k, {}).get('est', {})
    r = lambda est, c, src=e: f"{src[est]['ratio'][c]:.2f}" if est in src else '---'
    rows.append(f"{sc} & {des} & {a['bias_over_sd'][0]:+.1f} & {a['bias_over_sd'][3]:+.1f} & "
                f"{r('shipped', 3)} & {r('model', 3)} & {r('prop_k3', 3)} & {r('hyb', 3, h)} & {r('propo_k3', 3)} & & "
                f"{r('shipped', 0)} & {r('prop_k3', 0)} & {r('hyb', 0, h)} \\\\")
    if k == 'L1__gap1': rows.append('\\midrule')
open(sys.argv[3], 'w').write('\n'.join(rows) + '\n'); print('\n'.join(rows))
if a2:
    print('round 2 reproduces round 1 for shared estimators:')
    for k, c in a2.items():
        for est in ('shipped', 'prop_k3', 'propo_k3'):
            d = max(abs(x - y) for x, y in zip(c['est'][est]['ratio'], a1[k]['est'][est]['ratio']))
            print(f'   {k} {est} max |diff| {d:.2e}')
        print(f"   {k} hyb ratio {[round(x,3) for x in c['est']['hyb']['ratio']]} hybo {[round(x,3) for x in c['est']['hybo']['ratio']]} cov_true hyb {[round(x,2) for x in c['est']['hyb']['coverage_true']]}")

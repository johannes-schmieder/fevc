"""Rows of the six-year simulation table from rounds 1-3.  Usage: lp_table3.py <v1> <v2> <v3> <out.tex>"""
import sys, json
a1, a2, a3 = (json.load(open(p))['cells'] for p in sys.argv[1:4])
LAB = [('L1__default', '1', 'person-years'), ('L5__default', '5', 'person-years'), ('L4__default', '4', 'person-years'),
       ('L11__default', '11', 'person-years'), ('L12__default', '12', 'person-years'), ('L1__gap1', '1', 'two blocks, gap 1'),
       ('LAR5__default', 'AR', 'person-years'), ('LAR5__gap1', 'AR', 'two blocks, gap 1'), ('LAR5__firstlast', 'AR', 'first and last year'),
       ('LRW__default', 'RW', 'person-years'), ('LRW__gap1', 'RW', 'two blocks, gap 1')]
rows = []
for k, sc, des in LAB:
    a = a1[k]; e = a['est']; h = a2.get(k, {}).get('est', {}); t = a3.get(k, {}).get('est', {})
    r = lambda est, c, src: f"{src[est]['ratio'][c]:.2f}" if est in src else '---'
    rows.append(f"{sc} & {des} & {a['bias_over_sd'][0]:+.1f} & {a['bias_over_sd'][3]:+.1f} & "
                f"{r('shipped', 3, e)} & {r('model', 3, e)} & {r('prop_k3', 3, e)} & {r('propt', 3, t)} & {r('hyb', 3, h)} & {r('propo_k3', 3, e)} & & "
                f"{r('shipped', 0, e)} & {r('prop_k3', 0, e)} & {r('propt', 0, t)} \\\\")
    if k == 'L1__gap1': rows.append('\\midrule')
open(sys.argv[4], 'w').write('\n'.join(rows) + '\n'); print('\n'.join(rows))

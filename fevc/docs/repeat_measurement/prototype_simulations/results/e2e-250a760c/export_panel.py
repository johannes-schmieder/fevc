"""Write a private panel as a CSV for Stata (worker, firm, year, y). The file goes to node-local scratch and is never
copied out. Usage: export_panel.py <design dir> <output csv>"""
import sys
import numpy as np

z = np.load(sys.argv[1] + '/panel.npz')
w, f, year, y = z['w'], z['f'], z['year'], z['y']
assert len(w) == len(f) == len(year) == len(y) and np.isfinite(y).all()
np.savetxt(sys.argv[2], np.column_stack([w + 1, f + 1, year, y]), delimiter=',', header='worker,firm,year,y', comments='', fmt=['%d', '%d', '%d', '%.17g'])
print('person-years', len(w), 'workers', len(np.unique(w)), 'firms', len(np.unique(f)))

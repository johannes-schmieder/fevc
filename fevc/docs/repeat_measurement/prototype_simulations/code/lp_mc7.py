"""Validation harness for the final estimator (lp7.LPF): lp_mc6's designs, scenarios and theory, with
  - the estimator of the note only ('dq') and its version with movers' true variances ('or'), no PSD projection;
  - the stream of the error draws taken from the manifest ('draw_stream'; the development rounds used 1);
  - per task: non-positive variance estimates counted, eigenvalue ratio of the 3x3 matrix, floors.
Usage:
  lp_mc7.py prepare <design-dir> <design> [probes]
  lp_mc7.py theory  <design-dir> <out-root> <manifest> <cell-index>
  lp_mc7.py task    <design-dir> <out-root> <manifest> <task-index>
Every task writes one result.json in its own directory, atomically, and refuses to overwrite."""
import sys, os, json, time, numpy as np
import lp_mc6 as M
from lp6 import build_design, aggregate, T34
from lp7 import LPF

EST = ('dq', 'or')


def engine(ddir, P, design, probes=200):
    D, info = build_design(P, M.DESIGNS[design]); dfile = ddir + f'/diag_{design}_{probes}.npz'
    if os.path.exists(dfile):
        dz = np.load(dfile); E = LPF(D, exact_diag=(dz['Pd'], dz['Bd']))
    else:
        E = LPF(D, probes=probes); tmp = dfile + '.tmp.npz'; np.savez(tmp, Pd=E.A.Pd, Bd=E.A.Bd); os.replace(tmp, dfile)
    return D, info, E


M.engine = engine                                                  # prepare(), context() and theory() of lp_mc6 use it


def task(ddir, oroot, mfile, tindex):
    manifest = json.load(open(mfile)); pairs = [(ci, t) for ci, c in enumerate(manifest['cells']) for t in range(c['tasks'])]
    cell, t = pairs[tindex]; c, sc, P, D, info, E, tr, mu, v_py, sign_py, v_rows = M.context(ddir, manifest, cell)
    odir = f"{oroot}/{c['scenario']}__{c['design']}/task-{t:03d}"; rfile = odir + '/result.json'
    if os.path.exists(rfile): raise SystemExit(f'{rfile} exists; refusing to overwrite')
    assert tuple(manifest['estimators']) == EST, 'manifest estimators must be dq, or'
    stream = int(manifest['draw_stream']); sto = M.stayer_order(P) if sc['kind'] in ('ar', 'rw') else None
    th, V, eig, fails = [], {e: [] for e in EST}, {e: [] for e in EST}, 0; dg = {}; t0 = time.time()
    for r in range(c['reps_per_task']):
        rng = np.random.default_rng([sc['seed'], stream, t, r])
        err = aggregate(info, D.n, M.draw_errors(P, sc, v_py, sign_py, sto, rng))
        try:
            o = E.estimate_final(mu + err, draws=c.get('draws', 16), dseed=100000 * stream + 1000 * t + r, vtrue=v_rows)
        except Exception as ex:                                    # counted, never silently dropped
            fails += 1; print('rep failed', r, repr(ex), flush=True); continue
        th.append(o['theta'].tolist())
        for e in EST: V[e].append(np.diag(T34 @ o[e] @ T34.T).tolist()); eig[e].append(o['eig_' + e])
        for k in ('scale_floored', 'final_floored', 'profile_min', 'profile_max'): dg.setdefault('dq:' + k, []).append(o['diag'][k])
        if r == 0: print(f'task {tindex} cell {cell} rep 0 done in {time.time() - t0:.0f}s', flush=True)
    out = dict(schema='LP-MC-TASK-V1', cell=cell, scenario=c['scenario'], design=c['design'], task=t, reps_requested=c['reps_per_task'],
               reps_done=len(th), failures=fails, theta=th, V=V, seconds=time.time() - t0, rows=int(D.n), draw_stream=stream,
               nonpositive={e: int((np.array(V[e]) <= 0).any(1).sum()) if V[e] else 0 for e in EST},
               eig_ratio={e: ([float(np.mean(eig[e])), float(np.min(eig[e])), float(np.max(eig[e]))] if eig[e] else None) for e in EST},
               diag={k: [float(np.mean(x)), float(np.min(x)), float(np.max(x))] for k, x in dg.items()},
               job_id=os.environ.get('JOB_ID', ''), host=os.environ.get('HOSTNAME', ''))
    M.write_json(rfile, out); print('wrote', rfile, f'{out["seconds"]:.0f}s', flush=True)


if __name__ == '__main__':
    mode = sys.argv[1]
    if mode == 'prepare': M.prepare(sys.argv[2], sys.argv[3], int(sys.argv[4]) if len(sys.argv) > 4 else 200)
    elif mode == 'theory': M.theory(sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5]))
    elif mode == 'task': task(sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5]))
    else: raise SystemExit('unknown mode')

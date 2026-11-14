"""Package Monte Carlo: simulated panels through the Stata command with the plugin under test.

  pkgmc.py make  <run dir> <spec json>                export the designs and write the manifest
                                                      (refuses an existing run dir)
  pkgmc.py run   <run dir> <parallel> [max] [--stata PATH]
                                                      run the tasks that have no output yet,
                                                      <parallel> at a time
  pkgmc.py check <run dir>                            inventory: every task present, complete,
                                                      with the right keys

`run` needs the Stata executable: --stata PATH, or else the environment variable FEVC_STATA.

The spec names the run, its seed (1 to 20000), the package directory (ado files plus the plugin),
the source commit, the cells (design parameters of designs.two_block_panel and designs.truth,
optionally "control": true and a "realised" object describing the regime the design realises, which
is copied into the manifest verbatim), the deletion modes, replications per task and tasks per cell,
native threads, the fevc options ("none": the plain default command) and the standard-error model
the fits must report; a confirmation spec also carries its tolerances (see confirm.py). The manifest
adds the hashes of the plugin, of the task do-file, of this file, of the design code and of every
exported design, and the list of tasks. A task is (cell, deletion, task index); its replications are
first .. first + reps - 1. Replication r draws its outcomes from stream r + 1 of Stata's stream
generator (mt64s) under the cell's seed run_seed * 10^5 + cell_index, the same under both deletion
modes, so the draws do not depend on the split into tasks or on the order of execution.
"""

import glob
import hashlib
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
DESIGN_SOURCES = ('designs.py',)
TOLERANCES = {'min_success_rate', 'min_coverage', 'min_mean_coverage', 'max_length_ratio_none',
              'min_posted_none'}


def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for block in iter(lambda: f.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def tasks_of(man):
    out = []
    for ci, cell in enumerate(man['cells']):
        for deletion in man['deletions']:
            for t in range(man['tasks_per_cell']):
                out.append(dict(cell=cell['name'], cell_index=ci, deletion=deletion, task=t,
                                first=t * man['reps_per_task'], reps=man['reps_per_task'],
                                seed_base=man['run_seed'] * 10 ** 5 + ci,
                                errors=cell.get('errors', 'normal')))
    return out


def make(run_dir, spec_file):
    run_dir = os.path.abspath(run_dir)
    if os.path.exists(run_dir):
        raise SystemExit(f'{run_dir} exists; refusing to overwrite')
    spec = json.load(open(spec_file))
    for key in ('run', 'run_seed', 'pkgroot', 'source_commit', 'cells', 'deletions',
                'reps_per_task', 'tasks_per_cell', 'threads', 'options', 'expect_model'):
        if key not in spec:
            raise SystemExit(f'spec lacks {key}')
    if (not 1 <= spec['run_seed'] <= 20000 or len(spec['cells']) > 99999
            or spec['reps_per_task'] * spec['tasks_per_cell'] > 32766):
        raise SystemExit('seed layout exceeded')
    if '+' in spec['options'] or '"' in spec['options']:
        raise SystemExit('the options must not contain + or quotes')
    if 'tolerances' in spec and 'expect_threshold' not in spec:
        raise SystemExit(
            'a confirmation spec names the rule threshold it confirms (expect_threshold)')
    if 'tolerances' in spec and set(spec['tolerances']) != TOLERANCES:
        raise SystemExit('tolerances must be exactly min_success_rate, min_coverage, '
                         'min_mean_coverage, max_length_ratio_none, min_posted_none')
    names = [c['name'] for c in spec['cells']]
    if len(set(names)) != len(names):
        raise SystemExit('duplicate cell names')
    if (any(d not in ('match', 'observation') for d in spec['deletions'])
            or len(set(spec['deletions'])) != len(spec['deletions'])):
        raise SystemExit('invalid deletion modes')
    from designs import write_design
    os.makedirs(run_dir + '/designs')
    os.makedirs(run_dir + '/out')
    os.makedirs(run_dir + '/logs')
    man = dict(spec)
    man['schema'] = 'PKGMC-MANIFEST-V1'
    plugin = glob.glob(spec['pkgroot'] + '/fevc_rust_macos_arm64.plugin')
    if len(plugin) != 1:
        raise SystemExit('plugin not found in pkgroot')
    man['hashes'] = dict(
        plugin=sha(plugin[0]), task_do=sha(HERE + '/mc_task.do'),
        driver=sha(os.path.abspath(__file__)),
        aggregate=sha(HERE + '/aggregate.py'),
        confirm=sha(HERE + '/confirm.py') if os.path.exists(HERE + '/confirm.py') else None,
        design_code={f: sha(f'{HERE}/{f}') for f in DESIGN_SOURCES},
        ado={f: sha(f"{spec['pkgroot']}/{f}") for f in (
            'fevc.ado', 'fevc_residual_se.mata', 'fevc__residual_se_post.ado',
            'fevc__component_model_route.ado')})
    man['designs'] = {}
    for cell in spec['cells']:
        path = f"{run_dir}/designs/{cell['name']}.csv"
        info = write_design(cell, path)
        man['designs'][cell['name']] = dict(sha256=sha(path), **info)
        # The regime a design realises is computed elsewhere, without outcomes; a cell's name is not
        # evidence, so a spec that has it carries it and the manifest records it unchanged.
        realised = cell.get('realised')
        if realised is not None:
            man['designs'][cell['name']]['realised'] = realised
        text = ''
        if isinstance(realised, dict):
            text = ' '.join(f"{cp}: kappa {x['kappa']:.3f} share {x['share']:.3f}"
                            for cp, x in realised.items()
                            if isinstance(x, dict) and 'kappa' in x and 'share' in x)
        print(cell['name'], 'person-years', info['person_years'], text, flush=True)
    man['tasks'] = len(tasks_of(man))
    tmp = run_dir + '/manifest.json.tmp'
    json.dump(man, open(tmp, 'w'), indent=1)
    os.replace(tmp, run_dir + '/manifest.json')
    open(run_dir + '/manifest.sha256', 'w').write(sha(run_dir + '/manifest.json') + '\n')
    print('manifest', run_dir + '/manifest.json', 'cells', len(names), 'tasks', man['tasks'])


def stata_path(stata=None):
    """The Stata executable: the --stata argument, else FEVC_STATA. Licensed Stata runs locally."""
    path = stata or os.environ.get('FEVC_STATA')
    if not path:
        raise SystemExit('no Stata executable: pass --stata PATH or set FEVC_STATA')
    if not (os.path.isfile(path) and os.access(path, os.X_OK)):
        raise SystemExit(f'the Stata executable {path} does not exist or is not executable')
    return path


def verify_sources(man, run_dir):
    plugin = man['pkgroot'] + '/fevc_rust_macos_arm64.plugin'
    if sha(plugin) != man['hashes']['plugin']:
        raise SystemExit('the plugin differs from the manifest')
    if sha(HERE + '/mc_task.do') != man['hashes']['task_do']:
        raise SystemExit('the task do-file differs from the manifest')
    for f, h in man['hashes']['ado'].items():
        if sha(f"{man['pkgroot']}/{f}") != h:
            raise SystemExit(f'{f} differs from the manifest')
    for name, d in man['designs'].items():
        if sha(f'{run_dir}/designs/{name}.csv') != d['sha256']:
            raise SystemExit(f'design {name} differs from the manifest')
    if open(run_dir + '/manifest.sha256').read().strip() != sha(run_dir + '/manifest.json'):
        raise SystemExit('the manifest changed')


def run_one(run_dir, man, t, stata):
    out = f"{run_dir}/out/{t['cell']}__{t['deletion']}__{t['task']}.csv"
    if os.path.exists(out):
        return 'present'
    work = f"{run_dir}/logs/{t['cell']}__{t['deletion']}__{t['task']}"
    os.makedirs(work, exist_ok=True)
    # The command must see no switch for its default: a run with the options "none" is the plain
    # default command.
    env = dict(os.environ)
    env.pop('FEVC_DEFAULT_INFERENCE', None)
    # Stata's batch mode splits a quoted argument at its blanks: the options travel as one word.
    cmd = [stata, '-b', 'do', HERE + '/mc_task.do', man['pkgroot'],
           f"{run_dir}/designs/{t['cell']}.csv", t['cell'], t['deletion'], str(t['task']),
           str(t['first']), str(t['reps']), str(t['seed_base']), t['errors'], run_dir + '/out',
           str(man['threads']), '+'.join(man['options'].split())]
    t0 = time.time()
    subprocess.run(cmd, cwd=work, env=env, capture_output=True, text=True)
    ok = os.path.exists(out)
    print(('done ' if ok else 'FAILED ') + os.path.basename(out), f'{time.time() - t0:.0f}s',
          flush=True)
    if not ok:
        # Stata names the log after the last argument
        logs = sorted(glob.glob(work + '/*.log'), key=os.path.getmtime)
        print('   ' + ' | '.join(open(logs[-1]).read().strip().splitlines()[-4:]) if logs
              else '   no Stata log', flush=True)
    return 'done' if ok else 'failed'


def run(run_dir, parallel, limit=None, stata=None):
    stata = stata_path(stata)
    run_dir = os.path.abspath(run_dir)
    man = json.load(open(run_dir + '/manifest.json'))
    verify_sources(man, run_dir)
    todo = [t for t in tasks_of(man)
            if not os.path.exists(f"{run_dir}/out/{t['cell']}__{t['deletion']}__{t['task']}.csv")]
    if limit is not None:
        todo = todo[:limit]
    print('tasks to run', len(todo), 'of', man['tasks'], flush=True)
    with ThreadPoolExecutor(max_workers=parallel) as pool:
        res = list(pool.map(lambda t: run_one(run_dir, man, t, stata), todo))
    failed = res.count('failed')
    print('ran', len(res), 'failed', failed)
    if failed:
        raise SystemExit(1)


def check(run_dir, quiet=False):
    """Inventory of the task outputs. Returns (problems, frame) with one row per replication."""
    import numpy as np
    import pandas as pd
    run_dir = os.path.abspath(run_dir)
    man = json.load(open(run_dir + '/manifest.json'))
    problems = []
    frames = []
    if open(run_dir + '/manifest.sha256').read().strip() != sha(run_dir + '/manifest.json'):
        problems.append('the manifest changed')
    expected = {f"{t['cell']}__{t['deletion']}__{t['task']}.csv": t for t in tasks_of(man)}
    present = {os.path.basename(f) for f in glob.glob(run_dir + '/out/*.csv')}
    for f in sorted(present - set(expected)):
        problems.append(f'unexpected output {f}')
    for f in sorted(glob.glob(run_dir + '/out/*.tmp')):
        problems.append(f'partial output {os.path.basename(f)}')
    for f, t in expected.items():
        if f not in present:
            problems.append(f'missing {f}')
            continue
        try:
            d = pd.read_csv(f'{run_dir}/out/{f}')
        except Exception as ex:
            problems.append(f'unreadable {f}: {ex!r}')
            continue
        want = list(range(t['first'], t['first'] + t['reps']))
        if list(d.get('rep', [])) != want:
            problems.append(f'{f}: replications are not {want[0]}..{want[-1]}')
            continue
        if ((d['cell'] != t['cell']).any() or (d['deletion'] != t['deletion']).any()
                or (d['task'] != t['task']).any()):
            problems.append(f'{f}: keys disagree with the file name')
            continue
        if (d['seed'] != t['seed_base']).any() or (d['stream'] != d['rep'] + 1).any():
            problems.append(f'{f}: seeds disagree with the manifest')
            continue
        if d[['truth1', 'truth2', 'truth3', 'truth4']].isna().any().any():
            problems.append(f'{f}: missing truth')
            continue
        # The truth is computed on the whole design, so a fit on a pruned sample would be compared
        # with the wrong number.
        fitted = d[d['rc'] == 0]
        # A fit without a model name is the default's fallback to point estimates (its
        # standard-error pass stopped): an attempted replication without an interval, which the
        # aggregation counts. Any other model is a wrong run.
        fallback = (fitted['model'].isna() & (fitted['status'] == 'KSS_POINT_ESTIMATES_ONLY')
                    & fitted[['lb1', 'lb2', 'lb3', 'lb4', 'se1', 'qlb1']].isna().all(axis=1))
        with_model = fitted[~fallback]
        if (with_model['model'] != man['expect_model']).any():
            problems.append(f"{f}: a fit did not use the model {man['expect_model']}")
            continue
        if 'expect_threshold' in man and (
                'threshold' not in d
                or not np.allclose(with_model['threshold'], man['expect_threshold'], rtol=0,
                                   atol=1e-12)):
            problems.append(f"{f}: a fit did not use the rule threshold {man['expect_threshold']}")
            continue
        if (fitted['n_retained'] != man['designs'][t['cell']]['person_years']).any():
            problems.append(f'{f}: the retained sample is not the whole design')
            continue
        frames.append(d)
    frame = pd.concat(frames, ignore_index=True) if frames else pd.DataFrame()
    if len(frame) and frame.duplicated(['cell', 'deletion', 'rep']).any():
        problems.append('duplicate replication keys')
    if not quiet:
        print('tasks expected', len(expected), 'present', len(present & set(expected)),
              'replications', len(frame), 'problems', len(problems))
        for p in problems[:20]:
            print('  ', p)
    return problems, frame


def main(argv):
    args = list(argv)
    stata = None
    if '--stata' in args:
        i = args.index('--stata')
        if i + 1 >= len(args):
            raise SystemExit('--stata needs a path')
        stata = args[i + 1]
        del args[i:i + 2]
    if len(args) >= 3 and args[0] == 'make':
        make(args[1], args[2])
    elif len(args) >= 3 and args[0] == 'run':
        run(args[1], int(args[2]), int(args[3]) if len(args) > 3 else None, stata=stata)
    elif len(args) == 2 and args[0] == 'check':
        raise SystemExit(1 if check(args[1])[0] else 0)
    else:
        raise SystemExit(__doc__)


if __name__ == '__main__':
    main(sys.argv[1:])

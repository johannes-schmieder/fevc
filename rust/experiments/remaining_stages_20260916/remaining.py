#!/usr/bin/env python3
"""Frozen remaining FEVC campaign, reusing qualified native and comparator bytes.

No estimator compilation or input generation occurs in consumers. Submission
uses a real smoke and success-checked SGE dependencies. Private outputs never
enter the synthetic run directory.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import copy
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import shutil
import socket
import statistics
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
STAGES = ('screen_400k', 'validation_1_6m', 'private_veneto')
COUNTS = {'screen_400k': (20, 150, 50), 'validation_1_6m': (6, 90, 18),
          'private_veneto': (2, 12, 4), 'smoke': (6, 16, 0)}
CANDIDATE = 'b14496bc86cc25226aae07274c15b6afa454bcb0623b78aa01a867275dfdc7c6'
PRIOR_REPORT = '16150baa2acbf81e494ccaad5967268cf304a2948910a44c2035ab274873c92a'
PRIVATE_INPUT = '23093f69ea594ae1ebae348c70008934c5c6431221e2c3a8f0f44a7b1a7b7e2f'
BINARIES = {'baseline': '6937b0b1045383b2f1c6b7b7840d7f33f7f54b1dc580ba3f0a1c819b8e540a2a',
            'candidate': 'de363ed8cb33bdf84da691e25384f01352cdf12181223888e83db1bdeac2b146'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def put(path, value, replace=False):
    path = Path(path)
    if path.exists() and not replace:
        raise FileExistsError(path)
    temporary = path.with_suffix(path.suffix + '.tmp')
    with temporary.open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    temporary.replace(path)


def support(plan):
    """Import only the frozen first-screen and pinned pool-aware harness."""
    sys.path.insert(0, plan['support'])
    import campaign
    import screen_runner
    import screen_validate
    import screen_scc
    spec = importlib.util.spec_from_file_location('remaining_paper_support', plan['paper_support'])
    paper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(paper)
    profiles = copy.deepcopy(campaign.PROFILES)
    for deletion in ('observation', 'match'):
        profile = copy.deepcopy(campaign.PROFILES['mover_observation_both'])
        profile.update(deletion=deletion, deletion_id=None,
                       engine='auto', preconditioner='auto', family='point')
        profiles['remaining_' + deletion] = profile
    # The twelve-profile campaign definition remains unchanged, even when
    # qualification and this extension are imported in one Python process.
    screen_runner.PROFILES = profiles
    return campaign, screen_runner, screen_validate, screen_scc, paper


def request(call, runner):
    # Preserve the accepted paper's public point request. An explicit
    # deletionid/probeorder pair would change routing/capability on the
    # historical match/both baseline, even when its partition is identical.
    return (f"fevc y, worker(worker) firm(firm) deletion({call['deletion']}) stayers(both) "
            f"backend(rust) rng(counter_v1) probes(200) seed({call['seed']}) "
            "algorithm(jla) maxiter(10000) nodisplay")


def root_for(root, plan, stage):
    return Path(plan['private_output']) if stage == 'private_veneto' else root / stage


def check_private(path):
    path = Path(path)
    if path.is_symlink() or (path.stat().st_mode & 0o077):
        raise ValueError('private work directory is not owner-only')


def inventory_counts(tasks):
    calls = [c for t in tasks for c in t['calls']]
    ids = [c['id'] for c in calls]
    if len(ids) != len(set(ids)) or [t['id'] for t in tasks] != list(range(1, len(tasks)+1)):
        raise ValueError('duplicate/malformed task inventory')
    return len(tasks), sum(not c['warmup'] for c in calls), sum(c['warmup'] for c in calls)


def verify(root, expected=None):
    plan = read(root / 'plan.json')
    if expected is not None and sha(root / 'plan.json') != expected:
        raise ValueError('submitted plan changed')
    if plan['schema'] != 'FEVC-REMAINING-STAGES-V1' or plan['candidate_snapshot_sha256'] != CANDIDATE:
        raise ValueError('unqualified campaign source')
    for name, digest in plan['source_hashes'].items():
        if sha(name) != digest:
            raise ValueError('harness/source drift: ' + name)
    if sha(Path(plan['prior_root']) / 'snapshot.json') != CANDIDATE:
        raise ValueError('prior candidate identity changed')
    if sha(Path(plan['prior_root']) / 'screen/report.json') != PRIOR_REPORT:
        raise ValueError('prior passing report changed')
    modules = support(plan)
    for variant, package in plan['packages'].items():
        if modules[1].verify_package(package['path']) != package['receipt']:
            raise ValueError('qualified package changed')
        if sha(Path(package['path']) / 'adapter.json') != package['adapter_sha256']:
            raise ValueError('adapter identity changed')
    for stage, tasks in plan['stages'].items():
        if inventory_counts(tasks) != tuple(plan['counts'][stage]):
            raise ValueError('frozen stage inventory changed')
        for task in tasks:
            for call in task['calls']:
                if call['variant'] != 'matlab' and call['command'] != request(call, modules[1]):
                    raise ValueError('effective request drift')
    check_private(plan['private_output'])
    return plan, modules


def freeze(root, prior, paper_root, private_source, private_output, local=False, repair_prior=None):
    if (root / 'plan.json').exists():
        raise FileExistsError('campaign already frozen')
    if not local:
        if sha(prior / 'snapshot.json') != CANDIDATE or sha(prior / 'screen/report.json') != PRIOR_REPORT:
            raise ValueError('wrong accepted first-stage evidence')
        if read(prior / 'screen/report.json')['status'] != 'PASS':
            raise ValueError('first stage did not pass')
    private_output.mkdir(mode=0o700, parents=True, exist_ok=False)
    check_private(private_output)
    for directory in ('logs', 'submissions', 'qacct', 'receipts'):
        (root / directory).mkdir(exist_ok=True)
        (private_output / directory).mkdir(exist_ok=True)
    plan = dict(schema='FEVC-REMAINING-STAGES-V1', candidate_snapshot_sha256=CANDIDATE,
                prior_root=str(prior), support=str(prior / 'source/rust/experiments/pipeline_optimization_20260915'),
                paper_support=str(paper_root / 'harness/run.py'), private_output=str(private_output),
                private_input_authorized_on_scc=True, packages={}, inputs={}, stages={},
                counts=COUNTS, operational_repairs_available=3, operational_repairs_used=0,
                source_hashes={}, resources=dict(slots=28, seconds=2700, gib_per_slot=3,
                    smoke_slots=4, smoke_seconds=1500, smoke_rss_gib=10,
                    small_fevc_rss_gib=20, large_fevc_rss_gib=45, matlab_rss_gib=72),
                acceptance=dict(complete_command_geomean_ratio_max=.97, cell_median_ratio_max=1.05,
                    corrected_targets='unchanged first-stage common-draw policy',
                    matlab='descriptive independent-randomization and finite-projection-formula comparison; not finite-draw equality',
                    observation_400k_improvement_goal=.20),
                exclusions=['6.4m', 'new native build', 'installed package replacement', 'paper edits',
                            'fusion', 'spectral prototype', 'commit', 'push'])
    plan['counts'] = copy.deepcopy(COUNTS)
    if repair_prior is not None:
        plan.update(repair_prior=str(repair_prior),repair_prior_sha256=sha(repair_prior/'plan.json'),
                    operational_repairs_available=2,operational_repairs_used=1,
                    repair_scope='unique temporary directory for the deliberate Matlab failure; no estimator/input/gate changes')
        plan['counts']['smoke'] = (1,3,0)
    modules = support(plan)
    campaign, runner, _, _, _ = modules
    for variant in ('baseline', 'candidate'):
        package = prior / 'packages' / variant
        receipt = runner.verify_package(package)
        if not local and receipt != read(prior/'screen/manifest.json')['packages'][variant]:
            raise ValueError('package differs from the passing screen manifest')
        if not local and receipt['files'].get('fevc_rust_linux_x64.plugin') != BINARIES[variant]:
            raise ValueError('wrong qualified Linux binary')
        plan['packages'][variant] = dict(path=str(package), receipt=receipt, adapter_sha256=sha(package/'adapter.json'))
    runtime_path = paper_root / 'runtime.json'
    plan['matlab_runtime'] = read(runtime_path)
    source_files = list(HERE.glob('*.py')) + list(HERE.glob('*.do')) + list(HERE.glob('*.sge'))
    source_files += [HERE/'paper_matlab.m', runtime_path]
    source_files += [root/'local-workflow-report.json',root/'local-failure.json']
    source_files += [Path(plan['support']) / n for n in ('campaign.py', 'screen_runner.py', 'screen_validate.py',
        'screen_scc.py', 'screen_manifest.py', 'screen_input.py', 'thread_adapter.py')]
    source_files += [Path(plan['support']).parent/'optimization_parity_20260913/monitor_rss.py',
                     Path(plan['paper_support'])]
    plan['source_hashes'] = {str(p): sha(p) for p in source_files}

    def input_record(graph, rows):
        key = f'{rows}-{graph}'
        if key not in plan['inputs']:
            p = (private_source if graph == 'private_veneto' else root/'smoke-inputs/private-schema.csv'
                 if graph == 'pooled_schema' else paper_root / 'inputs' / (key+'.csv'))
            meta = read(p.with_suffix('.json'))
            if meta['rows'] != rows or sha(p) != meta['sha256']:
                raise ValueError('frozen input mismatch')
            if graph == 'private_veneto':
                if meta['sha256'] != PRIVATE_INPUT: raise ValueError('private input differs')
                if p.stat().st_mode & 0o077: raise ValueError('private raw input permissions')
            elif graph != 'pooled_schema' and (meta['pattern'] != graph or meta['components'] != 1 or meta['matches'] != rows):
                raise ValueError('graph/sample regime mismatch')
            plan['inputs'][key] = dict(path=str(p), metadata=meta,
                metadata_sha256=sha(p.with_suffix('.json')), sha256=meta['sha256'])
        return plan['inputs'][key]

    registered = campaign.protocol()['stages']
    smoke_cells = [dict(graph=g, rows=8000, threads=4, deletion=d)
                   for g in ('degree4_bottleneck', 'mixed_well_mixed') for d in ('observation', 'match')]
    smoke_cells += [dict(graph='pooled_schema',rows=8000,threads=4,deletion=d) for d in ('observation','match')]
    registered['smoke'] = campaign.calls('smoke', smoke_cells,
        lambda c: ('baseline','candidate') if c['graph']=='pooled_schema' else ('baseline','candidate','matlab'), 1)
    registered['smoke'] = [c for c in registered['smoke'] if not c['warmup']]
    if repair_prior is not None:
        registered['smoke'] = [c for c in registered['smoke'] if c['cell']==0]
    for stage in ('smoke', *STAGES):
        tasks = {}
        for definition in registered[stage]:
            c = copy.deepcopy(definition)
            config = c.pop('configuration')
            c.update(config)
            c['id'] = f"{stage}-c{c['cell']+1:02d}-{'w' if c['warmup'] else 'r'}{c['repetition']}-{c['variant']}"
            c['profile'] = 'remaining_' + c['deletion']
            inp = input_record(c['graph'], c['rows'])
            c['input'] = f"{c['rows']}-{c['graph']}"
            c['input_sha256'] = inp['sha256']
            if c['variant'] != 'matlab':
                c['command'] = request(c, runner)
                c['adapter_sha256'] = plan['packages'][c['variant']]['adapter_sha256']
            task = tasks.setdefault(c['cell']+1, dict(id=c['cell']+1, configuration=config, calls=[]))
            task['calls'].append(c)
        plan['stages'][stage] = list(tasks.values())
        if inventory_counts(plan['stages'][stage]) != tuple(plan['counts'][stage]):
            raise ValueError('registered stage count mismatch')
        (root_for(root, plan, stage)/'tasks').mkdir(parents=True, exist_ok=True)
    # This immutable manifest contains metadata/paths, not private rows or outputs.
    put(root/'plan.json', plan)
    return plan


def matlab_scratch(call, directory):
    # A deliberate-error attempt intentionally reuses the scientific call ID.
    # Bind scratch ownership to its distinct output directory as well.
    attempt_key = hashlib.sha256(str(directory.resolve()).encode()).hexdigest()[:16]
    return Path(os.environ.get('TMPDIR','/tmp')) / (
        f"fevc-remaining-{os.environ.get('JOB_ID','local')}-{call['id']}-{attempt_key}")


def validate_carried_smoke(plan):
    previous=Path(plan['repair_prior'])
    old,modules=verify(previous,plan['repair_prior_sha256'])
    report=validate(previous,old,modules,'smoke')
    if report['status']!='PASS' or len(report['inventory'])!=16:
        raise ValueError('previous normal smoke calls are not compatible and passing')
    failure=read(previous/'smoke/deliberate-matlab/attempt.json')
    if failure['status']!='FAIL' or not failure['error'].startswith('FileExistsError:') or failure['process_return_code'] is not None:
        raise ValueError('prior failure is not the isolated prelaunch scratch collision')
    if read(previous/'smoke/deliberate-candidate/attempt.json')['status']!='EXPECTED_FAILURE':
        raise ValueError('prior Stata failure check did not pass')
    job=read(previous/'submissions/smoke.json')['job_id']
    text=(previous/'qacct'/f'{job}.txt').read_text()
    fields={parts[0]:parts[1].strip() for line in text.splitlines()
            if len(parts:=line.split(None,1))==2}
    if fields.get('jobnumber')!=job or fields.get('failed','').split()[0]!='0' or fields.get('exit_status')!='1' or fields.get('slots')!='4':
        raise ValueError('prior accounting does not match the documented harness-only failure')
    return dict(prior_root=str(previous),prior_plan_sha256=plan['repair_prior_sha256'],
        prior_scheduler_exit_status=1,prior_overall_status='FAIL',normal_calls_validated=16,
        normal_calls_status='PASS',prior_report_sha256=sha(previous/'smoke/report.json'),
        unchanged=['native/package bytes','inputs','commands','RNG','scientific gates','resource limits'],
        changed='Matlab temporary path includes the distinct attempt output directory',
        scope='carry normal-call numerical/thread/input-format evidence only; not prior scheduler PASS')


def verify_matlab(settings):
    if settings['matlab_commit'] != '8b957ffe':
        raise ValueError('unregistered KSS Matlab revision')
    for section, base in [('matlab_files', 'matlab_root'), ('mex_files', 'mex_dir')]:
        for name, digest in settings[section].items():
            if sha(Path(settings[base])/name) != digest:
                raise ValueError('pinned Matlab/MEX source drift')


def run_call(root, plan, modules, call, directory, slots, deadline, stata='stata-mp', deliberate=False):
    _, runner, _, _, paper = modules
    directory.mkdir(parents=True, exist_ok=False)
    smoke = call['stage'] == 'smoke'
    limit = 10 if smoke else 72 if call['variant'] == 'matlab' else 45 if call['rows'] > 400000 else 20
    receipt = dict(call=call, status='ATTEMPTED', started=time.time(), host=socket.gethostname(),
                   job_id=os.environ.get('JOB_ID'), task_id=os.environ.get('SGE_TASK_ID'), slots=slots,
                   rss_guard_gib=limit, deadline=deadline, pool_limit_seconds=240)
    put(directory/'attempt.json', receipt)
    seen, maximum, primary_max, samples = {}, 0, 0, 0
    process = None
    pool_start = None
    try:
        if slots < call['threads'] or deadline-time.time() < 5:
            raise ValueError('thread allocation or remaining task window invalid')
        inp = plan['inputs'][call['input']]
        source = Path(inp['path'])
        if sha(source) != call['input_sha256'] or sha(source.with_suffix('.json')) != inp['metadata_sha256']:
            raise ValueError('input identity changed')
        env = dict(os.environ, OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1', NUMEXPR_NUM_THREADS='1')
        if call['variant'] == 'matlab':
            settings = plan['matlab_runtime']
            verify_matlab(settings)
            parallel = matlab_scratch(call,directory)
            parallel.mkdir(parents=True, exist_ok=False)
            env.update(LR_INPUT=str(source), LR_META=str(source.with_suffix('.json')), LR_OUTPUT=str(directory),
                LR_THREADS=str(call['threads']), LR_SEED=str(call['seed']), LR_DELETION=call['deletion'],
                LR_MATLAB_ROOT=settings['matlab_root'], LR_MEX_DIR=settings['mex_dir'],
                LR_PARALLEL=str(parallel), OMP_NUM_THREADS='1')
            if deliberate: env['LR_INPUT'] = str(directory/'missing-deliberate.csv')
            args = ['matlab','-batch',f"addpath('{HERE}'); paper_matlab"]
        else:
            package = plan['packages'][call['variant']]
            if runner.verify_package(package['path']) != package['receipt']:
                raise ValueError('package drift before execution')
            (directory/'stata-tmp').mkdir()
            env.update(PF_PACKAGE=package['path'], PF_INPUT=str(source), PF_OUTPUT=str(directory/'result.tsv'),
                PF_COMMAND=request(call,runner), PF_ROWS=str(call['rows']), PF_NATIVE_THREADS=str(call['threads']),
                PF_ASSIGNED_SLOTS=str(slots), PF_THREAD_CONTRACT='FEVC-PIPELINE-THREADS-V1',
                PF_DELIBERATE_FAILURE=str(int(deliberate)), PF_PRIVATE_INPUT=str(int(call['graph'] in ('private_veneto','pooled_schema'))),
                PF_OUTPUT_DIRECTORY=str(directory), STATATMP=str(directory/'stata-tmp'),
                OMP_NUM_THREADS=str(call['threads']), RAYON_NUM_THREADS=str(call['threads']))
            args = [stata, '-b' if sys.platform=='darwin' else '-q', 'do', str(HERE/'remaining_stata.do')]
        with (directory/'application.log').open('x') as stream:
            process = subprocess.Popen(args, cwd=directory, env=env, stdout=stream, stderr=subprocess.STDOUT,
                                       start_new_session=True)
            while process.poll() is None:
                active, current = paper.owned_processes(process.pid, seen)
                memory = sum(current[p][1] for p in active if p in current)
                maximum, samples = max(maximum, memory), samples+1
                if (directory/'primary.started').exists() and not (directory/'primary.ended').exists():
                    primary_max = max(primary_max, memory)
                if (directory/'pool.started').exists() and pool_start is None: pool_start = time.time()
                if maximum > limit*1024**2: raise MemoryError('physical RSS guard exceeded')
                if time.time() > deadline: raise TimeoutError('total task window exhausted')
                if pool_start and not (directory/'pool.ready').exists() and time.time()-pool_start > 240:
                    raise TimeoutError('Matlab pool startup exceeded 240 seconds')
                time.sleep(.1)
        active, current = paper.owned_processes(process.pid, seen)
        if any(p in current and current[p][1]>0 and current[p][2]==seen.get(p) for p in active):
            paper.stop_owned(process.pid, seen)
            raise RuntimeError('surviving application workers')
        transcript = '\n'.join(p.read_text(errors='replace') for p in directory.glob('*.log'))
        if deliberate:
            if (directory/'result.tsv').exists() or (directory/'result.json').exists():
                raise ValueError('deliberate failure produced an accepted output')
            if call['variant'] == 'matlab':
                if process.returncode == 0 or 'missing-deliberate' not in transcript:
                    raise ValueError('Matlab deliberate failure not propagated')
            elif 'DELIBERATE_PIPELINE_FAILURE' not in transcript or 'FEVC_PIPELINE_SCREEN_CALL_PASS' in transcript:
                raise ValueError('Stata deliberate failure not propagated')
            receipt['status'] = 'EXPECTED_FAILURE'
        else:
            if process.returncode != 0 or maximum == 0 or samples == 0:
                raise RuntimeError('application process/measurement failed')
            if call['variant'] == 'matlab':
                result = paper.validate_result(directory,call)
                receipt.update(command_seconds=result['primary_seconds'], estimator_boundary_seconds=result['estimator_seconds'],
                               result=result, result_sha256=sha(directory/'result.json'), rng_sha256=sha(directory/'rng.json'))
            else:
                if 'FEVC_PIPELINE_SCREEN_CALL_PASS' not in transcript: raise ValueError('native application PASS missing')
                receipt.update(runner.validate_output(directory/'result.tsv',call))
                data = runner.read_results(directory/'result.tsv')
                if runner.scalar(data,'probes') != 200 or runner.scalar(data,'rng_master_seed') != call['seed']:
                    raise ValueError('probe/seed mismatch')
                receipt.update(result_sha256=sha(directory/'result.tsv'), package_adapter_sha256=call['adapter_sha256'])
            receipt['status'] = 'PASS'
    except Exception as error:
        if process is not None:
            paper.stop_owned(process.pid, seen)
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired: pass
        receipt.update(status='FAIL', error=f'{type(error).__name__}: {error}')
    receipt.update(finished=time.time(), whole_process_seconds=time.time()-receipt['started'],
                   process_return_code=None if process is None else process.poll(), physical_rss_kib=maximum,
                   primary_physical_rss_kib=primary_max, rss_samples=samples)
    put(directory/'attempt.json',receipt,replace=True)
    return receipt


def prerequisite(root, plan, stage, modules):
    previous = {'smoke': None, 'screen_400k': 'smoke', 'validation_1_6m': 'screen_400k',
                'private_veneto': 'validation_1_6m'}[stage]
    if previous is None: return
    report = read(root_for(root,plan,previous)/'report.json')
    if report['status'] != 'PASS' or report['plan_sha256'] != sha(root/'plan.json'):
        raise ValueError('previous-stage success gate failed')
    submitted = read(root/'submissions'/f'{previous}.json')
    job = submitted['aggregate_job_id']
    modules[3].collect_accounting(root,job,[0],1)


def run_task(root, plan, modules, stage, task_id, stata, deadline, check_prior=True):
    task = plan['stages'][stage][task_id-1]
    if task['id'] != task_id: raise ValueError('invalid task index')
    directory = root_for(root,plan,stage)/'tasks'/f'task-{task_id:02d}'
    directory.mkdir(exist_ok=False)
    inventory = {c['id']: 'UNATTEMPTED' for c in task['calls']}
    put(directory/'inventory.json',inventory)
    slots = int(os.environ.get('NSLOTS','4' if stage=='smoke' else '28'))
    try:
        if slots != (4 if stage=='smoke' else 28): raise ValueError('granted allocation differs')
        if check_prior: prerequisite(root,plan,stage,modules)
        for call in task['calls']:
            attempt = run_call(root,plan,modules,call,directory/call['id'],slots,deadline,stata)
            inventory[call['id']] = attempt['status']
            put(directory/'inventory.json',inventory,replace=True)
            if attempt['status'] != 'PASS': raise RuntimeError('call failed: '+call['id'])
        put(directory/'task.json',dict(status='PASS',host=socket.gethostname(),job_id=os.environ.get('JOB_ID'),
            task_id=task_id,plan_sha256=sha(root/'plan.json'),calls=inventory))
        return True
    except Exception as error:
        put(directory/'task.json',dict(status='FAIL',error=str(error),task_id=task_id,
            job_id=os.environ.get('JOB_ID'),host=socket.gethostname(),plan_sha256=sha(root/'plan.json'),calls=inventory))
        return False


def validate(root, plan, modules, stage):
    _, runner, validator, _, paper = modules
    base = root_for(root,plan,stage)
    inventory, errors, pairs, comparisons = [], [], [], []
    input_keys={c['input'] for t in plan['stages'][stage] for c in t['calls']}
    for key in input_keys:
        item=plan['inputs'][key]
        if sha(item['path']) != item['sha256'] or sha(Path(item['path']).with_suffix('.json')) != item['metadata_sha256']:
            errors.append('input changed: '+key)
    for task in plan['stages'][stage]:
        directory = base/'tasks'/f"task-{task['id']:02d}"
        hosts, finished, groups = set(), 0., defaultdict(dict)
        try:
            receipt = read(directory/'task.json')
            if receipt['status'] != 'PASS' or receipt['plan_sha256'] != sha(root/'plan.json'):
                raise ValueError('missing/failed/incompatible task receipt')
            if set(p.name for p in directory.iterdir() if p.is_dir()) != {c['id'] for c in task['calls']}:
                raise ValueError('missing or extra call directories')
        except (OSError,ValueError,KeyError) as error: errors.append(f"task {task['id']}: {error}")
        for call in task['calls']:
            entry = dict(call=call,status='UNATTEMPTED')
            dest = directory/call['id']
            try:
                attempt = read(dest/'attempt.json')
                entry.update(status=attempt['status'], receipt=attempt)
                if attempt['call'] != call or attempt['status'] != 'PASS': raise ValueError('failed or incompatible call')
                if attempt['started'] < finished or attempt['finished'] < attempt['started']: raise ValueError('call order changed')
                finished = attempt['finished']
                hosts.add(attempt['host'])
                limit = 10 if stage=='smoke' else 72 if call['variant']=='matlab' else 45 if call['rows']>400000 else 20
                if attempt['rss_guard_gib'] != limit or not 0 < attempt['physical_rss_kib'] <= limit*1024**2:
                    raise ValueError('RSS limit or measurement invalid')
                if attempt['rss_samples'] <= 0 or attempt['finished'] > attempt['deadline']+5:
                    raise ValueError('measurement/deadline violation')
                if attempt['slots'] != (4 if stage=='smoke' else 28): raise ValueError('slot mismatch')
                if call['variant'] == 'matlab':
                    result = paper.validate_result(dest,call)
                    if sha(dest/'result.json') != attempt['result_sha256'] or sha(dest/'rng.json') != attempt['rng_sha256']:
                        raise ValueError('Matlab output changed')
                    if result != attempt['result']: raise ValueError('Matlab receipt inconsistent')
                    data = None
                else:
                    runner.validate_output(dest/'result.tsv',call)
                    if sha(dest/'result.tsv') != attempt['result_sha256']: raise ValueError('native output changed')
                    if attempt['package_adapter_sha256'] != call['adapter_sha256']: raise ValueError('adapter receipt mismatch')
                    data = runner.read_results(dest/'result.tsv')
                # Reconcile stored clocks against independently re-read outputs.
                clock = result['primary_seconds'] if data is None else float(runner.value(data,'timer','command_seconds'))
                if clock != attempt['command_seconds']: raise ValueError('primary clock receipt changed')
                key = (call['warmup'],call['repetition'])
                if call['variant'] in groups[key]: raise ValueError('duplicate variant')
                groups[key][call['variant']] = (call,attempt,data)
                entry['validation_status'] = 'PASS'
            except (OSError, ValueError, KeyError, AssertionError) as error:
                entry.update(validation_status='FAIL',error=str(error))
                errors.append(f"{call['id']}: {error}")
            inventory.append(entry)
        if len(hosts) > 1: errors.append('different hosts within task')
        for (warmup, repetition), variants in groups.items():
            try:
                expected = {c['variant'] for c in task['calls'] if c['warmup']==warmup and c['repetition']==repetition}
                if set(variants) != expected: raise ValueError('incomplete paired call group')
                left,right = variants['baseline'],variants['candidate']
                gaps = validator.compare(left[2],right[2])
                if not warmup:
                    pairs.append(dict(task=task['id'],repetition=repetition,profile=left[0]['profile'],
                        graph=left[0]['graph'],threads=left[0]['threads'],deletion=left[0]['deletion'],gaps=gaps,
                        ratio=right[1]['command_seconds']/left[1]['command_seconds']))
                    if 'matlab' in variants:
                        matlab = variants['matlab'][1]
                        comparisons.append(dict(task=task['id'],repetition=repetition,
                            candidate_over_matlab_command=right[1]['command_seconds']/matlab['command_seconds'],
                            candidate_over_matlab_estimator=right[1]['estimator_boundary_seconds']/matlab['estimator_boundary_seconds'],
                            fevc_targets=[float(runner.value(right[2],'matrix','results',3,c)) for c in range(1,5)],
                            matlab_targets=matlab['result']['targets'],finite_draw_equality_claim=False))
            except (ValueError,KeyError) as error: errors.append(f"pair {task['id']}/{repetition}: {error}")
    if {p.name for p in (base/'tasks').iterdir()} != {f"task-{t['id']:02d}" for t in plan['stages'][stage]}:
        errors.append('task directory inventory differs')
    expected_pairs = sum(not c['warmup'] and c['variant']=='candidate' for t in plan['stages'][stage] for c in t['calls'])
    if len(pairs) != expected_pairs: errors.append('missing measured pairs')
    cells, by_deletion = defaultdict(list), defaultdict(list)
    for p in pairs:
        cells[f"{p['graph']}/{p['deletion']}/T{p['threads']}"] .append(p['ratio'])
        by_deletion[p['deletion']].append(p['ratio'])
    medians = {k:statistics.median(v) for k,v in cells.items()}
    geomean = validator.geometric([p['ratio'] for p in pairs]) if pairs else None
    passing = geomean is not None and geomean <= .97 and all(v <= 1.05 for v in medians.values())
    status = 'FAIL' if errors else 'PASS' if stage=='smoke' or passing else 'PERFORMANCE_FAIL'
    return dict(schema='FEVC-REMAINING-STAGE-REPORT-V1',status=status,stage=stage,plan_sha256=sha(root/'plan.json'),
        errors=errors,inventory=inventory,pairs=pairs,matlab_comparisons=comparisons,
        performance=dict(pass_gate=passing if stage!='smoke' else None,geometric_candidate_over_baseline=geomean,
            by_deletion={k:validator.geometric(v) for k,v in by_deletion.items()},cell_medians=medians),
        publication_claim=None,private_details=stage=='private_veneto')


def smoke(root, plan, modules, stata):
    if 'repair_prior' in plan:
        put(root/'receipts/carried-smoke.json',validate_carried_smoke(plan))
    deadline = float(os.environ.get('PF_TASK_STARTED_UNIX',time.time())) + 1470
    for task in plan['stages']['smoke']:
        if not run_task(root,plan,modules,'smoke',task['id'],stata,deadline):
            raise RuntimeError('small-input smoke failed')
    for variant in ('candidate','matlab'):
        call = next(c for c in plan['stages']['smoke'][0]['calls'] if c['variant']==variant)
        attempt = run_call(root,plan,modules,call,root/'smoke'/f'deliberate-{variant}',4,deadline,stata,True)
        if attempt['status'] != 'EXPECTED_FAILURE': raise ValueError('deliberate failure propagation failed')
    report = validate(root,plan,modules,'smoke')
    if report['status'] != 'PASS': raise ValueError('smoke scientific/output validation failed')
    put(root/'receipts/smoke-application.json',dict(status='PASS',plan_sha256=sha(root/'plan.json'),
        deliberate_failures=['candidate','matlab'],job_id=os.environ.get('JOB_ID')))
    print('FEVC_REMAINING_SMOKE_PASS',flush=True)


def aggregate(root, plan, modules, stage):
    base = root_for(root,plan,stage)
    submission = read(root/'submissions'/f'{stage}.json')
    report = validate(root,plan,modules,stage)
    records = []
    try:
        tasks = [0] if stage=='smoke' else list(range(1,len(plan['stages'][stage])+1))
        records = modules[3].collect_accounting(root,submission['job_id'],tasks,4 if stage=='smoke' else 28)
        if stage=='smoke':
            if read(root/'receipts/smoke-application.json')['status'] != 'PASS': raise ValueError('smoke failure checks missing')
            if 'repair_prior' in plan:
                report['compatible_prior_smoke']=validate_carried_smoke(plan)
        else:
            for row in records:
                task = read(base/'tasks'/f"task-{int(row['taskid']):02d}"/'task.json')
                if task['job_id'] != submission['job_id'] or task['host'].split('.')[0] != row['hostname'].split('.')[0]:
                    raise ValueError('scheduler/application identity differs')
    except (OSError,ValueError,KeyError) as error:
        report['errors'].append(str(error))
        report['status'] = 'FAIL'
    report['scheduler'] = records
    put(base/'report.json',report)
    print('FEVC_REMAINING_AGGREGATE_'+report['status'],flush=True)
    return report['status']=='PASS'


def submit(root, plan, modules, only_smoke=False):
    # Existing stage-1 source/qualification remains exact-byte compatible;
    # this extension changes only inputs, task inventory, clocks and orchestration.
    prior = Path(plan['prior_root'])
    modules[3].collect_accounting(root,'7587665',[0],1)
    verify_matlab(plan['matlab_runtime'])
    for item in plan['inputs'].values():
        if sha(item['path']) != item['sha256'] or sha(Path(item['path']).with_suffix('.json')) != item['metadata_sha256']:
            raise ValueError('pre-submission input mismatch')
    prior_job = None
    stages = ['smoke'] if only_smoke else list(STAGES)
    if not only_smoke:
        prerequisite(root,plan,'screen_400k',modules)
        prior_job = read(root/'submissions/smoke.json')['aggregate_job_id']
    for stage in stages:
        destination = root/'submissions'/f'{stage}.json'
        if destination.exists(): raise FileExistsError('stage already submitted')
        base = root_for(root,plan,stage)
        log = plan['private_output']+'/logs' if stage=='private_veneto' else str(root/'logs')
        slots,minutes = (4,25) if stage=='smoke' else (28,45)
        env = f'PF_REMAIN_ROOT={root},PF_MODE={"smoke" if stage=="smoke" else "task"},PF_REMAIN_STAGE={stage},PF_PLAN_SHA256={sha(root/"plan.json")}'
        args = ['qsub','-terse','-P','welfgr','-pe','omp',str(slots),'-l','mem_per_core=3G',
            '-l',f'h_rt=00:{minutes}:00','-j','y','-m','n','-N','fevc_rem_'+stage,'-o',log,'-v',env]
        if stage!='smoke': args += ['-t',f'1-{len(plan["stages"][stage])}']
        if prior_job: args += ['-hold_jid',prior_job]
        args += [str(HERE/'remaining_job.sge')]
        raw = subprocess.check_output(args,text=True).strip()
        if not re.fullmatch(r'\d+(?:\.[\d:-]+)?',raw): raise ValueError('unexpected qsub receipt')
        job = raw.split('.')[0]
        record = dict(status='ARRAY_SUBMITTED',job_id=job,stage=stage,slots=slots,minutes=minutes,
                      command=args,plan_sha256=sha(root/'plan.json'),submitted=time.time())
        put(destination,record)
        aggregate_env = f'PF_REMAIN_ROOT={root},PF_MODE=aggregate,PF_REMAIN_STAGE={stage},PF_PLAN_SHA256={sha(root/"plan.json")}'
        agg = ['qsub','-terse','-P','welfgr','-pe','omp','1','-l','mem_per_core=3G','-l','h_rt=00:15:00',
               '-j','y','-m','n','-N','fevc_rem_agg','-o',log,'-hold_jid',job,'-v',aggregate_env,str(HERE/'remaining_job.sge')]
        aggregate_job = subprocess.check_output(agg,text=True).strip()
        if not aggregate_job.isdigit(): raise ValueError('unexpected aggregate qsub receipt')
        record.update(status='SUBMITTED',aggregate_job_id=aggregate_job,aggregate_command=agg)
        put(destination,record,replace=True)
        prior_job = aggregate_job
        print(json.dumps(dict(stage=stage,job_id=job,aggregate_job_id=aggregate_job,counts=plan['counts'][stage])),flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path)
    parser.add_argument('mode',choices=['freeze','verify','smoke','task','aggregate','submit-smoke','submit'])
    parser.add_argument('--stage',choices=['smoke',*STAGES],default='smoke')
    parser.add_argument('--prior',type=Path)
    parser.add_argument('--paper',type=Path)
    parser.add_argument('--private-source',type=Path)
    parser.add_argument('--private-output',type=Path)
    parser.add_argument('--repair-prior',type=Path)
    parser.add_argument('--stata',default='stata-mp')
    args = parser.parse_args()
    os.umask(0o077)
    if args.mode=='freeze':
        freeze(args.root,args.prior,args.paper,args.private_source,args.private_output,repair_prior=args.repair_prior)
        print(sha(args.root/'plan.json'))
        return
    plan,modules = verify(args.root,os.environ.get('PF_PLAN_SHA256'))
    if args.mode=='verify': print('PASS'); return
    if args.mode in ('submit','submit-smoke'):
        if not str(args.root).startswith('/projectnb/welfgr/vckss/runs/'):
            raise ValueError('submission outside established SCC run path')
        submit(args.root,plan,modules,args.mode=='submit-smoke')
    elif args.mode=='smoke': smoke(args.root,plan,modules,args.stata)
    elif args.mode=='aggregate':
        if not aggregate(args.root,plan,modules,args.stage): raise SystemExit(1)
    else:
        deadline=float(os.environ['PF_TASK_STARTED_UNIX'])+2670
        if not run_task(args.root,plan,modules,args.stage,int(os.environ['SGE_TASK_ID']),args.stata,deadline):
            raise SystemExit(1)


if __name__=='__main__':
    main()

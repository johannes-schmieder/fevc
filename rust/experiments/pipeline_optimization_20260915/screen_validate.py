#!/usr/bin/env python3
"""Complete inventory and paired public-command scientific/performance gates."""
from __future__ import annotations
import argparse
from collections import defaultdict
import json
import math
from pathlib import Path
import statistics
from campaign import PROFILES
from screen_runner import read_results,scalar,sha,validate_output,value,write_json,verify_package

MATRICES=('results','b','V','V_primitive','component_inference','projection_results','projection_V')
IDENTITIES=('N_stored','N_physical','N_requested','N_retained','N_stayers','N_stayer_rows',
    'worker_levels','firm_levels','deletion_units','full_parameters','correction_parameters',
    'probes','rng_master_seed','target_weight_sum')
AVAILABILITY=('status','inference_support_status','inference_reference_requested',
    'inference_reference_selected','inference_joint_available','inference_joint_status')


def numeric_gap(a,b):
    missing=lambda v: v=='.' or (len(v)==2 and v[0]=='.' and v[1].islower())
    if missing(a):
        if a!=b: raise ValueError('availability differs')
        return 0.
    if missing(b):
        raise ValueError('availability differs')
    x,y=float(a),float(b)
    if not math.isfinite(x) or not math.isfinite(y): raise ValueError('nonfinite paired output')
    return abs(x-y)/max(1.,abs(x),abs(y))


def compare(left,right):
    gaps={}
    for name in MATRICES:
        lkeys={k for k in left if k[0]=='matrix' and k[1]==name}
        rkeys={k for k in right if k[0]=='matrix' and k[1]==name}
        if lkeys!=rkeys: raise ValueError(f'output shape/availability differs: {name}')
        if lkeys:
            for kind in ('rownames','colnames'):
                if left.get((kind,name,0,0))!=right.get((kind,name,0,0)):
                    raise ValueError(f'output order differs: {name}')
            gaps[name]=max(numeric_gap(left[k],right[k]) for k in lkeys)
            for key in lkeys:
                # Plugin/correction/MCSE rows are reported diagnostics. The
                # registered point gate is on the four corrected targets.
                if name=='results' and key[2]!=3: continue
                allowance=1e-8
                if name in ('results','b'):
                    mcse=('matrix','numerical_mcse',1,key[3])
                    if mcse in left and mcse in right and left[mcse]!='.' and right[mcse]!='.':
                        scale=max(1.,abs(float(left[key])),abs(float(right[key])))
                        allowance=max(allowance,.1*max(float(left[mcse]),float(right[mcse]))/scale)
                if numeric_gap(left[key],right[key])>allowance:
                    raise ValueError(f'common-draw statistical gate: {name} gap={gaps[name]}')
    for name in IDENTITIES:
        key=('scalar',name,0,0)
        if (key in left)!=(key in right): raise ValueError(f'identity unavailable: {name}')
        if key in left and numeric_gap(left[key],right[key])>1e-12:
            raise ValueError(f'sample/RNG identity differs: {name}')
    for name in AVAILABILITY:
        for kind in ('scalar','macro'):
            key=(kind,name,0,0)
            if left.get(key)!=right.get(key): raise ValueError(f'availability differs: {name}')
    # Per-target q0/q1 availability is retained as numeric status matrices.
    for name in ('q0_status','q1_status','inference_status'):
        l={k:v for k,v in left.items() if k[1]==name}
        r={k:v for k,v in right.items() if k[1]==name}
        if l!=r: raise ValueError(f'target-specific availability differs: {name}')
    return gaps


def geometric(values):
    return math.exp(statistics.fmean(math.log(v) for v in values))


def performance(pairs):
    cells=defaultdict(list)
    groups=defaultdict(list)
    for pair in pairs:
        p=PROFILES[pair['profile']]
        cells[(pair['profile'],pair['threads'])].append(pair['ratio'])
        groups['all'].append(pair['ratio'])
        groups['deletion:'+p['deletion']].append(pair['ratio'])
        groups['family:'+p['family']].append(pair['ratio'])
    medians={f'{p}/T{t}':statistics.median(v) for (p,t),v in cells.items()}
    geomeans={name:geometric(v) for name,v in groups.items()}
    passed=geomeans['all']<=.97 and all(v<=1.05 for v in medians.values())
    return dict(pass_gate=passed,geometric_candidate_over_baseline=geomeans,
        profile_thread_median_candidate_over_baseline=medians,
        minimum_complete_command_improvement=.03,maximum_median_regression=.05)


def validate(manifest,root):
    inventory=[]
    errors=[]
    paired=defaultdict(dict)
    expected_dirs=set()
    for variant in ('baseline','candidate'):
        if verify_package(root/'packages'/variant)!=manifest['packages'][variant]:
            errors.append(f'package manifest drift: {variant}')
    for task in manifest['tasks']:
        task_dir=root/'tasks'/f"task-{task['id']:02d}"
        expected_dirs.add(task_dir.name)
        task_receipt=task_dir/'task.json'
        if not task_receipt.exists(): errors.append(f'missing task receipt: {task["id"]}')
        else:
            record=json.loads(task_receipt.read_text())
            if record['status']!='PASS' or record['manifest_sha256']!=sha(root/'manifest.json'):
                errors.append(f'invalid task receipt: {task["id"]}')
        expected_calls={call['id'] for call in task['calls']}
        actual={p.name for p in task_dir.iterdir() if p.is_dir()} if task_dir.exists() else set()
        if actual-expected_calls: errors.append(f'unexpected calls: {actual-expected_calls}')
        hosts=set()
        prior_finish=0
        for call in task['calls']:
            entry=dict(task=task['id'],call=call,status='UNATTEMPTED')
            directory=task_dir/call['id']
            attempt=directory/'attempt.json'
            try:
                if not attempt.exists(): raise ValueError('missing attempt')
                receipt=json.loads(attempt.read_text())
                entry['status']=receipt['status']
                entry['receipt']=receipt
                if receipt['call']!=call: raise ValueError('stale/incompatible attempted call')
                if receipt['status']!='PASS': raise ValueError('call failed')
                if receipt['started']<prior_finish or receipt['finished']<receipt['started']:
                    raise ValueError('execution order or sequential-pair contract violated')
                prior_finish=receipt['finished']
                hosts.add(receipt['host'])
                if receipt['slots']!=manifest['slots'] or receipt['physical_rss_kib']>manifest['rss_gib']*1024**2:
                    raise ValueError('resource contract mismatch')
                package=root/'packages'/call['variant']
                if receipt['package_adapter_sha256']!=sha(package/'adapter.json') or receipt['package_adapter_sha256']!=call['adapter_sha256']:
                    raise ValueError('package mismatch')
                if sha(root/'inputs'/f"{call['profile']}.csv")!=call['input_sha256'] or sha(root/'inputs'/f"{call['profile']}.json")!=call['input_receipt_sha256']:
                    raise ValueError('input or fixture receipt drift')
                if receipt['result_sha256']!=sha(directory/'result.tsv'): raise ValueError('output hash mismatch')
                validate_output(directory/'result.tsv',call)
                entry['status']='PASS'
                entry['validation_status']='PASS'
                if not call['warmup']:
                    key=(task['id'],call['repetition'])
                    if call['variant'] in paired[key]: raise ValueError('duplicate paired key')
                    paired[key][call['variant']]=(call,receipt,read_results(directory/'result.tsv'))
            except (ValueError,KeyError,OSError) as error:
                entry['error']=str(error)
                entry['validation_status']='FAIL'
                errors.append(f'task {task["id"]} {call["id"]}: {error}')
            inventory.append(entry)
        if len(hosts)>1: errors.append(f'variants used different hosts: {task["id"]}')
    if (root/'tasks').exists() and {p.name for p in (root/'tasks').iterdir()}!=expected_dirs:
        errors.append('task directory inventory differs')
    pairs=[]
    for key,pair in paired.items():
        try:
            if set(pair)!={'baseline','candidate'}: raise ValueError('incomplete pair')
            baseline,candidate=pair['baseline'],pair['candidate']
            gaps=compare(baseline[2],candidate[2])
            pairs.append(dict(task=key[0],repetition=key[1],profile=baseline[0]['profile'],
                threads=baseline[0]['threads'],gaps=gaps,
                ratio=candidate[1]['command_seconds']/baseline[1]['command_seconds']))
        except ValueError as error: errors.append(f'pair {key}: {error}')
    measured=sum(not x['call']['warmup'] for x in inventory)
    warmups=len(inventory)-measured
    expected=(manifest['expected_measured'],manifest['expected_warmups'],manifest['expected_pairs'])
    if (measured,warmups,len(pairs))!=expected: errors.append('incomplete stage inventory')
    production=manifest['status']=='FROZEN_SUPPORTED_PATH_SCREEN'
    if production and expected!=(216,72,108): errors.append('incorrect approved first-stage inventory')
    report=dict(schema='FEVC-PIPELINE-SCREEN-DECISION-V1',status='FAIL' if errors else 'SCIENTIFIC_PASS',
        errors=errors,expected_measured=expected[0],expected_warmups=expected[1],inventory=inventory,pairs=pairs,
        later_stages_authorized=False,matlab_claim=None)
    if not errors and production:
        report['performance']=performance(pairs)
        report['status']='PASS' if report['performance']['pass_gate'] else 'PERFORMANCE_FAIL'
    elif not errors:
        report['status']='SMOKE_PASS'
        report['performance_claim']=None
    return report


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('root',type=Path)
    p.add_argument('output',type=Path)
    a=p.parse_args()
    if a.output.exists(): raise SystemExit('report is write-once')
    report=validate(json.loads((a.root/'manifest.json').read_text()),a.root)
    write_json(a.output,report)
    print(report['status'])
    raise SystemExit(0 if report['status']=='PASS' else 1)

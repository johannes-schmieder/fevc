#!/usr/bin/env python3
"""Freeze only the approved first screen, or a separately labelled tiny smoke."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import shutil
from campaign import PROFILES, SEEDS, BASELINE_NATIVE_SHA256
from screen_runner import command, sha, verify_package, write_json

HERE=Path(__file__).parent
HARNESS=('campaign.py','screen_manifest.py','screen_runner.py','screen_validate.py',
    'screen_stata.do','thread_adapter.py','screen_input.py','screen_scc.py','screen_job.sge','public_exact_smoke.do',
    'public_exact_failure_smoke.do','compressed_boundary_smoke.do',
    '../optimization_parity_20260913/monitor_rss.py','../optimization_parity_20260913/generate.py')


def freeze(root,inputs,packages,smoke_threads=None,allocated_slots=None):
    if root.exists(): raise ValueError('stage directory already exists')
    receipts={v:verify_package(packages/v) for v in ('baseline','candidate')}
    if receipts['baseline']['source_snapshot_sha256']!='1d9968830ee27d5e801b7a4f27181a2fa240e1a9a84578fa2c8e53e464fb0a7e':
        raise ValueError('wrong performance baseline')
    threads=(smoke_threads,) if smoke_threads else (1,4,7)
    if smoke_threads is not None and smoke_threads not in (4,7): raise ValueError('invalid smoke allocation')
    requested_slots=smoke_threads or 14
    granted_slots=requested_slots if allocated_slots is None else allocated_slots
    if granted_slots not in ((7,8) if requested_slots==7 else (requested_slots,)):
        raise ValueError('unregistered scheduler slot allocation')
    manifest=dict(schema='FEVC-PIPELINE-SCREEN-MANIFEST-V1',
        status='FROZEN_DEVELOPMENT_SMOKE' if smoke_threads else 'FROZEN_SUPPORTED_PATH_SCREEN',
        slots=granted_slots,requested_slots=requested_slots,seconds=1500 if smoke_threads else 2700,
        rss_gib=10 if smoke_threads else 20,baseline_native_sha256=BASELINE_NATIVE_SHA256,
        later_stages_authorized=False,profiles=PROFILES,packages=receipts,
        harness_hashes={name:sha(HERE/name) for name in HARNESS},
        acceptance=dict(point_policy='VCKSS_DEVELOPMENT_ACCEPTANCE_V1',
            common_draw_limit='max(1e-8*scale,0.1*max(mcse_baseline,mcse_candidate))',
            inference_full_precision_relative_limit=1e-8,complete_command_geomean_ratio_max=.97,
            profile_thread_median_ratio_max=1.05),tasks=[])
    for profile,p in PROFILES.items():
        fixture=json.loads((inputs/f'{profile}.json').read_text())
        if fixture['profile']!=profile or fixture['smoke']!=bool(smoke_threads) or fixture['sha256']!=sha(inputs/f'{profile}.csv'):
            raise ValueError('input identity or stage mismatch')
        expected_rows=min(p['rows'],8000) if smoke_threads else p['rows']
        if fixture['rows']!=expected_rows: raise ValueError('input dimension mismatch')
        for t in threads:
            task=dict(id=len(manifest['tasks'])+1,profile=profile,threads=t,input=fixture,calls=[])
            reps=(1,) if smoke_threads else (0,1,2,3)
            for rep in reps:
                variants=['baseline','candidate']
                if rep and (task['id']-1+rep-1)%2: variants.reverse()
                for position,v in enumerate(variants):
                    task['calls'].append(dict(id=f'{len(task["calls"])+1:02d}-{v}-'+('warmup' if rep==0 else f'r{rep}'),
                        profile=profile,threads=t,variant=v,warmup=rep==0,repetition=rep,position=position,
                        seed=SEEDS[max(0,rep-1)],rows=expected_rows,input_sha256=fixture['sha256'],
                        input_receipt_sha256=sha(inputs/f'{profile}.json'),adapter_sha256=sha(packages/v/'adapter.json'),
                        command=command(profile,SEEDS[max(0,rep-1)])))
            manifest['tasks'].append(task)
    manifest['expected_measured']=sum(not c['warmup'] for task in manifest['tasks'] for c in task['calls'])
    manifest['expected_warmups']=sum(c['warmup'] for task in manifest['tasks'] for c in task['calls'])
    manifest['expected_pairs']=manifest['expected_measured']//2
    if not smoke_threads and (len(manifest['tasks']),manifest['expected_measured'],manifest['expected_warmups'])!=(36,216,72):
        raise ValueError('incorrect approved stage inventory')
    root.mkdir(parents=True)
    shutil.copytree(inputs,root/'inputs')
    shutil.copytree(packages,root/'packages')
    write_json(root/'manifest.json',manifest)
    return manifest


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('root',type=Path)
    p.add_argument('--inputs',type=Path,required=True)
    p.add_argument('--packages',type=Path,required=True)
    p.add_argument('--smoke-threads',type=int,choices=[4,7])
    a=p.parse_args()
    m=freeze(a.root,a.inputs,a.packages,a.smoke_threads)
    print(json.dumps(dict(manifest_sha256=sha(a.root/'manifest.json'),tasks=len(m['tasks']),
        measured=m['expected_measured'],warmups=m['expected_warmups'])))

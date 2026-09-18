#!/usr/bin/env python3
"""Exercise the exact screen generator/manifest/runner/validator locally."""
import argparse
import json
from pathlib import Path
import time
from screen_manifest import freeze
from screen_runner import run_task,sha,write_json
from screen_validate import validate


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('output',type=Path)
    p.add_argument('inputs',type=Path)
    p.add_argument('packages',type=Path)
    p.add_argument('--stata',default='/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp')
    a=p.parse_args()
    m=freeze(a.output,a.inputs,a.packages,7)
    started=time.time()
    for task in m['tasks']:
        passed=run_task(a.output/'manifest.json',task['id'],a.stata,7,started,sha(a.output/'manifest.json'))
        print(json.dumps(dict(task=task['id'],profile=task['profile'],passed=passed)),flush=True)
        if not passed: break
    report=validate(m,a.output)
    write_json(a.output/'report.json',report)
    if report['status']!='SMOKE_PASS': raise SystemExit(1)
    failure=a.output.with_name(a.output.name+'-deliberate')
    freeze(failure,a.inputs,a.packages,7)
    if run_task(failure/'manifest.json',1,a.stata,7,started,sha(failure/'manifest.json'),True):
        raise SystemExit('deliberate failure unexpectedly passed')
    states=json.loads((failure/'tasks/task-01/inventory.json').read_text())
    if list(states.values())!=['FAIL','UNATTEMPTED']: raise SystemExit('failure did not preserve inventory')
    print('LOCAL_PIPELINE_GENERATOR_VALIDATOR_FAILURE_PASS')

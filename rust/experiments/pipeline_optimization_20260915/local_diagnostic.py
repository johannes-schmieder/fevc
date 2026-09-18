#!/usr/bin/env python3
"""Single complete-command profiles; deliberately not performance acceptance."""
import argparse
import json
import os
from pathlib import Path
import time
from campaign import PROFILES
from screen_runner import command,run_call,sha,write_json


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('package',type=Path)
    p.add_argument('inputs',type=Path)
    p.add_argument('output',type=Path)
    p.add_argument('--stata',type=Path,default=Path('/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp'))
    a=p.parse_args()
    a.output.mkdir()
    os.environ['PF_DIAGNOSTIC']='1'
    receipts=[]
    for name in PROFILES:
        fixture=json.loads((a.inputs/f'{name}.json').read_text())
        call=dict(id=name,profile=name,variant='candidate',rows=fixture['rows'],threads=7,
            seed=104729,warmup=False,repetition=1,input_sha256=fixture['sha256'],
            adapter_sha256=sha(a.package/'adapter.json'),command=command(name,104729))
        receipt=run_call(call,a.package,a.inputs/f'{name}.csv',a.output/name,a.stata,time.time()+300,10,7)
        receipts.append(receipt)
        print(json.dumps(dict(profile=name,status=receipt['status'],error=receipt.get('error'),seconds=receipt.get('command_seconds'))),flush=True)
        if receipt['status']!='PASS': break
    write_json(a.output/'summary.json',dict(diagnostic_only=True,performance_claim=None,receipts=receipts))
    if len(receipts)!=12 or any(r['status']!='PASS' for r in receipts): raise SystemExit(1)

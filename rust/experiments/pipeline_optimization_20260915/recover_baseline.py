#!/usr/bin/env python3
"""Recover omitted baseline build inputs from manifest-matching archived bytes."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

BASELINE='1d9968830ee27d5e801b7a4f27181a2fa240e1a9a84578fa2c8e53e464fb0a7e'
NATIVE='6d8e86aafbe1224e29217ce4565a7c91c22fcc60583bd97b2805b910298ae18d'
ARCHIVE='82aecd7ec9b92627cdaab57f4d7e8e873be3cffa82b2f892ac9c19283d3b89ba'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def recover(baseline,archive,output):
    if output.exists(): raise ValueError('recovery output already exists')
    if sha(baseline/'snapshot.json')!=BASELINE or sha(baseline/'native-source-manifest.sha256')!=NATIVE:
        raise ValueError('baseline identity mismatch')
    if sha(archive/'snapshot.json')!=ARCHIVE: raise ValueError('archive identity mismatch')
    files=json.loads((baseline/'snapshot.json').read_text())['source']
    archived=json.loads((archive/'snapshot.json').read_text())['source']
    origins={name:baseline/'source'/name for name in files}
    recovered={}
    for line in (baseline/'native-source-manifest.sha256').read_text().splitlines():
        digest,name=line.split(None,1)
        if name not in files:
            if archived.get(name)!=digest: raise ValueError(f'no matching archived input: {name}')
            origins[name]=archive/'source'/name
            files[name]=digest
            recovered[name]=dict(sha256=digest,archive_snapshot_sha256=ARCHIVE)
        elif files[name]!=digest: raise ValueError(f'baseline native mismatch: {name}')
    for name,path in origins.items():
        if path.is_symlink() or sha(path)!=files[name]: raise ValueError(f'source drift: {name}')
    output.mkdir(parents=True)
    for name,path in origins.items():
        target=output/'source'/name
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(path,target)
    for name in ('snapshot.json','native-source-manifest.sha256'):
        shutil.copy2(baseline/name,output/name)
    receipt=dict(schema='FEVC-BASELINE-ARCHIVED-RECOVERY-V1',baseline_snapshot_sha256=BASELINE,
        native_manifest_sha256=NATIVE,archived_inputs=recovered,status='PASS',
        note='Original snapshot and all native inputs unchanged; no current-source substitutions.')
    (output/'recovery.json').write_text(json.dumps(receipt,indent=2,sort_keys=True)+'\n')
    return receipt

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('baseline','archive','output'): p.add_argument(name,type=Path)
    a=p.parse_args()
    print(json.dumps(recover(a.baseline,a.archive,a.output)))

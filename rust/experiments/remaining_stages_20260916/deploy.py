"""Guarded harness-only deployment. Does not deploy or rebuild FEVC source."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import remaining as r


def main(run_id, local_evidence):
    if not re.fullmatch(r'20260916T[0-9]{6}Z-pipeline-remaining',run_id):
        raise ValueError('unregistered remaining-campaign namespace')
    report=r.read(local_evidence/'report.json')
    if report['status']!='PASS' or len(report['inventory'])!=8:
        raise ValueError('local native workflow did not pass')
    if r.read(local_evidence/'deliberate/attempt.json')['status']!='EXPECTED_FAILURE':
        raise ValueError('local failure propagation did not pass')
    root='/projectnb/welfgr/vckss/runs/'+run_id
    subprocess.run(['ssh','scc',f'test ! -e {root} && umask 077 && mkdir {root} && mkdir {root}/harness {root}/smoke-inputs'],check=True)
    files=sorted(p for p in Path(__file__).parent.iterdir() if p.suffix in ('.py','.do','.sge','.m','.md'))
    subprocess.run(['rsync','-r',*[str(p) for p in files],f'scc:{root}/harness/'],check=True)
    subprocess.run(['rsync','-r',str(local_evidence/'inputs/private-schema.csv'),str(local_evidence/'inputs/private-schema.json'),f'scc:{root}/smoke-inputs/'],check=True)
    subprocess.run(['rsync','-r',str(local_evidence/'report.json'),f'scc:{root}/local-workflow-report.json'],check=True)
    subprocess.run(['rsync','-r',str(local_evidence/'deliberate/attempt.json'),f'scc:{root}/local-failure.json'],check=True)
    expected={p.name:r.sha(p) for p in files}
    # Only metadata and the public synthetic fixture cross this boundary.
    checks=' && '.join(f'test "$(sha256sum {root}/harness/{name} | cut -d " " -f 1)" = {digest}' for name,digest in expected.items())
    subprocess.run(['ssh','scc',checks],check=True)
    print(json.dumps(dict(remote_root=root,harness=expected,local_workflow_sha256=r.sha(local_evidence/'report.json'))))


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('run_id')
    p.add_argument('local_evidence',type=Path)
    a=p.parse_args()
    main(a.run_id,a.local_evidence)

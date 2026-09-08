"""Independent accounting rejects missing, duplicate and misreported results."""
import gzip
import importlib.util
import json
from pathlib import Path
import statistics

import pytest

ROOT=Path(__file__).resolve().parents[3]
SPEC=importlib.util.spec_from_file_location('individual_auditor',ROOT/'fevc/tools/audit_individual_development.py')
AUDIT=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(AUDIT)

def write(path,value):
    path.write_text(json.dumps(value))

@pytest.fixture
def evidence(tmp_path):
    tasks=[];cells=[];summaries=[]
    (tmp_path/'source.tar.gz').write_bytes(b'synthetic immutable source identity')
    calls=[{'replication':i,'status':'success','point':[float(i%2)*2-1]*4,'point_mcse':[.01]*4,
            'targets':[1,0]*4,'joint':0,'computed':4}for i in range(400)]
    rows=[{}, {'truth':[0]*4}, *calls]
    raw='\n'.join(json.dumps(row)for row in rows).encode()
    compressed=gzip.compress(raw)
    for i in range(48):
        task={'id':f'cell-{i}','family':'synthetic','cell':f'cell-{i}','k':20,'start':0,'reps':400}
        tasks.append(task);cells.append({**task,'reference_distribution':'q0'})
        folder=tmp_path/'tasks'/task['id'];folder.mkdir(parents=True)
        (folder/'rows.jsonl.gz').write_bytes(compressed);(folder/'stderr.log').write_text('')
        write(folder/'receipt.json',{'task':task,'output_sha256':AUDIT.sha(folder/'rows.jsonl.gz'),'binary_sha256':'synthetic-binary','calls':400,'targets':1600})
        for target in('worker','firm','covariance','total'):
            summaries.append({'family':'synthetic','cell':task['cell'],'k':20,'target':target,'attempts':400,'point_estimates':400,'successes':400,'success_rate':1.,
                              'bias':0.,'empirical_sd':statistics.stdev(c['point'][0]for c in calls),'coverage':1.,'coverage_among_all_attempts':1.,'mean_se':1.,'se_denominator':400})
    write(tmp_path/'manifest.json',{'tasks':tasks,'cells':cells,'binaries':{'synthetic':'synthetic-binary'},'bundle_sha256':AUDIT.sha(tmp_path/'source.tar.gz')})
    write(tmp_path/'result.json',{'status':'FAIL','manifest_sha256':AUDIT.sha(tmp_path/'manifest.json'),'summaries':summaries,'reports':{'synthetic':{'status':'FAIL','failures':['intentional scientific failure']}}})
    return tmp_path

def test_complete_accounting_does_not_promote_failed_science(evidence):
    result=AUDIT.audit(evidence)
    assert result['accounting_status']=='PASS'and result['scientific_status']=='FAIL'
    assert result['counts']['native_calls']==19200 and result['counts']['target_attempts']==76800

@pytest.mark.parametrize('fault',['missing','duplicate','hash','summary','partial'])
def test_corruption_is_rejected(evidence,fault):
    folder=evidence/'tasks/cell-0';path=folder/'rows.jsonl.gz'
    if fault=='missing':path.unlink()
    elif fault in('duplicate','partial'):
        rows=[json.loads(line)for line in gzip.decompress(path.read_bytes()).decode().splitlines()]
        if fault=='duplicate':rows[-1]=rows[-2]
        else:rows.pop()
        path.write_bytes(gzip.compress('\n'.join(json.dumps(r)for r in rows).encode()))
        receipt=json.loads((folder/'receipt.json').read_text());receipt['output_sha256']=AUDIT.sha(path);write(folder/'receipt.json',receipt)
    elif fault=='hash':path.write_bytes(b'corrupted')
    else:
        result=json.loads((evidence/'result.json').read_text());result['summaries'][0]['successes']=399;write(evidence/'result.json',result)
    with pytest.raises((AssertionError,FileNotFoundError)):
        AUDIT.audit(evidence)

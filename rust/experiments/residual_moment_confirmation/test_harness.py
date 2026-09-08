"""Pre-confirmation adversarial schema, inventory, source and gate regressions."""
import copy
import gzip
import importlib.util
import json
from pathlib import Path
import pytest

spec=importlib.util.spec_from_file_location('confirmation',Path(__file__).with_name('run.py'))
r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)

@pytest.fixture
def sample():
    task=dict(profile='tiny',cell='diffuse_common',k=16,start=0,reps=2,master=r.PLAN['rng']['pipeline_master'],numseed=r.NUMSEED,id='diffuse_common-16-0000')
    n=100
    geom=dict(h=[.2]*n,b=[[.01]*n for _ in range(3)],gram=[1.0]*225)
    fixture=dict(cell='diffuse_common',k=16,n=n,p=31,truth=[1.0]*4,leading_share=[.08]*4,remainder_share=[.07]*4,variance_minimum=1.0,geometry_sha256=r.geometry_hash(geom))
    rows=[{'kind':'task','schema':r.SCHEMA,**{q:task[q] for q in ('profile','cell','k','start','reps','master','numseed')}},
          {'kind':'design',**{q:fixture[q] for q in ('cell','k','n','p','truth','leading_share','remainder_share')},'min_variance':1.0,'max_h':.2}]
    for rep in range(2):
        rows.append(dict(kind='call',replication=rep,status='success',seconds=1,projection_count=512,gram_atoms=512*n,gram_words=1024*n,projection_residual=1e-12,projection_gate=1e-9,full_residual=1e-12,full_gate=1e-9,atoms=1642*n,words=3284*n,rcond=.1,memory=10000,floored=0,**copy.deepcopy(geom)))
        for arm in r.ARMS:
            for target in r.TARGETS:
                rows.append(dict(kind='target',cell=task['cell'],k=16,replication=rep,numseed=r.NUMSEED,seed=r.previous.old.semantic_seed(task['master'],task['cell'],16,rep),arm=arm,target=target,status='success',point_error=.1*(-1)**rep,variance=1.0,estimated_sd=1.0,width=3.92,covered=True,lower_miss=False,upper_miss=False,point_kernel_identity=0.0))
    return rows,task,fixture

def test_valid_inventory(sample):
    rows,t,f=sample
    targets,calls=r.validate(rows,t,f)
    assert len(targets)==16 and len(calls)==2

@pytest.mark.parametrize('change',[
    lambda rows:rows.pop(),lambda rows:rows.append(copy.deepcopy(rows[-1])),
    lambda rows:rows[0].update(profile='confirmation'),lambda rows:rows[0].update(master=1),
    lambda rows:rows[0].update(numseed=1),lambda rows:rows[0].update(schema='old'),
    lambda rows:rows[1].update(n=101),lambda rows:rows[1].update(truth=[2]*4),
    lambda rows:rows[2].update(projection_count=511),lambda rows:rows[2].update(gram_words=0),
    lambda rows:rows[2].update(projection_residual=1),lambda rows:rows[2].update(full_residual=1),
    lambda rows:rows[2].update(atoms=1),lambda rows:rows[2].update(words=1),
    lambda rows:rows[2]['h'].__setitem__(0,.3),lambda rows:rows[2].update(floored=101),
    lambda rows:rows[-1].update(seed=1),lambda rows:rows[-1].update(numseed=1),
    lambda rows:rows[-1].update(target='unknown'),lambda rows:rows[-1].update(arm='oracle'),
    lambda rows:rows[-1].update(status='unknown'),lambda rows:rows[-1].update(variance=float('nan')),
    lambda rows:rows[-1].pop('width'),lambda rows:rows[-1].update(covered=1),
    lambda rows:rows[-1].update(upper_miss=True),lambda rows:rows[-1].update(variance=-1),
    lambda rows:rows[-1].update(estimated_sd=2),lambda rows:rows[3].update(point_kernel_identity=1e-4),
    lambda rows:rows[3].update(remainder_identity=1e-4),lambda rows:rows[3].update(q1_status=0),
    lambda rows:rows.append({'kind':'invalid'}),
])
def test_corrupt_output_is_rejected(sample,change):
    rows,t,f=sample;change(rows)
    with pytest.raises((ValueError,KeyError)):r.validate(rows,t,f)

def test_native_failure_propagates_without_oracle_claim(sample):
    rows,t,f=sample
    for row in rows:
        if row.get('replication')!=0:continue
        if row['kind']=='call':row.update(status='JLA_CONSTRAINT_FAILED',phase='component_inference_psd',detail='deliberate failure')
        else:row['status']='native_call_failed'
    targets,calls=r.validate(rows,t,f)
    assert sum(c['status']=='success' for c in calls)==1
    s=r.scientific_summary([x for x in targets if x['target']=='firm' and x['arm']=='native'],r.CELL_MAP['diffuse_common',16],'firm',f)
    assert s['attempts']==2 and s['success_rate']==.5 and s['coverage_all']==.5
    assert s['failure_counts']=={'native_call_failed':1}

def ideal_summaries():
    out=[]
    for c in r.CELLS:
        for target in r.TARGETS:
            dominant=c['dominant'];covariance=target=='covariance'
            out.append(dict(cell=c['cell'],k=c['k'],target=target,gate=c['gate'],reference=c['reference'],success_rate=1,successes=2500,bias=0,bias_mcse=.02,coverage=.8 if c['gate']=='descriptive' else .95,coverage_mcse=.00436,se_ratio=1.0,mean_leading_share=(.99 if covariance else .9) if dominant else (.10 if c['k']==12 else .08),mean_remainder_share=(.8 if covariance else .10) if dominant else .08))
    return out

def test_original_gates_pass_then_enumerate_all_failures():
    rows=ideal_summaries();assert r.gates._scientific_failures(rows,'confirmation')[0]==[]
    row=next(x for x in rows if (x['cell'],x['k'],x['target'])==('dominant_common_controls',16,'firm'))
    row.update(success_rate=.98,coverage=.966,bias=1,se_ratio=1.2)
    failures=r.gates._scientific_failures(rows,'confirmation')[0]
    assert len(failures)==4
    assert any('coverage' in f for f in failures) and any('success rate' in f for f in failures)

def test_multimode_exclusion_retains_availability_gate():
    rows=ideal_summaries();row=next(x for x in rows if (x['cell'],x['k'],x['target'])==('dominant_common',16,'covariance'))
    row.update(coverage=.1,se_ratio=2,bias=10)
    assert r.gates._scientific_failures(rows,'confirmation')[0]==[]
    row['success_rate']=.94
    assert r.gates._scientific_failures(rows,'confirmation')[0]==['dominant_common/16/covariance: success rate']

def test_scientific_failure_has_nonzero_cli_exit(monkeypatch,tmp_path):
    monkeypatch.setattr(r,'aggregate',lambda _:dict(status='FAIL'))
    monkeypatch.setattr('sys.argv',['runner','aggregate',str(tmp_path)])
    assert r.main()==2

@pytest.fixture
def aggregate_fixture(sample,tmp_path,monkeypatch):
    rows,t,f=sample;c=r.CELL_MAP[t['cell'],t['k']]
    monkeypatch.setattr(r,'CELLS',[c])
    manifest=dict(profile='tiny',tasks=r.tasks_for('tiny'),source_bundle_sha256='source',binary_sha256='binary',expected_native_calls=2,expected_all_targets=16,preflight={'fixtures':{'diffuse_common/16':f}})
    out=tmp_path/'campaign';out.mkdir();r.write_json(out/'manifest.json',manifest)
    folder=out/'tasks'/t['id'];folder.mkdir(parents=True)
    with gzip.open(folder/'rows.jsonl.gz','wt') as stream:
        for row in rows:stream.write(json.dumps(row)+'\n')
    receipt=dict(schema=r.SCHEMA,status='execution_and_inventory_pass',task=t,manifest_sha256=r.sha(out/'manifest.json'),source_bundle_sha256='source',binary_sha256='binary',output_sha256=r.sha(folder/'rows.jsonl.gz'),native_calls=2,target_rows=16)
    r.write_json(folder/'receipt.json',receipt)
    return out,folder

def test_aggregate_round_trip(aggregate_fixture):
    out,_=aggregate_fixture
    result=r.aggregate(out)
    assert result['status']=='PIPELINE_PASS_NOT_CONFIRMATION' and result['counts']['all_targets']==16

@pytest.mark.parametrize('field',['task','manifest_sha256','source_bundle_sha256','binary_sha256','output_sha256','status','native_calls','target_rows'])
def test_wrong_receipt_rejected(aggregate_fixture,field):
    out,folder=aggregate_fixture;p=folder/'receipt.json';data=json.loads(p.read_text());data[field]='wrong';p.write_text(json.dumps(data))
    with pytest.raises(ValueError):r.aggregate(out)

def test_missing_task_receipt_rejected(aggregate_fixture):
    out,folder=aggregate_fixture;(folder/'receipt.json').unlink()
    with pytest.raises(FileNotFoundError):r.aggregate(out)

def test_overlapping_manifest_rejected(aggregate_fixture):
    out,_=aggregate_fixture;p=out/'manifest.json';data=json.loads(p.read_text());data['tasks'].append(data['tasks'][0]);p.write_text(json.dumps(data))
    with pytest.raises(ValueError,match='manifest'):r.aggregate(out)

def test_partial_gzip_rejected(aggregate_fixture):
    _,folder=aggregate_fixture;p=folder/'rows.jsonl.gz';p.write_bytes(p.read_bytes()[:-8])
    with pytest.raises(EOFError):r.read_rows(p)

def test_unknown_or_duplicate_task_directory_rejected(aggregate_fixture):
    out,_=aggregate_fixture;(out/'tasks'/'unexpected').mkdir()
    with pytest.raises(ValueError,match='folder'):r.aggregate(out)

def test_task_semantic_ranges_are_complete_and_disjoint():
    tasks=r.tasks_for('confirmation');assert len(tasks)==200
    keys=[(t['cell'],t['k'],i) for t in tasks for i in range(t['start'],t['start']+t['reps'])]
    assert len(keys)==len(set(keys))==50000
    assert len({t['id'] for t in tasks})==200

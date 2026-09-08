"""Adversarial inventory checks and real tiny generator-to-receipt pipeline."""
from copy import deepcopy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import pytest

spec=importlib.util.spec_from_file_location('followup_run',Path(__file__).with_name('run.py'))
run=importlib.util.module_from_spec(spec);spec.loader.exec_module(run)
EXE=Path(os.environ.get('FEVC_FOLLOWUP_TEST_EXECUTABLE',run.ROOT/'.local/diagnostics/residual-moment-followup-20260906/build-final/followup'))

@pytest.fixture(scope='module')
def tiny(tmp_path_factory):
    if not EXE.is_file():pytest.skip('build the independent follow-up executable first')
    out=tmp_path_factory.mktemp('followup')
    g=run.geometry(EXE,out/'geometry','dominant_common',12,200,610731)
    assert g['status']=='success'
    t={'cell':'dominant_common','k':12,'mode':'confirmation','reps':3,'start':0,'master':7346928104512301,'numseed':610731,'input':g['binary']}
    receipt=run.task_run(EXE,out/'tiny',t)
    rows=[json.loads(x) for x in (out/'tiny/rows.jsonl').read_text().splitlines()]
    return out,t,rows,receipt

def test_tiny_receipt_and_complete_inventory(tiny):
    out,t,rows,receipt=tiny
    assert receipt['target_rows']==24
    assert receipt['output_sha256']==run.sha(out/'tiny/rows.jsonl')
    assert len(run.validate(rows,t))==24
    assert len(receipt['summaries'])==8

@pytest.mark.parametrize('change',['duplicate','missing','wrong_seed','wrong_cell','nan','partial','bad_status','bad_indicator'])
def test_reject_bad_rows(tiny,change):
    _,t,rows,_=tiny
    rows=deepcopy(rows);i=next(i for i,r in enumerate(rows) if r['kind']=='target')
    if change=='duplicate':rows.append(deepcopy(rows[i]))
    elif change=='missing':rows.pop(i)
    elif change=='wrong_seed':rows[i]['seed']+=1
    elif change=='wrong_cell':rows[i]['cell']='unregistered'
    elif change=='nan':rows[i]['variance']=float('nan')
    elif change=='partial':rows[i].pop('width')
    elif change=='bad_status':rows[i]['status']='silently_skipped'
    elif change=='bad_indicator':rows[i]['covered']=2
    with pytest.raises((ValueError,KeyError)):run.validate(rows,t)

def test_reject_bad_json_and_nonzero_process(tmp_path):
    if not EXE.is_file():pytest.skip('build the independent follow-up executable first')
    with pytest.raises(json.JSONDecodeError):json.loads('{partial')
    with pytest.raises(subprocess.CalledProcessError):run.capture(EXE,['invalid'],tmp_path/'failed.jsonl')
    assert not (tmp_path/'failed.jsonl').exists()
    assert (tmp_path/'failed.jsonl.partial').exists()

def test_scientific_failure_is_not_a_schema_failure(tiny):
    _,t,rows,_=tiny;rows=deepcopy(rows)
    for r in rows:
        if r['kind']=='target':r.update(status='variance_fit_failed')
    run.validate(rows,t)
    for s in run.summaries(rows):
        assert s['success_rate']==0 and s['coverage_all']==0
        assert 'artificial: success rate' in run.gates._gate_correct('artificial',s)

def test_shards_and_reverse_order_equal_unsplit_targets(tiny):
    out,t,rows,_=tiny;expected=run.validate(rows,t);actual=[]
    for start,reps in [(2,1),(0,2)]:
        shard={**t,'start':start,'reps':reps}
        run.task_run(EXE,out/f'shard-{start}',shard)
        actual+=run.validate([json.loads(x) for x in (out/f'shard-{start}/rows.jsonl').read_text().splitlines()],shard)
    key=lambda r:(r['replication'],r['arm'],r['target'])
    assert sorted(actual,key=key)==sorted(expected,key=key)

def test_wrong_binary_shape_is_fatal(tiny,tmp_path):
    _,t,_,_=tiny;p=tmp_path/'bad.bin';p.write_bytes(b'bad')
    with pytest.raises(subprocess.CalledProcessError):run.task_run(EXE,tmp_path/'run',{**t,'input':str(p)})
    assert not (tmp_path/'run/receipt.json').exists()

def test_manifest_scientific_role_cannot_be_changed(tiny):
    _,t,rows,_=tiny
    with pytest.raises(ValueError):run.validate(rows,{**t,'gate':'diagnostic'})
    with pytest.raises(ValueError):run.validate(rows,{**t,'reference':'Q0'})

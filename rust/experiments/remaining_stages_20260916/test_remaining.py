import copy
from pathlib import Path
import sys
import types
import pytest

sys.path.insert(0,str(Path(__file__).parent))
import remaining as r


def test_registered_inventory():
    source=Path(__file__).parents[1]/'pipeline_optimization_20260915'
    sys.path.insert(0,str(source))
    import campaign
    plan=campaign.protocol()
    for name in r.STAGES:
        groups={}
        for c in plan['stages'][name]:
            value=copy.deepcopy(c)
            value['id']=f"{name}-{c['cell']}-{c['warmup']}-{c['repetition']}-{c['variant']}"
            groups.setdefault(c['cell'],dict(id=c['cell']+1,calls=[]))['calls'].append(value)
        assert r.inventory_counts(list(groups.values()))==r.COUNTS[name]
        for t in groups.values():
            measured=[c for c in t['calls'] if not c['warmup']]
            assert len({c['seed'] for c in measured})==(3 if name!='validation_1_6m' else 5)


@pytest.mark.parametrize('bad', ['duplicate_call','duplicate_task','missing_task'])
def test_malformed_inventory(bad):
    tasks=[dict(id=1,calls=[dict(id='a',warmup=False)]),dict(id=2,calls=[dict(id='b',warmup=True)])]
    if bad=='duplicate_call': tasks[1]['calls'][0]['id']='a'
    if bad=='duplicate_task': tasks[1]['id']=1
    if bad=='missing_task': tasks[1]['id']=3
    with pytest.raises(ValueError): r.inventory_counts(tasks)


def test_private_permissions(tmp_path):
    tmp_path.chmod(0o700)
    r.check_private(tmp_path)
    tmp_path.chmod(0o750)
    with pytest.raises(ValueError): r.check_private(tmp_path)


def test_write_once_and_atomic_update(tmp_path):
    p=tmp_path/'receipt.json'
    r.put(p,dict(status='ATTEMPTED'))
    with pytest.raises(FileExistsError): r.put(p,{})
    r.put(p,dict(status='PASS'),replace=True)
    assert r.read(p)==dict(status='PASS')


def test_private_routing(tmp_path):
    plan=dict(private_output=str(tmp_path/'private'))
    assert r.root_for(tmp_path,plan,'private_veneto')==tmp_path/'private'
    assert r.root_for(tmp_path,plan,'screen_400k')==tmp_path/'screen_400k'


def test_matlab_normal_and_deliberate_attempts_have_distinct_scratch(tmp_path,monkeypatch):
    monkeypatch.setenv('TMPDIR',str(tmp_path))
    monkeypatch.setenv('JOB_ID','1234')
    call=dict(id='same-scientific-call')
    normal=r.matlab_scratch(call,tmp_path/'normal')
    deliberate=r.matlab_scratch(call,tmp_path/'deliberate')
    assert normal!=deliberate
    normal.mkdir()
    deliberate.mkdir()
    assert r.matlab_scratch(call,tmp_path/'normal')==normal


@pytest.mark.parametrize('status',['FAIL','PERFORMANCE_FAIL','SCIENTIFIC_PASS'])
def test_dependency_rejects_failure(tmp_path,status):
    (tmp_path/'smoke').mkdir()
    r.put(tmp_path/'smoke/report.json',dict(status=status))
    with pytest.raises(ValueError): r.prerequisite(tmp_path,{},'screen_400k',None)


def test_matlab_pin_rejects_drift(tmp_path):
    p=tmp_path/'x.m'
    p.write_text('original')
    m=dict(matlab_commit='8b957ffe',matlab_files={'x.m':r.sha(p)},matlab_root=str(tmp_path),mex_dir=str(tmp_path),mex_files={})
    r.verify_matlab(m)
    p.write_text('changed')
    with pytest.raises(ValueError): r.verify_matlab(m)


def test_submission_dag(tmp_path,monkeypatch):
    (tmp_path/'submissions').mkdir()
    r.put(tmp_path/'plan.json',{})
    r.put(tmp_path/'submissions/smoke.json',dict(aggregate_job_id='9'))
    monkeypatch.setattr(r,'prerequisite',lambda *args:None)
    monkeypatch.setattr(r,'verify_matlab',lambda *args:None)
    commands=[]
    def qsub(args,text):
        commands.append(args)
        return str(10+len(commands))
    monkeypatch.setattr(r.subprocess,'check_output',qsub)
    modules=[None,None,None,types.SimpleNamespace(collect_accounting=lambda *args:None)]
    plan=dict(prior_root=str(tmp_path),matlab_runtime={},inputs={},private_output=str(tmp_path/'private'),
        stages={s:[{}]*r.COUNTS[s][0] for s in r.STAGES},counts=r.COUNTS)
    r.submit(tmp_path,plan,modules)
    assert len(commands)==6
    assert [c[c.index('-hold_jid')+1] for c in commands]==['9','11','12','13','14','15']
    assert [commands[i][commands[i].index('-t')+1] for i in [0,2,4]]==['1-20','1-6','1-2']
    assert str(tmp_path/'private/logs') in commands[4]
    assert all('-q' not in c and '-V' not in c for c in commands)


def test_stata_driver_restores_and_times_setup():
    text=(Path(__file__).parent/'remaining_stata.do').read_text()
    assert text.index('timer on 80')<text.index('adopath ++')<text.index("capture noisily `command'")
    for expected in ['e(sample)','c(rngstate)','c(sortrngstate)','r(state)==0','set processors `=min(4,`t\')\'']:
        assert expected in text
    assert 'rename y_adjusted y' in text
    assert 'group(worker firm)' in text


@pytest.fixture
def local_evidence(tmp_path):
    import os,shutil
    fixture=os.environ.get('PF_LOCAL_FIXTURE')
    if not fixture: pytest.skip('set PF_LOCAL_FIXTURE to the passing isolated local workflow')
    source=Path(fixture)
    shutil.copytree(source/'smoke',tmp_path/'smoke')
    shutil.copyfile(source/'plan.json',tmp_path/'plan.json')
    plan=r.read(tmp_path/'plan.json')
    return tmp_path,plan,r.support(plan)


def test_local_output_roundtrip(local_evidence):
    root,plan,modules=local_evidence
    result=r.validate(root,plan,modules,'smoke')
    assert result['status']=='PASS'
    assert len(result['inventory'])==8 and len(result['pairs'])==4
    assert max(g for pair in result['pairs'] for g in pair['gaps'].values())==0


@pytest.mark.parametrize('kind',['missing','stale','duplicate','partial','numerical','nonfinite','rss','clock'])
def test_rejects_corrupt_outputs(local_evidence,kind):
    root,plan,modules=local_evidence
    call=plan['stages']['smoke'][0]['calls'][1]
    dest=root/'smoke/tasks/task-01'/call['id']
    p=dest/'attempt.json'
    record=r.read(p)
    output=dest/'result.tsv'
    if kind=='missing': p.unlink()
    elif kind=='stale': record['call']['seed']+=1
    elif kind=='rss': record['rss_guard_gib']=100
    elif kind=='clock': record['command_seconds']*=2
    else:
        lines=output.read_text().splitlines()
        if kind=='duplicate': lines.append(lines[-1])
        elif kind=='partial': lines.append('matrix\tresults')
        else:
            index=next(i for i,line in enumerate(lines) if line.startswith('matrix\tresults\t3\t1\t'))
            fields=lines[index].split('\t')
            fields[4]='NaN' if kind=='nonfinite' else '1000'
            lines[index]='\t'.join(fields)
        output.write_text('\n'.join(lines)+'\n')
        record['result_sha256']=r.sha(output)
    if kind!='missing': r.put(p,record,replace=True)
    report=r.validate(root,plan,modules,'smoke')
    assert report['status']=='FAIL'
    assert len(report['inventory'])==8
    assert report['errors']

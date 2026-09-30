"""Private native adapter inventory/CLI gates, separate from plugin qualification."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess

import numpy as np
import pytest

from fevc.tools.all_probe_native import execute, native_input, semantic_seed, validate_inventory, core_source_identity, check_audit_dependencies
from fevc.tools.all_probe_reference import input_identity
from .test_all_probe_reference import fixture

ROOT=Path(__file__).resolve().parents[3]
BINARY=ROOT/'rust/target/release/examples/all_probe_reference'


def manifest(data=None,**kwargs):
    return dict(schema='FEVC_ALL_PROBE_NATIVE_REFERENCE_V1',input=fixture() if data is None else data,
                K=2,L=2,R=8,T=9,seed=9182,**kwargs)


def test_private_native_semantic_keys_are_schedule_invariant():
    m=manifest()
    assert semantic_seed(m,0,1,0)==semantic_seed(m,0,1,1)
    assert semantic_seed(m,1,1,0)!=semantic_seed(m,1,1,1)
    assert len({semantic_seed(m,d,k,l) for d in (0,1) for k in range(128)
                for l in (range(8) if d else [0])})==1152
    assert native_input(m).splitlines()[0].startswith('FEVC_ALL_PROBE_NATIVE_INPUT_V1 36 1 8 9 4 observation joint')
    with pytest.raises(ValueError):
        native_input(manifest(dict(worker=[0],firm=[0],outcome=[1.],frequency=[0])))
    for key in ('native_harness_sha256','reference_sha256'):
        bindings=dict(native_harness_sha256=hashlib.sha256((ROOT/'fevc/tools/all_probe_native.py').read_bytes()).hexdigest(),
                      reference_sha256=hashlib.sha256((ROOT/'fevc/tools/all_probe_reference.py').read_bytes()).hexdigest())
        check_audit_dependencies(bindings)
        bindings[key]='0'*64
        with pytest.raises(ValueError,match='dependency'):check_audit_dependencies(bindings)


@pytest.mark.skipif(not BINARY.exists(),reason='explicit local native-reference gate needs the built Rust example')
def test_native_attempts_corruption_failure_and_atomic_cli(tmp_path):
    m=manifest();m['input_sha256']=input_identity(m['input'])
    result=execute(m,BINARY)
    assert result['summary']['attempted']==4 and not result['summary']['point_failures']
    assert 'nested' in result['summary']
    for mutation in ('missing','duplicate','seed','count','nonfinite','accounting','raw','work','summary','status',
                     'replay_attempted_rhs','replay_executed_rhs','fractional_replay','missing_attempted',
                     'rhs','counter_unique_packed_words','counter_physical_trials','minimum_margin',
                     'minimum_constrained','sensitivity_ratio','replay_generator_word_evaluations','allocation_bound_bytes','conditioning','psd_adjustment','true_margin','manifest_route','manifest_gate'):
        broken=copy.deepcopy(result)
        if mutation=='missing': broken['attempts'].pop()
        elif mutation=='duplicate': broken['attempts'][1]=copy.deepcopy(broken['attempts'][0])
        elif mutation=='fractional_replay': broken['attempts'][0]['replay_attempted_rhs']=float(broken['attempts'][0]['replay_attempted_rhs'])
        elif mutation=='missing_attempted': broken['attempts'][0].pop('replay_attempted_rhs')
        elif mutation=='seed': broken['attempts'][0]['target_seed']+=1
        elif mutation=='count': broken['attempts'][0]['T']+=1
        elif mutation=='nonfinite': broken['attempts'][0]['point'][0]=np.nan
        elif mutation=='accounting': broken['attempts'][0]['point'][0]+=1
        elif mutation=='raw': broken['attempts'][0]['all_raw'][0][0]+=1
        elif mutation=='work': broken['attempts'][0]['replay_rhs']-=1
        elif mutation=='summary': broken['summary']['attempted']+=1
        elif mutation=='status': broken['attempts'][0]['diagnostic_status']='invented'
        elif mutation=='psd_adjustment': broken['attempts'][0]['psd_adjustment']=1.
        elif mutation=='true_margin': broken['summary']['true_maker_margin']=1000.
        elif mutation=='manifest_route': broken['manifest']['route']='automatic_cmg'
        elif mutation=='manifest_gate': broken['manifest']['full_residual_gate']=1e-8
        elif mutation=='conditioning': broken['summary']['conditioning']='success_conditional'
        else: broken['attempts'][0][mutation]=-1
        with pytest.raises((ValueError,KeyError)): validate_inventory(broken)
    m.update(source_sha=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
             rng='Counter-V1',seed_algorithm='numpy-seedsequence-u64-v1',
             native_binary_sha256=hashlib.sha256(BINARY.read_bytes()).hexdigest(),
             native_harness_sha256=hashlib.sha256((ROOT/'fevc/tools/all_probe_native.py').read_bytes()).hexdigest(),
             reference_sha256=hashlib.sha256((ROOT/'fevc/tools/all_probe_reference.py').read_bytes()).hexdigest(),
             core_source_sha256=core_source_identity(ROOT))
    input_file=tmp_path/'manifest.json'; output=tmp_path/'result.json'
    input_file.write_text(json.dumps(m))
    command=[str(ROOT/'.venv/bin/python'),str(ROOT/'fevc/tools/all_probe_native.py'),str(input_file),str(BINARY),str(output)]
    assert subprocess.run(command,cwd=ROOT,capture_output=True).returncode==0
    validate_inventory(json.loads(output.read_text()))
    before=output.read_bytes();m['native_binary_sha256']='0'*64;input_file.write_text(json.dumps(m))
    assert subprocess.run(command,cwd=ROOT,capture_output=True).returncode!=0
    assert output.read_bytes()==before
    m['input']=dict(worker=[0,0],firm=[0,1],outcome=[1.,2.],frequency=[1,1],deletion='observation')
    m['input_sha256']=input_identity(m['input']);m['native_binary_sha256']=hashlib.sha256(BINARY.read_bytes()).hexdigest()
    input_file.write_text(json.dumps(m))
    assert subprocess.run(command,cwd=ROOT,capture_output=True).returncode==1
    failed=json.loads(output.read_text());validate_inventory(failed)
    assert len(failed['summary']['point_failures'])==4
    assert failed['summary']['conditioning']=='success_conditional'
    assert not output.with_suffix('.json.tmp').exists()

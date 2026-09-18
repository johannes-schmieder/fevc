import json
from pathlib import Path
import sys
import time
import pytest

sys.path.insert(0,str(Path(__file__).parent))
from campaign import PROFILES
from screen_manifest import freeze
from screen_runner import run_task,sha,write_json
from screen_scc import accounting,scc_granted_slots
from screen_validate import validate
from thread_adapter import BASELINE_SNAPSHOT


def test_linux_cshim_link_commands_include_math_library(tmp_path,monkeypatch):
    import screen_scc
    root=tmp_path/'run'
    (root/'artifacts').mkdir(parents=True)
    monkeypatch.setenv('TMPDIR',str(tmp_path))
    monkeypatch.setenv('JOB_ID','1234')
    monkeypatch.setattr(screen_scc.subprocess,'check_output',lambda *a,**k:'rustc 1.85.1 (pinned)')
    monkeypatch.setattr(screen_scc.shutil,'copy2',lambda *a,**k:None)
    commands=[]

    def capture(arguments,output,cwd=None):
        commands.append([str(a) for a in arguments])
        if Path(output).name=='abi_header_compat_test-build.txt':
            raise RuntimeError('compile commands captured')

    monkeypatch.setattr(screen_scc,'checked',capture)
    with pytest.raises(RuntimeError,match='compile commands captured'):
        screen_scc.native_qualification(root,'stata-mp')
    compiles=[c for c in commands if c[0]=='cc']
    assert len(compiles)==3
    for command in compiles[:2]:
        source=next(a for a in command if a.endswith('.c'))
        assert command.count('-lm')==1
        assert command.index('-lm')>command.index(source)
        assert '-Wl,--gc-sections' in command
    assert '-c' in compiles[2] and '-lm' not in compiles[2]


def fixture(tmp_path,smoke=False):
    inputs=tmp_path/'inputs'
    inputs.mkdir()
    for name,p in PROFILES.items():
        data=inputs/f'{name}.csv'
        data.write_text('public fixture placeholder\n')
        write_json(inputs/f'{name}.json',dict(profile=name,smoke=smoke,
            rows=min(p['rows'],8000) if smoke else p['rows'],sha256=sha(data)))
    packages=tmp_path/'packages'
    for variant in ('baseline','candidate'):
        package=packages/variant
        package.mkdir(parents=True)
        (package/'fevc.ado').write_text('test package')
        write_json(package/'adapter.json',dict(variant=variant,
            source_snapshot_sha256=BASELINE_SNAPSHOT if variant=='baseline' else 'c'*64,
            files={'fevc.ado':sha(package/'fevc.ado')}))
    return inputs,packages


def test_native_install_catalog_from_readonly_source(tmp_path,monkeypatch):
    import os
    import screen_scc
    root=tmp_path/'run'
    (root/'artifacts').mkdir(parents=True)
    (root/'source/fevc').mkdir(parents=True)
    original=root/'source/fevc/fevc.pkg'
    original.write_text('v 3\nd isolated fixture\n')
    original.chmod(0o444)
    original_hash=sha(original)
    monkeypatch.setenv('TMPDIR',str(tmp_path))
    monkeypatch.setenv('JOB_ID','1234')
    monkeypatch.setattr(screen_scc.subprocess,'check_output',lambda *a,**k:'rustc 1.85.1 (pinned)')

    def capture(arguments,output,cwd=None):
        if Path(output).name=='build.txt':
            plugin=Path(os.environ['CARGO_TARGET_DIR'])/'release/libvckss_stata.so'
            plugin.parent.mkdir(parents=True)
            plugin.write_bytes(b'isolated fixture')
        if arguments[0]=='stata-mp': raise RuntimeError('install staging captured')

    monkeypatch.setattr(screen_scc,'checked',capture)
    with pytest.raises(RuntimeError,match='install staging captured'):
        screen_scc.native_qualification(root,'stata-mp')
    assert sha(original)==original_hash and original.stat().st_mode & 0o222==0
    assert (root/'native-package/fevc.pkg').read_text()==original.read_text()+'\nf fevc_rust_linux_x64.plugin\n'
    assert (root/'native-package/fevc.pkg').stat().st_mode & 0o200


def test_screen_inventory_rotation_and_missing_call_enumeration(tmp_path):
    inputs,packages=fixture(tmp_path)
    root=tmp_path/'stage'
    m=freeze(root,inputs,packages)
    assert len(m['tasks'])==36
    assert (m['expected_measured'],m['expected_warmups'])==(216,72)
    assert all(len(t['calls'])==8 for t in m['tasks'])
    for task in m['tasks']:
        calls=task['calls']
        assert all(c['warmup'] for c in calls[:2])
        assert calls[2]['variant']!=calls[4]['variant']
        assert calls[2]['seed']==calls[3]['seed']
    report=validate(m,root)
    assert report['status']=='FAIL'
    assert len(report['inventory'])==288
    assert all(x['status']=='UNATTEMPTED' for x in report['inventory'])
    with pytest.raises(ValueError,match='exists'): freeze(root,inputs,packages)


def test_scheduler_rounding_preserves_seven_native_threads_and_limits(tmp_path):
    inputs,packages=fixture(tmp_path,True)
    m=freeze(tmp_path/'rounded',inputs,packages,7,8)
    assert (m['requested_slots'],m['slots'],m['rss_gib'],m['seconds'])==(7,8,10,1500)
    assert all(t['threads']==7 for t in m['tasks'])
    assert all(c['threads']==7 for t in m['tasks'] for c in t['calls'])
    assert [scc_granted_slots(n) for n in (1,4,7,14)]==[1,4,8,14]
    for requested in (0,3,8,16):
        with pytest.raises(ValueError,match='unregistered'): scc_granted_slots(requested)
    for smoke,granted in ((7,6),(7,9),(4,8),(None,16)):
        with pytest.raises(ValueError,match='unregistered'):
            freeze(tmp_path/f'invalid-{smoke}-{granted}',inputs,packages,smoke,granted)
    text=record().replace('slots 14','slots 8')
    assert accounting(text,1234,[0],scc_granted_slots(7))[0]['slots']=='8'
    with pytest.raises(ValueError): accounting(text,1234,[0],7)


def test_malformed_manifest_and_prelaunch_failure_keep_unattempted_calls(tmp_path):
    inputs,packages=fixture(tmp_path,True)
    root=tmp_path/'stage'
    m=freeze(root,inputs,packages,4)
    path=root/'manifest.json'
    with pytest.raises(ValueError,match='identity'):
        run_task(path,1,Path(sys.executable),4,time.time(),'bad')
    with pytest.raises(ValueError,match='allocation'):
        run_task(path,1,Path(sys.executable),3,time.time(),sha(path))
    m['tasks'][0]['calls'][0]['command']='stale or tampered command'
    write_json(path,m)
    assert not run_task(path,1,Path(sys.executable),4,time.time(),sha(path))
    states=json.loads((root/'tasks/task-01/inventory.json').read_text())
    assert list(states.values())==['FAIL','UNATTEMPTED']
    receipt=json.loads((root/'tasks/task-01/01-baseline-r1/attempt.json').read_text())
    assert 'effective command changed' in receipt['error']
    assert receipt['rss_samples']==0


def record(task=0,exit_status=0):
    return f'''==============================================================
jobnumber 1234
taskid {task}
failed 0
exit_status {exit_status}
slots 14
hostname compute-1
qname shared
ru_wallclock 10
cpu 42
maxvmem 1.2G
'''


def test_accounting_rejects_failed_missing_duplicate_and_mixed_jobs():
    text=record(1)+record(2)
    assert len(accounting(text,1234,[1,2],14))==2
    for bad,match in [(record(1),'inventory'),(text+record(1),'inventory'),
        (record(1,1)+record(2),'failure'),(text.replace('1234','9999'),'identity')]:
        with pytest.raises(ValueError,match=match): accounting(bad,1234,[1,2],14)


def test_snapshot_source_builder_rejects_nonmatching_bytes(tmp_path):
    import importlib.util
    path=Path(__file__).parents[1]/'optimization_parity_20260913/build_dirty_linux_bundle.py'
    spec=importlib.util.spec_from_file_location('pipeline_bundle',path)
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    (tmp_path/'source/rust').mkdir(parents=True)
    source=tmp_path/'source/rust/test.rs'
    source.write_text('fn main() {}')
    snapshot=tmp_path/'snapshot.json'
    write_json(snapshot,dict(git_head='a'*40,source={'rust/test.rs':sha(source)}))
    a=module.build(tmp_path,'a'*40,snapshot)
    b=module.build(tmp_path,'a'*40,snapshot)
    assert a==b
    source.write_text('drift')
    with pytest.raises(ValueError,match='mismatch'): module.build(tmp_path,'a'*40,snapshot)

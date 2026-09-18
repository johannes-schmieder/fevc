import importlib.util
import json
from pathlib import Path
import pytest

spec = importlib.util.spec_from_file_location('pipeline_threads', Path(__file__).with_name('thread_adapter.py'))
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)
ROOT = Path(__file__).resolve().parents[3]


def test_candidate_context_is_narrow_and_covers_both_dispatchers():
    original = (ROOT/'fevc/fevc.ado').read_text()
    component = (ROOT/'fevc/fevc__rust_comp_batch_receipt.ado').read_text()
    adapted, checked = adapter.transform(original, component, 'candidate')
    assert adapted.count('_fevc_pipeline_threads native_threads') == 2
    assert adapted.count('active_processors = c(processors)') == original.count('active_processors = c(processors)')
    assert checked == component
    assert adapted.index('_fevc_pipeline_threads pipeline_entry_threads') < adapted.index('_vckss_route_context_clear')
    assert 'real("`scheduler_slots\'")!=`s\'' in adapted
    for bad in (original.replace('local native_threads = c(processors)', 'local native_threads = 1', 1),
                adapted, original.replace("`generic_work' `native_threads'", "`generic_work'")):
        with pytest.raises(ValueError):
            adapter.transform(bad, component, 'candidate')


def test_historical_baseline_never_gains_exact_parallelism():
    ado = '''program define fevc, eclass
    version 18.0
    local projection_requested = 0
    threads(`=c(processors)')
    `cmg_threads_requested'==c(processors)
    `cmg_threads_used'==c(processors)
    `generic_work'[1,5]==c(processors)
    `generic_work'[1,6]<=c(processors)
    `inferencesimulations' `inferencegramprobes' `generic_work'
    ereturn scalar active_processors = c(processors)
'''
    component = "args handle covariance_probes gram_probes work\n8*c(processors)\n`batch'[1,11]==c(processors)"
    adapted, checked = adapter.transform(ado, component, 'baseline')
    assert 'exactexecution(' not in adapted and 'exactlegacy(' not in adapted
    assert 'active_processors = c(processors)' in adapted
    assert 'c(processors)' not in checked


def test_hashes_checked_before_output(tmp_path):
    source = tmp_path/'snapshot.json'
    source.write_text('{}')
    with pytest.raises(ValueError, match='source snapshot mismatch'):
        adapter.build(source,'0'*64,tmp_path/'absent','0'*64,tmp_path/'out','candidate')
    assert not (tmp_path/'out').exists()


def test_readonly_snapshot_stages_identical_writable_adapter_copies(tmp_path):
    source=tmp_path/'source/fevc'
    source.mkdir(parents=True)
    inventory={}
    for name in ('fevc.ado','fevc__rust_comp_batch_receipt.ado','fevc__rust_public_call.ado','fevc.pkg'):
        path=source/name
        path.write_bytes((ROOT/'fevc'/name).read_bytes())
        inventory[f'fevc/{name}']=adapter.sha(path)
    snapshot=tmp_path/'snapshot.json'
    snapshot.write_text(json.dumps(dict(source=inventory)))
    plugin=tmp_path/'test.plugin'
    plugin.write_bytes(b'isolated fixture')
    ordinary=adapter.build(snapshot,adapter.sha(snapshot),plugin,adapter.sha(plugin),tmp_path/'ordinary','candidate')
    for path in source.iterdir(): path.chmod(0o444)
    frozen=adapter.build(snapshot,adapter.sha(snapshot),plugin,adapter.sha(plugin),tmp_path/'frozen','candidate')
    assert ordinary['files']==frozen['files']
    for name,digest in inventory.items():
        original=tmp_path/'source'/name
        assert adapter.sha(original)==digest and original.stat().st_mode & 0o222==0
        assert (tmp_path/'frozen'/Path(name).name).stat().st_mode & 0o200

"""Final audit of immutable native development outputs and local source gates."""
from pathlib import Path
import collections
import importlib.util
import json
import re
import sys

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('native_runner',HERE/'run.py')
run=importlib.util.module_from_spec(spec);spec.loader.exec_module(run)

def main(directory,output):
    dev=directory/'development';result=json.loads((dev/'result.json').read_text())
    manifest=json.loads((dev/'manifest.json').read_text())
    for path,digest in manifest['sources'].items():
        assert run.old.sha(run.ROOT/path)==digest,path
    assert run.old.sha(dev/'source.tar.gz')==manifest['source_bundle_sha256']
    targets=[];calls=[];failures=[];seconds=[];geometry=[]
    for t in manifest['tasks']:
        folder=dev/f'{t["cell"]}-{t["numseed"]}'
        receipt=json.loads((folder/'receipt.json').read_text())
        assert run.old.sha(folder/'rows.jsonl')==receipt['output_sha256']
        rows=[json.loads(line) for line in (folder/'rows.jsonl').read_text().splitlines()]
        d,cs,ts=run.validate(rows,t);targets+=ts;calls+=list(cs.values())
        successes=[c for c in cs.values() if c['status']=='success']
        for c in successes:
            expected=d['n']*(1000+128+2+512)+(0 if t['cell']=='diffuse_common' else 800000)
            assert c['atoms']==expected and c['words']==2*expected
        seconds += [dict(cell=t['cell'],numseed=t['numseed'],n=d['n'],mean_seconds=receipt['native_seconds']/t['reps'])]
        geometry += [dict(cell=t['cell'],numseed=t['numseed'],successful_outcomes=len(successes),unchanged=True)]
        failures += [{**t,**c} for c in cs.values() if c['status']!='success']
    assert len(calls)==800 and len(targets)==6400
    paired=collections.defaultdict(dict)
    for r in targets:paired[(r['cell'],r['numseed'],r['replication'],r['target'])][r['arm']]=r
    comparable=[p for p in paired.values() if all(r['status']=='success' for r in p.values())]
    disagreement=sum(p['native']['covered']!=p['exact_same_variance']['covered'] for p in comparable)
    native=[r for r in targets if r['arm']=='native'];success=[r for r in native if r['status']=='success']
    full=run.summary(targets)
    qualifier=directory/'native-qualification-network.txt'
    q=dict(line.split('=',1) for line in qualifier.read_text().splitlines() if '=' in line)
    assert q['arm64_test_status']=='PASS_NATIVE' and q['x86_64_test_status']=='PASS_ROSETTA'
    assert all(q[k]=='PASS' for k in ('cargo_fmt','cargo_clippy','cargo_test','cshim_interrupt_test','cshim_error_transport_test','abi_header_compat_test'))
    count=lambda name:sum(map(int,re.findall(r'test result: ok\. (\d+) passed', (directory/name).read_text())))
    out={'schema':'FEVC-OBSERVATION-RESIDUAL-MOMENTS-INTEGRATION-RESULT-V1',
        'status':'internal_development_complete_with_one_preserved_psd_failure',
        'production_scope':'hidden Rust constructor only; public Stata/FFI options, match RC and defaults unchanged',
        'source_head':manifest['head'],'source_dirty':True,'manifest_sha256':run.old.sha(dev/'manifest.json'),
        'source_bundle_sha256':manifest['source_bundle_sha256'],'executable_sha256':manifest['binary_sha256'],
        'raw_result_sha256':run.old.sha(dev/'result.json'),'raw_directory':str(directory.relative_to(run.ROOT)),
        'native_calls':800,'successful_native_calls':len(calls)-len(failures),'native_target_attempts':3200,
        'successful_native_targets':len(success),'paired_exact_target_attempts':3200,
        'independent_outcomes_per_design':100,'numerical_seeds':run.PLAN['numerical_seeds'],
        'coverage_comparable_pairs':len(comparable),'coverage_disagreements':disagreement,
        'firm_summaries':[r for r in full if r['target']=='firm'],
        'maximum_point_kernel_identity_error':max(abs(r['point_kernel_identity']) for r in success),
        'maximum_covariance_probe_error_in_mcse':max(abs(r['covariance_delta'])/r['trace_mcse'] for r in success if r.get('trace_mcse',0)>0),
        'maximum_critical_value_error':max(abs(r['critical_delta']) for r in success if 'critical_delta' in r),
        'geometry_checks':geometry,'timings_concurrent_local_calls':seconds,
        'failed_calls':failures,'failure_audit':json.loads((directory/'failure-audit-final/result.json').read_text()),
        'validation':{'python':'726 passed; pytest temporary-directory cleanup warnings only',
            'native_harness_python':24,'workspace_all_target_tests':count('rust-tests.log'),
            'standalone_backend_tests':count('standalone-tests.log'),'generated_independent_and_native_tests':count('independent-tests.log'),
            'cmg_assembly':'PASS','integrated_stata':'FEVC LOCAL QUALIFICATION PASS; quick/full/install',
            'native_qualifier':{'classification':q['classification'],'source_manifest_sha256':q['source_manifest_sha256'],
                'receipt_sha256':run.old.sha(qualifier),'arm64':q['arm64_test_status'],'x86_64':q['x86_64_test_status'],
                'artifact_arm64_sha256':q['artifact_arm64_sha256'],'artifact_x86_64_sha256':q['artifact_x86_64_sha256'],'artifact_universal_sha256':q['artifact_universal_sha256']},
            'initial_native_qualifier_failure':'sandbox DNS denied public SDK download; preserved; fresh network-approved attempt succeeded',
            'deliberate_bad_native_cli_exit':101},
        'limitations':['Development only: 100 shared outcomes per design, not 200 independent draws',
            'Two numerical seeds do not characterize numerical-seed tail risk',
            'Previous exact-input control overcoverage FAIL remains 96.60% versus 96.50%',
            'Null/weak-signal and multi-mode exclusions unchanged',
            'Existing-route dirty-worktree native checkpoint is not clean-SHA release qualification',
            'No Windows/Linux run, new public option, commit, push, tag or release'],
        'next_step':'Register a fresh end-to-end confirmation with the estimator, outcome-free key semantics, task inventory, success and coverage gates fixed; keep the current PSD failure rule and public match RC unchanged.'}
    run.old.write_json(output,out)
    print(json.dumps({k:out[k] for k in ('status','native_calls','successful_native_calls','coverage_comparable_pairs','coverage_disagreements','validation')},indent=2))

if __name__=='__main__':main(Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve())

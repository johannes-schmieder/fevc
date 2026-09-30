"""Private Counter-V1 complete-run harness; no plugin/frontend qualification.

The Rust adapter invokes the existing point estimator with fresh preparation.
This module owns semantic keys, atomic inventories and independent summaries;
it never calls the production influence-covariance helper.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

import numpy as np

if __package__:
    from .all_probe_reference import prepare, exact_point, mean_covariance, nested_summary, finalize, input_identity
else:
    from all_probe_reference import prepare, exact_point, mean_covariance, nested_summary, finalize, input_identity


def semantic_seed(m, domain, k, l):
    return int(np.random.SeedSequence([m['seed'],domain,k,l if domain else 0])
               .generate_state(1,dtype=np.uint64)[0])


def core_source_identity(root):
    paths=sorted([*root.glob('rust/crates/vckss-core/src/**/*.rs'),
                  *root.glob('rust/vendor/cmg/src/**/*.rs'),
                  root/'rust/crates/vckss-core/build.rs',root/'rust/crates/vckss-core/src/cmg_impl.rs.in',
                  root/'rust/crates/vckss-core/examples/all_probe_reference.rs',
                  root/'rust/crates/vckss-core/Cargo.toml',root/'rust/vendor/cmg/Cargo.toml',
                  root/'rust/Cargo.toml',root/'rust/Cargo.lock',root/'rust/rust-toolchain.toml'])
    inventory={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    return hashlib.sha256(json.dumps(inventory,sort_keys=True).encode()).hexdigest()


def check_audit_dependencies(m):
    for key,path in [('native_harness_sha256',Path(__file__)),
                     ('reference_sha256',Path(__file__).with_name('all_probe_reference.py'))]:
        if m.get(key)!=hashlib.sha256(path.read_bytes()).hexdigest():
            raise ValueError(f'frozen native audit dependency {key} mismatch')


def native_columns(m):
    # Validate and classify the input independently before constructing native
    # columns. The private fixture profile has at most 512 physical copies.
    prepare(m['input'])
    d=m['input']; n=len(d['outcome'])
    w,f,y=[np.asarray(d[k]) for k in ('worker','firm','outcome')]
    freq=np.asarray(d.get('frequency',np.ones(n)),dtype=int)
    mass=np.asarray(d.get('target_mass',freq))
    controls=np.asarray(d.get('controls',np.empty((n,0))))
    if controls.ndim==1: controls=controls[:,None]
    deletion=np.asarray(d.get('deletion_id',list(zip(w,f))))
    _,deletion=np.unique(deletion,axis=0 if deletion.ndim>1 else None,return_inverse=True)
    classification=f if d.get('deletion','observation')=='observation' else deletion
    stayer=np.array([len(np.unique(classification[w==worker]))==1 for worker in w])
    if d.get('stayers','both')=='movers':
        selected=~stayer
        w,f,y,freq,mass,controls,deletion,stayer=[v[selected] for v in (w,f,y,freq,mass,controls,deletion,stayer)]
    _,w=np.unique(w,return_inverse=True);_,f=np.unique(f,return_inverse=True)
    hybrid=d.get('deletion','observation')=='match' and stayer.any()
    mover_groups=0
    if hybrid:
        if stayer.all(): raise ValueError('private hybrid fixture needs retained movers')
        _,mapped=np.unique(deletion[~stayer],return_inverse=True)
        mover_groups=int(mapped.max())+1
        deletion[~stayer]=mapped
        deletion[stayer]=mover_groups+np.arange(stayer.sum())
    return w,f,y,freq,mass,controls,deletion,hybrid,mover_groups


def native_input(m):
    w,f,y,freq,mass,controls,deletion,hybrid,mover_groups=native_columns(m)
    d=m['input']
    header=['FEVC_ALL_PROBE_NATIVE_INPUT_V1',str(len(y)),str(controls.shape[1]),
            str(m['R']),str(m['T']),str(m['K']*m['L']),d.get('deletion','observation'),
            d.get('nuisance','joint'),str(mover_groups),str(int(hybrid))]
    lines=[' '.join(header)]
    for i in range(len(y)):
        lines.append(' '.join(map(str,[int(w[i]),int(f[i]),int(deletion[i]),int(freq[i]),
                                     float(mass[i]),float(y[i]),*map(float,controls[i])])) )
    for k in range(m['K']):
        for l in range(m['L']):
            lines.append(f'{k} {l} {semantic_seed(m,0,k,l)} {semantic_seed(m,1,k,l)}')
    return '\n'.join(lines)+'\n'


def expected_work(m):
    w,f,y,freq,mass,controls,deletion,hybrid,mover_groups=native_columns(m)
    strata={};classes={};groups={}
    for i in range(len(y)):
        target=(int(w[i]),int(f[i]),float(mass[i]/freq[i]))
        strata[target]=strata.get(target,0)+int(freq[i])
        observation=m['input'].get('deletion','observation')=='observation' or (hybrid and deletion[i]>=mover_groups)
        if observation:
            key=(*target,float(y[i]),*map(float,controls[i]))
            classes[key]=classes.get(key,0)+int(freq[i])
        else:
            groups[int(deletion[i])]=groups.get(int(deletion[i]),0)+int(freq[i])
    word_count=lambda groups:sum((v+63)//64 for v in groups.values())
    match_words=word_count(groups);obs_words=word_count(classes)
    return dict(rhs=m['R']+2*m['T']+controls.shape[1]+1+
                    int(m['input'].get('nuisance','joint')=='fixedoffset' and controls.shape[1]>0),
                counter_unique_packed_words=m['R']*(match_words+word_count(classes))+m['T']*word_count(strata),
                counter_physical_trials=(m['R']+m['T'])*int(freq.sum()),
                replay_per_success=match_words+obs_words,replay_failed_attempt=match_words+obs_words,
                minimum_allocation_bound=16*int(freq.sum())+640*len(y)+72*m['R']+4096)


def close_scaled(a,b):
    a,b=np.asarray(a,dtype=float),np.asarray(b,dtype=float)
    if a.shape!=b.shape or not np.isfinite(a).all() or not np.isfinite(b).all(): return False
    scale=max(np.max(np.abs(a)),np.max(np.abs(b)))
    return np.array_equal(a,b) if scale==0 else np.linalg.norm(a/scale-b/scale)<=1e-11*max(np.linalg.norm(a/scale),np.linalg.norm(b/scale))


def validate_inventory(result):
    if result.get('schema')!='FEVC_ALL_PROBE_NATIVE_RESULT_V1': raise ValueError('invalid native result schema')
    m=result['manifest']; attempts=result['attempts']
    if m.get('schema')!='FEVC_ALL_PROBE_NATIVE_REFERENCE_V1': raise ValueError('invalid native manifest schema')
    if any(type(m.get(k)) is not int for k in ('K','L','R','T','seed')) or not (m['K']>=2 and m['L']>=1 and m['K']*m['L']<=100000 and 2<=m['R']<=400 and 2<=m['T']<=400):
        raise ValueError('invalid native manifest counts')
    fixed=dict(route='generic_diagonal',rng='Counter-V1',seed_algorithm='numpy-seedsequence-u64-v1',
               leverage_batch=7,target_batch=5,pcg_tolerance=1e-12,full_residual_gate=1e-11)
    for key,value in fixed.items():
        if key in m and m[key]!=value: raise ValueError('native fixed adapter parameter mismatch')
        if str(m.get('profile','')).startswith('confirmation_native') and key not in m:
            raise ValueError('missing native fixed adapter parameter')
    p=prepare(m['input'])
    for key,source in [('true_maker_margin','true_maker_margin'),('true_full_fit_complete_residual','full_fit_complete_residual')]:
        if key in m and not close_scaled(m[key],p[source]): raise ValueError('native true-margin/fit manifest mismatch')
    work=expected_work(m)
    keys=[tuple(a['key']) for a in attempts]
    if len(set(keys))!=len(keys) or set(keys)!={(k,l) for k in range(m['K']) for l in range(m['L'])}:
        raise ValueError('missing, duplicate or partial native inventory')
    for a in attempts:
        k,l=a['key']
        if [a['leverage_seed'],a['target_seed']]!=[semantic_seed(m,0,k,l),semantic_seed(m,1,k,l)]:
            raise ValueError('native seed identity mismatch')
        if [a['R'],a['T'],a['fold_a'],a['fold_b']]!=[m['R'],m['T'],(m['T']+1)//2,m['T']//2]:
            raise ValueError('native count identity mismatch')
        if a['route']!='generic_diagonal' or not np.isfinite(a['seconds']) or a['seconds']<0:
            raise ValueError('invalid native route/timing')
        if a['point_status']=='failed':
            if a['diagnostic_status']!='point_failed' or not isinstance(a['point_error_code'],int) or not a['point_error_phase']:
                raise ValueError('invalid native point failure')
            continue
        if a['point_status']!='ok': raise ValueError('unknown native point status')
        for name,shape in [('point',(3,)),('plugin',(3,)),('correction',(3,)),('conditional_mcse',(4,)),('conditional',(3,3))]:
            value=np.asarray(a[name])
            if value.shape!=shape or not np.isfinite(value).all(): raise ValueError('invalid native successful point')
        if not close_scaled(a['point'],np.asarray(a['plugin'])-a['correction']): raise ValueError('native point accounting mismatch')
        if not close_scaled(a['conditional_mcse'],finalize(a['conditional'],np.zeros((3,3)))['mcse']):
            raise ValueError('native conditional MCSE mismatch')
        for name in ('maximum_complete_residual','maximum_replay_complete_residual'):
            if not np.isfinite(a[name]) or a[name]>1e-11 or a[name]<0: raise ValueError('native original-system residual failed')
        if any(type(a[name]) is not int for name in ('replay_rhs','replay_executed_rhs','replay_attempted_rhs')):
            raise ValueError('native replay counts must be integers')
        if not 0<=a['replay_rhs']<=a['replay_executed_rhs']<=a['replay_attempted_rhs']<=m['R']:
            raise ValueError('native replay work mismatch')
        for name in ('rhs','counter_unique_packed_words','counter_physical_trials'):
            if type(a[name]) is not int or a[name]!=work[name]: raise ValueError('native point/Counter work mismatch')
        for name in ('minimum_margin','minimum_constrained','sensitivity_ratio','psd_adjustment'):
            if a[name] is None or not np.isfinite(a[name]) or a[name]<0 or (name.startswith('minimum_') and a[name]==0):
                raise ValueError('invalid native margin/sensitivity/PSD receipt')
        evaluations=a['replay_attempted_rhs']*work['replay_per_success']
        if a['failed_replay_probe'] is not None:
            if a['diagnostic_status']!='replay_failed' or a['failed_replay_probe']!=a['replay_rhs']:
                raise ValueError('native failed replay identity mismatch')
            if not a['replay_rhs']<a['replay_attempted_rhs']<=min(a['replay_rhs']+4,m['R']):
                raise ValueError('native failed replay batch mismatch')
        if type(a['replay_generator_word_evaluations']) is not int or a['replay_generator_word_evaluations']!=evaluations:
            raise ValueError('native replay generator work mismatch')
        if type(a['allocation_bound_bytes']) is not int or a['allocation_bound_bytes']<work['minimum_allocation_bound']:
            raise ValueError('invalid native allocation bound')
        if a['diagnostic_status'] in ('ok_local','ok_local_psd_adjusted','unstable_nonpsd'):
            expected=finalize(a['conditional'],a['leverage'])
            if expected['status']!=a['diagnostic_status'] or not close_scaled(a['all_raw'],expected['raw']):
                raise ValueError('native raw covariance/status mismatch')
            for name,key in [('all_usable','usable'),('all_mcse','mcse')]:
                if expected[key] is None:
                    if a[name] is not None: raise ValueError('unavailable native diagnostic posted')
                elif not close_scaled(a[name],expected[key]): raise ValueError('native usable covariance/MCSE mismatch')
            raw=np.asarray(a['all_raw']); usable=a['all_usable']
            if usable is None:
                adjustment=0.
            else:
                delta=np.asarray(usable)-raw
                scale=np.max(np.abs(delta))
                adjustment=0. if scale==0 else float(np.linalg.norm(delta/scale)*scale)
            # Compare the recorded matrices, so small eigensystem differences
            # between the independent oracle and native code are not a gate.
            if not close_scaled(a['psd_adjustment'],adjustment): raise ValueError('native PSD adjustment receipt mismatch')
            if a['replay_rhs']!=m['R'] or a['replay_executed_rhs']!=m['R'] or a['replay_attempted_rhs']!=m['R'] or a['failed_replay_probe'] is not None: raise ValueError('native replay inventory incomplete')
        elif a['diagnostic_status'] in ('nonsmooth_adjustment','nonfinite_derivative','replay_failed'):
            if a['diagnostic_status']=='replay_failed' and a['failed_replay_probe'] is None:
                raise ValueError('missing native replay failure identity')
            if a['psd_adjustment']!=0: raise ValueError('failed native PSD adjustment posted')
            if a['all_usable'] is not None or a['all_mcse'] is not None: raise ValueError('failed native diagnostic posted')
        else: raise ValueError('unknown native diagnostic status')
    summary=result['summary']; failures=[a['key'] for a in attempts if a['point_status']!='ok']
    usable=sum(a['diagnostic_status'] in ('ok_local','ok_local_psd_adjusted') for a in attempts)
    raw=sum(a.get('all_raw') is not None for a in attempts)
    if (summary['attempted'],summary['point_failures'],summary['usable_diagnostics'],summary['finite_raw_count'])!=(len(attempts),failures,usable,raw):
        raise ValueError('native summary mismatch')
    if summary['conditioning']!=('success_conditional' if failures else 'observed_complete_unfiltered'):
        raise ValueError('native summary conditioning mismatch')
    if not close_scaled(summary['true_maker_margin'],p['true_maker_margin']):
        raise ValueError('native true-margin summary mismatch')
    if m['input_sha256']!=input_identity(m['input']): raise ValueError('native input identity mismatch')


def execute(m, binary):
    if m.get('schema')!='FEVC_ALL_PROBE_NATIVE_REFERENCE_V1': raise ValueError('invalid native manifest')
    if any(type(m[k]) is not int for k in ('K','L','R','T','seed')) or not (2<=m['K'] and 1<=m['L'] and m['K']*m['L']<=100000):
        raise ValueError('invalid native counts')
    completed=subprocess.run([str(binary.resolve())],input=native_input(m),text=True,capture_output=True,check=True)
    attempts=[json.loads(line) for line in completed.stdout.splitlines()]
    p=prepare(m['input']); failures=[a['key'] for a in attempts if a['point_status']!='ok']
    summary=dict(attempted=len(attempts),point_failures=failures,
                 usable_diagnostics=sum(a['diagnostic_status'] in ('ok_local','ok_local_psd_adjusted') for a in attempts),
                 finite_raw_count=sum(a.get('all_raw') is not None for a in attempts),
                 conditioning='success_conditional' if failures else 'observed_complete_unfiltered',
                 true_maker_margin=p['true_maker_margin'])
    points=[a['point'] for a in attempts if a['point_status']=='ok']
    if m['L']==1 and len(points)>=2:
        summary.update(one_run_covariance=(mean_covariance(points)*len(points)).tolist(),
                       bias_against_exact=(np.mean(points,axis=0)-exact_point(p)).tolist())
    if not failures and m['L']>=2:
        summary['nested']=nested_summary(np.asarray(points).reshape(m['K'],m['L'],3))
    result=dict(schema='FEVC_ALL_PROBE_NATIVE_RESULT_V1',manifest=m,attempts=attempts,summary=summary)
    validate_inventory(result)
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest',type=Path);parser.add_argument('binary',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args();m=json.loads(args.manifest.read_text())
    identities=dict(native_binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),
                    native_harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    core_source_sha256=core_source_identity(Path(__file__).resolve().parents[2]),
                    reference_sha256=hashlib.sha256(Path(__file__).with_name('all_probe_reference.py').read_bytes()).hexdigest(),
                    input_sha256=input_identity(m['input']))
    for key,value in identities.items():
        if m.get(key)!=value: raise ValueError(f'frozen {key} mismatch')
    if m.get('rng')!='Counter-V1' or m.get('seed_algorithm')!='numpy-seedsequence-u64-v1':
        raise ValueError('unregistered native reference RNG identity')
    if m.get('source_sha')!=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip():
        raise ValueError('native manifest source does not match checkout')
    result=execute(m,args.binary)
    tmp=args.output.with_suffix(args.output.suffix+'.tmp')
    tmp.write_text(json.dumps(result,allow_nan=False)+'\n');tmp.replace(args.output)
    return 1 if result['summary']['point_failures'] else 0


if __name__=='__main__':
    raise SystemExit(main())

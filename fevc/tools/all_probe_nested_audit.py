"""Independent nested stage references and delete-one-leverage uncertainty."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np

if __package__:
    from .all_probe_reference import validate_inventory
else:
    from all_probe_reference import validate_inventory


def _scaled_estimates(points):
    points=np.asarray(points,dtype=float)
    if points.ndim!=3 or points.shape[2]!=3 or points.shape[0]<3 or points.shape[1]<2 or not np.isfinite(points).all():
        raise ValueError("nested uncertainty needs a complete finite K by L by 3 inventory")
    k,l,_=points.shape
    scale=np.max(np.abs(points))
    if scale==0: scale=1.
    points=points/scale
    means=points.mean(axis=1)
    centered=points-means[:,None,:]
    wk=np.einsum('kli,klj->kij',centered,centered)/(l-1)
    w=wk.mean(axis=0)
    delta=means-means.mean(axis=0)
    b=delta.T@delta/(k-1)
    values=dict(target=w,leverage=b-w/l,all=b+(1-1/l)*w)
    # Stable sample-covariance removal avoids subtracting two uncentered
    # second moments when the fixed point level is much larger than dispersion.
    leave_b=((k-1)*b[None,:,:] - (k/(k-1))*np.einsum('ki,kj->kij',delta,delta))/(k-2)
    leave_w=(k*w[None,:,:]-wk)/(k-1)
    leave=dict(target=leave_w,leverage=leave_b-leave_w/l,all=leave_b+(1-1/l)*leave_w)
    return values, leave, scale


def estimates(points):
    values,leave,scale=_scaled_estimates(points)
    k=len(points)
    answer={}
    for name,value in values.items():
        jackknife=leave[name]-leave[name].mean(axis=0)
        se=np.sqrt((k-1)/k*np.sum(jackknife*jackknife,axis=0))
        answer[name]=((value*scale)*scale).tolist()
        answer[name+'_standard_error']=((se*scale)*scale).tolist()
    return answer


def comparisons(points, diagnostics):
    """Paired delete-one-sketch uncertainty includes reference/diagnostic covariance.

    The signed leverage reference has no positivity or ratio acceptance gate.
    Every target batch in each retained sketch contributes to the comparison.
    """
    values,leave,scale=_scaled_estimates(points)
    k,l,_=np.asarray(points).shape
    answer={}
    for name in ('target','leverage','all'):
        diagnostic=np.asarray(diagnostics[name],dtype=float)
        if diagnostic.shape!=(k,l,3,3) or not np.isfinite(diagnostic).all():
            raise ValueError('nested comparison needs every finite raw diagnostic')
        sketch_mean=((diagnostic/scale)/scale).mean(axis=1)
        mean=sketch_mean.mean(axis=0)
        difference=mean-values[name]
        leave_mean=(k*mean[None,:,:]-sketch_mean)/(k-1)
        leave_difference=leave_mean-leave[name]
        delta=leave_difference-leave_difference.mean(axis=0)
        se=np.sqrt((k-1)/k*np.sum(delta*delta,axis=0))
        answer[name]=dict(diagnostic_mean=((mean*scale)*scale).tolist(),
                          mean_difference=((difference*scale)*scale).tolist(),
                          difference_standard_error=((se*scale)*scale).tolist())
    return answer


def audit(result):
    native=result.get('schema')=='FEVC_ALL_PROBE_NATIVE_RESULT_V1'
    if native:
        if __package__:
            from .all_probe_native import validate_inventory as native_validate, check_audit_dependencies
        else:
            from all_probe_native import validate_inventory as native_validate, check_audit_dependencies
        check_audit_dependencies(result['manifest'])
        native_validate(result)
    else:
        validate_inventory(result)
    m=result['manifest']
    profile='confirmation_native_nested_v1' if native else 'confirmation_dense_nested_v1'
    if m.get('profile')!=profile or m['K']<128 or m['L']<8:
        raise ValueError('unregistered nested confirmation counts/profile')
    if m.get('nested_audit_sha256')!=hashlib.sha256(Path(__file__).read_bytes()).hexdigest():
        raise ValueError('frozen nested audit source mismatch')
    failures=result['summary']['point_failures']
    answer=dict(schema='FEVC_ALL_PROBE_NESTED_AUDIT_V1',manifest=m,
                implementation='rust_core_counter_v1' if native else 'independent_dense_python_only',attempted=m['K']*m['L'],
                point_failures=failures,conditioning=result['summary']['conditioning'],
                reference_uncertainty='delete_one_leverage_jackknife',
                usable_diagnostics=result['summary']['usable_diagnostics'])
    if failures:
        answer.update(status='incomplete',estimates=None)
    else:
        attempts=sorted(result['attempts'],key=lambda a:a['key'])
        points=np.array([a['point'] for a in attempts]).reshape(m['K'],m['L'],3)
        answer.update(status='observed_complete',estimates=estimates(points))
        if result['summary']['finite_raw_count']!=m['K']*m['L']:
            answer.update(status='incomplete_diagnostics',comparisons=None)
        else:
            diagnostics={name:np.array([a[field] for a in attempts]).reshape(m['K'],m['L'],3,3)
                         for name,field in [('target','conditional'),('leverage','leverage'),('all','all_raw')]}
            answer['comparisons']=comparisons(points,diagnostics)
    return answer


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('result',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args()
    answer=audit(json.loads(args.result.read_text()))
    tmp=args.output.with_suffix(args.output.suffix+'.tmp')
    tmp.write_text(json.dumps(answer,indent=2,allow_nan=False)+'\n');tmp.replace(args.output)
    return 0 if answer['status']=='observed_complete' else 1


if __name__=='__main__':
    raise SystemExit(main())

"""Independent dense diagnosis of the one frozen native PSD failure."""
from pathlib import Path
import argparse
import importlib.util
import json
import subprocess
import numpy as np

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('builder',HERE/'build.py')
builder=importlib.util.module_from_spec(spec);spec.loader.exec_module(builder)

def main(build,out):
    out.mkdir(parents=True,exist_ok=False)
    generated=(build/'integration.rs').read_text()
    assert generated.count('fn main()')==1
    source=out/'audit.rs'
    source.write_text(generated.replace('fn main()','fn integrated_main()',1)+'\n'+(HERE/'audit.rs').read_text())
    libraries=list((builder.base.ROOT/'rust/target/release/deps').glob('libvckss_core-*.rlib'));assert len(libraries)==1
    subprocess.run([str(builder.base.TOOLCHAIN/'rustc'),'--edition=2021','-O','-Awarnings',str(source),'--extern',f'vckss_core={libraries[0]}','-L',f'dependency={libraries[0].parent}','-o',str(out/'audit')],check=True)
    with (out/'export.json').open('x') as f:subprocess.run([str(out/'audit')],stdout=f,check=True)
    a=json.loads((out/'export.json').read_text());n=a['n'];p=a['p'];terms=a['terms']
    x=np.array(a['x']).reshape(n,p);inv=np.array(a['inverse']).reshape(p,p);hat=x@inv@x.T
    z=np.array(a['z']).reshape(n,terms);gram=np.array(a['gram']).reshape(terms,terms)
    def fitted(y):
        e=y-hat@y;raw=z@np.linalg.solve(gram,z.T@(e*e));floor=1e-8*np.median(e*e/(1-np.array(a['h'])))
        return np.maximum(raw,floor),int(np.sum(raw<floor))
    f0,_=fitted(np.array(a['y0']));error=float(np.max(np.abs(f0-a['fit0'])))
    assert error<1e-8,error
    variance,floored=fitted(np.array(a['y75']));y=np.array(a['y75'])
    ratio=np.array(a['b'])*np.array(a['maker'])
    factors=np.array(a['factors']).reshape(4,2*p,2*p)
    def kernels(ratios):
        result=[]
        for t in range(3):
            v=np.column_stack((x,ratios[t,:,None]*x));result.append(v@factors[t]@v.T-np.diag(ratios[t]))
        return np.array(result)
    c=kernels(ratio);exact_c=kernels(np.array(a['exact_ratios'])[:3])
    def covariance(c,s):
        u=c@y;weighted=c*np.sqrt(s[:,None]*s[None,:])
        trace=2*np.einsum('tij,sij->ts',weighted,weighted)
        influence=4*(u*s)@u.T
        return influence-trace,influence,trace
    exact,influence,trace=covariance(c,variance)
    g=np.array(a['gaussian']).reshape(1000,n)*np.sqrt(variance)
    q=np.array([np.sum((g@ct)*g,axis=1) for ct in c])
    noisy=influence-np.cov(q,ddof=1)
    oracle,_,_=covariance(c,np.array(a['variance_true']))
    exactkernel,_,_=covariance(exact_c,variance)
    values={'status':'post_run_diagnostic_not_replacement','cell':'dominant_common','k':16,'numseed':791503,'replication':75,
        'fit_reconstruction_max_error_rep0':error,'floored_predictions_rep75':floored,
        'exact_native_kernel_covariance':exact.tolist(),'finite_1000_probe_covariance':noisy.tolist(),
        'exact_native_kernel_eigenvalues':np.linalg.eigvalsh(exact).tolist(),
        'finite_1000_probe_eigenvalues':np.linalg.eigvalsh(noisy).tolist(),
        'exact_kernel_eigenvalues':np.linalg.eigvalsh(exactkernel).tolist(),
        'true_variance_exact_trace_eigenvalues':np.linalg.eigvalsh(oracle).tolist(),
        'relative_trace_error':float(np.linalg.norm(noisy-exact)/np.linalg.norm(exact)),
        'retained_native_failure':'JLA_CONSTRAINT_FAILED:component_inference_psd'}
    with (out/'result.json').open('x') as f:json.dump(values,f,indent=2,allow_nan=False)
    print(json.dumps(values,indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('build',type=Path);p.add_argument('out',type=Path);a=p.parse_args();main(a.build.resolve(),a.out.resolve())

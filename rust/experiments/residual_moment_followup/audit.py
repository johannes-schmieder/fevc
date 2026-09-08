"""Independent structural checks and bounded supplementary timing outputs."""
from pathlib import Path
import argparse
import json
import numpy as np
import run

def midranks(a):
    a=np.asarray(a);order=np.argsort(a,kind='stable');result=np.empty(len(a));begin=0
    while begin<len(a):
        end=begin+1
        while end<len(a) and a[order[end]]==a[order[begin]]:end+=1
        result[order[begin:end]]=2*((begin+end)/2)/len(a)-1
        begin=end
    return result

def structural(exe,out):
    out.mkdir(parents=True,exist_ok=True);results=[]
    for cell in run.PLAN['same_design']['cells']:
        for k in run.PLAN['same_design']['k']:
            d=run.capture(exe,['collapse',cell,k],out/f'{cell}-{k}.jsonl')[0]
            ranks=np.stack([midranks(v) for v in (d['h'],*d['b'],d['mass'])],axis=1)
            if cell.endswith('leverage'):ranks=ranks[:,:1]
            z=np.stack([np.ones(d['groups']),*ranks.T,*(ranks[:,i]*ranks[:,j] for i in range(ranks.shape[1]) for j in range(i,ranks.shape[1]))],axis=1)
            s=np.asarray(d['variance']);beta,_,rank,_=np.linalg.lstsq(z,s,rcond=1e-10)
            residual=s-z@beta
            r={q:d[q] for q in ('cell','k','n','groups','support_status','support_detail')}
            r.update(terms=z.shape[1],basis_rank=int(rank),true_variance_relative_projection_error=float(np.linalg.norm(residual)/np.linalg.norm(s)),
                     true_variance_maximum_projection_error=float(np.max(np.abs(residual))),input_sha256=run.sha(out/f'{cell}-{k}.jsonl'))
            results.append(r);print(json.dumps(r),flush=True)
    run.write_json(out/'result.json',results)

def scaling(exe,out):
    out.mkdir(parents=True,exist_ok=True);results=[]
    for cell in ('dominant_common','dominant_common_controls'):
        g=run.geometry(exe,out,cell,128,800,610731)
        r=run.capture(exe,['scale',cell,128,610731],out/f'fit-{cell}.jsonl')[0]
        results.append({'geometry':g,'candidate':r});print(json.dumps(r),flush=True)
    run.write_json(out/'result.json',results)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('stage',choices=('structural','scaling'));p.add_argument('output');p.add_argument('--executable',required=True);a=p.parse_args()
    globals()[a.stage](Path(a.executable).resolve(),Path(a.output).resolve())

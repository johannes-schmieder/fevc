"""Independent raw-output accounting; never reclassifies a scientific gate."""
from pathlib import Path
import argparse
import collections
import gzip
import hashlib
import json
import math
import statistics

def sha(path):
    with path.open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()

def audit(directory):
    manifest=json.loads((directory/'manifest.json').read_text())
    result=json.loads((directory/'result.json').read_text())
    assert result['manifest_sha256']==sha(directory/'manifest.json')
    assert manifest['bundle_sha256']==sha(directory/'source.tar.gz')
    tasks=manifest['tasks'];assert len(tasks)==len({t['id']for t in tasks})
    assert {p.name for p in(directory/'tasks').iterdir()}=={t['id']for t in tasks}
    summaries={(r['family'],r['cell'],r['k'],r['target']):r for r in result['summaries']}
    counters=collections.Counter();groups=collections.defaultdict(list);keys=set();joint=collections.defaultdict(collections.Counter)
    shared=collections.Counter()
    for task in tasks:
        folder=directory/'tasks'/task['id'];receipt=json.loads((folder/'receipt.json').read_text());path=folder/'rows.jsonl.gz'
        assert receipt['task']==task and receipt['output_sha256']==sha(path)
        assert receipt['binary_sha256']==manifest['binaries'][task['family']]
        assert set(p.name for p in folder.iterdir())=={'receipt.json','stderr.log','rows.jsonl.gz'}
        rows=[json.loads(line)for line in gzip.decompress(path.read_bytes()).decode().splitlines()]
        assert len(rows)==task['reps']+2
        design=rows[1];calls=rows[2:]
        assert len(calls)==receipt['calls']==task['reps']and receipt['targets']==4*len(calls)
        spec=next(c for c in manifest['cells']if(c['family'],c['cell'],c['k'])==(task['family'],task['cell'],task['k']))
        q1=spec.get('reference',spec.get('reference_distribution','')).lower()=='q1'
        for call in calls:
            key=(task['family'],task['cell'],task['k'],call['replication'])
            assert key not in keys;keys.add(key)
            assert task['start']<=call['replication']<task['start']+task['reps']
            counters['native_calls']+=1
            ok=call['status']=='success'
            counters['returned_calls'if ok else'shared_failures']+=1
            if not ok:shared[(task['family'],task['cell'],call['detail'])]+=1
            if ok:
                joint[key[:3]][str(call['joint'])]+=1
                if call['joint']!=0:
                    assert call['covariance']==[None]*16 and call['primitive']==[None]*9
                    counters['invalid_joint_calls']+=1
                    counters['computed_targets_from_invalid_joint_calls']+=call['computed']
            for i,target in enumerate(('worker','firm','covariance','total')):
                counters['target_attempts']+=1
                row={'status':'shared_failure','error':None,'covered':False,'se':None,'mcse':None}
                if ok:
                    v,status=call['targets'][2*i:2*i+2]
                    point=call['point'][i];row.update(error=point-design['truth'][i],mcse=call['point_mcse'][i])
                    if q1:status=call['q1'][20*i+16];lo,hi=call['q1'][20*i+10:20*i+12]
                    elif status==0:lo,hi=point-1.959963984540054*math.sqrt(v),point+1.959963984540054*math.sqrt(v)
                    if status==0:
                        assert math.isfinite(lo)and math.isfinite(hi)and lo<=hi
                        row.update(status='computed',covered=lo<=design['truth'][i]<=hi,se=math.sqrt(v)if v>0 else None)
                    else:row['status']=('q1_'if q1 else'q0_')+str(int(status))
                counters[row['status']]+=1;groups[key[:3]+(target,)].append(row)
    assert len(groups)==192 and counters['native_calls']==19200 and counters['target_attempts']==76800
    audited=[]
    for key,rows in groups.items():
        assert len(rows)==400
        summary=summaries[key];points=[r for r in rows if r['error']is not None];good=[r for r in rows if r['status']=='computed']
        se=[r['se']for r in good if r['se']is not None]
        empirical=statistics.stdev(r['error']for r in points)if len(points)>1 else None
        values={'attempts':400,'point_estimates':len(points),'successes':len(good),'success_rate':len(good)/400,
                'bias':statistics.fmean(r['error']for r in points)if points else None,'empirical_sd':empirical,
                'coverage':statistics.fmean(r['covered']for r in good)if good else None,'coverage_among_all_attempts':sum(r['covered']for r in good)/400,
                'mean_se':statistics.fmean(se)if se else None,'se_denominator':len(se)}
        for name,value in values.items():
            expected=summary[name]
            assert (value is None and expected is None)or(value is not None and expected is not None and math.isclose(value,expected,rel_tol=1e-12,abs_tol=1e-14)),(key,name,value,expected)
        audited.append({'family':key[0],'cell':key[1],'k':key[2],'target':key[3],**values,
                        'point_mcse_rms_over_empirical_sd':math.sqrt(statistics.fmean(r['mcse']**2 for r in points))/empirical if empirical else None,
                        'failures':dict(collections.Counter(r['status']for r in rows if r['status']!='computed'))})
    return {'schema':'FEVC-INDIVIDUAL-DEVELOPMENT-INDEPENDENT-AUDIT-V1','accounting_status':'PASS','scientific_status':result['status'],
            'manifest_sha256':sha(directory/'manifest.json'),'result_sha256':sha(directory/'result.json'),'auditor_sha256':sha(Path(__file__)),
            'counts':dict(counters),'shared_failures':[{'family':k[0],'cell':k[1],'detail':k[2],'count':n}for k,n in shared.items()],
            'joint_statuses':[{'family':k[0],'cell':k[1],'k':k[2],'counts':dict(v)}for k,v in joint.items()],
            'scientific_reports':result['reports'],'cells':audited}

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
    result=audit(a.directory)
    with a.output.open('x')as f:json.dump(result,f,indent=2,allow_nan=False)
    print(json.dumps({k:v for k,v in result.items()if k in('accounting_status','scientific_status','counts')}))

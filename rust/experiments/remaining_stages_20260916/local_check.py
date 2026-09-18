"""Tiny generator-to-native-runner-to-validator check; no campaign timings."""
from __future__ import annotations
import argparse
import csv
import importlib.util
import os
from pathlib import Path
import sys
import time
import remaining as r


def main(output, source, plugin, baseline, stata):
    output.mkdir(parents=True,exist_ok=False)
    support_path=source/'source/rust/experiments/pipeline_optimization_20260915'
    paper=Path(__file__).parents[3].parent/'fevc-paper/replication/benchmarks/light_refresh_20260914/run.py'
    plan=dict(support=str(support_path),paper_support=str(paper),packages={},inputs={},stages={'smoke':[]})
    modules=r.support(plan)
    from thread_adapter import build
    from screen_input import build as generate_pooled
    for variant,snapshot,native in [('candidate',source/'snapshot.json',plugin),
        ('baseline',baseline/'snapshot.json',baseline/'fevc_rust_macos_arm64.plugin')]:
        package=output/'packages'/variant
        receipt=build(snapshot,r.sha(snapshot),native,r.sha(native),package,variant)
        plan['packages'][variant]=dict(path=str(package),receipt=receipt,adapter_sha256=r.sha(package/'adapter.json'))
    generator_path=Path(__file__).parents[1]/'optimization_parity_20260913/generate.py'
    spec=importlib.util.spec_from_file_location('remaining_local_generator',generator_path)
    generator=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(generator)
    (output/'inputs').mkdir()
    public=output/'inputs/public.csv'
    meta=generator.generate(public,8000,'degree4_bottleneck')
    pooled=output/'inputs/pooled-original.csv'
    generate_pooled(pooled,'pooled_stayer_match',True)
    private_fixture=output/'inputs/private-schema.csv'
    counts={}
    rows=0
    with pooled.open() as src,private_fixture.open('x') as dst:
        reader=csv.DictReader(src)
        writer=csv.writer(dst)
        writer.writerow(['year','worker','firm','observation_key','y_adjusted'])
        for row in reader:
            worker=row['worker']
            counts[worker]=counts.get(worker,0)+1
            writer.writerow([counts[worker],worker,row['firm'],row['observation_key'],row['y']])
            rows+=1
    r.put(private_fixture.with_suffix('.json'),dict(rows=rows,sha256=r.sha(private_fixture)))
    for index,(graph,path) in enumerate([('degree4_bottleneck',public),('private_veneto',private_fixture)]):
        meta=r.read(path.with_suffix('.json'))
        plan['inputs'][graph]=dict(path=str(path),metadata=meta,sha256=r.sha(path),metadata_sha256=r.sha(path.with_suffix('.json')))
        for deletion in ('observation','match'):
            task=dict(id=len(plan['stages']['smoke'])+1,calls=[])
            for variant in ('baseline','candidate'):
                call=dict(id=f'local-{index}-{deletion}-{variant}',stage='smoke',cell=task['id']-1,
                    graph=graph,profile='remaining_'+deletion,deletion=deletion,rows=meta['rows'],threads=4,
                    seed=104729,repetition=1,warmup=False,variant=variant,input=graph,input_sha256=r.sha(path),
                    adapter_sha256=plan['packages'][variant]['adapter_sha256'])
                call['command']=r.request(call,modules[1])
                task['calls'].append(call)
            plan['stages']['smoke'].append(task)
    (output/'smoke/tasks').mkdir(parents=True)
    r.put(output/'plan.json',plan)
    deadline=time.time()+600
    for task in plan['stages']['smoke']:
        if not r.run_task(output,plan,modules,'smoke',task['id'],stata,deadline,False):
            raise RuntimeError('local native smoke failed')
    report=r.validate(output,plan,modules,'smoke')
    r.put(output/'report.json',report)
    if report['status']!='PASS': raise ValueError(report['errors'])
    call=plan['stages']['smoke'][0]['calls'][1]
    failed=r.run_call(output,plan,modules,call,output/'deliberate',4,deadline,stata,True)
    if failed['status']!='EXPECTED_FAILURE': raise ValueError('deliberate failure not observed')
    print('LOCAL REMAINING WORKFLOW PASS: eight native calls, public/private-schema, both deletions, deliberate failure')


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('output',type=Path)
    p.add_argument('--source',type=Path,required=True)
    p.add_argument('--plugin',type=Path,required=True)
    p.add_argument('--baseline',type=Path,required=True)
    p.add_argument('--stata',required=True)
    a=p.parse_args()
    main(a.output,a.source,a.plugin,a.baseline,a.stata)

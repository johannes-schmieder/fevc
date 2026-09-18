#!/usr/bin/env python3
"""Generate each public supported-path fixture once, before consumers start."""
from __future__ import annotations
import argparse
import importlib.util
import json
from pathlib import Path
import numpy as np
from campaign import PROFILES

GRAPH_SOURCE = Path(__file__).parents[1]/'optimization_parity_20260913/generate.py'
spec = importlib.util.spec_from_file_location('pipeline_graph_generator', GRAPH_SOURCE)
graph = importlib.util.module_from_spec(spec)
spec.loader.exec_module(graph)


def build(output: Path, profile: str, smoke: bool = False) -> dict:
    parameters = PROFILES[profile]
    rows = min(parameters['rows'],8000) if smoke else parameters['rows']
    fixture = parameters['fixture']
    workers = rows//20
    firms = 20 if rows<=8000 else fixture['firms']
    repeats = fixture['rows_per_match']
    if output.exists() or output.with_suffix('.json').exists():
        raise ValueError('input is write-once')
    degree_half = graph.half_degrees(workers//2,'mixed')
    rng_graph = np.random.default_rng(np.random.SeedSequence([fixture['graph_seed'],rows,repeats,31]))
    rng_graph.shuffle(degree_half)
    degrees = np.tile(degree_half,2)
    offsets = graph.offsets_for(degrees)*repeats
    edges = rows//repeats
    if edges%firms:
        raise ValueError('nonintegral firm-degree inventory')
    assignment = graph.distinct_ragged(np.repeat(np.arange(firms,dtype=np.int64),edges//firms),degrees,rng_graph)
    worker = np.repeat(np.arange(workers,dtype=np.int64),degrees*repeats)
    firm = np.repeat(assignment,repeats)
    converted = int(workers*fixture['stayer_fraction'])
    for index in range(converted):
        firm[offsets[index]:offsets[index+1]] = firm[offsets[index]]
    observation = np.arange(rows,dtype=np.int64)
    period = observation-offsets[worker]+1
    match = (worker+1)*1_000_000+firm+1
    rng = np.random.default_rng(np.random.SeedSequence([fixture['outcome_seed'],rows,31]))
    control_1,control_2 = rng.normal(size=rows),rng.normal(size=rows)
    projection = rng.normal(size=rows)+.15*control_1
    frequency = 1+observation%3
    target = .75+(observation%7)/10.
    outcome = rng.normal(size=workers)[worker]+.6*rng.normal(size=firms)[firm]+.3*control_1-.2*control_2+rng.normal(size=rows)
    realized = np.array([np.unique(firm[offsets[i]:offsets[i+1]]).size for i in range(workers)])
    if len(worker)!=rows or int(sum(realized==1))!=converted:
        raise ValueError('fixture population mismatch')
    if set(realized[converted:]) != set(range(2,9)):
        raise ValueError('fixture does not realize degrees 2–8')
    output.parent.mkdir(parents=True,exist_ok=True)
    temporary = output.with_suffix(output.suffix+'.tmp')
    with temporary.open('x') as stream:
        stream.write('observation_key,worker,firm,period,match,y,frequency,target_weight,control_1,control_2,projection\n')
        for start in range(0,rows,100_000):
            section = slice(start,min(rows,start+100_000))
            columns = [observation+1,worker+1,firm+1,period,match,outcome,frequency,target,control_1,control_2,projection]
            np.savetxt(stream,np.column_stack([column[section] for column in columns]),delimiter=',',
                fmt=['%d']*5+['%.17g','%d']+['%.17g']*4)
    temporary.replace(output)
    receipt = dict(schema='FEVC-PIPELINE-SCREEN-INPUT-V1',profile=profile,rows=rows,smoke=smoke,
        workers=workers,firms=firms,stayers=converted,degree_counts={str(d):int(sum(realized==d)) for d in range(1,9)},
        fixture=fixture,sha256=graph.sha(output),generator_sha256=graph.sha(Path(__file__)),
        graph_generator_sha256=graph.sha(GRAPH_SOURCE),campaign_sha256=graph.sha(Path(__file__).with_name('campaign.py')))
    graph.atomic_json(output.with_suffix('.json'),receipt)
    return receipt


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('directory',type=Path)
    p.add_argument('--smoke',action='store_true')
    a=p.parse_args()
    if a.directory.exists():
        raise SystemExit('input directory already exists')
    for name in PROFILES:
        print(json.dumps(build(a.directory/f'{name}.csv',name,a.smoke)))

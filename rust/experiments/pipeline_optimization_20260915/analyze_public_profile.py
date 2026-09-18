#!/usr/bin/env python3
"""Record every >=5% measured public/core phase and its disposition."""
import argparse
from collections import defaultdict
import json
from pathlib import Path
import re
from screen_runner import read_results,value,sha,write_json

REASONS={
    'fevc':'Retain: cold runtime loading plus caller RNG/sort/data guards and exactly-once cleanup are host operations. They cannot execute on native workers; cold setup remains in the primary timer.',
    '_fevc_rust_generic_planned':'Retain: host ingress, capability/plan reconciliation and result posting depend on Stata-owned state. Native row work uses the admitted bounded pools; do not move host API calls to worker threads.',
    'fevc_rust':'Retain this screen: measured scalar host cost includes result/receipt validation and exports. Dropping repeated checks would weaken the malformed/stale-result boundary. A future vectorized validator needs separate scratch admission and differential fault coverage, not a change during confirmation.',
    '_fevc_rust_plugin_call':'Implemented: native preparation/solve/attachment work is decomposed below; this aggregate also includes host/native transport and is not an additional disjoint phase.',
    'ingest':'Retain: Stata SPI reads are caller-thread-only. Columns are imported once per native preparation.',
    'canonicalize':'Retain: canonical ordering defines semantic RNG and sample identities; duplicate outer grouping/map work already removed in the qualified preparation implementation.',
    'graph':'Retain: pruning reaches a dependent fixed point and must preserve rank/connectivity decisions. The work is measured separately from numeric solve.',
    'compress':'Implemented: reuse canonical retained identifiers and the admitted compressed representation; no second Stata compression import on this native route.',
    'plan':'Implemented: compressed solver borrows preparation-time semantic plan. Diagnostic Stata execution separately proves the retained-plan branch, with zero-overhead instrumentation excluded from performance builds.',
    'stayer_augmentation':'Retain: construct the distinct match/observation hybrid when required. Sufficient statistics and shared bounded work are reused, without substituting ordinary observation correction for match hybrid correction.',
    'solve':'Implemented: bounded exact, compressed and generic executors; thread-aware batching, ordered queue, and admitted parallel statistical phases. Nested phases are reported separately.',
    'direct_solve':'Implemented: independent RHS solves use the bounded admitted pool; scalar fit and warm-start refinement retain their dependency order.',
    'component_gram':'Implemented: independent residual Gaussian probes/RHS/prediction are parallel; covariance accumulation retains deterministic probe order.',
    'component_spectrum':'Retain: measured spectral recurrence is sequential and depends on previous iterates, rank and residual decisions. The unproven parallel spectral-statistics prototype remains excluded.',
    'component_prepare':'Implemented shared independent preparation where supported; retain rank and factorization dependencies and unchanged capability restrictions.',
    'exact_information':'Implemented: bounded exact matrix products with pre-admitted worker scratch.',
    'exact_inverse':'Implemented: independent inverse-column work uses the bounded executor; factorization and triangular dependencies remain.',
    'exact_correction':'Implemented: independent deletion corrections use ordered bounded work, including the separate hybrid pass.',
    'exact':'Implemented: public explicit/auto exact now selects the qualified parallel executor without rewriting the original request.',
    'command':'Retain: exclusive command setup includes admission, immutable planning, pool ownership and lifecycle boundaries; it is not the sum of nested timings.',
    'generic_certification':'Implemented: independent original-system certification is parallel; failed controlled columns refine in the unchanged ordered ladder.',
    'target_statistics':'Implemented: combined target contractions and disjoint admitted statistical jobs; per-output arithmetic/reduction order is preserved.',
    'leverage_rhs':'Implemented: packed counter generation and independent leverage RHS construction use the bounded existing executor and admitted scratch in both compressed and generic paths.',
    'leverage_statistics':'Implemented: disjoint leverage accumulators run as fallible ordered jobs; output order and cancellation remain deterministic.',
    'target':'Implemented: bounded target batches and independent RHS/statistical work; any exclusive remainder retains ordered reduction/refinement.',
}


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('root',type=Path)
    p.add_argument('output',type=Path)
    p.add_argument('--plan-reuse-log',type=Path,required=True)
    a=p.parse_args()
    rows=[]
    missing=set()
    for directory in sorted(a.root.iterdir()):
        if not directory.is_dir(): continue
        result=directory/'result.tsv'
        data=read_results(result)
        total=float(value(data,'timer','command_seconds'))
        phases={}
        transcript=(directory/'screen_stata.log').read_text()
        for count,seconds,name in re.findall(r'^\s+(\d+)\s+(\d+\.\d+)\s+(\w+)\s*$',transcript,re.M):
            phases['ado:'+name]=float(seconds)
        names=data.get(('colnames','rust_phase_profile',0,0),'').split()
        for index,name in enumerate(names,1):
            if name!='native_total': phases['native:'+name]=float(value(data,'matrix','rust_phase_profile',1,index))
        core=defaultdict(float)
        for name,count,inclusive,exclusive in re.findall(r'FEVC_PIPELINE_PROFILE_V1\t(\w+)\t(\d+)\t(\d+)\t(\d+)',(directory/'application.txt').read_text()):
            core[name]+=int(exclusive)/1e9
        phases.update({'core:'+name:seconds for name,seconds in core.items()})
        significant=[]
        for phase,seconds in phases.items():
            if seconds/total<.05: continue
            name=phase.split(':')[1]
            if name not in REASONS: missing.add(name)
            significant.append(dict(phase=phase,seconds=seconds,command_fraction=seconds/total,disposition=REASONS.get(name)))
        rows.append(dict(profile=directory.name,command_seconds=total,result_sha256=sha(result),
            significant_phases=significant,all_phase_seconds=phases))
    reuse='FEVC_PIPELINE_PLAN_REUSE_V1\t1' in a.plan_reuse_log.read_text()
    report=dict(status='PASS' if not missing and len(rows)==12 and reuse else 'INCOMPLETE',
        diagnostic_only=True,performance_claim=None,threshold=.05,profiles=rows,unreviewed_phases=sorted(missing),
        preparation_plan_reuse_observed=reuse,plan_reuse_log_sha256=sha(a.plan_reuse_log),
        caution='ADO, native and nested core phases overlap across views; never add them together. One instrumented call per profile locates costs, not speedups.')
    write_json(a.output,report)
    print(json.dumps(dict(status=report['status'],unreviewed=sorted(missing),profiles=len(rows))))
    if report['status']!='PASS': raise SystemExit(1)

"""Validate all fifteen approximate calls and emit appendix-ready tables."""
import argparse,csv,hashlib,json,math,statistics
from pathlib import Path

ROLES=('fevc','matlab','julia','r','pytwoway')
TARGETS=('worker','firm','covariance','total')
LABELS=dict(fevc='FEVC',matlab='KSS MATLAB',julia='Julia',r='R',pytwoway='PyTwoWay')
def read(p):return json.loads(p.read_text())
def csvrows(p,delimiter=','):
    with p.open() as f:return list(csv.DictReader(f,delimiter=delimiter))
def writecsv(p,rows):
    with p.open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');w.writeheader();w.writerows(rows)

def load(run):
    identity=read(run/'input/identity.json'); tasks=csvrows(run/'input/tasks.tsv','\t')
    if len(tasks)!=3 or [int(t['task_id']) for t in tasks]!=[1,2,3]:raise ValueError('task inventory')
    if sorted(p.name for p in (run/'output/smoke').iterdir() if p.is_dir())!=[f'task-{i:03}' for i in range(1,4)]:raise ValueError('output inventory')
    for key,name in (('manifest_sha256','tasks.tsv'),('reference_sha256','oracle.json')):
        if hashlib.sha256((run/'input'/name).read_bytes()).hexdigest()!=identity[key]:raise ValueError('input identity')
    receipt=read(run/'input/fixture.json'); digest=receipt['sha256']
    if hashlib.sha256((run/'input/fixture.csv').read_bytes()).hexdigest()!=digest:raise ValueError('fixture hash')
    calls=[];checks=[];oracle=read(run/'input/oracle.json')
    for t in tasks:
        folder=run/'output/smoke'/f"task-{int(t['task_id']):03}"
        if not (folder/'wrapper.pass').exists() or (folder/'wrapper.fail').exists():raise ValueError('wrapper failure')
        if read(folder/'input.json')['sha256']!=digest:raise ValueError('task input')
        if any(str(read(folder/'task.json')[k])!=v for k,v in t.items()):raise ValueError('task metadata')
        for role in ROLES:
            d=folder/role;status=read(d/'status.json');monitor=read(d/'process_tree.json')
            if role=='fevc':
                rr=csvrows(d/'result.csv')
                if len(rr)!=1:raise ValueError('FEVC result count')
                r=rr[0]
            else:r=read(d/'result.json')
            if any(x.get('status')!='PASS' for x in (status,monitor,r)):raise ValueError('role failed')
            if r['role']!=role or r['algorithm']!='jla' or t['algorithm']!='jla':raise ValueError('role/algorithm')
            for k in ('rows','cores','seed','probes'):
                if int(float(r[k]))!=int(t[k]):raise ValueError(k)
            n=int(t['rows'])
            if n!=identity['rows'] or int(r['retained_rows'])!=n:raise ValueError('sample')
            if not monitor['phase_start_observed'] or not monitor['phase_end_observed'] or monitor['phase_sample_count']<1:raise ValueError('memory phase')
            factor=(n-1)/n if role in ('matlab','julia','r') else 1.
            if abs(float(r['normalization_factor'])-factor)>1e-15:raise ValueError('normalization')
            vals={k:float(r['normalized_'+k]) for k in TARGETS}
            for k,v in vals.items():
                if not math.isfinite(v) or abs(v-float(r['raw_'+k])*factor)>1e-12:raise ValueError('target')
            if abs(vals['total']-vals['worker']-vals['firm']-2*vals['covariance'])>1e-9:raise ValueError('accounting identity')
            seconds=float(r['primary_seconds']);rss=float(monitor['phase_peak_rss_kib'])/1024
            if not math.isfinite(seconds) or seconds<=0 or not math.isfinite(rss) or rss<=0:raise ValueError('performance')
            marker=f'FEVC_FIVE_WAY_ROLE_PASS {role} jla rows={n} cores=2'
            if marker not in (d/'application.txt').read_text():raise ValueError('application marker')
            row=dict(task_id=int(t['task_id']),repeat=int(t['repeat']),seed=int(t['seed']),role=role,
              rows=n,cores=int(t['cores']),probes=int(t['probes']),primary_seconds=seconds,phase_peak_rss_mib=rss,**vals)
            calls.append(row)
            for k,v in vals.items():
                gap=v-oracle['targets'][k]
                checks.append(dict(task_id=row['task_id'],role=role,target=k,estimate=v,oracle=oracle['targets'][k],
                  signed_gap=gap,absolute_gap=abs(gap),gap_fraction_of_correction=gap/abs(oracle['correction'][k])))
    return calls,checks,oracle

def summarize(run,plot=True):
    calls,checks,oracle=load(run);out=run/'diagnostic';out.mkdir(exist_ok=True)
    writecsv(out/'measurements.csv',calls);writecsv(out/'consistency.csv',checks)
    stats={}
    for role in ROLES:
        rr=[r for r in calls if r['role']==role]
        stats[role]={k:dict(median=statistics.median(r[k] for r in rr),minimum=min(r[k] for r in rr),maximum=max(r[k] for r in rr))
          for k in (*TARGETS,'primary_seconds','phase_peak_rss_mib')}
    n=oracle['rows']
    columns=('worker','firm','covariance','total','primary_seconds','phase_peak_rss_mib')
    table='\n'.join('| '+LABELS[role]+' | '+' | '.join(f"{stats[role][k]['median']:.6f}" for k in columns)+' |' for role in ROLES)
    refs='\n'.join('| '+label+' | '+' | '.join(f'{oracle[key][k]:.8f}' for k in TARGETS)+' |'
       for label,key in [('Latent truth','true_components'),('Uncorrected OLS','plugin'),('Exact Schur reference','targets')])
    report=f'''# Approximate comparison: {n:,} observations, two active cores

All 15 calls completed. Three calls per package, 280 projections, unchanged
comparator formulas and solver settings. Reported values are componentwise
medians; individual calls satisfy the variance accounting identity, but the
sum of medians need not equal the median total.

| Reference | Worker | Firm | Covariance | Total |
|---|---:|---:|---:|---:|
{refs}

The independent exact reference removes {oracle['correction_fraction_of_plugin']['worker']:.1%}
of the uncorrected worker variance and {oracle['correction_fraction_of_plugin']['firm']:.1%}
of the uncorrected firm variance. R-squared is {oracle['r_squared']:.4f}.
There are {oracle['workers']:,} workers and {oracle['firms']:,} firms, three
distinct firm matches per worker, one fixed outcome draw and no seed search.
The mean-centered outcome is identical across every call. Reference arithmetic
uses an independent worker-eliminated Schur system, not a comparator's output.
Maximum leverage is {oracle['maximum_leverage']:.6f}; original normal-equation
residual is {oracle['normal_equation_residual']:.3g}.

| Implementation | Worker | Firm | Covariance | Total | Seconds | Peak RSS MiB |
|---|---:|---:|---:|---:|---:|---:|
{table}

![Approximate estimates](estimates.png)

Ranges are observed minima/maxima, not confidence intervals. Julia initializes
fixed internal thread-local projection streams: the three caller seeds do not
provide three independent Julia projection realizations. Equal projection
counts need not provide equal accuracy across packages. This single outcome
does not establish statistical unbiasedness or sampling precision. Latent
truth differs from the exact sample estimator through sampling variation.

![Approximate performance](performance.png)

Timing includes package-specific preparation, pool setup/JIT, estimation and
target extraction, but excludes generic CSV import. Memory is peak process-tree
RSS during that phase, sampled every 100 ms. Four slots reserve 32 GiB for
headroom; each role is restricted to two active CPU cores. Shared-node timings
are descriptive and this is not a core-count sweep. Exact comparator paths
were not rerun; the earlier exact-path findings remain in the prior report.

See [all calls](measurements.csv), [all reference gaps](consistency.csv),
[median and range data](summary.json), and [frozen protocol](../code/PROTOCOL.md).
Scheduler accounting must be checked separately after job completion.
'''
    (out/'report.md').write_text(report)
    result=dict(schema='FEVC-LARGE-APPROX-SUMMARY-V1',status='OUTPUTS_COMPLETE',rows=n,calls=len(calls),
      comparisons=len(checks),statistics=stats,julia_rng_caveat=True)
    (out/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
    tex='\\begin{tabular}{lrrrr}\n\\hline\nImplementation & Worker variance & Firm variance & Seconds & RSS (MiB) \\\\\n\\hline\n'
    for role in ROLES:
        tex+=LABELS[role]+' & '+' & '.join(f"{stats[role][k]['median']:.6f}" for k in ('worker','firm','primary_seconds','phase_peak_rss_mib'))+' \\\\\n'
    (out/'appendix_table.tex').write_text(tex+'\\hline\n\\end{tabular}\n% Three calls; fixed Julia internal streams. See report.md for timing and accuracy caveats.\n')
    if plot:
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        fig,axes=plt.subplots(2,2,figsize=(11,7))
        for ax,k in zip(axes.flat,TARGETS):
            for i,role in enumerate(ROLES):
                v=stats[role][k];m=v['median']
                ax.errorbar(m,i,xerr=[[m-v['minimum']],[v['maximum']-m]],fmt='o',capsize=3,color='#0072B2')
            ax.axvline(oracle['targets'][k],color='#009E73',linestyle='--',label='Exact reference')
            ax.set_yticks(range(5),[LABELS[r] for r in ROLES]);ax.invert_yaxis();ax.set_title(k.capitalize());ax.grid(axis='x',alpha=.2)
        axes[0,0].legend(fontsize=8);fig.suptitle(f'{n:,} observations, 2 cores, 280 projections: median and observed range')
        fig.tight_layout()
        for ext in ('png','svg'):fig.savefig(out/f'estimates.{ext}',dpi=200,bbox_inches='tight')
        plt.close(fig)
        fig,axes=plt.subplots(1,2,figsize=(11,4))
        for ax,k,label in zip(axes,('primary_seconds','phase_peak_rss_mib'),('Primary runtime (seconds)','Primary peak process-tree RSS (MiB)')):
            ax.bar([LABELS[r] for r in ROLES],[stats[r][k]['median'] for r in ROLES],color='#0072B2')
            ax.set_ylabel(label);ax.set_yscale('log');ax.tick_params(axis='x',rotation=15);ax.grid(axis='y',alpha=.2)
        fig.suptitle(f'{n:,} observations, 2 cores: approximate paths, median of three calls');fig.tight_layout()
        for ext in ('png','svg'):fig.savefig(out/f'performance.{ext}',dpi=200,bbox_inches='tight')
        plt.close(fig)
    print(f'FEVC_LARGE_APPROX_OUTPUTS_PASS rows={n} calls={len(calls)}')
    return result

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);a=p.parse_args();summarize(a.run)

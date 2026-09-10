"""Independent exact quadratic-form reference after eliminating worker effects.

For three observations per worker, G is the worker's mean firm exposure,
S=F'F-3G'G, and L=S^-1 F'M_D. Worker influence is D'/3-G L.
Its squared norm uses 3G'G=diag(firm counts)-S, avoiding a worker-by-row
matrix. Work is blocked over observations; no N-by-N matrix is constructed.
This diagnostic is independent of every benchmark estimator.
"""
import numpy as np

TARGETS = ('worker', 'firm', 'covariance', 'total')


def moments(a, b):
    w, f = float(np.var(a)), float(np.var(b))
    c = float(np.mean((a-a.mean())*(b-b.mean())))
    return dict(worker=w, firm=f, covariance=c, total=w+f+2*c)


def fixture(n):
    if n < 960 or n % 120:
        raise ValueError('invalid fixture dimensions')
    worker=np.arange(n)//3; period=np.arange(n)%3+1; nf=n//120
    layer,base=worker//nf,worker%nf
    offset=np.where(period==1,0,np.where(period==2,1+layer%(nf//4-1),(nf+2)//3+(97*layer)%(nf//4)))
    firm=(base+offset)%nf
    a=np.random.Generator(np.random.PCG64(20260909201)).normal(size=n//3)
    b=np.random.Generator(np.random.PCG64(20260909202)).normal(size=nf)
    a=(a-a.mean())/a.std()*.5; b=(b-b.mean())/b.std()*.25
    y=a[worker]+b[firm]+2*np.random.Generator(np.random.PCG64(20260909203)).normal(size=n)
    y-=y.mean()
    ref=reference(worker,firm,y)
    ref.update(true_components=moments(a[worker],b[firm]),noise_sd=2.)
    return worker,firm,period,y,ref


def reference(worker, firm, y, block=256):
    n=len(y); nw=n//3; nf=int(max(firm))+1
    if not np.array_equal(worker,np.repeat(np.arange(nw),3)):
        raise ValueError('reference requires three sorted observations per worker')
    triples=firm.reshape(nw,3)
    if np.any(np.diff(np.sort(triples,axis=1),axis=1)==0):
        raise ValueError('matches must be distinct')
    counts=np.bincount(firm,minlength=nf).astype(float)
    s=np.diag(counts)
    for i in range(3):
        for j in range(3):
            np.add.at(s,(triples[:,i],triples[:,j]),-1/3)
    inverse=np.zeros((nf,nf))
    inverse[:-1,:-1]=np.linalg.inv(s[:-1,:-1])
    inverse_error=float(np.max(np.abs(s[:-1,:-1]@inverse[:-1,:-1]-np.eye(nf-1))))
    means=y.reshape(nw,3).mean(axis=1)
    psi=inverse@np.bincount(firm,weights=y-means[worker],minlength=nf)
    alpha=means-psi[triples].mean(axis=1)
    residual=y-alpha[worker]-psi[firm]
    plugin=moments(alpha[worker],psi[firm])
    v=(inverse[triples[:,0]]+inverse[triples[:,1]]+inverse[triples[:,2]])/3
    correction=np.zeros(4); hmin=1.; hmax=0.
    for start in range(0,n,block):
        stop=min(n,start+block); w=worker[start:stop]; f=firm[start:stop]
        l=inverse[f]-v[w]
        idx=np.arange(stop-start)
        lf=l[idx,f]; gl=sum(l[idx,triples[w,k]] for k in range(3))/3
        h=1/3+lf-gl
        if not np.all((h>=0)&(h<1-1e-10)):
            raise ValueError('fixture not leave-match-out estimable')
        hmin=min(hmin,float(h.min()));hmax=max(hmax,float(h.max()))
        mass=l@counts; norm=(l*l)@counts
        bw=1/3-gl-lf+norm-(1-mass)**2/n
        bf=norm-mass**2/n; bt=h-1/n; bc=(bt-bw-bf)/2
        score=y[start:stop]*residual[start:stop]/(1-h)
        correction+=np.array([q@score/n for q in (bw,bf,bc,bt)])
    error=max(np.max(np.abs(np.bincount(worker,weights=residual))),np.max(np.abs(np.bincount(firm,weights=residual))))
    if error>1e-8 or inverse_error>1e-8:
        raise ValueError('reference residual gate failed')
    corr=dict(zip(TARGETS,map(float,correction)))
    return dict(schema='FEVC-SCHUR-EXACT-REFERENCE-V1',status='PASS',rows=n,workers=nw,firms=nf,
        denominator='population_N',plugin=plugin,correction=corr,
        targets={k:plugin[k]-corr[k] for k in TARGETS},
        correction_fraction_of_plugin={k:corr[k]/plugin[k] for k in TARGETS},
        mean_outcome=float(y.mean()),outcome_variance=float(np.var(y)),
        residual_variance=float(np.mean(residual**2)),r_squared=float(1-np.mean(residual**2)/np.var(y)),
        minimum_leverage=hmin,maximum_leverage=hmax,normal_equation_residual=float(error),
        inverse_residual=inverse_error,reference_method='Exact worker-eliminated Schur; blocked target norms')

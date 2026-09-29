"""Independent original-dummy SVD and high-precision literal-deletion oracle.

Synthetic pooled firms retain distinct original match IDs. Attached stayers
have two endpoints. No FEVC projection, canonicalization or deletion code is
used; every deleted regression is fit directly on its surviving rows.
"""
from pathlib import Path
import argparse
import json
import numpy as np
import scipy.linalg as la
import mpmath as mp


def fixture():
    rng = np.random.default_rng(9282601)
    rows = []
    for w in range(1, 19):
        years = range(8) if w <= 12 else (0, 7)
        for t in years:
            firm = 1 + (w + t // (2+w%3)) % 3 if w <= 9 else 1 + w % 3
            match = 100*w + 10*firm + t//4 if w <= 12 else 100*w + 10*firm
            age = (23 + (w*7 % 29) + t - 40)/10
            edu = w % 3
            x = [age**power*(edu == e) for e in range(3) for power in (2, 3)]
            x += [float(t == year) for year in range(1, 8)]
            y = .07*w + .11*firm + .014*age**2 + .13*rng.normal()
            # Distinct target populations, each defined before subsetting.
            native = 1 + (w*3+t) % 7/10
            earnings = (1 + w % 4/8)*np.exp(.15*t)
            for replicate in range(4 if w <= 12 else 1):
                rows.append([w, firm, 10*match+replicate, int(w > 12),
                             y+.09*rng.normal(), native, earnings, *x])
    return np.asarray(rows, float)


def original_design(a, controls=True):
    workers = sorted(set(a[:, 0])); firms = sorted(set(a[:, 1]))
    w = (a[:, 0, None] == workers).astype(float)
    f = (a[:, 1, None] == firms[:-1]).astype(float)
    x = np.column_stack((w, f, a[:, 7:])) if controls else np.column_stack((w, f))
    aw = np.zeros_like(x); aw[:, :w.shape[1]] = w
    af = np.zeros_like(x); af[:, w.shape[1]:w.shape[1]+f.shape[1]] = f
    return x, aw, af


def blocks(a):
    out = []
    for key in sorted(set(a[a[:, 3] == 0, 2])):
        out.append(np.flatnonzero(a[:, 2] == key))
    out.extend(np.asarray([i]) for i in np.flatnonzero(a[:, 3] == 1))
    return out


def svd_oracle(a, weight, nuisance):
    x, aw, af = original_design(a); y = a[:, 4].copy()
    fit, _, rank, _ = la.lstsq(x, y, lapack_driver='gelsd')
    assert rank == x.shape[1]
    if nuisance == 'fixedoffset':
        y -= a[:, 7:] @ fit[-13:]
        x, aw, af = original_design(a, False)
        fit = la.lstsq(x, y, lapack_driver='gelsd')[0]
    pinv = la.pinv(x)
    mass = a[:, weight]/a[:, weight].sum()
    aw -= mass @ aw; af -= mass @ af
    targets = [aw.T@(mass[:, None]*aw), af.T@(mass[:, None]*af)]
    cross = aw.T@(mass[:, None]*af); targets += [(cross+cross.T)/2]
    targets += [targets[0]+targets[1]+2*targets[2]]
    plugin = np.array([fit @ t @ fit for t in targets]); correction = np.zeros(4)
    for block in blocks(a):
        keep = np.ones(len(a),bool); keep[block] = False
        deleted, _, rank, _ = la.lstsq(x[keep], y[keep], lapack_driver='gelsd')
        assert rank == x.shape[1], (block, rank, x.shape[1])
        residual = y[block]-x[block]@deleted
        score = pinv[:, block]
        correction += [y[block] @ (score.T@t@score) @ residual for t in targets]
    return plugin-correction


def mp_oracle(a, weight, nuisance, precision):
    with mp.workdps(precision):
        convert = lambda v: mp.matrix([[mp.mpf(float(z)) for z in row] for row in np.atleast_2d(v)])
        x0, aw0, af0 = original_design(a); x = convert(x0); y = convert(a[:,4,None])
        fit = mp.lu_solve(x.T*x, x.T*y)
        if nuisance == 'fixedoffset':
            y -= convert(a[:,7:])*mp.matrix(list(fit)[-13:])
            x0,aw0,af0=original_design(a,False); x=convert(x0)
            fit=mp.lu_solve(x.T*x,x.T*y)
        aw=convert(aw0);af=convert(af0)
        mass=mp.matrix([mp.mpf(float(z)) for z in a[:,weight]]); mass/=sum(mass)
        for design in (aw,af):
            mean=mass.T*design
            for i in range(len(a)):
                for j in range(design.cols):design[i,j]-=mean[j]
        h=mp.diag(list(mass)); cross=aw.T*h*af
        targets=[aw.T*h*aw,af.T*h*af,(cross+cross.T)/2]
        targets += [targets[0]+targets[1]+2*targets[2]]
        pinv=(x.T*x)**-1*x.T
        correction=[mp.mpf(0)]*4
        for block in blocks(a):
            keep=[i for i in range(len(a)) if i not in block]
            xd=mp.matrix([[x[i,j] for j in range(x.cols)] for i in keep]);yd=mp.matrix([y[i] for i in keep])
            deleted=mp.lu_solve(xd.T*xd,xd.T*yd)
            xb=mp.matrix([[x[int(i),j] for j in range(x.cols)] for i in block]);yb=mp.matrix([y[int(i)] for i in block])
            residual=yb-xb*deleted
            score=mp.matrix([[pinv[j,int(i)] for i in block] for j in range(x.cols)])
            for j,t in enumerate(targets):correction[j]+=(yb.T*score.T*t*score*residual)[0]
        return np.array([float((fit.T*t*fit)[0]-c) for t,c in zip(targets,correction)])


def write(output, high_precision=True):
    output.mkdir(parents=True,exist_ok=True); a=fixture()
    np.savetxt(output/'input.csv',a,delimiter=',',header=','.join(['worker','firm','match','stayer','y','native','earnings']+[f'x{j}' for j in range(1,14)]),comments='',fmt='%.17g')
    receipt={'schema':'FEVC_CONTROL_SPAN_ORACLE_V1','rows':len(a),'cases':[]}
    lines=[]
    for population in ('movers','both'):
        subset=a[a[:,3]==0] if population=='movers' else a
        for weight,label in ((5,'native'),(6,'earnings')):
            for nuisance in ('joint','fixedoffset'):
                result=svd_oracle(subset,weight,nuisance);case=dict(population=population,weight=label,nuisance=nuisance,svd=result.tolist())
                if high_precision:
                    hi=mp_oracle(subset,weight,nuisance,80); higher=mp_oracle(subset,weight,nuisance,160)
                    case.update(mp80=hi.tolist(),mp160=higher.tolist(),svd_gap=float(max(abs(result-hi))),precision_gap=float(max(abs(hi-higher))))
                    assert max(abs(result-hi))<1e-8 and np.array_equal(hi,higher)
                name=f'o_{population}_{label}_{"joint" if nuisance=="joint" else "offset"}'
                lines.append(f'matrix {name} = ('+','.join(f'{v:.17g}' for v in result)+')')
                receipt['cases'].append(case)
                print(population,label,nuisance,case.get('svd_gap'),flush=True)
    (output/'oracle.do').write_text('\n'.join(lines)+'\n');(output/'oracle.json').write_text(json.dumps(receipt,indent=2)+'\n')
    return receipt

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('output',type=Path);p.add_argument('--svd-only',action='store_true');args=p.parse_args();write(args.output,not args.svd_only)

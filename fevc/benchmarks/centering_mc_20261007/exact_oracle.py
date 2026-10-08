"""Independent dense Gaussian moments for the fixed synthetic design only.

The observation-square matrices below are an audit oracle, never production.
"""
import argparse
import json
from pathlib import Path

import numpy as np
import pandas as pd


def oracle(fixture, draws=None):
    data = pd.read_csv(fixture)
    n = len(data)
    D = pd.get_dummies(data.workerid, dtype=float).to_numpy()
    F = pd.get_dummies(data.firmid, dtype=float).to_numpy()[:, 1:]
    X = np.column_stack([D, F])
    A = np.linalg.inv(X.T @ X)
    H = X @ A @ X.T
    M = np.eye(n)-H
    C = np.eye(n)-np.ones((n,n))/n
    U = np.column_stack([D, np.zeros_like(F)])
    V = np.column_stack([np.zeros_like(D), F])
    U, V = C@U, C@V
    Q = [U.T@U/n, V.T@V/n, (U.T@V+V.T@U)/(2*n)]
    Q.append(Q[0]+Q[1]+2*Q[2])
    movers = data.groupby('workerid').firmid.transform('nunique').to_numpy()>1
    groups = [np.flatnonzero((data.workerid.to_numpy()==w)&(data.firmid.to_numpy()==f))
              for w,f in data[movers].groupby(['workerid','firmid']).groups]
    groups += [np.array([i]) for i in np.flatnonzero(~movers)]
    Z = np.zeros((n,len(groups)))
    R = np.zeros((n,n))
    masses = np.array([len(g) for g in groups])
    a = np.empty(len(groups))
    for j,g in enumerate(groups):
        Z[g,j] = 1
        Mgg = M[np.ix_(g,g)]
        R[g,:] = np.linalg.solve(Mgg,M[g,:])
        a[j] = Mgg.sum()/len(g)
    K = n*np.diag(a)-Z.T@M@Z
    assert np.max(np.abs(M@data.signal.to_numpy()))<1e-10
    assert np.max(np.abs(R@np.ones(n)))<1e-10
    sizes = Z@masses
    group_sums = Z@Z.T
    mu = data.signal.to_numpy()
    sigma2 = 2.25
    result = []
    yy = pd.read_csv(draws).to_numpy() if draws else None
    for target,q in enumerate(Q,1):
        B = X@A@q@A@X.T
        block = np.zeros_like(B)
        for g in groups:
            block[np.ix_(g,g)] = B[np.ix_(g,g)]
        correction = block@R
        none = B-correction
        mean = B-C@correction
        ell = block@np.ones(n)
        k = np.linalg.solve(K.T,(Z.T@ell)/masses)
        x = -ell.copy()
        for j,g in enumerate(groups):
            x[g] += n*k[j]*M[np.ix_(g,g)].sum(axis=1)
        v = -M@Z@k
        delta = C@(((x/sizes+v)[:,None]*group_sums - group_sums*(x/sizes)[None,:])@R)
        corrected = mean+delta
        for mode,T in [('plugin',B),('none',none),('mean',mean),('corrected',corrected)]:
            W = (T+T.T)/2
            expected = mu@W@mu + sigma2*np.trace(W)
            variance = 2*sigma2**2*np.sum(W*W)+4*sigma2*np.sum((W@mu)**2)
            row = dict(target=target,mode=mode,expectation=float(expected),sd=float(np.sqrt(variance)))
            if yy is not None:
                row['points'] = [float(y@W@y) for y in yy.T]
            result.append(row)
    return result


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('fixture',type=Path)
    p.add_argument('output',type=Path)
    p.add_argument('--draws',type=Path)
    a=p.parse_args()
    result=oracle(a.fixture,a.draws)
    a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(pd.DataFrame(result).drop(columns='points',errors='ignore').to_string(index=False))

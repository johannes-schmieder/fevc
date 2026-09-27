"""Exact-binary-input Decimal oracle for a fixed-anchor reconstruction certificate."""
from decimal import Decimal, localcontext
from pathlib import Path
import math

FIXTURE = Path(__file__).resolve().parents[1] / 'fixtures/control_posterior.txt'

def inverse(a):
    q=len(a)
    x=[list(row)+[Decimal(int(i==j)) for j in range(q)] for i,row in enumerate(a)]
    for k in range(q):
        pivot=max(range(k,q),key=lambda i:abs(x[i][k]))
        x[k],x[pivot]=x[pivot],x[k]
        scale=x[k][k]
        x[k]=[v/scale for v in x[k]]
        for i in range(q):
            if i!=k:
                scale=x[i][k]
                x[i]=[u-scale*v for u,v in zip(x[i],x[k])]
    return [row[q:] for row in x]

def upper(value):
    rounded=float(value)
    return math.nextafter(rounded,math.inf) if Decimal.from_float(rounded)<value else rounded

def fixture_text():
    lines=['# name;q;n;X_rowmajor;transform_rowmajor;computed_C_rowmajor;exact_error_infinity_upper;exact_relative_frobenius_upper;exact_weighted_relative_frobenius_upper']
    with localcontext() as ctx:
        ctx.prec=180
        for q,perturb,weak in [(1,0.,False),(2,0.,False),(7,1e-8,False),(13,0.,False),(13,1e-6,False),(32,0.,False),(2,0.,True)]:
            n=q+7
            x=[[float((i*13+j*7)%11-5)/32 for j in range(q)] for i in range(n)]
            for i in range(q):x[i][i]+=2
            if weak:x[1]=[x[0][0],x[0][1]+2**-20]
            dx=[[Decimal.from_float(v) for v in row] for row in x]
            a_inv=inverse(dx[:q])
            truth=[[sum((dx[i][k]*a_inv[k][j] for k in range(q)),Decimal(0)) for j in range(q)] for i in range(n)]
            t=[[float(v) for v in row] for row in a_inv]
            c=[[float(v) for v in row] for row in truth]
            c[-1][-1]+=perturb
            if perturb:t[0][-1]+=perturb/3
            err=[[abs(Decimal.from_float(c[i][j])-truth[i][j]) for j in range(q)] for i in range(n)]
            infinity=max(sum(row,Decimal(0)) for row in err)
            fnorm=sum((v*v for row in err for v in row),Decimal(0)).sqrt()
            cnorm=sum((Decimal.from_float(v)**2 for row in c for v in row),Decimal(0)).sqrt()
            relative=fnorm/max(Decimal(1),cnorm)
            frequency=[Decimal(1+i*977%10000) for i in range(n)]
            werr=sum((frequency[i]*sum((v*v for v in err[i]),Decimal(0)) for i in range(n)),Decimal(0)).sqrt()
            wnorm=sum((frequency[i]*sum((Decimal.from_float(v)**2 for v in c[i]),Decimal(0)) for i in range(n)),Decimal(0)).sqrt()
            weighted=werr/max(Decimal(1),wnorm)
            flatten=lambda a:','.join(repr(v) for row in a for v in row)
            lines.extend([';'.join([f'q{q}_p{perturb}_weak{weak}',str(q),str(n)]),flatten(x),flatten(t),flatten(c),';'.join([repr(upper(infinity)),repr(upper(relative)),repr(upper(weighted))])])
    return '\n'.join(lines)+'\n'

if __name__=='__main__':FIXTURE.write_text(fixture_text())

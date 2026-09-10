"""Independent exact-binary-input Decimal dot oracles; no production imports."""
from decimal import Decimal, localcontext
from pathlib import Path
import math

FIXTURE = Path(__file__).resolve().parents[1] / 'fixtures/control_crossproducts.txt'

def fixture_text():
    eta = math.ulp(0.0)
    cases = [
        ('cancellation', [1.,1.,1.], [1e16,1.,-1e16], [1,1,1], False),
        ('signed_weights', [1e100,-1e100,3.], [1e-100,1e-100,-7.], [37,37,19], False),
        ('two_roundings', [1.0000000000000002,-.123456789,1.23456789], [9.123456789,1.0000000000000004,7.123456789], [9007199254740990,1,1], False),
        ('zero_products', [0.,-0.,1e300,0.], [1e300,-1e300,0.,-0.], [1]*4, False),
        ('subnormal', [eta,2*eta,3*eta,-eta], [.5,.25,.75,1.5], [1,2,3,7], False),
        ('underflow_to_zero', [eta], [.25], [1], False),
        ('tiny_normal', [2.**-1022,-2.**-1022,2.**-1021], [.125,.25,.5], [3,5,7], False),
        ('extreme_balanced', [1e300,-1e300,1e-300,-1e-300], [1e-300,1e-300,1e300,1e300], [1,2,3,4], False),
        ('weight_product_overflow', [1e308], [0.], [2], True),
        ('second_product_overflow', [1e200], [1e200], [1], True),
        ('absolute_sum_overflow', [1e308,-1e308], [1.,1.], [1,1], True),
    ]
    cases.append(('cancellation_lanes',[1.]*385,[1e16,1.,-1e16]*128+[.5],[1]*385,False))
    for n in (2,8,32,257):
        left = [math.ldexp(float((i*37)%101-50), (i%13)-6) for i in range(n)]
        right = [math.ldexp(float((i*71)%103-51), 6-(i%17)) for i in range(n)]
        cases.append((f'signed_{n}',left,right,[1+i%17 for i in range(n)],False))
    lines = ['# name;left;right;integer_frequency;exact_sum_lower;exact_sum_upper']
    with localcontext() as ctx:
        ctx.prec = 2500  # exact binary products, including all subnormal exponents
        for name,left,right,weights,fail in cases:
            exact = sum((Decimal.from_float(x)*Decimal.from_float(y)*f for x,y,f in zip(left,right,weights)), Decimal(0))
            if fail:
                endpoints = ['FAIL','FAIL']
            else:
                rounded = float(exact)
                lo = math.nextafter(rounded,-math.inf) if Decimal.from_float(rounded)>exact else rounded
                hi = math.nextafter(rounded,math.inf) if Decimal.from_float(rounded)<exact else rounded
                assert Decimal.from_float(lo)<=exact<=Decimal.from_float(hi)
                endpoints = [repr(lo),repr(hi)]
            lines.append(';'.join([name,','.join(map(repr,left)),','.join(map(repr,right)),','.join(map(str,weights)),*endpoints]))
    return '\n'.join(lines)+'\n'

if __name__ == '__main__':
    FIXTURE.write_text(fixture_text())

SCORE_FIXTURE = FIXTURE.with_name('control_scores.txt')

def score_fixture_text():
    with localcontext() as ctx:
        ctx.prec = 160
        xy = [(Decimal(i-10)*Decimal('1.375'),Decimal(i%5-2)*Decimal(i+1)*Decimal('.8125')) for i in range(1,21)]
        weights = [1+i%4 for i in range(1,21)]
        g00=sum(f*x*x for (x,y),f in zip(xy,weights))
        g01=sum(f*x*y for (x,y),f in zip(xy,weights))
        g11=sum(f*y*y for (x,y),f in zip(xy,weights))
        det=g00*g11-g01*g01
        inv=(g11/det,-g01/det,g00/det)
        h=inv; scores=[]; anchors=[]
        for _ in range(2):
            score=[h[0]*x*x+2*h[1]*x*y+h[2]*y*y for x,y in xy]
            cutoff=max(score)-Decimal.from_float(1e-7)*max(Decimal(1),max(score))
            chosen=next(i for i,s in enumerate(score) if s>cutoff)
            scores.append(score); anchors.append(chosen)
            x,y=xy[chosen]; a=inv[0]*x+inv[1]*y; b=inv[1]*x+inv[2]*y; v=x*a+y*b
            h=(inv[0]-a*a/v,inv[1]-a*b/v,inv[2]-b*b/v)
        (a,b),(c,d)=[xy[i] for i in anchors]; det=a*d-b*c
        def endpoints(value):
            rounded=float(value)
            return [math.nextafter(rounded,-math.inf) if Decimal.from_float(rounded)>value else rounded,
                    math.nextafter(rounded,math.inf) if Decimal.from_float(rounded)<value else rounded]
        lines=['#anchors='+','.join(str(i+1) for i in anchors),'# x1,x2,f,s1lo,s1hi,s2lo,s2hi,c1lo,c1hi,c2lo,c2hi']
        for i,((x,y),f) in enumerate(zip(xy,weights)):
            values=[float(x),float(y),f,*endpoints(scores[0][i]),*endpoints(scores[1][i]),*endpoints((x*d-y*c)/det),*endpoints((-x*b+y*a)/det)]
            lines.append(','.join(map(str,values)))
        return '\n'.join(lines)+'\n'

if __name__ == '__main__':
    SCORE_FIXTURE.write_text(score_fixture_text())

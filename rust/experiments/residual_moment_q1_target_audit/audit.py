"""Independent dense q1 audit. No production finalizers or covariance repair."""
import math
from functools import lru_cache
import numpy as np

Q_FIELDS = (
    'point', 'score', 'recenter', 'vb', 'leading_component', 'remainder',
    'identity_error', 'cross', 'vr', 'trace_mcse', 'determinant',
    'influence_variance', 'trace', 'curvature', 'critical', 'lower', 'upper',
    'f_statistic', 'influence_concentration',
)
TARGETS = ('worker', 'firm', 'covariance', 'total')


def pair_status(vb, cross, vr, certified=True):
    if not certified:
        return 6
    if vb <= 0 or vr <= 0:
        return 1
    determinant = 1 - cross**2 / vb / vr
    return 0 if math.isfinite(determinant) and determinant > 1e-8 else 2


@lru_cache(maxsize=4)
def quadrature(nodes):
    return np.polynomial.legendre.leggauss(nodes)


def critical_cdf(radius, curvature, nodes=128):
    if curvature <= 1e-10:
        return math.erf(radius / math.sqrt(2))
    # Independent Gaussian integration of the half-normal reference law.
    x, w = quadrature(nodes)
    t = (x + 1) / 2
    horizontal = radius * (1 - t*t)
    vertical = np.sqrt((radius-horizontal)*(2/curvature+radius+horizontal))
    integrand = 2*radius*t*math.sqrt(2/math.pi)*np.exp(-horizontal**2/2)
    return float(w @ (integrand*np.array([math.erf(v/math.sqrt(2)) for v in vertical])) / 2)


def critical_value(curvature):
    lo, hi = 1.0, 4.0
    for _ in range(44):
        mid = (lo+hi)/2
        if critical_cdf(mid, curvature) < .95:
            lo = mid
        else:
            hi = mid
    return (lo+hi)/2


def ellipse(center, covariance, radius, eigenvalue):
    """All real stationary points via a quartic, not an angular search."""
    b, r = center
    vb, cross, vr = covariance
    a = radius*math.sqrt(vb)
    c = radius*cross/math.sqrt(vb)
    d = radius*math.sqrt(vr-cross*cross/vb)
    linear = 4*eigenvalue*a*b+2*c
    quadratic = 4*eigenvalue*a*a
    roots = np.roots([-d, -linear+quadratic, 0., -linear-quadratic, d])
    angles = [math.pi] + [2*math.atan(z.real) for z in roots if abs(z.imag) < 1e-8]
    values = [eigenvalue*(b+a*math.cos(t))**2+r+c*math.cos(t)+d*math.sin(t) for t in angles]
    return [min(values), max(values)]


def remainder_kernel(point_kernel, maker, inverse_diagonal, mode, eigenvalue):
    v = np.asarray(mode)
    correction = eigenvalue*v*v*inverse_diagonal
    return point_kernel-eigenvalue*np.outer(v, v)+(correction[:, None]*maker+maker*correction[None, :])/2


def quadratic_draws(x, ratio, factor, gaussian, mode, eigenvalue):
    v = np.column_stack((x, ratio[:, None]*x))
    g = gaussian@v
    return np.sum((g@factor)*g, axis=1)-(gaussian*gaussian)@ratio-eigenvalue*(gaussian@mode)**2


def calculate(y, s, mode, eigenvalue, point_kernel, remainder, maker, inverse_diagonal,
              trace, certified=True, radius=None):
    u = remainder@y
    point = float(y@point_kernel@y)
    score = float(mode@y)
    recenter = float((mode*mode*y*inverse_diagonal)@(maker@y))
    r = point-eigenvalue*(score*score-recenter)
    vb = float((mode*mode)@s)
    cross = float(2*(mode*s)@u)
    influence = 4*s*u*u
    vr = float(influence.sum()-trace)
    status = pair_status(vb, cross, vr, certified)
    determinant = 1-cross*cross/vb/vr if vb > 0 and vr > 0 else None
    result = dict(status=status, point=point, score=score, recenter=recenter, vb=vb,
                  leading_component=eigenvalue*(score*score-recenter), remainder=r,
                  identity_error=abs(r-float(y@u)), cross=cross, vr=vr,
                  determinant=determinant, influence_variance=float(influence.sum()), trace=float(trace),
                  f_statistic=score*score/vb, influence_concentration=float(influence.max()/influence.sum()))
    if status == 0:
        curvature = 2*abs(eigenvalue)*vb/math.sqrt(vr-cross*cross/vb)
        radius = critical_value(curvature) if radius is None else radius
        lo, hi = ellipse([score, r], [vb, cross, vr], radius, eigenvalue)
        result.update(curvature=curvature, critical=radius, lower=lo, upper=hi)
    return result


def exact_trace(remainder, variance):
    return float(2*np.einsum('ij,ij,i,j->', remainder, remainder, variance, variance))


def relative_error(a, b):
    a, b = np.asarray(a), np.asarray(b)
    return float(np.max(np.abs(a-b)/np.maximum(1., np.maximum(np.abs(a), np.abs(b)))))


def inspect(geometry, exact_modes, draws, failed):
    n, p = geometry['n'], geometry['p']
    x = np.array(geometry['x']).reshape(n, p)
    hat = x@np.array(geometry['inverse']).reshape(p, p)@x.T
    maker = np.eye(n)-hat
    inverse = np.array(geometry['maker'])
    exact_inverse = 1/np.diag(maker)
    factors = np.array(geometry['factors']).reshape(4, 2*p, 2*p)
    ratios3 = np.array(geometry['b'])*inverse
    ratios = np.vstack((ratios3, ratios3[0]+ratios3[1]+2*ratios3[2]))
    exact_ratios = np.array(geometry['exact_ratios'])
    truth_s = np.array(geometry['variance_true'])
    normals = np.array(geometry['gaussian']).reshape(1000, n)
    z = np.array(geometry['z']).reshape(n, geometry['terms'])
    gram = np.array(geometry['gram']).reshape(geometry['terms'], geometry['terms'])
    def kernels(rs):
        result = []
        for r, f in zip(rs, factors):
            v = np.column_stack((x, r[:, None]*x))
            result.append(v@f@v.T-np.diag(r))
        return result
    point_kernels, exact_points = kernels(ratios), kernels(exact_ratios)
    errors = dict(fit=0., influence=0., ratio=0., reconstructed_q=0., ellipse=0.,
                  same_probe_dense_scalar=0., critical_cdf=0., quadrature_convergence=0.,
                  eigen_residual=0., remainder_identity=0.)
    output = []
    for draw in draws:
        start, captured, targets, end = draw
        rep = start['replication']
        order = np.array(start['order'])
        physical = np.argsort(order)
        y = np.array(start['y'])
        s = np.array(captured['variance'])[physical]
        e = maker@y
        fit = np.maximum(z@np.linalg.solve(gram, z.T@(e*e)), 1e-8*np.median(e*e/(1-np.array(geometry['h']))))
        errors['fit'] = max(errors['fit'], relative_error(s, fit))
        gaussian = normals*np.sqrt(s)
        for t, row in enumerate(targets):
            native = dict(zip(Q_FIELDS, row['q']))
            v = np.array(row['mode'])[physical]
            lam = row['eigenvalue']
            c = point_kernels[t]
            r = remainder_kernel(c, maker, inverse, v, lam)
            ratio = ratios[t]-lam*v*v*inverse
            qdraw = quadratic_draws(x, ratio, factors[t], gaussian, v, lam)
            errors['same_probe_dense_scalar'] = max(errors['same_probe_dense_scalar'], relative_error(
                qdraw[:6], np.einsum('ki,ij,kj->k', gaussian[:6], r, gaussian[:6])))
            errors['influence'] = max(errors['influence'], relative_error(r@y, np.array(row['influence'])[physical]))
            errors['ratio'] = max(errors['ratio'], relative_error(ratio, np.array(row['ratio'])[physical]))
            reconstructed = calculate(y, s, v, lam, c, r, maker, inverse, np.var(qdraw, ddof=1),
                                      row['certified'], radius=native['critical'])
            if reconstructed['status'] != row['status']:
                raise ValueError(f'target status reconstruction: {rep}/{t}')
            compare = [key for key in reconstructed if key not in ('status', 'identity_error') and native[key] is not None]
            errors['reconstructed_q'] = max(errors['reconstructed_q'], relative_error(
                [reconstructed[k] for k in compare], [native[k] for k in compare]))
            errors['remainder_identity'] = max(errors['remainder_identity'], reconstructed['identity_error'])
            if row['status'] == 0:
                errors['ellipse'] = max(errors['ellipse'], relative_error(
                    [reconstructed['lower'], reconstructed['upper']], [native['lower'], native['upper']]))
                cdf = critical_cdf(native['critical'], native['curvature'])
                errors['critical_cdf'] = max(errors['critical_cdf'], abs(cdf-.95))
                errors['quadrature_convergence'] = max(errors['quadrature_convergence'], abs(
                    cdf-critical_cdf(native['critical'], native['curvature'], nodes=64)))
            # A = C + sym(diag(ratio) M) is the target plug-in kernel.
            a = c+(ratios[t][:, None]*maker+maker*ratios[t][None, :])/2
            errors['eigen_residual'] = max(errors['eigen_residual'], float(np.linalg.norm(a@v-lam*v)))
            ev = np.array(exact_modes['modes'][t])
            el = exact_modes['eigenvalues'][t]
            er = remainder_kernel(exact_points[t], maker, exact_inverse, ev, el)
            arms = {
                'original_probes_reconstruction': reconstructed,
                'exact_trace_native_kernel': calculate(y, s, v, lam, c, r, maker, inverse, exact_trace(r, s), row['certified']),
                'exact_design_exact_trace': calculate(y, s, ev, el, exact_points[t], er, maker, exact_inverse, exact_trace(er, s)),
                'true_variance_exact_trace': calculate(y, truth_s, v, lam, c, r, maker, inverse, exact_trace(r, truth_s), row['certified']),
            }
            for arm in arms.values():
                if arm['status'] == 0:
                    arm['covered'] = arm['lower'] <= exact_modes['truth'][t] <= arm['upper']
                    arm['width'] = arm['upper']-arm['lower']
            output.append(dict(replication=rep, group='rejected' if rep in failed else 'comparison',
                               target=TARGETS[t], native_status=row['status'], certified=row['certified'],
                               leading_share=row['leading_share'], remainder_share=row['remainder_share'],
                               mode_max_weight=row['mode_max_weight'], native=native, arms=arms))
    for key, value in errors.items():
        limit = .003 if key == 'critical_cdf' else 1e-7 if key == 'quadrature_convergence' else .002 if key == 'eigen_residual' else 1e-8
        if value > limit:
            raise ValueError(f'independent reconstruction {key}: {value} > {limit}')
    return output, errors

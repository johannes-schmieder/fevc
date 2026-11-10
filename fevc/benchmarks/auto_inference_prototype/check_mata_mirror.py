"""Check the algorithm of ``fevc_residual_se.mata`` against the dense oracle.

Stata was not available when the Mata routine was written, so it could not be
executed.  ``mata_mirror`` below is a statement-by-statement transliteration of
``vckss_residual_se__stata`` and its helpers into numpy: same inputs (physical
rows), same unit order (mover units by deletion identifier, then stayer
copies), same formulas.  This script compares every intermediate quantity
with the independent dense oracle in ``physical_mc.py``.  ``mata_jla_mirror``
does the same for the matrix-free routine ``vckss_residual_se_jla__stata``,
whose standard errors are compared with the exact sampling standard deviation
of the jointly estimated estimator.

It checks the algorithm, not the Mata syntax.

Usage:
    python check_mata_mirror.py FIRMS MOVERS STAYERS SEED CONTROLS DRAWS
"""

import sys

import numpy as np

from akm_design import make_design
from dense_mc import true_effects
from physical_mc import PhysicalOracle, error_cov, expand, make_controls


def midranks(x):
    """vckss_inf__midranks: (first + last - 1) / n - 1 over tied runs."""
    n = len(x)
    order = np.argsort(x, kind="stable")
    out = np.empty(n)
    first = 0
    while first < n:
        last = first
        while last + 1 < n and x[order[last + 1]] == x[order[first]]:
            last += 1
        out[order[first : last + 1]] = (first + 1 + last + 1 - 1) / n - 1
        first = last + 1
    return out


def center(value, root):
    """vckss_rse__center."""
    return (
        value - np.outer(root, root @ value) / (root @ root)
        if value.ndim == 2
        else (value - root * (root @ value) / (root @ root))
    )


def kernel(value, design, inverse, target, ratio):
    """vckss_rse__kernel."""
    r = ratio[:, None] if value.ndim == 2 else ratio
    fitted = inverse @ (design.T @ value)
    scaled = r * value
    return design @ (inverse @ (target @ fitted)) - 0.5 * (
        r * (value - design @ fitted) + scaled - design @ (inverse @ (design.T @ scaled))
    )


def features(leverage, mass, with_mass):
    """vckss_rse__features."""
    lever_rank = midranks(leverage)
    one = np.ones(len(leverage))
    if with_mass:
        mass_rank = midranks(mass)
        basis = np.column_stack([one, lever_rank, mass_rank, lever_rank**2, mass_rank**2])
    else:
        basis = np.column_stack([one, lever_rank, lever_rank**2])
    permitted = max(1, min(basis.shape[1], len(leverage) // 100))
    return basis[:, :permitted]


def working(product, residual, leverage, mass, movers):
    """vckss_rse__working."""
    n = len(product)
    fitted = np.empty(n)
    terms = 0
    if movers > 0:
        basis = features(leverage[:movers], mass[:movers], True)
        terms = basis.shape[1]
        fitted[:movers] = basis @ (np.linalg.pinv(basis.T @ basis) @ (basis.T @ product[:movers]))
    if movers < n:
        basis = features(leverage[movers:], mass[movers:], False)
        terms += basis.shape[1]
        fitted[movers:] = basis @ (np.linalg.pinv(basis.T @ basis) @ (basis.T @ product[movers:]))
    floor_value = 1e-8 * np.median(residual**2 / (1 - leverage))
    variance = np.maximum(fitted, floor_value)
    return variance, (terms, floor_value, int((fitted < floor_value).sum()), fitted.min())


def unit_order(deletion, stayer):
    """Mover units by deletion identifier, then stayer rows (vckss_inf__units)."""
    ix = np.flatnonzero(stayer == 0)
    if len(ix) == 0:
        return [], np.flatnonzero(stayer == 1)
    permutation = ix[np.argsort(deletion[ix], kind="stable")]
    ids = deletion[permutation]
    starts = np.flatnonzero(np.r_[True, ids[1:] != ids[:-1]])
    ends = np.r_[starts[1:], len(ids)]
    groups = [permutation[a:b] for a, b in zip(starts, ends)]
    return groups, np.flatnonzero(stayer == 1)


def aggregate(value, other, frequency, deletion, stayer, within):
    """vckss_rse__aggregate."""
    groups, stay_rows = unit_order(deletion, stayer)
    total = len(groups) + int(frequency[stay_rows].sum())
    output = np.zeros((total, value.shape[1]))
    for g, members in enumerate(groups):
        weight = frequency[members]
        mass = weight.sum()
        if within:
            deviation = value[members] - (weight @ value[members]) / mass
            output[g] = (weight * (other[members] - weight @ other[members] / mass)) @ deviation
        else:
            output[g] = (weight @ value[members]) / np.sqrt(mass)
    if within:
        return output
    out = len(groups)
    for row in stay_rows:
        for _ in range(int(frequency[row])):
            output[out] = value[row]
            out += 1
    return output


def mata_mirror(y, design, controls, frequency, deletion, stayer, targets, draws, rng):
    """vckss_residual_se__stata with mean centering; returns its intermediates."""
    parameters = design.shape[1]
    control_count = controls.shape[1]
    working_y = y.copy()
    if control_count > 0:
        full_design = np.column_stack([design, controls])
        full_inverse = np.linalg.inv(full_design.T @ (frequency[:, None] * full_design))
        beta = full_inverse @ (full_design.T @ (frequency * y))
        working_y = y - controls @ beta[parameters:]
    inverse = np.linalg.inv(design.T @ (frequency[:, None] * design))
    beta = inverse @ (design.T @ (frequency * working_y))
    residual = working_y - design @ beta
    if control_count > 0:
        residualized = controls - design @ (inverse @ (design.T @ (frequency[:, None] * controls)))
        precision = np.linalg.inv(residualized.T @ (frequency[:, None] * residualized))
        collapsed = aggregate(controls, working_y, frequency, deletion, stayer, 0)
        collapsed_residualized = aggregate(residualized, working_y, frequency, deletion, stayer, 0)
        within = aggregate(controls, working_y, frequency, deletion, stayer, 1)
    # vckss_inf__units
    groups, stay_rows = unit_order(deletion, stayer)
    movers = len(groups)
    n = movers + int(frequency[stay_rows].sum())
    unit_design = np.zeros((n, parameters))
    unit_y = np.zeros(n)
    unit_residual = np.zeros(n)
    mass = np.ones(n)
    for g, members in enumerate(groups):
        m = frequency[members].sum()
        unit_design[g] = np.sqrt(m) * design[members[0]]
        unit_y[g] = frequency[members] @ working_y[members] / np.sqrt(m)
        unit_residual[g] = frequency[members] @ residual[members] / np.sqrt(m)
        mass[g] = m
    out = movers
    for row in stay_rows:
        for _ in range(int(frequency[row])):
            unit_design[out] = design[row]
            unit_y[out] = working_y[row]
            unit_residual[out] = residual[row]
            out += 1
    design, residual = unit_design, unit_residual
    root = np.sqrt(mass)
    yu = center(unit_y, root)
    leverage = ((design @ inverse) * design).sum(1)
    maker_inverse = 1 / (1 - leverage)
    product = yu * residual * maker_inverse
    ratio = np.empty((n, 3))
    influence = np.empty((n, 3))
    for j in range(3):
        diagonal = (((design @ inverse) @ targets[j]) * (design @ inverse)).sum(1)
        ratio[:, j] = diagonal * maker_inverse
        influence[:, j] = center(kernel(yu, design, inverse, targets[j], ratio[:, j]), root)
    variance, info = working(product, residual, leverage, mass, movers)
    scaled_residual = residual * np.sqrt(maker_inverse)
    score = 2 * influence * scaled_residual[:, None]
    if control_count > 0:
        load = precision @ (-2 * collapsed.T @ influence)
        score = (2 * influence + collapsed_residualized @ load) * scaled_residual[:, None] + (
            within @ load
        )
    trace = np.zeros((3, 3))
    calibration = np.zeros((3, 3))
    cross = np.zeros(4)
    pairs = 0
    previous = None
    batch = min(32, draws)
    for begin in range(0, draws, batch):
        width = min(batch, draws - begin)
        noise = np.sqrt(variance)[:, None] * rng.standard_normal((n, width))
        noise = center(noise, root)
        fitted = inverse @ (design.T @ noise)
        pseudo_residual = (noise - design @ fitted) ** 2 * maker_inverse[:, None]
        image = [
            center(kernel(noise, design, inverse, targets[j], ratio[:, j]), root) for j in range(3)
        ]
        for j in range(3):
            for k in range(3):
                trace[j, k] += (variance[:, None] * image[j] * image[k]).sum()
                calibration[j, k] += (pseudo_residual * image[j] * image[k]).sum()
        for k in range(width):
            total_image = np.column_stack([image[0][:, k], image[1][:, k], image[2][:, k]])
            total_image = np.column_stack(
                [total_image, total_image[:, 0] + total_image[:, 1] + 2 * total_image[:, 2]]
            )
            if previous is not None:
                cross += np.diag(previous.T @ (variance[:, None] * total_image)) ** 2
                pairs += 1
            previous = total_image
    trace /= draws
    calibration /= draws
    signal = score.T @ score - 4 * calibration
    signal = (signal + signal.T) / 2
    eigenvalues, eigenvectors = np.linalg.eigh(signal)
    covariance = (eigenvectors * np.maximum(eigenvalues, 0)) @ eigenvectors.T + 2 * trace
    total_trace = (
        trace[0, 0]
        + trace[1, 1]
        + 4 * trace[2, 2]
        + 2 * trace[0, 1]
        + 4 * trace[0, 2]
        + 4 * trace[1, 2]
    )
    effective = np.array(
        [max(1.0, t**2 / (c / pairs)) for t, c in zip([*np.diag(trace), total_trace], cross)]
    )
    return dict(
        y=yu,
        influence=influence,
        variance=variance,
        score=score,
        trace=trace,
        calibration=calibration,
        covariance=covariance,
        leverage=leverage,
        info=info,
        effective=effective,
        groups=groups,
        stay_rows=stay_rows,
    )


# ---------------------------------------------------------------------------
# Matrix-free route: transliteration of vckss_residual_se_jla__stata.
# ---------------------------------------------------------------------------


class FeDesign:
    """The parts of vckss_fe_design that the routine uses."""

    def __init__(self, worker, firm, frequency):
        import scipy.sparse as sp
        import scipy.sparse.linalg as spla

        self.worker, self.firm, self.frequency = worker, firm, frequency
        self.worker_levels = int(worker.max()) + 1
        self.firm_levels = int(firm.max()) + 1
        n = len(worker)
        cols = np.r_[worker, self.worker_levels + firm]
        rows = np.r_[np.arange(n), np.arange(n)]
        full = sp.csr_matrix((np.ones(2 * n), (rows, cols)))
        self.full = full
        reduced = full[:, : self.worker_levels + self.firm_levels - 1]
        self._solve = spla.splu((reduced.T @ sp.diags(frequency) @ reduced).tocsc())

    def transpose_full(self, values):  # vckss__fe_transpose_full: plain group sums
        return self.full.T @ values

    def transpose(self, values):  # vckss__fe_transpose
        return self.transpose_full(values)[: self.worker_levels + self.firm_levels - 1]

    def predict(self, coefficient):  # vckss__fe_predict: last firm grounded at zero
        pad = np.zeros((1,) + coefficient.shape[1:])
        return self.full @ np.concatenate([coefficient, pad])

    def solve(self, rhs):  # vckss__fe_solve_matrix
        return self._solve.solve(rhs)


def unit_transpose(design, root, value):
    return design.transpose(root[:, None] * value if value.ndim == 2 else root * value)


def unit_predict(design, root, coefficient):
    fitted = design.predict(coefficient)
    return root[:, None] * fitted if fitted.ndim == 2 else root * fitted


def target_action(base, share, coefficient):
    """vckss_rse__target_action: columns stacked worker, firm, covariance."""
    workers, firms = base.worker_levels, base.firm_levels
    columns = coefficient.shape[1]
    worker_effect = coefficient[:workers][base.worker]
    firm_coefficient = np.zeros((firms, columns))
    firm_coefficient[: firms - 1] = coefficient[workers:]
    firm_effect = firm_coefficient[base.firm]
    worker_effect = share[:, None] * (worker_effect - share @ worker_effect)
    firm_effect = share[:, None] * (firm_effect - share @ firm_effect)
    zero_worker = np.zeros((workers, columns))
    zero_firm = np.zeros((firms - 1, columns))
    full = base.transpose_full(worker_effect)
    worker_side, firm_side = full[:workers], full[workers : workers + firms - 1]
    full = base.transpose_full(firm_effect)
    return np.hstack(
        [
            np.vstack([worker_side, zero_firm]),
            np.vstack([zero_worker, full[workers : workers + firms - 1]]),
            0.5 * np.vstack([full[:workers], firm_side]),
        ]
    )


def solve_kernel(value, base, units, root, share, ratio):
    """vckss_rse__solve_kernel: images (three column blocks) and the residual."""
    columns = value.shape[1]
    coefficient = units.solve(unit_transpose(units, root, value))
    residual = value - unit_predict(units, root, coefficient)
    rhs = target_action(base, share, coefficient)
    for j in range(3):
        rhs[:, j * columns : (j + 1) * columns] += 0.5 * unit_transpose(
            units, root, ratio[:, [j]] * value
        )
    image = unit_predict(units, root, units.solve(rhs))
    for j in range(3):
        image[:, j * columns : (j + 1) * columns] -= 0.5 * ratio[:, [j]] * (residual + value)
    return image, residual


def finite_projection(moments, probes):
    """vckss_rse__finite_projection: ratio estimate of the maker diagonal with
    the second-order correction of docs/JLA_FINITE_PROJECTION.md."""
    maker = moments[:, 1] / (moments[:, 0] + moments[:, 1])
    clipped = int((maker < 0.02).sum())
    maker = np.maximum(maker, 0.02)
    lever = 1 - maker
    bias = (maker * moments[:, 2] - lever * moments[:, 3] + (maker - lever) * moments[:, 4]) / (
        probes
    )
    spread = (
        maker**2 * moments[:, 2] + lever**2 * moments[:, 3] - 2 * lever * maker * moments[:, 4]
    ) / probes
    multiplier = np.clip(1 - spread / maker**2 + bias / maker, 0.5, 2.0)
    return multiplier / maker, lever, clipped


def mata_jla_mirror(
    y, worker, firm, controls, frequency, target_weight, deletion, stayer, draws, probes, rng
):
    """vckss_residual_se_jla__stata with mean centering."""
    n = len(y)
    control_count = controls.shape[1]
    base = FeDesign(worker, firm, frequency)
    workers, firms = base.worker_levels, base.firm_levels
    share = target_weight / target_weight.sum()
    outcome_fit = base.predict(base.solve(base.transpose(frequency * y)))
    working_y = y.copy()
    residual = y - outcome_fit
    if control_count > 0:
        control_fit = base.predict(base.solve(base.transpose(frequency[:, None] * controls)))
        residualized = controls - control_fit
        precision = np.linalg.inv(residualized.T @ (frequency[:, None] * residualized))
        gamma = precision @ (residualized.T @ (frequency * residual))
        working_y = y - controls @ gamma
        residual = residual - residualized @ gamma
        collapsed = aggregate(controls, working_y, frequency, deletion, stayer, 0)
        collapsed_residualized = aggregate(residualized, working_y, frequency, deletion, stayer, 0)
        within = aggregate(controls, working_y, frequency, deletion, stayer, 1)
    groups, stay_rows = unit_order(deletion, stayer)
    movers = len(groups)
    total = movers + int(frequency[stay_rows].sum())
    unit_worker = np.empty(total, int)
    unit_firm = np.empty(total, int)
    mass = np.ones(total)
    unit_y = np.zeros(total)
    unit_residual = np.zeros(total)
    for g, members in enumerate(groups):
        m = frequency[members].sum()
        unit_worker[g], unit_firm[g], mass[g] = worker[members[0]], firm[members[0]], m
        unit_y[g] = frequency[members] @ working_y[members] / np.sqrt(m)
        unit_residual[g] = frequency[members] @ residual[members] / np.sqrt(m)
    out = movers
    for row in stay_rows:
        for _ in range(int(frequency[row])):
            unit_worker[out], unit_firm[out] = worker[row], firm[row]
            unit_y[out], unit_residual[out] = working_y[row], residual[row]
            out += 1
    residual = unit_residual
    n = total
    root = np.sqrt(mass)
    yu = center(unit_y, root)
    units = FeDesign(unit_worker, unit_firm, mass)
    moments = np.zeros((n, 5))
    target_diagonal = np.zeros((n, 3))
    batch = min(32, probes)
    for begin in range(0, probes, batch):
        width = min(batch, probes - begin)
        probe = rng.standard_normal((n, width))
        projected = unit_predict(units, root, units.solve(unit_transpose(units, root, probe)))
        # Squared projection and squared residual of each probe, and their
        # second moments, for the finite-projection maker inverse.
        square = projected**2
        complement = (probe - projected) ** 2
        moments[:, 0] += square.sum(1)
        moments[:, 1] += complement.sum(1)
        moments[:, 2] += (square**2).sum(1)
        moments[:, 3] += (complement**2).sum(1)
        moments[:, 4] += (square * complement).sum(1)
        direction = np.sqrt(share)[:, None] * rng.standard_normal((len(share), width))
        direction = direction - np.outer(share, direction.sum(0))
        full = base.transpose_full(direction)
        rhs = np.hstack(
            [
                np.vstack([full[:workers], np.zeros((firms - 1, width))]),
                np.vstack([np.zeros((workers, width)), full[workers : workers + firms - 1]]),
            ]
        )
        projected = unit_predict(units, root, units.solve(rhs))
        target_diagonal[:, 0] += (projected[:, :width] ** 2).sum(1)
        target_diagonal[:, 1] += (projected[:, width:] ** 2).sum(1)
        target_diagonal[:, 2] += (projected[:, :width] * projected[:, width:]).sum(1)
    moments /= probes
    target_diagonal /= probes
    maker_inverse, leverage, clipped = finite_projection(moments, probes)
    ratio = target_diagonal * maker_inverse[:, None]
    product = yu * residual * maker_inverse
    influence, _ = solve_kernel(yu[:, None], base, units, root, share, ratio)
    influence = center(influence, root)
    variance, info = working(product, residual, leverage, mass, movers)
    scaled_residual = residual * np.sqrt(maker_inverse)
    score = 2 * influence * scaled_residual[:, None]
    if control_count > 0:
        load = precision @ (-2 * collapsed.T @ influence)
        score = (2 * influence + collapsed_residualized @ load) * scaled_residual[:, None] + (
            within @ load
        )
    quadratic = np.zeros((3, 3))
    calibration = np.zeros((3, 3))
    batch = min(16, draws)
    for begin in range(0, draws, batch):
        width = min(batch, draws - begin)
        noise = center(np.sqrt(variance)[:, None] * rng.standard_normal((n, width)), root)
        image, kernel_residual = solve_kernel(noise, base, units, root, share, ratio)
        image = center(image, root)
        pseudo_residual = kernel_residual**2 * maker_inverse[:, None]
        for j in range(3):
            for k in range(3):
                a = image[:, j * width : (j + 1) * width]
                b = image[:, k * width : (k + 1) * width]
                quadratic[j, k] += (variance[:, None] * a * b).sum()
                calibration[j, k] += (pseudo_residual * a * b).sum()
    quadratic /= draws
    calibration /= draws
    signal = score.T @ score - 4 * calibration
    signal = (signal + signal.T) / 2
    eigenvalues, eigenvectors = np.linalg.eigh(signal)
    covariance = (eigenvectors * np.maximum(eigenvalues, 0)) @ eigenvectors.T + 2 * quadratic
    return dict(
        covariance=covariance,
        theta=yu @ influence,
        gamma=gamma if control_count > 0 else np.zeros(0),
        clipped=clipped,
        info=info,
        units=n,
        movers=movers,
    )


def check_jla(nf, nm, ns, seed, k, draws, probes, deletion_mode):
    """Standard errors of the matrix-free routine against the exact sampling SD."""
    from dense_mc import T34

    d = make_design(nf, nm, ns, seed)
    rng = np.random.default_rng(seed + 100)
    ph = expand(d, deletion_mode, rng)
    Z = make_controls(d, ph, k, rng)
    o = PhysicalOracle(d, ph, Z)
    a, p = true_effects(d, np.random.default_rng(seed + 1))
    gamma = np.array([0.03, 0.10] + [0.02] * max(k - 2, 0))[:k]
    mu = a[ph["w"]] + p[ph["f"]] + Z @ gamma
    Gam = error_cov(d, ph, "base", s_m2=0.0 if deletion_mode == "observation" else 0.02)
    exact, _ = o.joint_variance(mu, Gam)
    sd = np.sqrt(np.diag(T34 @ exact @ T34.T))
    L = np.linalg.cholesky(Gam + 1e-12 * np.eye(o.nP))
    frequency = np.ones(o.nP)
    # Observation deletion: every physical copy is its own unit.
    stayer = np.ones(o.nP, int) if deletion_mode == "observation" else ph["stay"].astype(int)
    ratios = []
    samples = 12
    for sample in range(samples):
        y = mu + L @ np.random.default_rng(seed + 500 + sample).standard_normal(o.nP)
        r = mata_jla_mirror(
            y,
            ph["w"],
            ph["f"],
            Z,
            frequency,
            np.ones(o.nP),
            ph["unit"],
            stayer,
            draws,
            probes,
            np.random.default_rng(seed + 900 + sample),
        )
        se = np.sqrt(np.diag(T34 @ r["covariance"] @ T34.T))
        ratios.append(se / sd)
        if sample == 0 and k:
            assert np.abs(r["gamma"] - o.gamma_map @ y).max() < 1e-9
    ratios = np.array(ratios)
    rms = np.sqrt((ratios**2).mean(0))
    print(
        "matrix-free route, %s deletion, %d controls: %d units (%d mover units), "
        "%d probes, %d draws, %d samples"
        % (deletion_mode, k, r["units"], r["movers"], probes, draws, samples)
    )
    print("   RMS standard error / exact sampling SD of the joint estimator:", np.round(rms, 3))
    print("   maker diagonals floored at 0.02 in the last sample: %d" % r["clipped"])
    assert np.abs(rms - 1).max() < 0.10, rms


def main(argv):
    nf, nm, ns, seed, k, draws = map(int, argv[:6])
    d = make_design(nf, nm, ns, seed)
    rng = np.random.default_rng(seed + 100)
    ph = expand(d, "match", rng)
    Z = make_controls(d, ph, k, rng)
    o = PhysicalOracle(d, ph, Z)
    a, p = true_effects(d, np.random.default_rng(seed + 1))
    gamma = np.array([0.03, 0.10] + [0.02] * max(k - 2, 0))[:k]
    mu = a[ph["w"]] + p[ph["f"]] + Z @ gamma
    Gam = error_cov(d, ph, "base")
    y = mu + np.linalg.cholesky(Gam + 1e-12 * np.eye(o.nP)) @ rng.standard_normal(o.nP)

    # Physical inputs as the Mata routine reads them: worker dummies and all
    # but the last firm dummy, unit frequencies, a stayer mask.
    du = o.du
    design = np.zeros((o.nP, du.nW + du.nJ - 1))
    design[np.arange(o.nP), ph["w"]] = 1.0
    keep = ph["f"] < du.nJ - 1
    design[np.flatnonzero(keep), du.nW + ph["f"][keep]] = 1.0
    eye = np.eye(du.nW + du.nJ - 1)
    aw = eye[: du.nW]
    pf = np.vstack([eye[du.nW :], np.zeros((1, du.nW + du.nJ - 1))])
    targets = []
    for t in range(3):
        ra, rp = du.A(t, aw, pf)
        At = np.vstack([ra, rp[:-1]])
        targets.append(0.5 * (At + At.T))
    frequency = np.ones(o.nP)
    stayer = ph["stay"].astype(int)
    r = mata_mirror(
        y, design, Z, frequency, ph["unit"], stayer, targets, draws, np.random.default_rng(seed + 9)
    )

    # Map the Mata unit order onto the oracle's unit labels.
    perm = np.array(
        [ph["unit"][members[0]] for members in r["groups"]]
        + [ph["unit"][i] for i in r["stay_rows"]]
    )
    assert len(set(perm)) == o.nU == len(perm)
    gam = o.gamma_map @ y if o.k else np.zeros(0)
    U = y - Z @ gam if o.k else y
    uc = o.T.T @ U
    zc = o.Hc @ uc
    ec = o.Mc @ uc
    w = 1 / o.Mcd
    G = np.column_stack([K @ uc for K in o.Kc])
    worst = {}
    worst["centred outcome"] = np.abs(r["y"] - zc[perm]).max()
    worst["leverage"] = np.abs(r["leverage"] - (1 - o.Mcd)[perm]).max()
    worst["kernel image of the outcome"] = np.abs(r["influence"] - G[perm]).max()
    theta = np.array([uc @ o.Kc[t] @ uc for t in range(3)])
    worst["point identity"] = np.abs(r["y"] @ r["influence"] - theta).max()
    V = np.empty(o.nU)
    V[perm] = r["variance"]
    et = ec * np.sqrt(w)
    psi = 2 * G * et[:, None]
    if o.k:
        Uw = U - o.T @ uc
        W = np.column_stack([o.T.T @ (o.Zw[:, j] * Uw) * o.sqm for j in range(o.k)])
        lam = o.Lam @ (-2 * o.Zc.T @ G)
        psi = (2 * G + o.Ztc @ lam) * et[:, None] + W @ lam
    worst["unit scores"] = np.abs(r["score"] - psi[perm]).max() / np.abs(psi).max()
    # Closed-form pure-noise expectations under the routine's own variances.
    EV = o.MM @ V
    km = [o.KM[t] @ V for t in range(3)]
    tau = np.empty((3, 3))
    noise = np.empty((3, 3))
    for t in range(3):
        for u in range(3):
            c = o.KK[(min(t, u), max(t, u))] @ V
            tau[t, u] = V @ c
            noise[t, u] = (w * (EV * c + 2 * km[t] * km[u])).sum()
    signal = psi.T @ psi - 4 * noise
    ev, Q = np.linalg.eigh((signal + signal.T) / 2)
    exact = (Q * np.maximum(ev, 0)) @ Q.T + 2 * tau
    print(
        "design: %d physical rows, %d units (%d mover units), %d controls, %d draws"
        % (o.nP, o.nU, len(r["groups"]), k, draws)
    )
    print("working-variance terms %d, floored %d" % (r["info"][0], r["info"][2]))
    for name, value in worst.items():
        print("   max abs difference, %-28s %.2e" % (name + ":", value))
        assert value < 1e-8, name
    rel_tau = np.abs(r["trace"] - tau).max() / np.abs(tau).max()
    rel_noise = np.abs(r["calibration"] - noise).max() / np.abs(noise).max()
    se_mirror = np.sqrt(np.diag(r["covariance"]))
    se_exact = np.sqrt(np.diag(exact))
    print("   pseudo-noise trace vs closed form, max relative error        %.4f" % rel_tau)
    print("   pseudo-noise calibration vs closed form, max relative error  %.4f" % rel_noise)
    print("   standard errors, pseudo-noise / closed form:", np.round(se_mirror / se_exact, 4))
    print("   effective rank (worker, firm, covariance, total):", np.round(r["effective"], 1))
    tolerance = 6 * np.sqrt(2 / (draws * r["effective"][:3].min()))
    assert rel_tau < max(tolerance, 0.02) and rel_noise < max(3 * tolerance, 0.05)
    assert np.abs(se_mirror / se_exact - 1).max() < 0.02
    probes = 200
    for mode in ("match", "observation"):
        check_jla(nf, nm, ns, seed, k, 8, probes, mode)
    print("OK")


if __name__ == "__main__":
    main(sys.argv[1:])

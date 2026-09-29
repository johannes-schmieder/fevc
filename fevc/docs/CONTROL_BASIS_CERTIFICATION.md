# Certified control accumulation and anchor selection

This is the API 25 control-basis contract. It changes representation preparation,
not the model, sample, estimator, rank tolerance, inverse-residual gates, solver
or downstream `1e-8` forward-error ceiling. The implementation retains the
existing semantic ordering and at most 32 controls. API identity is
`vckss-api25-control-posterior-timers1`; native layouts are unchanged.

The September 28 development candidate supersedes the **generic Rust JLA**
downstream propagation rules below with a [weighted span certificate](CONTROL_SPAN_REPAIR_2026-09-28.md).
Its compensated posterior residual changes only the error enclosure. Exact
and Mata downstream gates retain the API 25 rules; historical receipts retain
their original source and acceptance status.

## Accumulation

Assume binary64, round to nearest, gradual underflow and no reassociation of
compensated additions. Write `u=2^-53`, `eps=2u`, `eta=2^-1074` and
`gamma_m=m*eps/(1-m*eps)`. Invalid denominators and nonfinite arithmetic withhold.
Frequency weights are positive, exactly representable integers; the public
physical-total check is unchanged. The Rust conversion additionally rejects
`u64::MAX`, whose saturating round trip could otherwise hide rounding to `2^64`.

For each entry the actual terms are `p_i=fl(fl(f_i*x_i)*y_i)`. The first
multiplication cannot incur an inexact subnormal result: multiplying a stored
subnormal by an integer at least one produces an integer multiple of eta.
The second multiplication can underflow. Thus, with exact term `t_i=f_i*x_i*y_i`,

```
|p_i-t_i| <= gamma_2*|t_i| + eta.
```

Both backends sum these terms using Neumaier's magnitude-ordered error-free
residual and a separately accumulated correction, then round the final sum.
An independent compensated sum accumulates `abs(p_i)`. This retains the unit
term in `[1e16,1,-1e16]`. Plain pairwise summation is not interchangeable here.

The standard compensated bound has second-order row-count dependence; the
conservative coefficient used here is `rho=eps+gamma_(4n)^2`, covering the
correction accumulation and final rounding. The algorithm and error-free
transformation under gradual underflow are discussed in Ogita, Rump and Oishi,
[Accurate Sum and Dot Product](https://www.tuhh.de/ti3/paper/rump/OgRuOi05.pdf),
§§1, 3 and 4 (in particular Proposition 4.5 and Remark 4). We use full epsilon
and `4n`, rather than unit roundoff and `n-1`, to overbound those terms.
Mata uses the algebraically equivalent TwoSum residual to avoid matrix masks.
It accumulates at most 256 independent lanes, then compensates the sum of their
high parts and corrections **separately**, avoiding an extra final rounding
per lane. If a lane has length `m` and there are `L` lanes, the lane correction
error is bounded by `gamma_m^2*A`, and the absolute sum of merge inputs by
`(1+gamma_m)^2*A`. Combining with the `2L`-term compensated merge is bounded by
`eps+gamma_(m+2L+1)^2 <= eps+gamma_(4n)^2`; the full-epsilon allowance covers
the final rounding. For `L=min(256,n)` and `m=ceil(n/L)`,
`m+2L+1 <= n+2n+1 <= 4n` for every positive integer `n`; the
existing coefficient therefore covers the larger block without relaxation.
Zero padding contributes no error. Rust's serial Neumaier
sum satisfies the same conservative bound.

If `a` is the computed compensated absolute sum, put

```
A = up(a/(1-rho))
D = up((rho + gamma_2/(1-gamma_2))*A + n*eta/(1-gamma_2)).
```

Then `|computed_dot - sum(t_i)| <= D`: bound `sum|p_i|` by `A`, solve the
product-error inequality for `sum|t_i|`, and add the compensated-sum error.
Cancellation does not invalidate this absolute bound. Zero and subnormal
products retain an absolute allowance. Overflow of a term, either accumulator,
or its certificate fails closed, even if a real-arithmetic final sum is finite.
Mata uses the literal minimum subnormal: its `2^(-1074)` power expression rounds
to zero. Rust constructs eta from its binary representation.

Rust uses `q x q` scratch; Mata uses at most `256 x q^2` scratch. Rust retains interrupt checkpoints inside the
row loop. Mata evaluates weighted outer products in a breakable fixed-block loop; it
never materializes `n x q x q` terms. The new helper is used for the canonical
Gram, whitening residual and span reconstruction. Unrelated reductions and
solvers retain their arithmetic.

The four persistent Mata lane accumulators occupy at most 8 MiB at `q=32`.
Temporary products, the TwoSum expression and merging need additional storage.
The control-phase allocation forecast is

```
8 * [ n*(24+6q) + (16*min(256,n)+128)*q^2 + 8*min(256,n)*q ].
```

Six row-by-control slots cover canonical matrices and row-product temporaries;
the row-vector reserve is 24 doubles per observation. Sixteen lane-matrix slots
cover the four accumulators, weighted products, replicated operands, and live
TwoSum temporaries; eight lane-vector/control slots cover slices. The 128 small
matrix slots cover the Gram, inverse/Cholesky, projector and merge work. This is
a conservative allocation count, not a process-RSS forecast. Exact control
preparation checks it before building the Gram; the generic resource model
also places it in phase scratch alongside the already prepared solver. The
core and standalone resource runtimes share the same tested count. Advisory
and omitted-budget behavior is unchanged; an explicit strict budget can reject
with `RESOURCE_LIMIT` before the larger workspace is allocated. Resource
runtime identity is `vckss-resource-api12-control-scratch`.

Mata's ordinary interrupt-on-break behavior stays enabled. The public wrapper
propagates return code 1 as UserBreak, clears partial/prior estimates and runs
the normal data, RNG, timer and memory-policy cleanup. It must not relabel that
interrupt as `MATA_RUNTIME_FAILED`.

## Propagation

Let `Ghat` and `D` denote the computed Gram and entrywise error bounds. For its
computed positive diagonal, form

```
B_ab = up(up(max(D_ab,D_ba)/sqrt(Ghat_aa))/sqrt(Ghat_bb))
delta = max_a up(sum_b B_ab)
E_gram = up(2*delta/rcond).
```

The symmetric maximum covers Gram symmetrization, and the maximum row sum
bounds the spectral norm. Dividing in two outward-rounded steps avoids a product of extreme
diagonal scales and covers an underflowed intermediate before the next division. The diagonally scaled matrix has diagonal one up to the
scaling rounding; its largest eigenvalue exceeds 1/2 throughout the certified
range. Consequently its smallest eigenvalue exceeds `rcond/2`, under the
registered scaled-inverse conditioning certificate. The factor two covers
scaling roundoff. `E_gram` must be finite and below 1/4.

The whitening residual is now a bound on the *exact* weighted product of the
stored orthonormal columns: the measured Frobenius residual is increased by
`q*max(D_whitening)`. This also checks the composition of diagonal scaling,
inversion, Cholesky and multiplication. The pre-existing additional contributions
remain: `q*gamma_(2q+1)/rcond` for Cholesky, `gamma_(2q)*(1+1/rcond)` for the
basis product, and `sqrt(q)*relres/(rcond-sqrt(q)*relres)` for the inverse.
A nonpositive inverse remainder withholds. Add these to `E_gram` and the
whitening residual, require the sum below 1/4, and replace it by `E/(1-E)`.
The original rank and measured inverse gates are checked before this step.

Every subsequent anchor uses the same small Gram inverse and projector as
before. Add its inverse forward bound, the dimensioned matrix-product term
`gamma_(6q)*(1+p/max(rcond_anchor,ranktol))`, measured projector idempotence and
symmetry residuals, and the selected-row residual. Require the accumulated
bound below 1/4 after each update. These small products have dimension at most
32; their rounding terms have not been replaced by a row-count summation bound.
After whitening, frequency at least one bounds the row norms. Accepted anchor
rank and inverse gates bound amplification by subsequent projectors. Absolute
underflow in these bounded small operations is dominated by their full-epsilon
terms and the retained score floor; the potentially badly scaled *input*
weighted products instead need the explicit eta terms above.

For a row score use

```
score_error = 2*E + gamma_(2q)*(1+E)^2
uncertainty = max(1e-12, up(4*score_error)).
```

The row norm is bounded by the weighted orthogonal projector. Squaring its
perturbed norm gives the first-order perturbation and product-rounding terms;
the retained factors and `E/(1-E)` cover the higher-order remainder. Four score
envelopes cover the candidate, the global maximum, cross-representation
comparison, and cutoff arithmetic. The maximum is 1-Lipschitz in the infinity
norm of all row scores; it must still include **every** row. The cutoff map
`m-margin*max(1,m)` is also 1-Lipschitz for the registered margin. Selection is
unchanged: choose the first score strictly above that cutoff. Certification
examines that row and all predecessors. Any uncertain comparison in that
prefix withholds, including exact equality; rows after it cannot change the
first eligible row and cannot veto it. The uncertainty must remain below
`margin/4`, with `margin=max(1e-10,1000*ranktol)` unchanged.

Finally form the same canonical matrix `C=U A' (AA')^-1`. Add the anchor identity
residual, anchor inverse forward error, `gamma_(8q)*(1+q/rcond_anchor)`, and the
relative span-reconstruction residual. The latter includes the error of
`U' W C`: its Frobenius bound is `q*max(D_span)`, multiplied by an upper bound
`sqrt(q*(1+whitening_error))` on `||U||_2` and divided by `max(1,||C||_F)`.
The multiplier follows from `W>=I` and the certified whitening residual.
Downstream propagation remains `E/(r_*-E)` through the full-system and relevant
deletion margins, with positive denominators and the unchanged `1e-8` ceiling.

## Independent final-basis certificate

After every existing anchor-selection, rank, inverse, and span gate passes,
API 25 also certifies the computed canonical matrix directly. Let `X` be the
stored original controls, `A=X[selected,.]` their square selected-row matrix,
`C` the unchanged computed canonical matrix, and `T` the stored product of
the whitening and final anchor transforms. All following identities refer to
exact arithmetic on these stored binary64 inputs. Define

```
C* = X A^-1,       R = I-A T,       F = C-X T.
C-C* = (F-C R)(I-R)^-1.
```

The second equality follows by multiplying both sides by `I-R=A T`.
Certification of `||R||<1` establishes invertibility. Anchor decisions have
already been certified; the accumulated selection uncertainty need not be
charged again as an error in `C`. This identity bounds the distance to the
same exact canonical representation and leaves the computed coefficients
unchanged.

For each residual entry, a length-`q` dot product and its absolute-product
sum give an enclosure using `gamma_(2q+2)`, subtraction rounding, and an
absolute `(2q+2)*eta/(1-gamma)` underflow allowance. With outward-enlarged
row and column sums, choose `rho_inf >= ||R||_inf` and
`rho_2 >= max(||R||_1,||R||_inf) >= ||R||_2`. Both must be below one.
The identity then gives three bounds:

```
||C-C*||_F / max(1,||C||_F)
  <= (||F||_F/max(1,||C||_F) + rho_2)/(1-rho_2),
||W^(1/2)(C-C*)||_F / max(1,||W^(1/2) C||_F)
  <= (||W^(1/2) F||_F/max(1,||W^(1/2) C||_F) + rho_2)/(1-rho_2),
||C-C*||_inf <= (||F||_inf + ||C||_inf*rho_inf)/(1-rho_inf).
```

Here `W` is the diagonal matrix of physical frequency weights. The last bound
also encloses every individual coefficient. Positive reductions and norms
are enlarged by `1/(1-gamma_(8nq+128))`; denominator norms are reduced by
the same factor. This allowance covers dot-magnitude reductions, norm
accumulation, square roots and frequency multiplication. Mata scales norms
before squaring; Rust uses successive `hypot` calls. Norm numerators retain
an `nq*eta` underflow allowance. Final short expressions use `up`.

The maximum of these three bounds is an independent forward-error
certificate. The reported error is the smaller of this certificate and the
existing accumulated certificate. An unavailable direct certificate leaves
the existing certificate in force. All previous gates execute first, and the
full-system/deletion propagation and `1e-8` ceiling remain unchanged.

Rust streams rows with interrupt checkpoints and `O(q)` extra scratch;
Mata processes at most 256 rows at once. Small transform matrices fit within
the existing control-phase memory reserve. The extra work is `O(n q^2)`
and is performed once per control preparation, independently of probe count.

Shared fixtures generated by an independent 180-digit Decimal inverse cover
1--32 controls, weak anchors, deliberately perturbed transforms and outputs,
and unequal frequency weights. Both backends must enclose the exact-input
errors; an unusable inverse residual must remain unavailable. A synthetic
age-polynomial/year-control panel exercises exact and JLA public estimation,
movers and eligible stayers, a nonsingular coordinate change, target weights,
caller restoration, and an independent dummy-variable OLS plug-in oracle.

## Certified acceptance of the eight-row witness

The historical `d=.002` eight-row control-anchor test required every route
to refuse because the former cumulative certificate exhausted its conditioning
margin. The direct certificate can admit this case without changing the
registered deterministic acceptance limit. A refusal is still appropriate
when a route cannot certify its own computation.

`tests/oracles/control_anchor.py` independently refits the original dummy
regression after every literal observation or match deletion, using frozen
binary64 inputs and 120-digit Decimal arithmetic. The resulting four corrected
targets are unchanged when precision doubles to 240 digits; a separate
100-digit mpmath implementation gives the same rounded oracle. The test checks
both original and transformed controls, joint and fixed-offset nuisance,
exact and automatic routing, and simultaneous worker/firm/deletion-ID relabeling.
Every accepted result must meet the existing `1e-8*max(1,abs(candidate),abs(reference))`
limit. The largest measured scaled native error is approximately `1.2e-9`.
The test also retains typed withholding on a tenfold more difficult member
(`d=.0002`), including an inverse-residual refusal that can precede the
control certificate. The fuzzy-anchor and rank-boundary tests remain in place.
This updates an implementation-dependent refusal expectation; it changes no
production gate or tolerance.

## Rounding the bounds

`up(x)=fl(fl(x*(1+64*eps))+eta)` for nonnegative short bound expressions.
The multiplicative cushion dominates at most 16 positive binary64 operations
and square roots between applications; the additive eta covers subnormal
rounding. Accumulated Gram row sums apply `up` at each addition. Error denominators
`1-rho`, `1-gamma_2` and `1-E` stay above 3/4 in this path. Thus rounding these
bounds cannot remove the required upper allowance. Inverse-conditioning
remainders retain their separate fail-closed test. Nonfinite norm/residual or
bound evaluations reject. Tests enclose independently computed exact-input
Decimal endpoints; they do not use the production error expression as truth.

## Reporting and evidence

A control-Gram inverse residual failure is publicly `AMBIGUOUS_CONTROL_BASIS`
in both backends. Detail retains `INVERSE_RESIDUAL_FAILED`, its phase, measured
residual and gate. An actual rank failure retains the singularity classification.
Ambiguity means numerical certification was unavailable; it does not demonstrate
model singularity. No centering, ridge, seed alteration or estimator fallback is
performed. The owner's manual file and prior investigation remain unchanged.

The accompanying local checkpoint record distinguishes new validation from
historical accepted evidence. It does not qualify a public binary distribution,
a release, or a remote platform.

## Repeated-eigenvalue rotation direction

The public age/year-control regression also exposed slow convergence in the
small symmetric Jacobi kernel used to certify a match-maker inverse. Its
21-dimensional synthetic matrix has two exchangeable groups (sizes 9 and 12),
19 analytically known repeated eigenvalues and two group-mean eigenvalues.
Its smallest eigenvalue is about 0.749; identification is well supported.

When rounded diagonal entries are equal, a negative off-diagonal entry yields
`tau=-0.0`. The former comparison `tau>=0` selected the positive rotation.
Preserving the sign with `copysign` gives a consistent direction for these
ties. The reproduced matrix then passes the existing residual criterion;
the former path reaches a residual of about `2.409e-12` after 128 sweeps,
above its `1.370e-12` gate. The repair changes neither that gate nor the sweep
limit, eigenvalue enclosure or inverse-residual checks. A direct analytic
spectrum test and twenty deterministic simultaneous row/column permutations
exercise the kernel, and the native public panel compares exact corrected
targets to Mata as well as testing JLA and coordinate invariance.

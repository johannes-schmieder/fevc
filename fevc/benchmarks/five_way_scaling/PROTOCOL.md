# Five-way scaling benchmark protocol

This is a new benchmark namespace. It does not modify or supersede accepted
evidence under `comparative_scaling/`.

## Frozen comparison

- Implementations: FEVC Rust at `f3098bc1369992fccbc1d276aac5fc65ceb3f404`;
  maintained MATLAB LeaveOutTwoWay at `8b957ff`; Julia
  VarianceComponentsHDFE.jl at `fa3d66e`; CRAN LeaveOutKSS 0.1.0; and
  PyTwoWay 0.3.21 at `7208bda`.
- The PyTwoWay Python 3.10 environment pins NumPy 1.26.4, pandas 2.0.3, and
  SciPy 1.13.1.
  PyTwoWay's unconstrained NumPy requirement otherwise permits an ABI-
  incompatible NumPy 2.x resolution with this pandas release. The complete
  transitive environment is retained in the preparation receipt. Its
  documented `solver_tol` default remains 1e-10 and is asserted after
  parameter construction; the driver does not resupply it because PyTwoWay
  0.3.21's override validator incorrectly declares this floating-point option
  as integer-only. PyTwoWay uses its supported Jacobi preconditioner, which
  the package recommends for large inputs; its default incomplete-Cholesky
  implementation fixes Numba sparse-index signatures at 32 bit and fails when
  the pinned SciPy stack supplies 64-bit indices. Solver tolerance, sample,
  estimand, leverage draws, and trace draws are unchanged.
  SciPy 1.13.1 is the newest compatible line used here: it satisfies current
  PyAMG's SciPy >=1.11 requirement and still accepts the `tol` keyword that
  PyTwoWay 0.3.21 passes to `scipy.sparse.linalg.minres`; SciPy 1.14 removed
  that deprecated spelling.
- LeaveOutKSS's compiled `sanic` dependency is installed from the frozen CRAN
  0.0.2 source archive (SHA-256
  `998041c3303f63c3651070c49d9d991118223a8ba4d7e8199ea6c3ce3e38175e`),
  with its build toolchain and Rcpp/RcppEigen requirements supplied by the
  isolated Conda environment. The comparator package source is unchanged.
- The Julia comparator runs under SCC Julia 1.9.1, which satisfies its declared
  Julia 1.8 compatibility floor. Preparation instantiates the upstream
  Julia-1.8.5-generated manifest, accepts Julia's explicit version warning,
  verifies that VarianceComponentsHDFE 0.1.0 imports, and retains the complete
  manifest status. A redundant explicit `Pkg.precompile()` is omitted because
  Julia 1.9 rejects the older manifest's versionless `DelimitedFiles` stdlib
  entry after `Pkg.instantiate()` has already precompiled all dependencies.
- Data: deterministic `strong_d3`; one stored row per unique worker--firm
  match, three firms per worker, no controls, and no stayers. Match and
  observation deletion are therefore the same physical deletion.
- Size sweep: 7,680, 30,720, 122,880, and 491,520 rows at 28 cores.
- Core sweep: 1, 2, 4, 8, 14, and 28 cores at 122,880 rows. The shared
  122,880-row/28-core cell is run once, giving nine unique cells.
- Randomized work: 280 projections. PyTwoWay uses 280 leverage and 280 trace
  draws. Five fixed seeds and a five-position Latin role order yield 45 tasks
  and 225 estimator calls.
- Each confirmation task reserves a complete 32-core Cascade Lake Gold-6242
  node with 8 GiB per core. The largest registered active subset remains 28
  cores, so four reserved cores remain idle. Roles execute sequentially and
  use `taskset` for the registered active-core subset. BLAS/OpenMP libraries
  are capped at one thread wherever the implementation owns a separate worker
  pool. This schedulable replacement is necessary because SCC's active
  `econ_queue_limits` rule assigns zero serial and OpenMP slots to every
  28-core `econ` host, including both Broadwell E5-2680v4 hosts.

The primary elapsed phase begins after CSV import and implementation-neutral
validation, immediately before implementation-specific cleaning, graph/sample
construction, worker-pool startup, or estimator preparation. It ends after
the four corrected targets are extracted. Process-tree RSS is sampled every
100 ms over this phase. Full fresh-process elapsed time and the estimator's
own reported time are retained separately.

MATLAB, Julia, and R use sample covariance (`N-1`) internally. Their raw
targets are multiplied by `(N-1)/N` before comparison with FEVC and PyTwoWay,
which use population `N` target masses. Both raw and normalized values remain
in the task output.

## Exact consistency gate

Before confirmation, every implementation runs at one active core on an
N=960 input with its exact leverage/trace route. A separate dense NumPy oracle
builds the full-rank grounded worker--firm design and independently evaluates
the population-N KSS correction. Every target must satisfy

`abs(result - oracle) <= max(1e-8, 1e-5 * max(1, abs(oracle)))`.

The randomized comparison is descriptive. Reports show normalized estimates,
paired gaps from FEVC, each five-run full range, and whether the implementation
and FEVC ranges overlap. These are not confidence intervals or equality gates:
the packages use different RNGs, projection distributions, parallel stream
partitioning, and finite-JLA formulas.

## Execution and acceptance

Run a complete local generator/manifest/validator test, then SCC preparation,
one two-core N=7,680 smoke through the real wrapper, the exact gate, and one
N=491,520/28-core pilot. Submit confirmation only after all four gates pass.
All five roles are attempted even if one fails. Accept a stage only when all
expected SGE accounting records have `failed=0` and `exit_status=0`, each
application has its success marker, and every task validation is present.

The benchmark produces a standalone Markdown/PDF report and PDF/SVG/PNG
figures. It does not edit the companion paper, publish artifacts, or promote a
package release.

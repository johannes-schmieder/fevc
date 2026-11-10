# Automatic standard errors: numerical prototype

Python prototype behind
[`AUTOMATIC_INFERENCE_PROPOSAL.md`](../../docs/AUTOMATIC_INFERENCE_PROPOSAL.md).
It is exploratory development evidence for a design decision. It is not a
registered campaign, it does not run Stata, Mata or the Rust plugin, and it
qualifies nothing.

## What it contains

| File | Purpose |
|---|---|
| `akm_design.py` | Synthetic worker–firm designs in the pooled match/stayer row convention, leave-one-match-out pruning, and a worker-eliminated sparse solver. |
| `dense_mc.py` | Explicit $n\times n$ kernels and exact noise terms, so estimators can be compared without numerical approximation. |
| `sparse_auto.py` | Matrix-free version of the proposed two-pass calculation: leverages and target diagonals from Rademacher probes, then influence solves and pseudo-noise draws. |
| `run_dense.py` | Monte Carlo comparison of variance estimators on one dense design. |
| `run_sparse.py` | `draws`: numerical noise against the number of draws. `mc`: Monte Carlo of the matrix-free pipeline. |
| `run_bottleneck.py` | Two firm clusters joined by a few movers, to test the concentration diagnostic. |
| `run_scale.py` | One larger design: solve counts, timing and draw-to-draw noise. |
| `check_identities.py` | Compares pseudo-noise averages and the data statistic with closed forms from explicit kernels; exits non-zero on disagreement. |
| `results/` | Console output of the runs quoted in the proposal. |

Estimators in `run_dense.py`:

- `true variances`: the variance formula with the true error variances.
- `current (moment, common)`: the residual-moment fit of the released
  structured route with the `structured_common` basis and an exact Gram
  matrix.
- `model (proxy, …)`: least squares of the leave-out products on the basis.
- `naive plug-in`: raw leave-out products in the signal term.
- `robust (…)`: squared residuals in the signal term with exact noise
  calibration under the fitted model.
- `robust-dr (…)`: the fitted variance plus the residual surprise.

`raw` rows report the estimate as computed; `trunc` rows truncate the signal
part to be positive semidefinite. "SE ratio" is the root-mean-square (or
median) standard error over the Monte Carlo standard deviation.

## Reproducing the results

Requires Python 3.11 or later with NumPy and SciPy. The runs used NumPy 2.5.3
and SciPy 1.18.1 on four cores. Run from this directory.

```bash
python run_dense.py 500 1700 150 5 1000 homo,mass,leverage,firm_omit,worker_omit > results/dense_main.txt
python run_dense.py 300 900 700 8 800 homo,mass,firm_omit,worker_omit           > results/dense_stayers.txt
python run_dense.py 500 1700 150 5 1000 mass,firm_omit 0.03                     > results/dense_weak.txt
python run_dense.py 500 1700 150 5 1000 mass 0.0                                > results/dense_very_weak.txt
python run_dense.py 500 1700 150 5 1000 mass,firm_omit 0.18 t5 6                > results/dense_t5.txt
python run_dense.py 500 1700 150 5 1000 firm_omit 0.18 skew                     > results/dense_skew.txt
python run_sparse.py draws                                                      > results/sparse_draws.txt
python run_sparse.py mc 500 1700 150 5 mass,firm_omit 500 200 8                 > results/sparse_mc_4k.txt
python run_sparse.py mc 5000 22000 2500 11 homo,mass,firm_omit,worker_omit 120 200 8 fast > results/sparse_mc_56k.txt
python run_bottleneck.py 2 600                                                  > results/bottleneck_2.txt
python run_bottleneck.py 12 600                                                 > results/bottleneck_12.txt
python run_scale.py 10000 120000 25000 31                                       > results/scale_359k.txt
python run_dense.py 900 1900 0 13 800 homo,mass,firm_omit,worker_omit            > results/dense_limited.txt
python check_identities.py                                                      > results/check_identities.txt
```

The whole set takes roughly forty minutes. Seeds are fixed in the commands and
scripts, so statistical output repeats on the same library versions; timings
do not.

## Limits

- The solver is a direct sparse factorisation of the firm Laplacian, not the
  package's preconditioned conjugate gradients. Timings say nothing about
  `fevc`; solve counts do.
- Leverage estimates use the normalised form $a/(a+b)$ and a plain
  reciprocal, not the package's finite-probe maker formula.
- No controls, no frequency weights beyond spell mass, no target weights
  other than physical mass.
- Errors are independent across rows by construction.

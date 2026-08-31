# Source and provenance ledger

## Active governing sources

- `fevc/PLAN.md`: active milestone scope and completion gates.
- `fevc/docs/DECISIONS.md`: owner decisions and implementation boundaries.
- `fevc/docs/ESTIMATOR_CONTRACT.md`: current statistical contract.
- `fevc/cmg/docs/SOURCE_PROVENANCE.md`: package-owned CMG source ledger.

The former root-level `varcomp_hdfe_specification.md` was an owner-authored
planning input for the initial implementation and has been superseded and
removed. Historical review packets and handovers retain their copies and
references as provenance; they are not current governing specifications.

## KSS sources

- Kline, Saggio, and Sølvsten (2020), *Leave-out estimation of variance
  components*, Econometrica.
- Published main article:
  `https://eml.berkeley.edu/~pkline/papers/KSS2020.pdf`.
- Published supplementary appendix:
  `https://eml.berkeley.edu/~pkline/papers/KSS2020_SUPP.pdf`.

## Maintained MATLAB oracle

- Repository: `https://github.com/rsaggio87/LeaveOutTwoWay`
- Pinned commit: `8b957ffeb10b8465a3584fceb0265cccc48379e1`
- Main behavior: `codes/leave_out_KSS.m`, blob
  `8873ab04715a1e51adf6619be4df78b3baf28296`.
- Improved leverage code: `codes/leverages.m`, blob
  `1e3a2cccd50246b2f0f68e1202344c324c11f14b`.
- Authors' derivation: `doc/improved_JLA.tex`, blob
  `0ac4d84e4e6ee36dcb54d342706ec354d4cb283d`.
- License status: the repository contains no root license and issue 14,
  “Choose a license,” remains open. No source is copied into this package.

The maintained MATLAB source is a behavior and scientific reference only.
Its missing license means that neither its source nor its binary
`tabulation_10K.mat` critical-value table may be copied into FEVC. Inference
code and any critical-value table generator are repository-authored from the
published KSS formulas and distributed under GPL-3.0-only.

The shipped `fevc_inference.mata` runtime is an independent implementation.
The maintained files `leave_out_COMPLETE.m`,
`leave_out_estimation_two_way.m`, `llr_fit.m`, `AM_CI.m`, `lincom_KSS.m`, and
`leave_out_KSS.m` were inspected only to establish option and return behavior.
No text, table entries, or binary assets from those files are distributed.

The maintained code uses coefficient two on the mixed fourth moment. The
authors' final raw-moment display, independent Hessian calculation, and
simulation use coefficient one. This package implements coefficient one and
retains coefficient two only as a legacy expected-value fixture.

## Data

- Shipped tests use generated synthetic data only.
- Public example-data staging belongs to the sibling paper repository and is
  not a runtime or source dependency of this package.
- No restricted Separations input may be transferred or used in KB0--KB6.
- KSS-PROD-1 has separate owner authorization to use restricted Separations
  wage inputs in place on SCC. Raw and row-level derived data remain on SCC;
  only source/input identities, aggregate estimator output, residual summaries,
  timing, memory, and scheduler evidence may be collected into this repository.

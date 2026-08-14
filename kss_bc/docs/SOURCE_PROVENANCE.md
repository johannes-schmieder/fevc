# Source and provenance ledger

## Governing specification

- Local file: `varcomp_hdfe_specification.md`
- Role: owner-authored functional and numerical requirements.

## KSS sources

- Kline, Saggio, and Sølvsten (2020), *Leave-out estimation of variance
  components*, Econometrica.
- Local computational appendix:
  `application/veneto-kss/docs/DataComputationAppendix.pdf`.
- Historical official MATLAB source:
  `application/veneto-kss/code/upstream/analysis/codes/leave_out_KSS.m`, SHA-256
  `2e705422781da96feb546bc9920f4ebd30df4b7369c8e579493f1cecb9313d31`.
  This imported application material is read-only.

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

The maintained code uses coefficient two on the mixed fourth moment. The
authors' final raw-moment display, independent Hessian calculation, and
simulation use coefficient one. This package implements coefficient one and
retains coefficient two only as a legacy expected-value fixture.

## R translation

- Archive: `application/veneto-kss/docs/LeaveOutKSS_0.1.0.tar.gz`.
- Package author/maintainer: Vahid Moghani.
- Role: secondary adversarial cross-check only; it is not authored by the KSS
  authors and does not override the paper, authors' derivation, or independent
  algebra.

## Data

- Shipped tests use generated synthetic data only.
- The local public Veneto example may be staged for SCC validation but is not
  copied into the package or release artifacts.
- No restricted Separations input may be transferred or used in KB0--KB6.

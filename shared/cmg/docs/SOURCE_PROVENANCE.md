# Clean-room source provenance

## Authorization

The owner requested implementation of the reviewed plan on 2026-08-14. The
source commit at CMG0 startup was
`744ca8ed7271b791a721d1d05011d864d439801a`.

## Mathematical references

The design uses only public mathematical descriptions:

1. Ioannis Koutis, Gary L. Miller, and David Tolliver, *Combinatorial
   Preconditioners and Multilevel Solvers for Problems in Computer Vision and
   Image Processing* (2009/2011).
2. Ioannis Koutis and Gary L. Miller, *Graph Partitioning into Isolated, High
   Conductance Clusters: Theory, Computation and Applications to
   Preconditioning* (SPAA 2008).
3. Standard Schur-complement, graph-Laplacian, Galerkin, Cholesky, and
   preconditioned conjugate-gradient algebra as restated in
   `docs/MATH_CONTRACT.md`.

## Excluded implementation sources

The repository preserves imported upstream CMG files under
`application/veneto-kss/`. They are provenance material and include GPL
implementation notices. CMG authors and reviewers are prohibited from
opening, reading, running, copying, translating, or deriving implementation
choices from those files. Searches may establish only the existence and
licensing boundary; they may not inspect implementation bodies.

No upstream CMG source has been copied into `shared/cmg/`. The hierarchy's
tree aggregation and conductance screen are decision-complete clean-room
choices because the paper does not fully specify its practical tree-splitting
implementation. The v1 one-child symmetric cycle also differs deliberately
from the paper's possible multiple recursive corrections.

## Independent oracle boundary

`oracle/cmg_oracle.py` is a dense development oracle. It is allowed to form
dense matrices and use NumPy. Production Mata code may not import, execute,
or share helpers with it. Tests compare the two implementations through
serialized fixtures and public numerical interfaces.

Initial hashes:

- `cmg_plan.md`: `5a9d648e086b229b62563aec0b431f40ef05aea31f7bb0389633ac998aa29a44`
- `oracle/cmg_oracle.py`: `3beea9b00a79ebbe3b331a14d3481e013f0800a5aa8e38db96ceb28fa4f4415c`
- `tests/test_oracle.py`: `61b9a46546a68a7211ed81a1143c26d3e5ef37d496afc9c04762e3a8b020964f`

These are CMG0 creation hashes, not release hashes. Later milestone reports
must bind the exact source under test.

## Licensing boundary

The root repository has no selected public software license. Development,
internal tests, and private install tests are authorized. Publication,
redistribution, or a public package release is not authorized.

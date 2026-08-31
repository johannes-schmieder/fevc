# CMG source provenance

## CMG-MATA-1 boundary change

On 2026-08-18 the owner explicitly overrode the clean-room restriction for the
CMG modernization work, authorized inspection and a Mata port of the official
implementation, and selected GPLv3 for the code. `CMG-MATA-1` therefore uses
SPDX identifier `GPL-3.0-only`, the conservative reading of the upstream
notice's reference to GNU GPL Version 3.0 without an express "or later"
grant. The pre-existing API 5 Mata implementation remains historically
clean-room-authored; new source-informed work must not be described as
clean-room.

The authoritative maintained source inspected for design review is:

- SCC path: `/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay`
- public repository: `https://github.com/rsaggio87/LeaveOutTwoWay`
- bound upstream commit: `8b957ffeb10b8465a3584fceb0265cccc48379e1`

The Mata implementation must use a file-level manifest before porting code.
It must retain the copyright of Ioannis Koutis and Gary Miller,
the GPL Version 3 notice, modification dates, repository-authored notices, and
exact hashes. The immutable predecessor-era snapshot is retained as
[`rust/vendor/cmg/LEGACY_UPSTREAM_SOURCE_MANIFEST.yaml`](../../../rust/vendor/cmg/LEGACY_UPSTREAM_SOURCE_MANIFEST.yaml);
its former path in the archived VCKSS tree is not a live dependency.

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

## Historical clean-room boundary

Before the owner's 2026-08-18 override, authors of API 1--5 were prohibited
from inspecting upstream implementation bodies. No upstream CMG source was
copied into those versions. Their tree aggregation, conductance screen, and
one-child symmetric cycle were independently specified from the papers. This
history remains true, but it is not a restriction on the authorized Mata
milestone.

## Independent oracle boundary

`oracle/cmg_oracle.py` is a dense development oracle. It is allowed to form
dense matrices and use NumPy. Production Mata code may not import, execute,
or share helpers with it. Tests compare the two implementations through
serialized fixtures and public numerical interfaces.

Initial hashes:

- `docs/history/plans/cmg_plan.md`: `5a9d648e086b229b62563aec0b431f40ef05aea31f7bb0389633ac998aa29a44`
- `oracle/cmg_oracle.py`: `3beea9b00a79ebbe3b331a14d3481e013f0800a5aa8e38db96ceb28fa4f4415c`
- `tests/test_oracle.py`: `61b9a46546a68a7211ed81a1143c26d3e5ef37d496afc9c04762e3a8b020964f`

These are CMG0 creation hashes, not release hashes. Later milestone reports
must bind the exact source under test.

## Licensing boundary

CMG and containing-package code covered by `CODE_LICENSE.md` is selected for
GPL-3.0-only. Research content and data remain outside that grant. Development,
internal tests, and private SCC runs are authorized. Public distribution waits
for complete corresponding source/notices and the plan's final human review.

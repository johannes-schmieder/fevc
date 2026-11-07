# CMG consolidation comparisons — November 6, 2026

These are frozen complete-command comparisons, not released distributions.
Each record retains all 96 native observations, 48 timed commands, six seeds,
four cells, input hashes, scientific checks, and the fixed schedule. There were
no exclusions or retries. The allowance is max(5%, 1 ms) per complete command.

- `fevc-comparison.json`: initial source `dcfb9d64`; match with one and four
  workers failed timing. Both observation cells and all scientific gates passed.
- `fevc-release-profile-comparison.json`: standalone release-profile correction;
  all four cells passed for the recorded development binary. This does not
  qualify a differently built artifact from the same source.
- `fevc-final-native-comparison.json`: exact qualified Mac arm64 binary
  `087a96083191d5194cc247aac21c21791996f76646382a823356b441262878a8`,
  source `b773a94d12cd162e1a65609918a5dff3bc3246be`. Match with one worker
  failed (+9.2502%, 0.5135 to 0.561 seconds); the other three timing cells and
  all scientific gates passed. This failed artifact remains unqualified.

The replacement upstream CMG `e5613e81fb3b03c78de98658b372646fa802aabb`
changes scalar vector and triangular-correction loops while preserving arithmetic
order, storage, and bounded polling. Its debug all-feature and release parallel
suites and strict Clippy pass. Development profiling shows lower vector-update
cost with identical solution bits; these diagnostics do not establish command
acceptance. Newly built exact artifacts require a newly frozen comparison and
platform qualification before release. Earlier results remain unchanged.

The replacement exact build `94d94999be54e9a2309497d5fbe9899e2822cd51`
passes all four cells in `fevc-e561-final-native-comparison.json`. Its qualified
arm64 binary is `d995be6a336ef85dd60d5a06530d1fe888cbb6c5c8f5259870f7e050a65cb4e9`.
Median ratios are 1.03327/1.01718 for match at one/four workers and
1.01555/1.00634 for observation. All statistical and original-residual checks
pass. This is one new fixed schedule for changed upstream code, with no
retries or excluded observations. Mac native/Rosetta and private Windows
qualification pass; Linux native qualification remains in progress.

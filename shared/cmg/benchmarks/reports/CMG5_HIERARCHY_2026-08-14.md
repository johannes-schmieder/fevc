# CMG5 local hierarchy calibration

## Scope and evidence class

This report measures the standalone clean-room Mata hierarchy builder and one
batched V-cycle on unweighted path graphs. It is local feasibility evidence,
not package qualification and not a claim about estimator runtime. The runs
used Stata/MP 18 with eight reported processors on Apple silicon. Each row is
one fresh Stata process and therefore does not meet the repeated-run promotion
protocol in `cmg_plan.md`.

Source binding:

- repository commit at the start of the CMG series:
  `744ca8ed7271b791a721d1d05011d864d439801a`;
- canonical template SHA-256:
  `11e87b2cf1eddbbc5bc457da7190bd10be746319637ce319be8c714475b08c06`;
- generated test artifact SHA-256:
  `7fe4ef883f84906a6e185ec6f71b55d398e66b5bf4c48b1f8b30e5427d3a356a`;
- benchmark driver SHA-256:
  `dd76dd58e251bedf09a06d14baff1956efaa90030373555244aa76312b668978`.

The driver times `cmgtest__hierarchy_build()` and one
`cmgtest__apply()` separately with `timer_value()`. The apply has eight
compatible right-hand-side columns. Structural bytes are the core's explicit
level-accounting estimate; they are not peak RSS.

## Final path measurements

| Vertices | Edges | RHS | Setup (s) | Apply (s) | Levels | Edge complexity | Vertex complexity | Structural bytes | Dense-factor bytes |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 10,000 | 9,999 | 8 | 0.176 | 0.007 | 5 | 1.331633 | 1.332000 | 2,555,616 | 11,552 |
| 100,000 | 99,999 | 8 | 1.812 | 0.057 | 6 | 1.332943 | 1.332990 | 25,589,208 | 73,728 |
| 1,000,000 | 999,999 | 8 | 18.701 | 0.573 | 8 | 1.333305 | 1.333312 | 255,993,032 | 28,800 |

Across this ladder, both setup and apply are approximately linear in graph
size. The reported structural storage is about 256 bytes per fine vertex on
this path fixture. All hierarchy complexity guards pass with substantial
margin.

## Optimization audit

An earlier 100,000-vertex run took 22.168 seconds to build. Vectorizing the
bounded conductance masks reduced this to 6.945 seconds. Two remaining
quadratic scans were then removed:

1. cluster screening now advances through a deterministic cursor instead of
   selecting the first unscreened cluster by scanning all vertices; and
2. component preservation is checked by one sorted panel pass instead of one
   full assignment scan per aggregate.

The final 100,000-vertex setup time is 1.812 seconds. These changes preserve
the aggregation, level counts, complexities, and V-cycle outputs in the
registered correctness tests.

## Limits and next gate

Paths exercise deep, sparse hierarchies but do not cover hubs, duplicate-heavy
contraction, disconnected graphs, extreme weight ratios, or low-conductance
cluster chains. Peak RSS and allocator high-water marks are still absent. CMG5
therefore remains a local candidate until the adversarial graph campaign and
repeated benchmarks pass. Package speed claims require a batched-diagonal B1
comparison and complete `ppml_talo`/`kss_bc` adapter runs.

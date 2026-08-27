# Production full-CMG benchmark

This harness measures the normal-build `CMG_FULL_V2` route against the
maintained MATLAB KSS implementation. It is separate from the frozen private
spike evidence under `../full_cmg_spike/`.

The registered macOS headline uses the exact 1,966,080-row, 327,680-worker,
8,192-firm, degree-six input with 200 probes and four workers. The runner builds
the normal plugin from a clean source SHA under pinned Rust 1.85.1, isolates and
hashes the maintained MATLAB source, runs one cold plus five position-balanced
warm repetitions, and validates the complete production receipt and caller
state after every VCkss command.

```bash
/opt/anaconda3/bin/python3 vckss/benchmarks/full_cmg_production/run_local.py \
  --output-dir /private/tmp/vckss-full-cmg-production \
  --input-csv /private/tmp/vckss-local-matrix-787327f/run/input/input.csv \
  --matlab-root /Users/johannes/Git/varcomp_hdfe/Monte_Carlo/LeaveOutTwoWay
```

Promotion evidence requires a warm median faster than matched MATLAB and no
more than five percent slower than the source-bound private winner at
`598a08d5c0792519b3d87d6f56f743cacbf93a24`. The 2x MATLAB result remains an
optimization objective. Because the maintained MATLAB comparator uses a
different RNG and solver contract and does not report compatible MCSEs, its
four corrected targets receive a clearly labelled descriptive scale check;
VCkss release-blocking statistical gates remain the exact-reference and
common-probe differential suites.

The final explicit-route macOS promotion matrix is summarized in
[`MACOS_HEADLINE_DD39F04.md`](MACOS_HEADLINE_DD39F04.md). It records a
75.485-second VCkss warm median versus 104.648 seconds for MATLAB, with less
than half MATLAB's process-tree peak memory. The source-bound receipts are in
the adjacent `evidence/macos/` tree. The earlier accepted production
checkpoint remains preserved in
[`MACOS_HEADLINE_F0E5D79.md`](MACOS_HEADLINE_F0E5D79.md).

The fixed-CZ18 SCC matrix is submitted only from a clean `main` checkout:

```bash
vckss/benchmarks/full_cmg_production/submit_scc_cz18.sh RUN_ID
```

It builds the ordinary Linux plugin with the VCkss-owned Rust 1.85.1
toolchain, uses Stata/MP 19 and MATLAB R2024b, runs the same cold-plus-five
position-balanced comparison, and writes all artifacts below
`/projectnb/welfgr/vckss/runs/RUN_ID`.

The final explicit-route SCC promotion matrix is summarized in
[`SCC_CZ18_DD39F04.md`](SCC_CZ18_DD39F04.md). Job `7328200` records a
31.365-second VCkss warm median versus 55.148 seconds for MATLAB, with less
than one quarter of MATLAB's process-tree peak memory. Its pinned validation,
task, environment, wrapper, and qacct receipts are under the adjacent
`evidence/scc/` tree.

The normal macOS plugin qualifier passed at source
`011fcc774a7dee9fae1d98b623c0175ce375f2b8`, including thin arm64, thin
x86_64 under Rosetta, universal, clean-install, ABI, lifecycle, routing, and
focused `CMG_FULL_V2` coverage. Its byte-preserved receipt and sanitized
transcript are under the matching SHA in `evidence/macos/`.

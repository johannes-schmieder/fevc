# FEVC/MATLAB 2026 campaign status

This file records the terminal checkpoint for the registered comparative
scaling campaign. The protocol and reproducible harness remain in this
directory. Earlier failed attempts are preserved in Git history and their
source-bound records; they are not instructions to rerun the campaign.

## Accepted execution

- Execution source: `f5532649212bcb95ae6000f600821597f1724822`
- Collector source: `477b72ae8dee6b695f7cf57f1f670f80dddbe5f7`
- Run: `20260901T231921Z-f5532649-submit`
- Diagnostic smoke: job `7410150`
- Dual-boundary pilot: job `7410151.24`
- Production array: jobs `7410152.1-24`

The collector-only compatibility review changed parsing and accounting logic,
not production code, binaries, inputs, manifests, estimator settings,
tolerances, jobs, or result bytes. It admitted SCC's observed `omp28` PE name,
normalized the array accounting token, and separately receipted the collector
source.

## Accepted scope

All 24 production accounting records, 240 registered cells, and 480 rankable
application calls were validated. Pilot and production used one 28-core
`omp28` E5-2680v4 node, 8 GiB per core, linear 28-core affinity, and the
registered eight-hour limit. Smoke and pilot timings were gates and are
excluded from the production comparison.

The campaign is complete. It does not authorize a retry, a broader platform
claim, or a new scientific or release claim. Any reuse against later source
requires the compatibility review described in the repository agent and
testing guidance.

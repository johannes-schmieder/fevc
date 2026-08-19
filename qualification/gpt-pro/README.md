# GPT Pro performance reviews

This directory records current-name, source-bound external reviews of
`varcomp_kss`. Historical predecessor reviews under `reviews/gpt-pro/` remain
immutable.

Each request directory contains the exact prompt, structured request, source
map, source manifest, and uploaded ZIP digest. Source files are embedded only
inside the ZIP so the packet is transportable while the repository retains a
single active source tree. Responses are preserved verbatim with submission
metadata. Adjudications distinguish accepted recommendations from conjecture
and bind any resulting work to new benchmarks and numerical-contract gates.

The MATLAB-parity round uses two independent chats and two independently
identified packets over the same commit and evidence set:

- `VARCOMP-KSS-MATLAB-PARITY-A`: Mata systems and execution-path review.
- `VARCOMP-KSS-MATLAB-PARITY-B`: numerical architecture and algorithm review.

Neither reviewer receives the other review before both responses are saved.

The completed round is recorded in:

- `responses/VARCOMP-KSS-MATLAB-PARITY-A.md`;
- `responses/VARCOMP-KSS-MATLAB-PARITY-B.md`;
- `adjudications/VARCOMP-KSS-MATLAB-PARITY.md`.

The reconciled decision is to begin the `PREP-RHS-1` milestone with
instrumentation, exact-output plan reuse, command-boundary preparation, and
bounded repeated-RHS workspaces. Public prepared-state APIs and higher-risk
solver changes remain separate owner decisions.

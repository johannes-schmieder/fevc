# Windows full-CMG qualification

The 0.5.2 source enables the existing public full-CMG point, automatic-routing,
projection and component-inference interfaces on Windows x86-64. Exact and
explicit diagonal Stata routing keep their existing behavior. There is no ABI
layout change. CMG remains the unmodified common revision
`e5613e81fb3b03c78de98658b372646fa802aabb`, with 128 levels explicitly configured.

**Native qualification passes.** All 18 source-bound private Windows groups
and all 12 registered scientific/performance cells pass for the adopted 0.5.2
DLL. See `native/windows-cmg-20261107/manifest.json` for exact identities and
platform limitations. Published installation checks are recorded separately.

## Fixed campaign

`windows-campaign-protocol.json` freezes the scope, incumbent, fixtures, seeds,
worker counts, schedule and acceptance rules before confirmation measurement.
The six cases cover automatic compressed routing, generic match and observation,
weighted controls, projection and pooled component inference. Each arm/block
uses a fresh licensed process and one warm-up plus one timed complete command
per case/worker cell. All 144 timed commands are retained. Compare the six-block
medians using `max(5%, 1 ms)`; require the registered common-draw scientific
criterion for every target and block. Do not resample a failed candidate.

Freeze `windows-campaign.json` after selecting the exact build source and hosted
Windows artifact. It has schema `FEVC-WINDOWS-CMG-CAMPAIGN-V1`, `cmg_commit`,
`candidate` and `incumbent` objects containing `production_commit` and
`binary_sha256`, `harness_sha256`, `protocol_sha256` and
`performance_driver_sha256`. Record the creation time, hosted build/test run IDs
and the full protocol as well. Hash the complete bytes; every group binds that
same campaign hash. Never modify it after the first confirmation run.

The first registration failed in its first incumbent block because forced
full-CMG generic execution was unavailable in Windows 0.5.1. Its receipt and
original registration remain under `native/windows-cmg-20261107/`. The corrected
comparison uses the incumbent's accepted diagonal solver for generic cases and
mover-only weighted observation deletion in both arms. Candidate observation/both
support remains covered by independent runtime/oracle tests. Inputs, acceptance
thresholds and the new confirmation schedule are fixed before new measurement.
A development pilot uses a separate campaign schema that the release gate rejects.

The incumbent package source is the released 0.5.1 payload at `117d09c5`, whose
binary build source is `94d94999`. Its accepted Windows routes are compared with
new full CMG using the same estimator requests and inputs. The driver asserts
the full-CMG identity and worker count for the candidate. It asserts the old
version and absence of that identity for the Windows incumbent.

## Bounded private runs

Use the existing owner-controlled Windows wrapper and shared lock. Start with
`smoke`; then run `legacy`, `cmg_point`, `cmg_attachments`, `cmg_lifecycle` and
`installation`, plus twelve `performance` arm/block runs. The group inventory
is in `windows-qualification-groups.json` and independently validated by
`windows_qualification.py`. Stata appends each completed test only after its
assertions return successfully. Missing, skipped or duplicate tests fail.

The source archive includes the unchanged candidate production source, the
exact current qualification harness, the frozen campaign, the hosted plugin in
`windows-candidate/`, and the released payload in `windows-incumbent/` where
needed. Audit every overlay against the two source commits before transfer.
Only harness/configuration and those declared package inputs may differ from
the production source. Record the archive commit/hash separately from the
production commit. The collector verifies all transferred file bytes before
execution and after execution, with its established plugin-staging exception.

The clean archive includes `windows-run.json` with exactly these fields:

- `qualification_group`, `campaign_sha256`
- `performance_arm` (`candidate` or `incumbent`), `performance_block` (0–5)
- `production_commit`, `candidate_sha256`
- `hosted_build_run_id`, `hosted_rust_run_id`

Non-performance groups use candidate/block zero. For incumbent performance,
`production_commit`, `candidate_sha256` and hosted provenance describe the
incumbent binary. The `windows-candidate/` slot holds the selected arm's binary.
The frozen-input helper derives the smoke/full profile from the group.

The shared collector now accepts input V3/checks V4 while retaining legacy
input V2/checks V3 validation. Owner-authorized repair cycle 1 passed native
PowerShell/hash/ACL checks and restricted backend acceptance on November 7:
authentication denial, clean smoke, deliberate failure, concurrency rejection,
cleanup and stopped-state verification. Project qualification remains separate. Do not bypass
that boundary or relax its artifact inventory, 900-second timeout, cleanup,
source checks, single-process lock, or automatic stop.

## Acceptance and publication

Collect only the three allowlisted artifacts: the plugin, bounded build JSON
and bounded checks JSON. Retain the wrapper receipt proving source identity,
cleanup and stopped state with each run. Raw Stata startup logs are excluded.

Run `python rust/stata_backend/windows_qualification.py CHECKS...` with all 18
checks files. The aggregator repeats validation, requires one candidate binary
and source, one incumbent binary and source, one harness and one campaign, and
reports every scientific/timing cell with its raw six-block observations.

After all required native platforms and installation gates pass, create an
index with schema `FEVC-WINDOWS-CMG-ADOPTION-V1`, a `campaign` file descriptor
and a `checks` list of 18 descriptors. Each descriptor has `path` and `sha256`,
relative to the adoption manifest directory. Add its descriptor as
`windows_cmg_campaign` to the normal binary adoption manifest. The release
builder independently revalidates the entire campaign and binds it to the
adopted Windows binary and build source. A legacy runtime PASS cannot satisfy
this new release gate.

Rebuild all five distributed plugins from the same source commit, adopt their
exact bytes and corresponding source, then publish 0.5.2 and verify actual
published fresh/replacement installations. Preserve the explicit one/two-core
Windows and Rosetta-versus-native-Intel limitations. No release is permitted
while required Windows evidence is missing or failing.

# Corrected inference: native qualification and pipeline checkpoint

## Decision and exact source

The repaired source `31dd37f2954c02d223ad81175dd4ded7b5840b8d` passes
clean-source macOS native qualification, the complete local tiny campaign,
and the real-launcher one-task SCC smoke. Runs completed on September 4
local time / September 5 UTC. These are prerequisites for the registered
development experiment, not statistical coverage confirmation or public
match-inference promotion.

The repair was implemented at `da43670905c1f02d92e916f807a73265ebbdf082`.
The only intervening commit added the synthetic manual `fevc_bug.do`, its
manual inventory test, and manual README entry. Production inference, native
build inputs, campaign code, registration, seeds, and thresholds did not change.
The example binary remained byte-identical to the repair replay binary:
`3adbbad4193dcf9f62ac097f453d9bf5e851df0b49fdfaf76f92df2486470ce0`.
The exact current source was nevertheless used for both new pipeline receipts
and the native qualifier; no historical q1 coverage claim is carried forward.

The prospective contract remains `inference_repair_v1.json`, and the frozen
campaign is `inference_repair_match_campaign_v1.json`, SHA-256
`9d32eabd8f07152f0b6bb64552ee33c2dd31ba6b355e329e3bd92bc4e1f469d8`.
No cutoffs, designs, exclusions, or outcome keys were changed during these runs.

## Native qualification

Command: `PATH=<Rust-1.85.1-toolchain>/bin:$PATH ./ci/run_ci_profile.sh plugin-build`.
The ordinary profile completed in 231.37 seconds with `status=success`,
`CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`, `PASS_NATIVE`, and
`PASS_ROSETTA`. Stata/MP was version 19, bundle 19.0.115. The receipt covers
pinned Rust format/Clippy/backend tests, C-shim interrupt/error transport,
ABI-header compatibility, thin and universal candidates, both architectures'
public component-inference tests, native lifecycle, routing, and clean install.
Required exports include the additive component-result V4 entrypoint.

The common receipt is
`../../.ci/stata/results/31dd37f2954c02d223ad81175dd4ded7b5840b8d.json`,
SHA-256 `b213365005f9869a1a6dbec8ec51595aba6c99a6b16d64c6d8664cc73157ec4e`.
The sanitized packet is under
`../../rust/qualification/evidence/INFERENCE-REPAIR-MACOS/31dd37f2954c02d223ad81175dd4ded7b5840b8d/`.
It contains no binary or raw licensed Stata log. Candidate hashes are:

| Candidate | SHA-256 |
|---|---|
| arm64 | `21c79c7422a480073ff29d1b29f1a008d53ca2172b7921806dcf7cef431fb179` |
| x86-64 | `0cc823a9e18267ebc2d48d88361f03f081bdd33fb74282b0dcc8768323748060` |
| universal | `0587e44fcaeeee746f50d5b4d8df558e795114dfd4b95daf8b8e994ed4617003` |

This closes the repaired native-build and existing public-interface gate.
It does not qualify Windows, native Intel hardware, SCC's Stata plugin,
internal match coverage, production scale, or a public binary release.

## Complete clean-source local pipeline

The release example and registered runner executed, in order:

1. `run-preflight --profile tiny --root . --output-dir RUN/preflight --binary BINARY`;
2. `create-manifest --profile tiny --root . --output RUN/manifest.json --preflight-receipt RUN/preflight/receipt.json`;
3. `run-task RUN/manifest.json TASK_ID RUN/tasks --binary BINARY`, IDs 1--14;
4. `aggregate RUN/manifest.json RUN/tasks RUN/aggregate`.

`RUN` is `/private/tmp/fevc-repair-31dd37f-tiny`; `BINARY` is
`rust/target/release/examples/inference_repair_match`. All 56 target attempts
are present: 44 computed intervals and 12 retained shared-failure rows
(four each in severe omitted-driver, weak-signal, and null-signal cells).
The receipt is `COMPLETE`, explicitly non-evidentiary for coverage.
The 4,096-probe outcome-free geometry checks pass before manifest creation.

The exact task-manifest hash is
`f7b652fe32a80f44e465dc47ea5c3a0e74f157b2af1e730632ef511783d5cc5a`;
aggregate payload
`a6967ff57f2aac851b946e8ac88363953b5805c80772ed45802a339bf5216d38`;
summaries
`25b4d393ffbe835b05a46125107a7260ede9233b762e431edce8516713ac81f1`;
receipt
`f92af011e4673d4f6f96da9af844b1818450ac36daf12346a8ad8b115cb5670d`.

The focused campaign suite passes all 23 tests, including malformed, missing,
duplicate, partial, mixed-source/binary, fixed-fold, and scientific-failure
inputs. A deliberate real CLI request for task 9999 exits 1 without creating
an output directory. Full Python source checks after the manual inventory
update pass 585 tests, and CMG `assemble.py --all --check` passes. Known
sandbox cleanup warnings concern old pytest temporary directories, not test
assertions. The preceding repair's complete Rust workspace, standalone
backend, strict Clippy, independent geometry/critical-value, and integrated
licensed-Stata checks remain source-compatible because those files are unchanged.

## Real-launcher SCC smoke

Run: `/projectnb/welfgr/vckss/runs/20260905T000530Z-repair-smoke-31dd37f`.
The established deploy and submit scripts staged an immutable synthetic
source bundle and submitted a build → task → aggregate dependency chain.
Each stage requested one slot and 4 GiB; build/task limits were 30 minutes,
aggregation 15 minutes. No host or queue was explicitly selected.

| Stage | Job | Host | Wall seconds | Maximum virtual memory |
|---|---:|---|---:|---|
| Build | 7461649 | scc-ei3 | 45 | 2.589 GiB |
| Task 1 | 7461650 | scc-gd4 | 5 | 110.758 MiB |
| Aggregate | 7461651 | scc-gr4 | 1 | scheduler recorded 0; not a useful measurement |

All three complete accounting records have `failed=0`, `exit_status=0`, one
slot, project `welfgr`, and queue `econ`. Python is 3.13.8 and Rust is 1.85.1.
The two controls-varying replications produce all eight target rows and eight
computed intervals. The aggregate receipt is `COMPLETE`. Collection and local
reaggregation reproduce the raw aggregate and summaries byte-for-byte.
Exact output inventory, row schemas, semantic keys, folds, source, binary,
task receipts, output hashes, and all three application success markers pass.

| Artifact | SHA-256 |
|---|---|
| Source bundle | `ca83f4b426103060c08ee110aaf75ea9fbbe1f756ac9022601733136dadce8db` |
| Task manifest | `ee352330efa20d3b281d7cfadd91515d334478ddadfe06004757a225f0c500e5` |
| Linux example | `42e81d72989d5ac42e97890d9b7b7f75462d69cd677330464c929b7e238b620f` |
| Aggregate | `e28a9adda52788c521f34c0662e6ef7c37c1de3ac291e86347da5a4888c208ec` |
| Summaries | `c1f84d31c7c68e86c515de1f5cebd589ef1f0b129aee156a409e1cb19bda731b` |
| Aggregate receipt | `b42a224c4618461680249123dc1ac92f8468cb7a4d82aad478c7698514051832` |
| Build accounting | `e77b3c702dd4be3b087ebec65adcf327c85e3a54be6dddf8527b011da93b005b` |
| Task accounting | `3aa17c36542d6f2d2ca184b31b0553a1624953cda1fdc3640564c32cefcc3b3d` |
| Aggregate accounting | `7b65ee89055ea777c6bfc03a1fcb89af0061dbcf8a875b10b8548f1abeef420e` |

Collected evidence is under `/private/tmp/fevc-repair-31dd37f-smoke/`.
The initial download could not preserve SCC group/permission bits on macOS;
resuming with `rsync -rt` succeeded, and content hashes passed. No job, source,
seed, setting, or scientific output was rerun or changed. A temporary local
accounting parser was corrected to strip qacct padding before comparison;
the unchanged complete scheduler records then passed. Neither was a campaign
or statistical failure.

## Boundary after this checkpoint

The unchanged registered 14-design, 400-replication development profile may
run at this source. Confirmation still requires its frozen development gates
to pass. Match confirmation, full match q0 confirmation, and fresh independent
observation q1 confirmation remain separate requirements before public match
integration. The q1 campaign's single diffuse q0 comparator is not full q0
qualification. Estimated-control uncertainty remains omitted by choice:
describe the method as a fixed-offset approximation, not valid conditional
inference given the same-sample estimated control offset.

# Source map

| Repository path | SHA-256 | Note |
|---|---|---|
| `varcomp_hdfe_specification.md` | `8fb1c9bd5c5ab1238d05949794d3af9771241dea0ab4dc6c701e3cea9b9184f5` | Governing owner-supplied finite point-estimator specification. |
| `kss_bc/README.md` | `a0769ea9ea3e1d7f5efb1469ee6014950a73fc7c283de9b68db5bf41c90a296f` | Implemented scope and release boundary. |
| `kss_bc/CHANGELOG.md` | `768ce990d1fe6cf3c3a50e00e347014f447ff49cdaaf69046b8fe620b63a08d1` | Candidate history and API10 repairs. |
| `kss_bc/docs/DECISIONS.md` | `6bf9b536af134da061b26c991e4a7c761cd3ad1df9e74766d8c13bb1cc30adbc` | Owner-authorized choices. |
| `kss_bc/docs/ESTIMATOR_CONTRACT.md` | `74b1a845f1fbff8c54f161a5c781ec6d63e84ec14267e3e156fae09086c990da` | Exact estimator, nuisance, deletion, target, and invariance contract. |
| `kss_bc/docs/JLA_FINITE_PROJECTION.md` | `27bbf09bd2a67c0278ab5f12c9162f195efd06c3d017f1711a44fff8f0836e56` | Literal-copy finite-projection derivation and conceptual stream. |
| `kss_bc/docs/BLOCK_CONTROL_DERIVATION.md` | `41dd2c7f837e728b56a45ede24812300f01a45d7ffcdcd971a97eaa8defe58ba` | General controls, inverse expansion, and deterministic rank certificate. |
| `kss_bc/docs/NUMERICAL_ARCHITECTURE.md` | `40304202c59d0ed7b7649afe9014390382e8b44d7a2e561caa0978e592258e54` | Matrix-free solver, relative gates, graph, and runtime architecture. |
| `kss_bc/docs/FAILURES_AND_RETURNS.md` | `80c23409f3a7d0647dfd74ebbe370148087777aa08eafbc2da7a81c88977bbe5` | Typed fail-closed and posted-result contract. |
| `kss_bc/kss_bc.sthlp` | `f8b4868b67cc4b198883d72f683be4bbe04f976ed19c54bdf804621851808304` | User-facing command and limitations. |
| `kss_bc/kss_bc.ado` | `799619c4c92958c3c5c854d75f80ef884754d2d60b1ee6057974aec476adfbaf` | Complete public validation, invariant-order gate, graph dispatch, backend dispatch, and posting. |
| `kss_bc/kss_bc.mata` | `2213f11e952c18c21e95fd5bb090fe734c6279860ef3fab60d303aa8b8e23ef3` | Complete API10 exact, matrix-free, control, JLA, target, and graph implementation. |
| `kss_bc/tests/python/oracle.py` | `38c7f68b9e823d9dd76ea0470b5935a16ed37c159f9d0f0c9a682c3f04f5589f` | Independent literal-expanded dense oracle implementation. |
| `kss_bc/tests/python/test_dense_oracle.py` | `f3588709c28b77a84c7632552d3c1915facf2c47ab0feac32c24a1f37f965a93` | Dense refit and literal-expansion oracle comparisons. |
| `kss_bc/tests/python/test_frequency_probes.py` | `ca4b93fa638e4b25591f64c54200712a4780befa1333223931d1c68292e3830f` | Exhaustive physical-copy sufficient-statistic and contraction identities. |
| `kss_bc/tests/python/test_jla_formula.py` | `088c48286024cbabae07e11c4211307b830b57497e51f4327b7ca0c53df25477` | Coefficient-one delta algebra and copywise finite-probe simulation. |
| `kss_bc/tests/python/test_block_controls.py` | `ee7132764426caf92a66bc9cbbe94defb22f9a82f2136febb544825c24b7d721` | Noncommuting block-control and rank-certificate counterexamples. |
| `kss_bc/tests/python/test_monte_carlo_bias.py` | `909391337ea3154e66d1d1a1b0e16901a6d95fa8caf9906227d38a36d26c20da` | Independent coefficient-one Monte Carlo falsification. |
| `kss_bc/tests/python/test_package_layout.py` | `dcd7b399ef2ffa44a058539472691b1cf791dfb662368244f673040a10059e63` | Static audit of production tokens, relative residual gates, and forbidden shortcuts. |
| `kss_bc/tests/stata/test_load.do` | `bdc6d85e7d5327974ca5ed308a0e4b87a967bb5c825a4d430aab61b50644790e` | API10 semantic token and scale-relative residual regression. |
| `kss_bc/tests/stata/test_exact_fixture.do` | `1cf88e06d0ca16cf5759a69a7854ff7cab649be113b3643e1403b70501294152` | Public dense exact overlap fixture. |
| `kss_bc/tests/stata/test_frequency.do` | `a2c73bc86d1e5906e64c7a5d84690ec05a199473defbdcf7fbcde3828cac8317` | Fixed-seed split/regroup, ID relabeling, control-transform, and minimal six-copy invariance tests. |
| `kss_bc/tests/stata/test_failures.do` | `39e80746262229aebca30c4eb4e1af718ed853ffb3ec5805eaac14bf1e6f5ac6` | Ambiguous ordering and fixed-offset full-rank false-acceptance regression fixtures. |
| `kss_bc/tests/stata/test_graph_pruning.do` | `c26d57e7eaf95a1a56a0d1123986ce136f4de18b074889b9a6c62cd14c85a963` | Graph pruning, relabeling, and component-tie tests. |
| `kss_bc/tests/stata/test_semantics.do` | `e1a143bde0d93f19cde1cfdc6db9351ee1dadcbbfbeb519491ce86bccd161309` | Deletion, target, mover/stayer, and input-semantics tests. |
| `kss_bc/tests/stata/test_jla_fixture.do` | `7d76f18a9fdd818cf50300e75165506873a9bd384f013ce681aa18d7cf11fc80` | Exact/JLA overlap and successful joint/fixed-offset certificate diagnostics. |
| `kss_bc/tests/stata/test_jla_convergence.do` | `503b62c6818951d719bcf0eb4741efc584328fc82c6e14b0c2c20bcc7156370d` | Probe convergence and conditional MCSE evidence. |
| `kss_bc/tests/stata/test_stale_runtime.do` | `b0c61fed9723afc335519f700ad4f291323fd5b57b4dea71b35ba6cd878c185c` | Same-level foreign-runtime rejection. |

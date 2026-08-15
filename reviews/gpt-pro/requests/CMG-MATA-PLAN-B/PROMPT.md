# External proof-review request: Production-grade pure-Mata CMG plan for two-way fixed effects

You are an adversarial mathematical reviewer. Reconstruct and challenge the supplied proof step. Do not assume the author's conclusion. Distinguish an exposition omission from a genuine missing argument.

## Milestone and obligation

- Milestone: CMG0
- Obligation: CMG-MATA-ARCHITECTURE-PLAN
- Critical: True

## Question

Act as a senior numerical-linear-algebra, graph-algorithms, and Mata/Stata performance engineer. Produce an adversarial, decision-complete implementation plan—not implementation code—for a production-grade pure-Mata combinatorial multigrid (CMG) solver for the two-way fixed-effect systems in ppml_talo and kss_bc. Do not assume that CMG is the right answer: identify conditions under which the existing diagonal-preconditioned PCG should remain the default, and define evidence-based routing and abandonment rules. The objective is one canonical clean-room Mata core, assembled into each package's standalone artifact, that is highly efficient yet robust and numerically stable for ppml_talo's frozen PPML weights and kss_bc's repeated improved-JLA solves. reghdfe, ppmlhdfe, three-or-more fixed effects, heterogeneous slopes, and estimator changes are out of scope.
Both packages eliminate worker effects from the weighted two-way dummy normal equations. For firm coefficients they solve the singular mobility-graph Laplacian
    S_F = F' W F - F' W D (D' W D)^(-1) D' W F,
with connected-component nullspaces, grounding/quotient conventions, and recovery of worker effects. The current production method is PCG with the exact Schur diagonal. ppml_talo already supports lockstep multi-right-hand-side PCG; kss_bc batches estimator work but currently solves matrix columns separately. The supplied benchmarks include a large stable case that converges in about 19 iterations and a weak ring case that reaches the 5000-iteration cap. Plan for the whole lifecycle: construction, application, batching, diagnostics, fallbacks, validation, benchmarking, package assembly, and later red-team review.
The architecture decision must explicitly compare (i) an explicitly assembled worker-eliminated firm clique graph, (ii) the original weighted bipartite graph, (iii) an implicit clique/hypergraph representation, and (iv) a deterministic hybrid. Choose a primary representation and deterministic routing rules. Quantify asymptotic and practical setup cost, V-cycle cost, memory, data passes, clique fill, and reusable workspaces. Explain how CMG should interact with scalar and batched PCG and when a simple exact-Schur-diagonal path wins.
State the mathematical obligations precisely: Schur-complement and graph equivalence; nullspaces by connected component; grounding versus quotient solutions; deterministic aggregation; restriction and prolongation; Galerkin coarse operators; preservation of symmetry, positive semidefiniteness, connectivity, and nullspaces; smoother choice and damping; pre/post smoothing; coarsest-level solution; and conditions making the full V-cycle a symmetric positive-definite preconditioner on the grounded/quotient space so ordinary PCG remains valid. Cover scalar and multiple right-hand sides, floating-point edge cases, residual replacement, stagnation, breakdown, overflow/underflow, attainable precision, full-system residual gates, and deterministic fallback.
Give a concrete Mata design: APIs and structs/classes, compact group/edge storage, hierarchy construction, duplicate-edge collapse, level operators, symmetric V-cycles, scalar and matrix RHS paths, active-column handling, deterministic ordering, workspace reuse, instrumentation, two thin package adapters, and a reproducible assembly mechanism that puts the same canonical source into both standalone packages. Identify which operations can be vectorized efficiently in Mata and which are likely interpreter or memory-bandwidth bottlenecks.
Design an independent test program. It must include exhaustive small dense pseudoinverse or grounded-Cholesky oracles; exact Schur-action comparisons; scalar-versus-separate-versus-batched solves; quotient, grounding, permutation, relabeling, observation-order, batch-partition, and RHS-scale invariance; and final-estimator equivalence. Include adversarial one- and two-firm cases, trees, paths, rings, stars, expanders, lollipops, barbells, chains of dense clusters, hubs, multiedges, high worker mobility, clique-fill explosions, disconnected components, extreme positive weights, cancellation-heavy and nearly-null RHSs, dependent/zero columns, ill-conditioned coarse levels, false convergence, and forbidden-allocation checks. Specify direct numerical tests of preconditioner symmetry and positive curvature.
Design reproducible benchmarks against the current diagonal-PCG baseline. Separate setup, solve, and total time; report the distribution of PCG iterations, V-cycles/operator passes, level vertices/edges/reduction ratios, estimated hierarchy/workspace bytes, phase wall times, peak RSS where available, and a batch-size ladder. Cover stable, moderate, and weak graphs from roughly 10,000 to 10,000,000 observations on Stata/MP 18 on Apple silicon and Stata/MP 19 on the BU SCC. Freeze seeds, tolerances, accounting rules, and source hashes; retain failures. Pre-register explicit go/no-go thresholds for correctness, easy-case slowdown, moderate-case gain, weak-ring rescue, memory, cross-version reproducibility, and automatic routing.
Make milestones small and gated: baseline/profiler; mathematical contract; independent dense oracle; hierarchy builder; symmetric V-cycle; scalar then batched PCG; ppml_talo adapter; kss_bc adapter; adversarial numerical campaign; SCC qualification; standalone assembly/release. For every milestone list exact deliverables, acceptance gates, failure/rollback rules, dependencies, and the artifact that makes the result independently inspectable. Include four future independent GPT Pro red teams with no cross-contamination: two after the math and design are candidate-complete, focusing on equivalence, hierarchy, quotient/grounding, and SPD validity; and two after code/tests are candidate-complete, inspecting the complete Mata implementation and accepted execution paths for false convergence, memory blowups, and executable counterexamples. Model review is evidence, not human certification.
Use the supplied repository evidence rather than generic advice. Cite packet paths and line ranges for every factual claim. Explicitly flag uncertain Mata capabilities or performance assumptions that require a microbenchmark.

## Assumptions you may use

- Runtime must be pure Stata/Mata 18 and 19; no compiled plugin, external compiler, MATLAB, Python, Julia, network service, or runtime internet dependency. Development-time oracle tools may be used only for independent validation.
- This is a clean-room design. Do not copy or translate imported GPL CMG source. The primary external mathematical reference is Koutis, Miller, and Tolliver, Combinatorial preconditioners and multilevel solvers for problems in computer vision and image processing, https://www.cs.cmu.edu/~jkoutis/papers/cviu_preprint.pdf. Identify any additional primary references that implementers must consult.
- Preserve both estimators exactly: samples, singleton/component/deletion rules, targets, probe laws, random streams, tolerances, residual gates, warnings, and return contracts may not change merely to accommodate CMG.
- No hidden ridge, silent generalized inverse, silent component dropping, or tolerance relaxation is permitted. Approximate solves must pass the existing full normal-equation residual and validity gates.
- Prohibit observation-by-observation, observation-by-parameter, and unbounded dense firm-by-firm allocations. Require approximately linear memory in observations plus retained graph/hierarchy size, with explicit guards for clique fill and coarse densification.
- Per-RHS convergence, breakdown, and validity status must survive batching. Reproducibility must cover fixed seed, batch partition, input order after canonicalization, and Stata 18/19 within declared floating-point tolerances.
- Public and synthetic data are the qualification boundary. The low-dimensional control Schur complement is unchanged and out of the CMG core.
- One canonical Mata source should be assembled into two local standalone package artifacts until a unified package exists; the assembly must be hash-checked and drift-detecting.

## Review constraints

- Use only the supplied packet unless a standard theorem is explicitly identified and stated.
- Check quantifiers, conditioning, uniformity, exceptional events, normalization, and rate arithmetic.
- Search for counterexamples and hidden dependence.
- Do not certify the entire paper from a local argument.
- Cite packet file paths and line ranges for every criticism.

## Required output

- Use exactly these top-level headings: Verdict; Assumptions; Findings; Counterexample search; Repairs; Uncertainty.
- Under Verdict, give one of valid, valid_with_repairs, unresolved, or false, then a one-paragraph executive assessment of whether a pure-Mata CMG project is justified.
- Under Assumptions, list every premise required for the chosen representation and SPD argument, plus every Mata capability or cost assumption needing measurement.
- Under Findings, provide the representation comparison, selected architecture and routing policy, mathematical contract, concrete Mata API/data-layout design, benchmark design with numeric go/no-go thresholds, test matrix, milestone/dependency table, risk register, effort estimate by milestone and in total, and expected speedup ranges by graph regime and batch size. Distinguish evidence from conjecture.
- Under Counterexample search, actively try to break graph equivalence, SPD symmetry, component handling, coarse solves, batching, determinism, linear-memory claims, and the claimed performance case; give minimized counterexample families where possible.
- Under Repairs, state decision-complete changes needed before coding, exact stop/rollback rules, and the four independent post-design/post-code GPT Pro review packets.
- Under Uncertainty, identify unresolved mathematical, Mata-version, memory, runtime, packaging, licensing, and benchmark questions and the cheapest experiment that resolves each.
- Cite exact packet file paths and line ranges for all repository-specific claims. Do not infer correctness from tests alone, and do not assign checked or independently_checked status.

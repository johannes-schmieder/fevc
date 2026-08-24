# Public `algorithm(auto)` routing audit

Date: 2026-08-24

This generated checkpoint enumerates the current public/native routing and
receipt-reconciliation anchors that mention requested or selected algorithm
semantics. It is deliberately read-only: no estimator, planner, receipt, or
test behavior is changed by this commit.

The implementation milestone must preserve pre-RNG routing and reconcile the
actual selected result family. In particular, an `algorithm(auto)` request
must not be treated as a JLA request merely because JLA is selected, and an
exact selection must not be forced through generic or compressed JLA result
semantics.

## Matched anchors

### `varcomp_kss/README.md`

- L17:    `backend(rust) rng(counter_v1) algorithm(jla) preconditioner(diagonal)` and a
- L229:        algorithm(jla) engine(compressed) preconditioner(diagonal) batch(8) ///
- L239:        algorithm(jla) nuisance(joint) targetweight(target_mass) ///

### `varcomp_kss/_vckss_rust_plan_receipt.ado`

- L5:        local integer_names plan_struct plan_schema plan_alg_schema plan_alg_req ///
- L6:            plan_alg_sel plan_alg_reason plan_eng_schema plan_eng_req           ///
- L138:            local plan_names plan_alg_req plan_alg_sel plan_eng_req plan_eng_sel ///

### `varcomp_kss/_vckss_rust_post_comp_v7.ado`

- L23:            requested_algorithm_code:h_algreq selected_algorithm_code:h_algsel ///
- L59:            plan_algorithm_requested:h_planalgreq                          ///
- L450:        ereturn scalar rust_requested_algorithm_code = `h_algreq'
- L451:        ereturn scalar rust_selected_algorithm_code = `h_algsel'
- L490:        ereturn scalar rust_plan_algorithm_requested = `h_planalgreq'

### `varcomp_kss/_vckss_rust_reconcile_comp_v7.ado`

- L5:            rank_tolerance block_tolerance algorithm_requested nuisance_code route_requested        ///
- L20:            requested_algorithm_code:r_alg_req selected_algorithm_code:r_alg_sel ///
- L67:            plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel          ///
- L93:            rank_tolerance block_tolerance algorithm_requested nuisance_code route_requested           ///
- L122:        if `ok' & (!inlist(`algorithm_requested',0,2) |                 ///
- L245:                `r_alg_req'==`algorithm_requested' & `r_alg_sel'==2 & `r_eng_req'==0 & `r_eng_sel'==1 & ///
- L330:                `r_plan_alg_req'==`algorithm_requested' & `r_plan_alg_sel'==2 &                 ///
- L381:        return scalar requested_algorithm_code = `r_alg_req'
- L382:        return scalar selected_algorithm_code = `r_alg_sel'
- L451:        return scalar plan_algorithm_requested = `r_plan_alg_req'
- L452:        return scalar plan_algorithm_selected = `r_plan_alg_sel'

### `varcomp_kss/benchmarks/README.md`

- L230:      `algorithm(auto)` and forced-JLA/CMG smokes in Stata 18 and 19, plus

### `varcomp_kss/benchmarks/cmg_mata_command_matrix.do`

- L74:            targetweight(target_weight) algorithm(jla) engine(compressed)      ///

### `varcomp_kss/benchmarks/estimator_cmg_benchmark.do`

- L76:        deletion(match) algorithm(jla) probes(`probes') batch(8) ///

### `varcomp_kss/benchmarks/fe_buf1/cz18_driver.do`

- L45:        deletion(match) probeorder(observation_key) algorithm(jla)       ///

### `varcomp_kss/benchmarks/fe_buf1/local_driver.do`

- L56:            probeorder(observation_key) algorithm(jla) engine(compressed)     ///

### `varcomp_kss/benchmarks/fe_buf1/scc_driver.do`

- L61:            probeorder(observation_key) algorithm(jla) engine(compressed)     ///

### `varcomp_kss/benchmarks/local_processor_scaling.do`

- L103:            probeorder(observation_key) algorithm(jla) nuisance(joint) ///

### `varcomp_kss/benchmarks/numopt2_batch_local.do`

- L36:        probeorder(observation_key) algorithm(jla) engine(compressed)       ///
- L53:            probeorder(observation_key) algorithm(jla) engine(compressed)   ///

### `varcomp_kss/benchmarks/numopt2_local.do`

- L54:            probeorder(observation_key) algorithm(jla) engine(compressed)     ///

### `varcomp_kss/benchmarks/paper_matlab_scaling/stata_run.do`

- L61:        probeorder(observation_key) algorithm(jla) engine(compressed)     ///

### `varcomp_kss/benchmarks/prep_bnd1/local_driver.do`

- L94:                deletion(match) probeorder(observation_key) algorithm(jla)  ///
- L102:                probeorder(observation_key) algorithm(jla) engine(compressed) ///

### `varcomp_kss/benchmarks/prep_bnd1_matlab/stata_run.do`

- L52:        probeorder(observation_key) algorithm(jla) engine(compressed)     ///

### `varcomp_kss/benchmarks/profile_batch_runtime.do`

- L58:            probeorder(observation_key) algorithm(jla) nuisance(joint) ///

### `varcomp_kss/benchmarks/scc/kss_prod_driver.do`

- L198:    local algorithm_requested = cond("`stage'" == "install_auto", "auto", "jla")
- L199:    local command_options "deletion(match) algorithm(`algorithm_requested') probes(`probes')"
- L242:        byte installed_public_path str8 algorithm_requested str8 algorithm_selected ///
- L394:            (`installed_public_path') ("`algorithm_requested'") ///

### `varcomp_kss/benchmarks/scc/kss_scale_driver.do`

- L171:    local command_options "`command_options' algorithm(jla) engine(auto)"

### `varcomp_kss/benchmarks/separations_wage_estimator.do`

- L53:            deletion(match) algorithm(jla) probes(`probes') batch(8) ///

### `varcomp_kss/benchmarks/synthetic_benchmark.do`

- L68:        algorithm(jla) nuisance(joint) targetweight(target_mass) ///

### `varcomp_kss/tests/equivalence/equivalence_driver.do`

- L671:            targetweight(target) algorithm(jla) nuisance(joint)        ///
- L687:            targetweight(target) algorithm(jla) nuisance(fixedoffset)  ///
- L703:            targetweight(target) probeorder(atom_key) algorithm(jla)   ///
- L720:            deletion(match) algorithm(jla) engine(generic)             ///
- L736:            deletion(match) algorithm(jla) engine(generic)             ///
- L752:            deletion(match) algorithm(jla) probeorder(observation_key) ///
- L798:            deletion(observation) algorithm(jla) probes(5) nodisplay
- L813:            targetweight(target) probeorder(atom_key) algorithm(jla)    ///

### `varcomp_kss/tests/stata/test_backend_routing.do`

- L281:        deletion(match) algorithm(jla) engine(compressed)           ///
- L300:        deletion(match) deletionid(match) algorithm(jla)            ///
- L319:        deletion(match) algorithm(jla) engine(compressed)           ///
- L339:        deletion(match) algorithm(jla) engine(compressed)           ///
- L390:            deletion(observation) nuisance(fixedoffset) algorithm(jla) ///

### `varcomp_kss/tests/stata/test_batch_invariance.do`

- L51:                algorithm(jla) nuisance(`nuisance_mode') preconditioner(diagonal) ///
- L61:                algorithm(jla) nuisance(`nuisance_mode') preconditioner(diagonal) ///
- L75:                algorithm(jla) nuisance(`nuisance_mode') preconditioner(diagonal) ///

### `varcomp_kss/tests/stata/test_batch_memory_limit.do`

- L30:        deletion(observation) algorithm(jla) probes(2) batch(8)       ///
- L46:        deletion(observation) algorithm(jla) probes(2) batch(8)       ///

### `varcomp_kss/tests/stata/test_failures.do`

- L100:        deletion(observation) algorithm(jla) probes(5) nodisplay
- L123:                algorithm(jla) probes(5) nodisplay
- L129:                algorithm(auto) exact_limit(100) nodisplay
- L135:                algorithm(auto) exact_limit(2) probes(5) nodisplay
- L143:        deletion(match) deletionid(match) algorithm(jla) ///
- L151:        algorithm(jla) probes(5) nodisplay
- L165:        firm(firm) deletion(match) deletionid(match) algorithm(jla) ///
- L173:        deletion(observation) algorithm(jla) probes(5) ///
- L178:        deletion(match) deletionid(match) algorithm(jla) probes(5) ///
- L183:        deletion(match) deletionid(match) algorithm(auto) exact_limit(2) ///
- L191:        deletion(observation) algorithm(jla) probes(5) tolerance(.09) nodisplay
- L241:        deletion(observation) algorithm(jla) probes(5) seed(1) nodisplay
- L269:        algorithm(jla) nuisance(fixedoffset) probes(20) seed(7) nodisplay
- L275:        deletion(observation) targetweight(target) algorithm(jla) ///
- L293:        algorithm(jla) nuisance(fixedoffset) probes(20) seed(7) ///
- L300:        deletion(observation) targetweight(target) algorithm(jla) ///
- L331:        deletion(match) deletionid(match) algorithm(jla) probes(2) ///
- L361:        deletion(match) deletionid(match) algorithm(jla) probes(2) ///
- L394:        deletion(match) deletionid(match) algorithm(jla) probes(20) ///

### `varcomp_kss/tests/stata/test_forced_cmg_e2e.do`

- L25:        algorithm(jla) engine(generic) preconditioner(diagonal) ///
- L42:        algorithm(jla) engine(generic) preconditioner(cmg) memory_gib(4) ///

### `varcomp_kss/tests/stata/test_frequency.do`

- L37:        algorithm(jla) probes(4000) batch(13) seed(20260818) ///
- L53:        deletion(observation) targetweight(target) algorithm(jla) ///
- L70:        algorithm(jla) probes(4000) batch(13) seed(20260818) ///
- L74:        deletion(observation) targetweight(target) algorithm(jla) ///
- L89:        deletionid(match) targetweight(expanded_target) algorithm(jla) ///
- L104:        targetweight(expanded_target) algorithm(jla) probes(4000) batch(13) ///
- L128:        deletion(observation) targetweight(target) algorithm(jla) ///
- L139:        deletion(observation) targetweight(target) algorithm(jla) ///

### `varcomp_kss/tests/stata/test_graph_pruning.do`

- L134:        algorithm(jla) engine(compressed) probes(8) batch(3)          ///

### `varcomp_kss/tests/stata/test_install.do`

- L89:        algorithm(jla) preconditioner(cmg) probes(4) batch(4) ///

### `varcomp_kss/tests/stata/test_jla_convergence.do`

- L36:            deletionid(match) algorithm(jla) nuisance(joint) ///
- L43:            deletionid(match) algorithm(jla) nuisance(joint) ///

### `varcomp_kss/tests/stata/test_jla_fixture.do`

- L25:        deletionid(match) algorithm(jla) nuisance(joint) ///
- L62:        deletionid(match) algorithm(jla) nuisance(joint) ///
- L68:        deletionid(match) algorithm(jla) nuisance(joint) ///
- L73:        deletionid(match) algorithm(jla) nuisance(joint) ///
- L78:        deletionid(match) algorithm(jla) nuisance(fixedoffset) ///
- L92:        algorithm(jla) nuisance(joint) probes(2000) seed(20260814) ///

### `varcomp_kss/tests/stata/test_perf_batch_runtime.do`

- L34:        algorithm(jla) preconditioner(diagonal) memory_gib(4) probes(`probes') ///
- L55:        algorithm(jla) preconditioner(diagonal) memory_gib(4) probes(`probes') ///

### `varcomp_kss/tests/stata/test_probe_order.do`

- L23:        algorithm(jla) probes(40) batch(1) seed(8675309)              ///
- L35:        algorithm(jla) probes(40) batch(17) seed(8675309)             ///
- L46:        algorithm(jla) probeorder(duplicate_key) probes(40) batch(8)  ///
- L55:        algorithm(jla) probeorder(incomplete_key) probes(40) seed(8675309) ///
- L64:        deletion(match) algorithm(jla) probes(40) batch(8)            ///

### `varcomp_kss/tests/stata/test_routing_api.do`

- L17:        algorithm(jla) probeorder(observation_key) probes(40) ///
- L36:        algorithm(jla) probeorder(observation_key) probes(40) ///
- L50:        algorithm(jla) probeorder(observation_key) probes(40) ///

### `varcomp_kss/tests/stata/test_rust_exact_controls.do`

- L43:    varcomp_kss_rust requestcapability, algorithm(jla) deletion(match)   ///
- L49:    varcomp_kss_rust requestcapability, algorithm(jla) deletion(match)   ///

### `varcomp_kss/tests/stata/test_rust_generic_jla.do`

- L27:    varcomp_kss_rust requestcapability, algorithm(jla) deletion(match)   ///
- L47:                quietly varcomp_kss_rust requestcapability, algorithm(jla) ///
- L72:                quietly varcomp_kss_rust solve `handle', algorithm(jla)          ///
- L143:    quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
- L154:    capture noisily varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
- L175:    quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
- L273:        quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
- L300:        quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
- L318:    quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
- L337:        quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
- L361:        quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
- L413:        quietly varcomp_kss_rust requestcapability, algorithm(jla)             ///
- L429:        quietly varcomp_kss_rust solve `handle', algorithm(jla)                ///

### `varcomp_kss/tests/stata/test_rust_mata_diagnostic.do`

- L33:        deletion(match) deletionid(deletion) algorithm(jla) probes(200)      ///

### `varcomp_kss/tests/stata/test_rust_planned_compressed.do`

- L25:    quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
- L57:    quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
- L90:    assert r(requested_algorithm_code) == 2
- L91:    assert r(selected_algorithm_code) == 2
- L144:    assert r(plan_algorithm_requested) == 2
- L182:    assert r(requested_algorithm_code) == 2
- L183:    assert r(selected_algorithm_code) == 2
- L203:    assert r(plan_alg_req) == 2
- L204:    assert r(plan_alg_sel) == 2

### `varcomp_kss/tests/stata/test_rust_planned_compressed_post.do`

- L91:    assert e(rust_requested_algorithm_code) == 2
- L92:    assert e(rust_selected_algorithm_code) == 2
- L179:    // The native V3 planner also accepts an explicit algorithm(auto) request.
- L189:        "varcomp_kss outcome [fw=frequency], backend(rust) algorithm(auto) engine(auto)" ///
- L198:    assert e(rust_requested_algorithm_code) == 0
- L199:    assert e(rust_selected_algorithm_code) == 2
- L200:    assert e(rust_plan_algorithm_requested) == 0
- L235:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L306:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L384:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///

### `varcomp_kss/tests/stata/test_rust_planned_v4.do`

- L25:    quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
- L55:    capture quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
- L68:    capture quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
- L81:    quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match)   ///
- L98:    assert r(requested_algorithm_code) == 2
- L99:    assert r(selected_algorithm_code) == 2
- L122:    assert r(plan_alg_req) == r(requested_algorithm_code)
- L123:    assert r(plan_alg_sel) == r(selected_algorithm_code)
- L207:    quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
- L223:    quietly varcomp_kss_rust solve `cmg_handle', algorithm(jla) deletion(match) ///

### `varcomp_kss/tests/stata/test_rust_public.do`

- L42:        targetweight(target) algorithm(jla) engine(compressed)      ///
- L160:        targetweight(target) algorithm(jla) engine(compressed)      ///
- L200:        algorithm(jla) engine(compressed) preconditioner(diagonal)  ///
- L219:        algorithm(jla) engine(auto) preconditioner(diagonal)        ///
- L231:        targetweight(target) algorithm(jla) engine(compressed)      ///
- L250:        algorithm(jla) engine(compressed) preconditioner(diagonal)   ///
- L408:        targetweight(target) algorithm(jla) engine(compressed)     ///

### `varcomp_kss/tests/stata/test_rust_public_generic.do`

- L34:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L49:    assert strpos(`"`e(cmdline)'"',"algorithm(jla)") > 0
- L126:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L136:        deletion(match) nuisance(joint) algorithm(jla) backend(rust)        ///
- L144:        algorithm(jla) backend(rust) rng(counter_v1) engine(generic)       ///
- L152:    quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
- L165:    quietly varcomp_kss_rust solve `private_handle', algorithm(jla) deletion(match) ///
- L185:            deletion(observation) nuisance(joint) algorithm(jla) engine(generic) ///
- L200:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L277:    // Exercise requested algorithm(auto) with the generic JLA result family
- L299:        "varcomp_kss outcome control [fw=frequency], backend(rust) algorithm(auto) engine(generic)" ///
- L308:    assert e(rust_requested_algorithm_code) == 0
- L309:    assert e(rust_selected_algorithm_code) == 2
- L318:    assert e(rust_plan_algorithm_requested) == 0
- L360:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L420:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L479:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L523:        deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
- L533:        deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
- L537:        deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
- L541:        deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
- L545:        deletion(observation) backend(rust) algorithm(jla) engine(generic) ///
- L552:            deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
- L560:    foreach auto_option in "algorithm(auto)" "engine(compressed)" {
- L561:        local algorithm_option algorithm(jla)
- L564:        if "`auto_option'" == "algorithm(auto)" local algorithm_option
- L587:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L622:    assert e(rust_requested_algorithm_code) == 2
- L623:    assert e(rust_selected_algorithm_code) == 2
- L642:    assert e(rust_plan_algorithm_requested) == 2
- L757:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L822:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L880:        deletion(match) deletionid(deletion_id) nuisance(fixedoffset) algorithm(jla) ///
- L938:                algorithm(jla) backend(rust) rng(counter_v1) engine(generic) ///
- L957:        deletion(match) deletionid(deletion_id) nuisance(fixedoffset) algorithm(jla) ///
- L977:        deletion(match) deletionid(deletion_subcell) nuisance(joint) algorithm(jla) ///
- L996:        deletion(observation) nuisance(joint) algorithm(jla)           ///
- L1005:        nuisance(joint) algorithm(jla) backend(rust)                    ///
- L1013:        algorithm(jla) backend(rust) rng(counter_v1) engine(generic)   ///
- L1020:        firm(firm) deletion(observation) nuisance(joint) algorithm(jla) ///
- L1044:        deletion(observation) nuisance(joint) algorithm(jla)            ///
- L1055:        deletion(observation) nuisance(joint) algorithm(jla)            ///
- L1072:        worker(worker) firm(firm) deletion(observation) nuisance(joint) algorithm(jla) ///
- L1081:        nuisance(joint) algorithm(jla) backend(rust) rng(counter_v1) engine(generic) ///
- L1089:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L1104:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L1121:        deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
- L1128:        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
- L1147:        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
- L1166:        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
- L1183:        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
- L1288:            deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
- L1324:            deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
- L1348:        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///

### `varcomp_kss/tests/stata/test_rust_public_install.do`

- L61:            algorithm(jla) engine(compressed)                        ///

### `varcomp_kss/tests/stata/test_scale_command.do`

- L128:        probeorder(atom_key) algorithm(jla) engine(compressed)         ///
- L330:        probeorder(atom_key) algorithm(jla) engine(compressed)         ///
- L357:        probeorder(atom_key) algorithm(jla) engine(compressed)         ///
- L402:        probeorder(atom_key) algorithm(jla) engine(generic)            ///
- L425:        probeorder(atom_key) algorithm(jla) engine(generic)            ///
- L442:        probeorder(atom_key) algorithm(jla) engine(compressed)         ///
- L462:        targetweight(target) probeorder(atom_key) algorithm(jla)       ///
- L482:        algorithm(jla) engine(auto) preconditioner(diagonal)            ///
- L507:        targetweight(target) probeorder(atom_key) algorithm(jla)       ///
- L537:        targetweight(target) probeorder(atom_key) algorithm(jla)       ///
- L551:        targetweight(target) probeorder(atom_key) algorithm(jla)       ///

### `varcomp_kss/tests/stata/test_scale_resource.do`

- L192:        algorithm(jla) engine(compressed) preconditioner(diagonal)     ///

### `varcomp_kss/tests/stata/test_semantics.do`

- L61:        deletionid(match) algorithm(jla) probes(400) batch(11) ///
- L65:        deletionid(match_r) algorithm(jla) probes(400) batch(11) ///
- L75:        deletionid(match) algorithm(jla) probes(400) batch(11) ///
- L90:            deletionid(match) algorithm(jla) nuisance(`nuisance_mode') ///
- L94:            deletion(match) deletionid(match) algorithm(jla) ///
- L103:        deletionid(match) algorithm(jla) probes(80) batch(7) ///
- L107:        deletionid(match_r) algorithm(jla) probes(80) batch(7) ///
- L131:        algorithm(jla) probes(2) batch(1) seed(1) tolerance(1e-4) nodisplay
- L134:        algorithm(jla) probes(2) batch(1) seed(1) tolerance(1e-4) nodisplay
- L141:        algorithm(jla) probes(2) batch(2) seed(1) tolerance(1e-4) nodisplay

### `varcomp_kss/tests/stata/test_solver_prod_fixes.do`

- L142:        algorithm(jla) preconditioner(cmg) memory_gib(4) ///
- L200:        algorithm(jla) preconditioner(diagonal) memory_gib(4) ///
- L225:        algorithm(jla) preconditioner(cmg) memory_gib(4) ///

### `varcomp_kss/tests/stata/test_stayers_hybrid.do`

- L395:        firm(firm) deletion(match) deletionid(match_id) algorithm(jla) ///

### `varcomp_kss/varcomp_kss.ado`

- L157:            algorithm(jla) deletion(`deletionmode') nuisance(`nuisance') ///
- L396:            maxiter(`maxiter') algorithm(jla) deletion(`deletionmode')  ///
- L458:            requested_algorithm_code:r_algorithm_req selected_algorithm_code:r_algorithm_sel ///
- L1122:    program define _vckss_rust_generic_planned, eclass sortpreserve
- L1126:            maxiter memorygib algorithm_requested enginerequested       ///
- L1142:        local algorithm_requested = lower(strtrim("`algorithm_requested'"))
- L1143:        local algorithm_expected_code = cond("`algorithm_requested'"=="auto",0,2)
- L1144:        local algorithm_defer_expected = cond("`algorithm_requested'"=="auto",1,0)
- L1147:        local engine_defer_expected = cond("`algorithm_requested'"=="auto" | ///
- L1176:        if !inlist("`algorithm_requested'","jla","auto") |             ///
- L1177:            ("`algorithm_requested'"=="auto" &                         ///
- L1200:            algorithm(`algorithm_requested') deletion(`deletionmode') nuisance(`nuisance') ///
- L1493:            tolerance(`tolerance') maxiter(`maxiter') algorithm(`algorithm_requested')    ///
- L1655:            requested_algorithm_code:r_algorithm_req selected_algorithm_code:r_algorithm_sel ///
- L1694:            plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel      ///
- L1860:            r_plan_applicability r_plan_alg_req r_plan_alg_sel          ///
- L1888:            `r_plan_alg_req'==`algorithm_expected_code' &            ///
- L1889:            `r_plan_alg_sel'==2 &                                    ///
- L2261:        ereturn scalar rust_requested_algorithm_code = `r_algorithm_req'
- L2262:        ereturn scalar rust_selected_algorithm_code = `r_algorithm_sel'
- L2317:        ereturn scalar rust_plan_algorithm_requested = `r_plan_alg_req'
- L2318:        ereturn scalar rust_plan_algorithm_selected = `r_plan_alg_sel'
- L4139:            di as error "engine(compressed) requires algorithm(jla)"
- L5958:        local r_algorithm_req = r(requested_algorithm_code)
- L5959:        local r_algorithm_sel = r(selected_algorithm_code)
- L7325:            local suggestion "Use algorithm(auto) or algorithm(jla) for a large identified design; increase a safety limit only after confirming the required allocation is appropriate."

### `varcomp_kss/varcomp_kss_rust.ado`

- L803:            return scalar requested_algorithm_code = scalar(__vckss_rust_algorithm_req)
- L804:            return scalar selected_algorithm_code = scalar(__vckss_rust_algorithm_sel)
- L883:            local requested_algorithm_code = scalar(__vckss_rust_algorithm_req)
- L884:            local selected_algorithm_code = scalar(__vckss_rust_algorithm_sel)
- L888:            if `requested_algorithm_code' == 0 local requested_algorithm auto
- L889:            else if `requested_algorithm_code' == 1 local requested_algorithm exact
- L891:            if `selected_algorithm_code' == 1 local selected_algorithm exact

### `rust/CHATGPT_PROGRESS.md`

- L82:    `algorithm(jla)`, `engine(generic)`, `preconditioner(auto|cmg)`,
- L104:    2. Widen public planned routing toward `engine(auto)` and `algorithm(auto)`

### `rust/IMPLEMENTATION_STATUS.md`

- L51:    Exact estimation uses no estimator RNG. Structural `algorithm(auto)` selects
- L118:    - `algorithm(auto)` uses only the retained W/F/Q dimensions and
- L167:       `backend(rust) algorithm(jla) engine(generic)
- L173:    `algorithm(auto)`, `engine(auto)` across compressed/generic selection,

### `rust/crates/vckss-core/src/engine_plan.rs`

- L97:        /// Frozen false: algorithm(auto) never reroutes on a later memory,
- L176:    /// Resolve `algorithm(auto)` exactly once, after retained W/F/Q dimensions

### `rust/crates/vckss-plugin/src/ffi_engine.rs`

- L1135:        pub algorithm_requested: u32,
- L1227:        pub algorithm_requested: u32,
- L1567:        algorithm_requested: u32,
- L2665:        let algorithm_requested = algorithm_request_from_code(request.v3.v2.algorithm)?;
- L2729:                algorithm: algorithm_requested,
- L2893:                algorithm_requested: request.v3.v2.algorithm,
- L3074:                algorithm_requested: VCKSS_ALGORITHM_JLA,
- L3115:        let algorithm_requested = algorithm_from_code(request.algorithm)?;
- L3135:            let algorithm = match algorithm_requested {
- L3215:                algorithm_requested,
- L3293:                algorithm_requested: VCKSS_ALGORITHM_JLA,
- L4287:            algorithm_requested: solved.algorithm_requested,
- L4465:            algorithm_requested: algorithm_request_code(plan.algorithm.requested),

### `rust/crates/vckss-plugin/tests/engine_ffi.rs`

- L432:            offset_of!(VckssEngineDetailedReceiptV4, algorithm_requested),
- L1786:        assert_eq!(detailed.v5.v4.algorithm_requested, VCKSS_ALGORITHM_JLA);

### `rust/progress/2026-08-23-compressed-accounting-reconciliation.md`

- L31:    No claim is made here for public release, `backend(auto)` selecting Rust, `algorithm(auto)`, stayers, Windows, Linux, native Intel hardware, broad numerical parity, or production-scale performance.

### `rust/progress/2026-08-23-compressed-engine-auto-public-checkpoint.md`

- L31:    - explicit `algorithm(jla)` and `rng(counter_v1)`;
- L114:    - public `algorithm(auto)`;
- L129:    to public `algorithm(auto)`, preserving pre-RNG routing and explicitly testing

### `rust/progress/2026-08-23-forced-cmg-candidate.md`

- L8:    `backend(rust) algorithm(jla) engine(generic)`. The legacy explicit

### `rust/progress/2026-08-23-forced-cmg-qualified.md`

- L62:    - `algorithm(jla)`;
- L81:       `algorithm(auto)` only with complete exact/compressed/generic receipt

### `rust/progress/2026-08-23-forced-diagonal-qualified.md`

- L69:    4. Then address `algorithm(auto)`, Rust stayer-hybrid parity, and large-N

### `rust/progress/2026-08-23-generic-engine-auto-qualified.md`

- L79:    4. Then address `algorithm(auto)`, Rust stayer-hybrid parity, and large-N

### `rust/progress/2026-08-23-generic-engine-auto-resume.md`

- L24:    `algorithm(jla)`. Omitted `backend()` and `backend(auto)` continue to use Mata.

### `rust/progress/2026-08-23-public-planned-route-resume.md`

- L32:    - `algorithm(jla)`;
- L72:       `algorithm(auto)` only after exact public receipt tests are in place.

## Required implementation sequence

1. Carry the literal requested algorithm through the planned public runner.
2. Parameterize V4/V7 reconciliation by requested and selected algorithm.
3. Dispatch exact, generic-JLA, and compressed-JLA result families only
   after the frozen native plan is reconciled and before estimator RNG.
4. Add public tests for exact selection, generic JLA selection, compressed
   JLA selection, unsupported tuples, and fail-closed receipt mismatches.
5. Obtain exact-SHA quick evidence and then comprehensive `plugin-build`
   qualification before claiming public `algorithm(auto)` support.

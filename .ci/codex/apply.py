from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    path.write_text(text.replace(old, new))


reconcile = Path("varcomp_kss/_vckss_rust_reconcile_comp_v7.ado")
replace_once(
    reconcile,
    "inlist(`r_route_sel',2,3)",
    "inlist(`r_route_sel',1,2,3)",
    "compressed selected-route reconciliation",
)

poster = Path("varcomp_kss/_vckss_rust_post_comp_v7.ado")
replace_once(
    poster,
    "ereturn local preconditioner_selected = cond(`h_rtsel'==3,\"cmg\",\"diagonal\")",
    "ereturn local preconditioner_selected = cond(`h_rtsel'==1,\"exact\", ///\n        cond(`h_rtsel'==3,\"cmg\",\"diagonal\"))",
    "compressed selected-route posting",
)

test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
test_text = test_path.read_text()
start_marker = "// The public engine-auto boundary must preserve the compressed result family\n"
end_marker = "// Forced CMG must remain fail-closed, select CMG before Counter addressing,\n"
if test_text.count(start_marker) != 1 or test_text.count(end_marker) != 1:
    raise SystemExit("public compressed route-test block anchors changed")
start = test_text.index(start_marker)
end = test_text.index(end_marker, start)
replacement = '''// The public engine-auto boundary must preserve the compressed result family.\n// On this small F-1=3 quotient the registered pre-RNG automatic solver rule\n// selects the exact/direct route without changing the JLA estimator family.\nquietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///\n    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///\n    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto)   ///\n    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///\n    targetweight(target_weight) nodisplay\nassert `"`e(engine_requested)'"' == "auto"\nassert `"`e(engine_selected)'"' == "compressed"\nassert `"`e(result_family)'"' == "compressed"\nassert `"`e(preconditioner_requested)'"' == "auto"\nassert `"`e(preconditioner_selected)'"' == "exact"\nassert `"`e(fallback_status)'"' == "ELIGIBLE_NOT_USED"\nassert e(rust_requested_route) == 0\nassert e(rust_selected_route) == 1\nassert e(rust_solver_fallback) == 0\nassert e(rust_solver_fallback_error) == 0\nassert e(rust_plan_route_requested) == 0\nassert e(rust_plan_route_selected) == 1\nassert e(rust_full_fit_route) == 1\nassert e(rust_rhs_receipt_schema) == 1\nassert e(rust_rhs_v2_copy_bytes) == 0\nassert e(rust_counter_plan_complete) == 1\nassert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0\nassert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)\nassert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)\nassert e(target_identity_residual) == e(rust_actual_accounting_residual)\nassert e(targetweight_option_supplied) == 1\n\ntempname auto_results auto_rhs\nmatrix `auto_results' = e(results)\nmatrix `auto_rhs' = e(rust_rhs_receipts)\nassert colsof(`auto_rhs') == 8\nforvalues row = 1/`=rowsof(`auto_rhs')' {\n    assert `auto_rhs'[`row',4] == 1\n}\nquietly count if e(sample)\nassert r(N) == e(N_retained)\nquietly varcomp_kss_rust snapshot\nassert r(state) == 0 & r(handle) == 0\nassert `"`c(rng)'"' == `"`caller_rng'"'\nassert c(rngstream) == `caller_stream'\nassert `"`c(rngstate)'"' == `"`caller_state'"'\nlocal auto_sortedby_after : sortedby\nassert `"`auto_sortedby_after'"' == `"`caller_sortedby'"'\nquietly _datasignature\nassert `"`r(datasignature)'"' == `"`caller_signature'"'\n\n// Lowering only the exact solver limit below F-1 preserves engine(auto) and\n// forces the middle automatic route to diagonal before Counter addressing.\nquietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///\n    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///\n    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto)   ///\n    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///\n    exact_limit(2) targetweight(target_weight) nodisplay\nassert `"`e(engine_requested)'"' == "auto"\nassert `"`e(engine_selected)'"' == "compressed"\nassert `"`e(result_family)'"' == "compressed"\nassert `"`e(preconditioner_requested)'"' == "auto"\nassert `"`e(preconditioner_selected)'"' == "diagonal"\nassert `"`e(fallback_status)'"' == "ELIGIBLE_NOT_USED"\nassert e(rust_requested_route) == 0\nassert e(rust_selected_route) == 2\nassert e(rust_solver_fallback) == 0\nassert e(rust_solver_fallback_error) == 0\nassert e(rust_plan_route_requested) == 0\nassert e(rust_plan_route_selected) == 2\nassert e(rust_full_fit_route) == 2\nassert e(rust_rhs_receipt_schema) == 1\nassert e(rust_rhs_v2_copy_bytes) == 0\nassert e(rust_counter_plan_complete) == 1\nassert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0\nassert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)\nassert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)\nassert e(target_identity_residual) == e(rust_actual_accounting_residual)\nassert e(targetweight_option_supplied) == 1\n\ntempname diagonal_results diagonal_rhs\nmatrix `diagonal_results' = e(results)\nmatrix `diagonal_rhs' = e(rust_rhs_receipts)\nassert colsof(`diagonal_rhs') == 8\nforvalues row = 1/`=rowsof(`diagonal_rhs')' {\n    assert `diagonal_rhs'[`row',4] == 2\n}\nforvalues row = 1/4 {\n    forvalues column = 1/4 {\n        assert abs(`diagonal_results'[`row',`column']-              ///\n            `auto_results'[`row',`column']) <=                     ///\n            1e-8*max(1,abs(`auto_results'[`row',`column']))\n    }\n}\nquietly count if e(sample)\nassert r(N) == e(N_retained)\nquietly varcomp_kss_rust snapshot\nassert r(state) == 0 & r(handle) == 0\nassert `"`c(rng)'"' == `"`caller_rng'"'\nassert c(rngstream) == `caller_stream'\nassert `"`c(rngstate)'"' == `"`caller_state'"'\nlocal diagonal_sortedby_after : sortedby\nassert `"`diagonal_sortedby_after'"' == `"`caller_sortedby'"'\nquietly _datasignature\nassert `"`r(datasignature)'"' == `"`caller_signature'"'\n\n'''
test_path.write_text(test_text[:start] + replacement + test_text[end:])

qualifier = Path("rust/stata_backend/qualify_macos.sh")
qualifier_text = qualifier.read_text()
scope_phrase = "automatic diagonal and forced CMG routes"
if qualifier_text.count(scope_phrase) != 2:
    raise SystemExit("qualifier public compressed scope anchors changed")
qualifier_text = qualifier_text.replace(
    scope_phrase,
    "automatic exact, automatic diagonal, and forced CMG routes",
)
route_phrase = (
    "public-compressed-jla-backend-rust-engine-auto-no-controls-match-joint-"
    "fixedoffset-auto-to-diagonal-forced-cmg-independent-numeric-batches-"
    "wall-advisory-counter-v1-fweights-stored-targetweights-matchid"
)
if qualifier_text.count(route_phrase) != 1:
    raise SystemExit("qualifier tested-routes anchor changed")
qualifier_text = qualifier_text.replace(
    route_phrase,
    "public-compressed-jla-backend-rust-engine-auto-no-controls-match-joint-"
    "fixedoffset-auto-to-exact-auto-to-diagonal-forced-cmg-independent-"
    "numeric-batches-wall-advisory-counter-v1-fweights-stored-targetweights-"
    "matchid",
)
selftest_anchor = '    fail "available receipt scope omitted public compressed qualification"\n'
if qualifier_text.count(selftest_anchor) != 1:
    raise SystemExit("qualifier public compressed selftest anchor changed")
qualifier_text = qualifier_text.replace(
    selftest_anchor,
    selftest_anchor
    + "  [[ \"${available}\" == *'automatic exact, automatic diagonal, and forced CMG routes'* ]] || \\\n"
    + '    fail "available receipt scope omitted the complete automatic compressed route matrix"\n',
)
qualifier.write_text(qualifier_text)

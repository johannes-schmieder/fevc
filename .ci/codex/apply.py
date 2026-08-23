from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    path.write_text(text.replace(old, new), encoding="utf-8")


reconcile = Path("varcomp_kss/_vckss_rust_reconcile_comp_v7.ado")
replace_once(
    reconcile,
    "    local accounting_truth = 0\n"
    "    if `ok' {\n",
    "    // V4 reports a scale-normalized accounting residual, while V5 adds\n"
    "    // the absolute result identity residual.  Validate each diagnostic\n"
    "    // against the matching recomputation rather than against each other.\n"
    "    local accounting_truth = 0\n"
    "    local actual_accounting_truth = 0\n"
    "    if `ok' {\n",
    "declare separate accounting truths",
)
replace_once(
    reconcile,
    "            local identity = abs(`raw_results'[`row',4] -              ///\n"
    "                `raw_results'[`row',1] - `raw_results'[`row',2] -     ///\n"
    "                2*`raw_results'[`row',3])/`component_scale'\n"
    "            local accounting_truth = max(`accounting_truth',`identity')\n"
    "            if `identity' > `roundoff_gate' local ok = 0\n",
    "            local identity_absolute = abs(`raw_results'[`row',4] -     ///\n"
    "                `raw_results'[`row',1] - `raw_results'[`row',2] -     ///\n"
    "                2*`raw_results'[`row',3])\n"
    "            local identity = `identity_absolute'/`component_scale'\n"
    "            local accounting_truth = max(`accounting_truth',`identity')\n"
    "            local actual_accounting_truth = max(                      ///\n"
    "                `actual_accounting_truth',`identity_absolute')\n"
    "            if `identity' > `roundoff_gate' local ok = 0\n",
    "compute separate accounting truths",
)
replace_once(
    reconcile,
    "            `r_accounting'>=0 & `r_accounting'<=`roundoff_gate' &        ///\n"
    "            `r_actual_accounting'>=0 &                                  ///\n"
    "            abs(`r_actual_accounting'-`accounting_truth')<=              ///\n"
    "                `roundoff_gate'*max(1,abs(`accounting_truth')) &         ///\n"
    "            abs(`r_accounting'-`r_actual_accounting')<=                  ///\n"
    "                `roundoff_gate'*max(1,abs(`r_actual_accounting')) &      ///\n"
    "            `r_weighted_rss'>=0\n",
    "            `r_accounting'>=0 & `r_accounting'<=`roundoff_gate' &        ///\n"
    "            abs(`r_accounting'-`accounting_truth')<=                     ///\n"
    "                `roundoff_gate'*max(1,abs(`accounting_truth')) &         ///\n"
    "            `r_actual_accounting'>=0 &                                  ///\n"
    "            abs(`r_actual_accounting'-`actual_accounting_truth')<=       ///\n"
    "                `roundoff_gate'*max(1,abs(`actual_accounting_truth')) &  ///\n"
    "            `r_weighted_rss'>=0\n",
    "validate accounting diagnostics by schema semantics",
)
replace_once(
    reconcile,
    "    return scalar accounting_truth = `accounting_truth'\n"
    "    return scalar seed = `r_seed'\n",
    "    return scalar accounting_truth = `accounting_truth'\n"
    "    return scalar actual_accounting_truth = `actual_accounting_truth'\n"
    "    return scalar seed = `r_seed'\n",
    "return absolute accounting truth",
)

compressed_test = Path("varcomp_kss/tests/stata/test_rust_planned_compressed.do")
replace_once(
    compressed_test,
    "assert r(full_fit_complete_residual) <= r(full_residual_tolerance)\n"
    "assert r(max_complete_residual) <= r(full_residual_tolerance)\n"
    "assert r(leverage_rhs_count) == `probes'\n",
    "assert r(full_fit_complete_residual) <= r(full_residual_tolerance)\n"
    "assert r(max_complete_residual) <= r(full_residual_tolerance)\n"
    "local accounting_gate = 4096*c(epsdouble)\n"
    "assert r(accounting_residual) >= 0\n"
    "assert abs(r(accounting_residual)-r(accounting_truth)) <=            ///\n"
    "    `accounting_gate'*max(1,abs(r(accounting_truth)))\n"
    "assert r(actual_accounting_residual) >= 0\n"
    "assert abs(r(actual_accounting_residual)-r(actual_accounting_truth)) <= ///\n"
    "    `accounting_gate'*max(1,abs(r(actual_accounting_truth)))\n"
    "assert r(actual_accounting_truth) >= r(accounting_truth)\n"
    "assert r(leverage_rhs_count) == `probes'\n",
    "regress accounting diagnostic semantics",
)

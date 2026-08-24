from pathlib import Path

report_path = Path("rust/progress/2026-08-24-algorithm-auto-routing-audit.md")
if report_path.exists():
    raise SystemExit(f"audit report already exists: {report_path}")

search_roots = [Path("varcomp_kss"), Path("rust")]
patterns = (
    "program define _vckss_rust_generic_planned",
    "algorithm_requested",
    "algorithm(jla)",
    "algorithm(auto)",
    "requested_algorithm_code",
    "selected_algorithm_code",
    "r_alg_req",
    "r_alg_sel",
    "plan_alg_req",
    "plan_alg_sel",
)
allowed_suffixes = {".ado", ".do", ".mata", ".rs", ".sh", ".md"}

matches = []
for root in search_roots:
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix not in allowed_suffixes:
            continue
        try:
            lines = path.read_text().splitlines()
        except UnicodeDecodeError:
            continue
        for lineno, line in enumerate(lines, start=1):
            if any(pattern in line for pattern in patterns):
                matches.append((str(path), lineno, line.rstrip()))

if not matches:
    raise SystemExit("algorithm-auto routing audit found no relevant anchors")

required_files = {
    "varcomp_kss/varcomp_kss.ado",
    "varcomp_kss/_vckss_rust_reconcile_comp_v7.ado",
}
matched_files = {path for path, _, _ in matches}
missing = sorted(required_files - matched_files)
if missing:
    raise SystemExit(f"algorithm-auto routing audit missed required files: {missing}")

lines = [
    "# Public `algorithm(auto)` routing audit",
    "",
    "Date: 2026-08-24",
    "",
    "This generated checkpoint enumerates the current public/native routing and",
    "receipt-reconciliation anchors that mention requested or selected algorithm",
    "semantics. It is deliberately read-only: no estimator, planner, receipt, or",
    "test behavior is changed by this commit.",
    "",
    "The implementation milestone must preserve pre-RNG routing and reconcile the",
    "actual selected result family. In particular, an `algorithm(auto)` request",
    "must not be treated as a JLA request merely because JLA is selected, and an",
    "exact selection must not be forced through generic or compressed JLA result",
    "semantics.",
    "",
    "## Matched anchors",
    "",
]

current = None
for path, lineno, text in matches:
    if path != current:
        if current is not None:
            lines.append("")
        lines.extend((f"### `{path}`", ""))
        current = path
    lines.append(f"- L{lineno}:    {text}")

lines.extend(
    (
        "",
        "## Required implementation sequence",
        "",
        "1. Carry the literal requested algorithm through the planned public runner.",
        "2. Parameterize V4/V7 reconciliation by requested and selected algorithm.",
        "3. Dispatch exact, generic-JLA, and compressed-JLA result families only",
        "   after the frozen native plan is reconciled and before estimator RNG.",
        "4. Add public tests for exact selection, generic JLA selection, compressed",
        "   JLA selection, unsupported tuples, and fail-closed receipt mismatches.",
        "5. Obtain exact-SHA quick evidence and then comprehensive `plugin-build`",
        "   qualification before claiming public `algorithm(auto)` support.",
        "",
    )
)

report_path.write_text("\n".join(lines))

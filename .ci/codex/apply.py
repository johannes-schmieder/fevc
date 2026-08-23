from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
text = path.read_text()
start_marker = "// Automatic preconditioning must preserve the compressed engine choice and\n"
end_marker = "// A forced CMG compressed request must remain on CMG: setup failure may not\n"
if text.count(start_marker) != 1 or text.count(end_marker) != 1:
    raise SystemExit("legacy compressed automatic-route block anchors changed")
start = text.index(start_marker)
end = text.index(end_marker, start)
block = text[start:end]

replacements = [
    (
        "// Automatic preconditioning must preserve the compressed engine choice and\n"
        "// select its frozen diagonal route before Counter-V1 begins.  Advisory wall\n"
        "// planning may report work but may not change results or caller state.\n",
        "// Automatic preconditioning must preserve the compressed engine choice and\n"
        "// select the registered exact/direct route for this small quotient before\n"
        "// Counter-V1 begins.  Advisory wall planning may report work but may not\n"
        "// change results or caller state.\n",
        "automatic-route comment",
    ),
    (
        "tempname compressed_preauto_results compressed_preauto_memory\n"
        "matrix `compressed_preauto_results' = e(results)\n"
        "matrix `compressed_preauto_memory' = e(rust_memory_receipt)\n",
        "tempname compressed_preauto_results compressed_preauto_memory ///\n"
        "    compressed_preauto_rhs\n"
        "matrix `compressed_preauto_results' = e(results)\n"
        "matrix `compressed_preauto_memory' = e(rust_memory_receipt)\n"
        "matrix `compressed_preauto_rhs' = e(rust_rhs_receipts)\n",
        "automatic-route receipt matrices",
    ),
    (
        "assert `\"`e(preconditioner_selected)'\"' == \"diagonal\"",
        "assert `\"`e(preconditioner_selected)'\"' == \"exact\"",
        "selected preconditioner",
    ),
    (
        "assert e(rust_selected_route) == 2\nassert e(route_code) == 2",
        "assert e(rust_selected_route) == 1\nassert e(route_code) == 1",
        "selected route",
    ),
    (
        "assert e(rust_plan_route_selected) == 2",
        "assert e(rust_plan_route_selected) == 1",
        "selected plan route",
    ),
    (
        "assert e(rust_plan_route_error) == 0\n"
        "assert e(rust_wallseconds_supplied) == 1",
        "assert e(rust_plan_route_error) == 0\n"
        "assert colsof(`compressed_preauto_rhs') == 8\n"
        "forvalues row = 1/`=rowsof(`compressed_preauto_rhs')' {\n"
        "    assert `compressed_preauto_rhs'[`row',4] == 1\n"
        "}\n"
        "assert e(rust_wallseconds_supplied) == 1",
        "automatic-route RHS receipt",
    ),
]

for old, new, label in replacements:
    count = block.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    block = block.replace(old, new)

path.write_text(text[:start] + block + text[end:])

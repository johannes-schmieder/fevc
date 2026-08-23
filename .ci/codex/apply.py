from pathlib import Path

production = Path("varcomp_kss/varcomp_kss.ado")
source = production.read_text(encoding="utf-8")
start = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
end = "program define _vckss_impl, eclass sortpreserve\n"
if source.count(start) != 1 or source.count(end) != 1:
    raise RuntimeError("planned Rust program boundaries are not unique")
planned = source.split(start, 1)[1].split(end, 1)[0]
lines = planned.splitlines()
offenders = [
    index + 1
    for index in range(1, len(lines))
    if lines[index - 1].rstrip().endswith("///")
    and lines[index].lstrip().startswith("//")
]
if offenders:
    raise RuntimeError(
        f"planned program still has full-line comments after continuation at {offenders}"
    )

test_path = Path("ci/tests/test_stata_return_names.py")
test_source = test_path.read_text(encoding="utf-8")
anchor = """

if __name__ == "__main__":
    unittest.main()
"""
method = """

    def test_planned_program_has_no_comment_after_continuation(self) -> None:
        source = (ROOT / "varcomp_kss" / "varcomp_kss.ado").read_text(
            encoding="utf-8"
        )
        start = "program define _vckss_rust_generic_planned, eclass sortpreserve\\n"
        end = "program define _vckss_impl, eclass sortpreserve\\n"
        self.assertEqual(1, source.count(start))
        self.assertEqual(1, source.count(end))
        planned = source.split(start, 1)[1].split(end, 1)[0]
        offenders: list[int] = []
        lines = planned.splitlines()
        for index in range(1, len(lines)):
            if lines[index - 1].rstrip().endswith("///") and lines[index].lstrip().startswith("//"):
                offenders.append(index + 1)
        self.assertEqual(
            [],
            offenders,
            f"full-line comments interrupt continued planned commands at relative lines {offenders}",
        )
"""
if test_source.count(anchor) != 1:
    raise RuntimeError("Stata static-test insertion anchor is not unique")
if "test_planned_program_has_no_comment_after_continuation" in test_source:
    raise RuntimeError("continued-command regression test already exists")
test_path.write_text(test_source.replace(anchor, method + anchor, 1), encoding="utf-8")
print("added a static regression guard for continued planned Stata commands")

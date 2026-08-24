from pathlib import Path

ado_path = Path("varcomp_kss/varcomp_kss.ado")
text = ado_path.read_text()

start = text.index("program define _vckss_rust_generic_planned, eclass sortpreserve")
end = text.index("\nend\n", start) + len("\nend\n")
block = text[start:end]

args_old = "        maxiter memorygib enginerequested backendsupplied rngsupplied ///"
args_new = """        maxiter memorygib algorithm_requested enginerequested       ///
        backendsupplied rngsupplied                                  ///"""
if block.count(args_old) != 1:
    raise SystemExit("planned-runner argument anchor changed")
block = block.replace(args_old, args_new)

if block.count("algorithm(jla)") != 2:
    raise SystemExit("planned-runner algorithm call count changed")
block = block.replace("algorithm(jla)", "algorithm(`algorithm_requested')")
text = text[:start] + block + text[end:]

call_start = text.index("capture noisily _vckss_rust_generic_planned")
engine_pos = text.index("`engine_requested'", call_start)
call_tail = text[engine_pos:engine_pos + 256]
backend_pos = call_tail.find("`backend_supplied'")
rng_pos = call_tail.find("`rng_supplied'")
if not call_tail.startswith("`engine_requested'") or not (0 < backend_pos < rng_pos):
    raise SystemExit("public planned-runner call token order changed")
text = text[:engine_pos] + "`algorithm' " + text[engine_pos:]
ado_path.write_text(text)

post_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
post = post_path.read_text()
post_old = "`nstayerrows' 7 2 81227 1e-12 10000 1 auto 1 1 1 1 1 1 1 0"
post_new = "`nstayerrows' 7 2 81227 1e-12 10000 1 jla auto 1 1 1 1 1 1 1 0"
if post.count(post_old) != 1:
    raise SystemExit("standalone planned-runner call anchor changed")
post_path.write_text(post.replace(post_old, post_new))

hits = {}
for path in Path("varcomp_kss").rglob("*"):
    if path.suffix not in {".ado", ".do"} or not path.is_file():
        continue
    count = path.read_text().count("_vckss_rust_generic_planned")
    if count:
        hits[str(path)] = count
expected = {
    "varcomp_kss/varcomp_kss.ado": 2,
    "varcomp_kss/tests/stata/test_rust_planned_compressed_post.do": 1,
}
if hits != expected:
    raise SystemExit(f"unexpected planned-runner call sites: {hits}")

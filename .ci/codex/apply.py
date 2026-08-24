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
call_end = text.index("\n        local rust_rc", call_start)
call = text[call_start:call_end]
call_old = "`memory_gib'                ///\n                `engine_requested'"
call_new = "`memory_gib'                ///\n                `algorithm' `engine_requested'"
if call.count(call_old) != 1:
    raise SystemExit("public planned-runner call anchor changed")
call = call.replace(call_old, call_new)
text = text[:call_start] + call + text[call_end:]
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

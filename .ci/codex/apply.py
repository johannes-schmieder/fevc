from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
text = path.read_text()
old = '''// The public engine-auto boundary must preserve the compressed result family
// while the native pre-RNG plan resolves an automatic route to diagonal.
quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto)   ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
'''
new = '''// The public engine-auto boundary must preserve the compressed result family
// while the native pre-RNG plan resolves an automatic route to diagonal.
// Keep this trace tightly scoped to the unresolved public-boundary failure so
// the comprehensive qualifier exports the first raw Stata/native 498 rather
// than only the outer withheld-result display.
set tracedepth 6
set trace on
capture noisily varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto)   ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
local public_auto_rc = _rc
set trace off
if `public_auto_rc' {
    noisily di as error "PUBLIC_COMPRESSED_AUTO_RC=`public_auto_rc'"
    capture noisily ereturn list
    capture noisily varcomp_kss_rust lasterror
    capture noisily return list
    capture noisily varcomp_kss_rust snapshot
    capture noisily return list
}
assert `public_auto_rc' == 0
'''
if text.count(old) != 1:
    raise SystemExit("public compressed auto diagnostic anchor changed")
path.write_text(text.replace(old, new))

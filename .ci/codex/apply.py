from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
text = path.read_text(encoding="utf-8")
old = r'''tempname planned_reference planned_memory
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
'''
new = r'''tempname planned_reference planned_memory
local planned_rng `"`c(rng)'"'
local planned_stream = c(rngstream)
local planned_state `"`c(rngstate)'"'
local planned_sortedby : sortedby
quietly _datasignature
local planned_signature `"`r(datasignature)'"'
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
'''
if text.count(old) != 1:
    raise RuntimeError(f"planned state capture anchor: expected one block, found {text.count(old)}")
text = text.replace(old, new, 1)
old = r'''assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
'''
new = r'''assert `"`c(rng)'"' == `"`planned_rng'"'
assert c(rngstream) == `planned_stream'
assert `"`c(rngstate)'"' == `"`planned_state'"'
local planned_sortedby_after : sortedby
assert `"`planned_sortedby_after'"' == `"`planned_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`planned_signature'"'
'''
if text.count(old) != 1:
    raise RuntimeError(f"planned state restoration block: expected one match, found {text.count(old)}")
text = text.replace(old, new, 1)
path.write_text(text, encoding="utf-8")
print("bound planned-route state restoration to its immediate pre-call state")

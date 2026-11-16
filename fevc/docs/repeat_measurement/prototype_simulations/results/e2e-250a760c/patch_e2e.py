root = '<session-tmp>/e2e/'


def edit(name, pairs):
    s = open(root + name).read()
    for x, y in pairs:
        assert s.count(x) == 1, (name, s.count(x), x[:70]); s = s.replace(x, y)
    open(root + name, 'w').write(s)


edit('veneto_e2e.do', [("""capture matrix V = e(V)
local have_V = (_rc==0)
""", """* An assignment from a missing e() matrix does not fail in Stata; only confirm tells whether e(V) is posted.
capture confirm matrix e(V)
local have_V = (_rc==0)
""")])
edit('job_build.sh', [("""grep -q "E2E SMOKE PASS" smoke.log || { echo "smoke failed" >&2; tail -40 smoke.log >&2; exit 4; }
grep -E "SMOKE|deletion" smoke.log | tail -12
""", """cat ./*.log > "$work/smoke_all.log"                      # the batch log's name differs between platforms
grep -q "^E2E SMOKE PASS" "$work/smoke_all.log" || { echo "smoke failed" >&2; tail -40 "$work/smoke_all.log" >&2; exit 4; }
grep -E "^SMOKE|^E2E" "$work/smoke_all.log" | tail -12
""")])
edit('local_test.sh', [("""grep -E "^SMOKE|E2E SMOKE PASS|^r\\(" "$here/lt/smoke/smoke.log"
""", """grep -hE "^SMOKE|^E2E SMOKE PASS|^r\\(" "$here/lt/smoke/"*.log
""")])
print('patched')

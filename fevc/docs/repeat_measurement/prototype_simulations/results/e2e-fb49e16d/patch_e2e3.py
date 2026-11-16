"""The end-to-end scripts run the plain default command (no inference option, no environment switch)."""
root = '<session-tmp>/e2e/'


def edit(name, pairs):
    s = open(root + name).read()
    for x, y in pairs:
        assert s.count(x) == 1, (name, s.count(x), x[:70]); s = s.replace(x, y)
    open(root + name, 'w').write(s)


edit('veneto_e2e.do', [
    ("""* End-to-end run of the package on a private panel: the point estimate alone and the automatic interval of the
* repeat-measurement model, under one deletion mode.""", """* End-to-end run of the package on a private panel: the point estimate alone and the plain default command (no
* inference option: repeat-measurement standard errors with the automatic interval), under one deletion mode."""),
    ("""fevc y, worker(worker) firm(firm) backend(rust) deletion(`deletion') nativethreads(`threads') ///
    inference(auto) inferencemodel(repeat_measurement)
timer off 2
""", """fevc y, worker(worker) firm(firm) deletion(`deletion') nativethreads(`threads')
timer off 2
assert "`e(backend_selected)'"=="rust" & e(inference_auto)==1 & "`e(inference_auto_reason)'"==""
"""),
])
edit('smoke.do', [
    ("""    fevc y, worker(worker) firm(firm) backend(rust) deletion(`deletion') nativethreads(`threads') ///
        inference(auto) inferencemodel(repeat_measurement)
    assert "`e(inference_model)'"=="repeat_measurement"
""", """    * The plain default command.
    fevc y, worker(worker) firm(firm) deletion(`deletion') nativethreads(`threads')
    assert "`e(inference_model)'"=="repeat_measurement" & "`e(backend_selected)'"=="rust"
    assert e(inference_auto)==1 & "`e(inference_auto_reason)'"==""
"""),
    ("""capture noisily fevc y, worker(worker) firm(firm) backend(rust) nativethreads(`threads') inference(auto) inferencemodel(no_such_model)
""", """capture noisily fevc y, worker(worker) firm(firm) nativethreads(`threads') inferencemodel(no_such_model)
"""),
])
edit('job_build.sh', [("""FEVC_DEFAULT_INFERENCE=none "$stata" -b do "$BASE/code/smoke.do" "$work/pkg/fevc" 4
""", """env -u FEVC_DEFAULT_INFERENCE "$stata" -b do "$BASE/code/smoke.do" "$work/pkg/fevc" 4
""")])
edit('job_run.sh', [("""    FEVC_DEFAULT_INFERENCE=none "$stata" -b do "$BASE/code/veneto_e2e.do" """, """    env -u FEVC_DEFAULT_INFERENCE "$stata" -b do "$BASE/code/veneto_e2e.do" """)])
s = open(root + 'local_test.sh').read()
assert s.count('FEVC_DEFAULT_INFERENCE=none "$stata"') == 3
s = s.replace('FEVC_DEFAULT_INFERENCE=none "$stata"', 'env -u FEVC_DEFAULT_INFERENCE "$stata"')
x = 'pkg=<scratch>/pkgmc/pkg-250a760c/fevc'
assert s.count(x) == 1; s = s.replace(x, 'pkg=${PKG:-<session-tmp>/pkgflip/fevc}')
open(root + 'local_test.sh', 'w').write(s)
print('patched')

version 18.0

clear
set obs 24
generate long obsid = _n
generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
generate double c1 = time-1.5
generate double c2 = time==2
generate byte firm = .
generate long match = .
generate double noise = .

local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `matches'
    quietly replace match = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5+.3*worker-.2*firm+.4*c1-.15*c2+noise
generate int frequency = 1+mod(obsid,3)
generate double target = 1+time+worker/10

fevc y c1 c2 [fw=frequency], worker(worker) firm(firm)   ///
    deletion(match) deletionid(match) targetweight(target)      ///
    nuisance(joint) algorithm(exact)

assert rowsof(e(decomposition)) == 4
assert colsof(e(decomposition)) == 7
assert abs(el(e(decomposition),1,1)-el(e(plugin),1,1)) < 1e-14
assert abs(el(e(decomposition),2,2)-el(e(correction),1,2)) < 1e-14
assert abs(el(e(decomposition),3,3)-2*el(e(kss),1,3)) < 1e-14
assert abs(el(e(decomposition),4,3)-el(e(kss),1,4)) < 1e-14

forvalues result_column = 1/3 {
    assert abs(el(e(decomposition),4,`result_column')-            ///
        el(e(decomposition),1,`result_column')-                   ///
        el(e(decomposition),2,`result_column')-                   ///
        el(e(decomposition),3,`result_column')) < 2e-12
}

quietly summarize y [aw=target] if e(sample), meanonly
local target_mean = r(mean)
tempvar target_ss frequency_ss
quietly generate double `target_ss' = target*(y-`target_mean')^2 ///
    if e(sample)
quietly summarize `target_ss', meanonly
local target_ss_total = r(sum)
quietly summarize target if e(sample), meanonly
local target_variance = `target_ss_total'/r(sum)
assert abs(e(target_outcome_variance)-`target_variance') < 2e-12

quietly summarize y [aw=frequency] if e(sample), meanonly
local frequency_mean = r(mean)
quietly generate double `frequency_ss' =                         ///
    frequency*(y-`frequency_mean')^2 if e(sample)
quietly summarize `frequency_ss', meanonly
local regression_variance = r(sum)/e(N_physical)
assert abs(e(regression_outcome_variance)-`regression_variance') < 2e-12
assert abs(e(residual_variance)-e(weighted_rss)/e(N_physical)) < 2e-14
assert abs(e(full_model_explained_variance)-                    ///
    (e(regression_outcome_variance)-e(residual_variance))) < 2e-14
assert abs(e(full_model_explained_share)-                       ///
    e(full_model_explained_variance)/e(regression_outcome_variance)) < 2e-14
assert abs(e(target_outcome_variance)-e(regression_outcome_variance)) > 1e-6

forvalues component = 1/4 {
    assert abs(el(e(decomposition),`component',4)-               ///
        el(e(decomposition),`component',1)/                      ///
        e(target_outcome_variance)) < 2e-14
    assert abs(el(e(decomposition),`component',5)-               ///
        el(e(decomposition),`component',3)/                      ///
        e(target_outcome_variance)) < 2e-14
    assert abs(el(e(decomposition),`component',6)-               ///
        el(e(decomposition),`component',1)/                      ///
        el(e(decomposition),4,1)) < 2e-14
    assert abs(el(e(decomposition),`component',7)-               ///
        el(e(decomposition),`component',3)/                      ///
        el(e(decomposition),4,3)) < 2e-14
}

clear
input double(y worker firm match)
1 1 1 11
1 1 1 11
1 1 2 12
1 1 2 12
1 2 1 21
1 2 1 21
1 2 2 22
1 2 2 22
end
fevc y, worker(worker) firm(firm) deletion(match)         ///
    deletionid(match) algorithm(exact) nodisplay
assert e(target_outcome_variance) == 0
assert e(regression_outcome_variance) == 0
assert missing(e(full_model_explained_share))
forvalues share_column = 4/7 {
    forvalues component = 1/4 {
        assert missing(el(e(decomposition),`component',`share_column'))
    }
}

di as result "PASS test_output.do"

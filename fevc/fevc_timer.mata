*! Private caller-safe profiling; timing availability never gates estimation.
version 18.0
mata:
mata set matastrict on

// Columns: owned physical ID, accumulated seconds, running flag. Logical
// slots preserve existing nested profiling meanings without reserving caller IDs.
__fevc_timer_state = J(128,3,0)
__fevc_timer_depth = 0

real scalar vckss_timer__api_level()
{
    return(1)
}

void vckss_timer__reset()
{
    external real matrix __fevc_timer_state
    real scalar slot, id
    for (slot=1; slot<=rows(__fevc_timer_state); slot++) {
        id = __fevc_timer_state[slot,1]
        if (id>0) {
            timer_off(id)
            timer_clear(id)
        }
    }
    __fevc_timer_state = J(128,3,0)
}

void vckss_timer__begin()
{
    external real scalar __fevc_timer_depth
    if (__fevc_timer_depth==0) vckss_timer__reset()
    __fevc_timer_depth++
}

void vckss_timer__end()
{
    external real scalar __fevc_timer_depth
    if (__fevc_timer_depth>0) __fevc_timer_depth--
    if (__fevc_timer_depth==0) vckss_timer__reset()
}

real scalar vckss_timer__reserve(real scalar slot)
{
    external real matrix __fevc_timer_state
    real scalar id
    if (__fevc_timer_state[slot,1]!=0) return(__fevc_timer_state[slot,1])
    for (id=1; id<=100; id++) {
        if (any(__fevc_timer_state[.,1]:==id)) continue
        // A running timer has a nonzero count too. Never clear or stop it to
        // inspect availability; accumulated timers remain caller-owned.
        if (timer_value(id)[2]!=0) continue
        __fevc_timer_state[slot,1] = id
        return(id)
    }
    __fevc_timer_state[slot,1] = -1
    return(-1)
}

void vckss_timer__clear(real scalar slot)
{
    external real matrix __fevc_timer_state
    external real scalar __fevc_timer_depth
    real scalar id
    id = __fevc_timer_state[slot,1]
    if (id<0 & __fevc_timer_depth==0) {
        id = 0
        __fevc_timer_state[slot,1] = 0
    }
    if (id>0) {
        timer_off(id)
        timer_clear(id)
    }
    __fevc_timer_state[slot,2] = 0
    __fevc_timer_state[slot,3] = 0
}

void vckss_timer__on(real scalar slot)
{
    external real matrix __fevc_timer_state
    real scalar id
    id = vckss_timer__reserve(slot)
    if (id<0) {
        __fevc_timer_state[slot,2] = .
        return
    }
    if (__fevc_timer_state[slot,3]) return
    timer_on(id)
    __fevc_timer_state[slot,3] = 1
}

void vckss_timer__off(real scalar slot)
{
    external real matrix __fevc_timer_state
    external real scalar __fevc_timer_depth
    real scalar id
    id = __fevc_timer_state[slot,1]
    if (id<=0 | !__fevc_timer_state[slot,3]) return
    timer_off(id)
    __fevc_timer_state[slot,2] = __fevc_timer_state[slot,2]+timer_value(id)[1]
    timer_clear(id)
    __fevc_timer_state[slot,3] = 0
    // Standalone internal Mata calls do not retain host timer reservations.
    if (__fevc_timer_depth==0) __fevc_timer_state[slot,1] = 0
}

real scalar vckss_timer__seconds(real scalar slot)
{
    external real matrix __fevc_timer_state
    return(__fevc_timer_state[slot,2])
}

// Stata/Mata max() ignores missing values. Timing arithmetic must not turn an
// unavailable measurement into a plausible zero or partial total.
real scalar vckss_timer__max(real matrix values)
{
    if (hasmissing(values)) return(.)
    return(max(values))
}
end

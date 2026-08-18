# Sum RSS, in KiB from ps, for one root PID and its descendants only.
#
# AWK array lookup creates a key even when the value is false.  Membership
# tests therefore use `in`; a truth-value lookup would contaminate the
# selected set with every PID visited while discovering descendants.
{
    parent[$1] = $2
    rss[$1] = $3
}

END {
    included[root] = 1
    changed = 1
    while (changed) {
        changed = 0
        for (pid in parent) {
            if (!(pid in included) && (parent[pid] in included)) {
                included[pid] = 1
                changed = 1
            }
        }
    }

    total = 0
    for (pid in included) {
        if (included[pid] == 1 && (pid in rss)) {
            total += rss[pid]
        }
    }
    printf "%.0f\n", 1024 * total
}

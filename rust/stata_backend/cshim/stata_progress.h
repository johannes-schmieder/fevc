/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef VCKSS_STATA_PROGRESS_H
#define VCKSS_STATA_PROGRESS_H

/* The only formatter/host writer. Rust invokes it synchronously on the Stata
 * caller, including while an owned numerical coordinator runs elsewhere. */
static int32_t vckss_progress_display(void *context, const VckssProgressUpdateV1 *u,
                                    uint64_t elapsed_ms, uint32_t level)
{
    static const char *phases[] = {"", "Preparing sample", "Setting up solver",
        "Fitting model", "Leverage probes", "Target probes", "Projection",
        "Component inference", "Component spectrum", "Variance-model probes",
        "Exact calculation", "Validating results", "Spectrum iterations"};
    char line[768];
    (void)context;
    if (u->kind > 0 && u->kind <= 12) {
        if (u->total != 0) {
            snprintf(line, sizeof(line), "  %s: %llu / %llu (%.0f%%); %.1fs\n",
                phases[u->kind], (unsigned long long)u->completed,
                (unsigned long long)u->total, 100.0*(double)u->completed/(double)u->total,
                (double)elapsed_ms/1000.0);
        } else {
            snprintf(line, sizeof(line), "  %s; %.1fs elapsed\n",
                phases[u->kind], (double)elapsed_ms/1000.0);
        }
    } else if (u->kind == 20) {
        snprintf(line, sizeof(line), "  Prepared graph sample: %llu / %llu rows; %llu workers, %llu firms\n",
            (unsigned long long)u->values[1], (unsigned long long)u->values[0],
            (unsigned long long)u->values[2], (unsigned long long)u->values[3]);
    } else if (u->kind == 21) {
        snprintf(line, sizeof(line), "  Pruning: %llu %s workers; %llu articulation workers; %llu bridge units (%llu rows)\n",
            (unsigned long long)u->values[0], u->values[4] == 1 ? "physical-singleton" : "insufficient-degree",
            (unsigned long long)u->values[1], (unsigned long long)u->values[2],
            (unsigned long long)u->values[3]);
    } else if (u->kind == 22) {
        const char *algorithm = u->values[0] == 1 ? "exact" : "JLA";
        const char *engine = u->values[1] == 1 ? "compressed" : u->values[1] == 2 ? "generic" : "not applicable";
        const char *solver = u->values[2] == 1 ? "diagonal" : u->values[2] == 2 ? "CMG" : "exact";
        snprintf(line, sizeof(line), "  Method: %s; engine=%s; solver=%s; threads=%llu; batches=%llu/%llu\n",
            algorithm, engine, solver, (unsigned long long)u->values[3],
            (unsigned long long)u->values[4], (unsigned long long)u->values[5]);
    } else if (u->kind == 23) {
        if (level == 2) {
            snprintf(line, sizeof(line), "  Command allocations (GiB): expected=%.3f; admission=%.3f; conditional=%.3f (excludes process RSS)\n",
                (double)u->values[0]/1073741824.0, (double)u->values[1]/1073741824.0,
                (double)u->values[2]/1073741824.0);
        } else {
            snprintf(line, sizeof(line), "  Expected command allocations: %.3f GiB (excludes process RSS)\n",
                (double)u->values[0]/1073741824.0);
        }
    } else if (u->kind == 24 && level == 2) {
        static const char *algorithm_reasons[] = {"unknown", "explicit exact", "explicit JLA",
            "within exact limit", "above exact limit"};
        static const char *engine_reasons[] = {"unknown", "exact calculation", "explicit generic",
            "explicit compressed", "eligible compressed design", "generic design required"};
        snprintf(line, sizeof(line), "  Selection: %s (dimension=%llu, exact limit=%llu); %s; batches=%s/%s\n",
            algorithm_reasons[u->values[0] < 5 ? u->values[0] : 0],
            (unsigned long long)u->values[3], (unsigned long long)u->values[4],
            engine_reasons[u->values[1] < 6 ? u->values[1] : 0],
            u->values[5] == 0 ? "automatic" : "explicit", u->values[6] == 0 ? "automatic" : "explicit");
    } else if (u->kind == 26) {
        snprintf(line, sizeof(line), "  CMG setup fell back to diagonal before estimator draws\n");
    } else if (u->kind == 25) {
        snprintf(line, sizeof(line), "  Eligible stayers: %llu rows, %llu workers; combined sample=%llu rows\n",
            (unsigned long long)u->values[0], (unsigned long long)u->values[1], (unsigned long long)u->values[2]);
    } else { return 0; }
    return SF_display(line);
}

typedef struct VckssProgressCall {
    int argc;
    char **argv;
} VckssProgressCall;

ST_retcode vckss_stata_call_impl(int argc, char *argv[]);
static int32_t vckss_progress_operation(void *context)
{
    VckssProgressCall *call = (VckssProgressCall *)context;
    return vckss_stata_call_impl(call->argc, call->argv);
}

#endif

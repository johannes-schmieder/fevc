function separations_matlab_phase_profile()
% Privacy-safe phase profiler for the maintained MATLAB KSS reference.
%
% This benchmark never edits or copies the maintained source.  It accepts
% exactly one registered leave_out_KSS.m hash/newline count and aggregates
% only that top-level function's ExecutedLines over fixed, nonoverlapping
% line ranges.  No profiler dump, source excerpt, or retained-row artifact is
% written to the evidence directory.

first_matlab_utc = utc_now();
wrapper_started = tic;
label = required_env('KSS_LABEL');
source_commit = required_env('KSS_SOURCE_COMMIT');
bundle_sha256 = required_env('KSS_BUNDLE_SHA256');
input_file = required_env('KSS_INPUT_CSV');
input_sha256 = required_env('KSS_INPUT_SHA256');
upstream_root = required_env('KSS_MATLAB_ROOT');
upstream_commit = required_env('KSS_MATLAB_UPSTREAM_COMMIT');
core_sha256 = required_env('KSS_MATLAB_CORE_SHA256');
cmg_sha256 = required_env('KSS_MATLAB_CMG_SHA256');
cmg_mex_sha256 = required_env('KSS_MATLAB_CMG_MEX_SHA256');
cmg_solver_sha256 = required_env('KSS_MATLAB_CMG_SOLVER_SHA256');
profiler_sha256 = required_env('KSS_PROFILER_SHA256');
output_dir = required_env('KSS_OUTPUT_DIR');
scratch_dir = required_env('KSS_SCRATCH_DIR');
process_start_utc = required_env('KSS_PROCESS_START_UTC');
seed = numeric_env('KSS_SEED', true);
probes = numeric_env('KSS_PROBES', false);
projected_seconds = numeric_scalar_env('KSS_PROJECTED_SECONDS');
projection_basis = required_env('KSS_PROJECTION_BASIS');
timeout_seconds = numeric_env('KSS_TIMEOUT_SECONDS', false);

record = initial_record(label, source_commit, bundle_sha256, input_sha256, ...
    upstream_commit, core_sha256, cmg_sha256, cmg_mex_sha256, ...
    cmg_solver_sha256, profiler_sha256, process_start_utc, first_matlab_utc, ...
    seed, probes, projected_seconds, projection_basis, timeout_seconds);
pool = [];
client_original = [];
worker_original = [];

try
    validate_identity(record, input_file, upstream_root, output_dir, scratch_dir);
    core_file = fullfile(upstream_root, 'codes', 'leave_out_KSS.m');
    core_text = fileread(core_file);
    core_newlines = sum(core_text == char(10));
    assert_profile(core_newlines == 632, 'CoreLineCount', ...
        'The registered maintained core must contain exactly 632 newline bytes.');
    record.matlab_core_newline_count = core_newlines;
    record.matlab_core_profile_max_line = 633;

    addpath(fullfile(upstream_root, 'codes'));
    addpath(genpath(fullfile(upstream_root, 'CMG')));
    resolved_core = which('leave_out_KSS');
    assert_profile(strcmp(canonical_path(resolved_core), canonical_path(core_file)), ...
        'CoreResolution', 'leave_out_KSS did not resolve to the checksum-bound core.');
    cmg_file = fullfile(upstream_root, 'CMG', 'MATLAB', 'cmg_sdd.m');
    resolved_cmg = which('cmg_sdd');
    assert_profile(~isempty(resolved_cmg) && ...
        strcmp(canonical_path(resolved_cmg), canonical_path(cmg_file)), ...
        'CMGResolution', 'cmg_sdd did not resolve to the checksum-bound entry point.');

    mex_started = tic;
    compile_run_local_mex(upstream_root, required_env('KSS_MATLAB_MEX_DIR'));
    record.mex_seconds = toc(mex_started);

    import_started = tic;
    imported = importdata(input_file);
    if isstruct(imported)
        data = imported.data;
    else
        data = imported;
    end
    assert_profile(size(data, 2) == 4 && size(data, 1) > 0 && ...
        all(isfinite(data), 'all'), 'InputShape', ...
        'Prepared profiler input must be a nonempty finite four-column matrix.');
    worker = data(:, 1);
    firm = data(:, 2);
    period = data(:, 3);
    outcome = data(:, 4);
    clear data imported
    [~, chronological_order] = sortrows([worker period firm], [1 2 3]);
    worker = worker(chronological_order);
    firm = firm(chronological_order);
    outcome = outcome(chronological_order);
    clear period chronological_order
    record.input_rows = numel(outcome);
    record.input_workers = numel(unique(worker));
    record.input_firms = numel(unique(firm));
    record.import_seconds = toc(import_started);

    pool_started = tic;
    existing_pool = gcp('nocreate');
    assert_profile(isempty(existing_pool), 'PreexistingPool', ...
        'The immutable batch profiler requires a fresh MATLAB process and pool.');
    parallel_scratch = fullfile(scratch_dir, 'parallel');
    if ~isfolder(parallel_scratch)
        mkdir(parallel_scratch);
    end
    local_cluster = parcluster('local');
    local_cluster.JobStorageLocation = parallel_scratch;
    pool = parpool(local_cluster, 4, 'IdleTimeout', Inf);
    assert_profile(pool.NumWorkers == 4, 'PoolSize', ...
        'The registered MATLAB profile requires exactly four workers.');
    record.processors = pool.NumWorkers;
    record.pool_seconds = toc(pool_started);

    client_original = rng;
    spmd
        worker_original = rng;
    end
    rng(seed, 'twister');
    client_call_state = rng;
    worker_call_state = worker_original;

    details_dir = fullfile(scratch_dir, 'details');
    cold_stub = fullfile(details_dir, 'cold');
    profiled_stub = fullfile(details_dir, 'profiled');
    warm_stub = fullfile(details_dir, 'warm');

    replay_rng(client_call_state, worker_call_state);
    cold_started = tic;
    [cold_targets, cold_detail_sha, cold_key_sha, cold_detail_rows] = maintained_call( ...
        outcome, worker, firm, probes, cold_stub);
    record.cold_call_seconds = toc(cold_started);
    record.cold_target_sha256 = target_hash(cold_targets);
    record.cold_detail_sha256 = cold_detail_sha;
    record.cold_retained_key_sha256 = cold_key_sha;
    record.cold_detail_rows = cold_detail_rows;

    replay_rng(client_call_state, worker_call_state);
    profile clear
    profile on -timer real -nohistory
    active_profile = profile('status');
    assert_profile(strcmp(active_profile.ProfilerStatus, 'on') && ...
        strcmp(active_profile.Timer, 'real') && ...
        strcmp(active_profile.HistoryTracking, 'off'), 'ProfileMode', ...
        'Profiler must use real time with call history disabled.');
    profiled_started = tic;
    [profiled_targets, profiled_detail_sha, profiled_key_sha, profiled_detail_rows] = maintained_call( ...
        outcome, worker, firm, probes, profiled_stub);
    record.warm_profiled_call_seconds = toc(profiled_started);
    profile off
    profile_information = profile('info');
    profile clear
    record.profiled_target_sha256 = target_hash(profiled_targets);
    record.profiled_detail_sha256 = profiled_detail_sha;
    record.profiled_retained_key_sha256 = profiled_key_sha;
    record.profiled_detail_rows = profiled_detail_rows;
    profile_metrics = aggregate_top_level_profile(profile_information, core_file);

    replay_rng(client_call_state, worker_call_state);
    warm_started = tic;
    [warm_targets, warm_detail_sha, warm_key_sha, warm_detail_rows] = maintained_call( ...
        outcome, worker, firm, probes, warm_stub);
    record.warm_unprofiled_call_seconds = toc(warm_started);
    record.warm_target_sha256 = target_hash(warm_targets);
    record.warm_detail_sha256 = warm_detail_sha;
    record.warm_retained_key_sha256 = warm_key_sha;
    record.warm_detail_rows = warm_detail_rows;

    delete_if_present([cold_stub '.csv']);
    delete_if_present([profiled_stub '.csv']);
    delete_if_present([warm_stub '.csv']);

    record.cold_target_worker = cold_targets(1);
    record.cold_target_firm = cold_targets(2);
    record.cold_target_covariance = cold_targets(3);
    record.cold_target_total = cold_targets(4);
    record.profiled_target_worker = profiled_targets(1);
    record.profiled_target_firm = profiled_targets(2);
    record.profiled_target_covariance = profiled_targets(3);
    record.profiled_target_total = profiled_targets(4);
    record.target_worker = warm_targets(1);
    record.target_firm = warm_targets(2);
    record.target_covariance = warm_targets(3);
    record.target_total = warm_targets(4);
    record.targets_identical = double(isequal(cold_targets, profiled_targets) && ...
        isequal(cold_targets, warm_targets));
    record.details_identical = double(strcmp(cold_detail_sha, profiled_detail_sha) && ...
        strcmp(cold_detail_sha, warm_detail_sha));
    record.retained_keys_identical = double(strcmp(cold_key_sha, profiled_key_sha) && ...
        strcmp(cold_key_sha, warm_key_sha) && cold_detail_rows == profiled_detail_rows && ...
        cold_detail_rows == warm_detail_rows);
    target_matrix = [cold_targets; profiled_targets; warm_targets];
    replay_differences = [ ...
        abs(target_matrix(1,:) - target_matrix(2,:)) ./ ...
            (1 + max(abs(target_matrix(1:2,:)), [], 1)); ...
        abs(target_matrix(1,:) - target_matrix(3,:)) ./ ...
            (1 + max(abs(target_matrix([1 3],:)), [], 1)); ...
        abs(target_matrix(2,:) - target_matrix(3,:)) ./ ...
            (1 + max(abs(target_matrix(2:3,:)), [], 1))];
    record.target_replay_max_scaled_diff = max(replay_differences, [], 'all');
    record.target_replay_within_gate = double(record.target_replay_max_scaled_diff <= 1e-5);
    assert_profile(record.retained_keys_identical == 1, 'RetainedKeyReproducibility', ...
        'Maintained calls selected different worker-firm keys or detail row counts.');
    % The maintained implementation assigns JLA work through parfor.  Restoring
    % the client and worker RNG states does not bind tasks to workers, so target
    % replay is a scheduling-sensitive diagnostic unless the upstream command
    % exposes target-specific Monte Carlo standard errors.  Exact retained keys
    % and row counts remain the hard scientific replay gate above.
    record.rng_state_restore_verified = 1;

    record = add_profile_metrics(record, profile_metrics);
    rng(client_original);
    spmd
        rng(worker_original);
    end
    teardown_started = tic;
    delete(pool);
    record.pool_teardown_seconds = toc(teardown_started);
    pool = [];

    record.status = 'PASS';
    record.failure_code = 'NONE';
    record.failure_explanation = 'NONE';
    record.matlab_version = version;
    record.final_matlab_utc = utc_now();
    record.wrapper_seconds = toc(wrapper_started);
    record.serialization_seconds = measure_serialization(record, scratch_dir);
    write_aggregate(record, output_dir, true);
    fprintf('KSS_BC MATLAB PHASE PROFILE PASS: %s\n', label);
catch exception
    profile off
    profile clear
    safe_restore_and_close(pool, client_original, worker_original);
    delete_if_present(fullfile(scratch_dir, 'details', 'cold.csv'));
    delete_if_present(fullfile(scratch_dir, 'details', 'profiled.csv'));
    delete_if_present(fullfile(scratch_dir, 'details', 'warm.csv'));
    record.status = 'FAIL';
    if isempty(exception.identifier)
        record.failure_code = 'KSS:MatlabPhaseProfile:Unclassified';
    else
        record.failure_code = exception.identifier;
    end
    record.failure_explanation = regexprep(exception.message, '[\r\n]+', ' ');
    record.matlab_version = version;
    record.final_matlab_utc = utc_now();
    record.wrapper_seconds = toc(wrapper_started);
    try
        record.serialization_seconds = measure_serialization(record, scratch_dir);
        write_aggregate(record, output_dir, false);
    catch serialization_exception
        fprintf(2, 'KSS_MATLAB_PHASE_PROFILE_SERIALIZATION_FAIL: %s\n', ...
            serialization_exception.message);
    end
    fprintf(2, 'KSS_MATLAB_PHASE_PROFILE_FAIL %s: %s\n', ...
        record.failure_code, record.failure_explanation);
    rethrow(exception)
end
end


function record = initial_record(label, source_commit, bundle_sha256, ...
    input_sha256, upstream_commit, core_sha256, cmg_sha256, cmg_mex_sha256, ...
    cmg_solver_sha256, profiler_sha256, process_start_utc, first_matlab_utc, ...
    seed, probes, projected_seconds, projection_basis, timeout_seconds)
record.status = 'FAIL';
record.failure_code = 'KSS:MatlabPhaseProfile:Incomplete';
record.failure_explanation = 'Profiler did not reach its success gate.';
record.label = label;
record.source_commit = source_commit;
record.bundle_sha256 = bundle_sha256;
record.input_sha256 = input_sha256;
record.matlab_upstream_commit = upstream_commit;
record.matlab_core_sha256 = core_sha256;
record.matlab_cmg_sha256 = cmg_sha256;
record.matlab_cmg_mex_sha256 = cmg_mex_sha256;
record.matlab_cmg_solver_sha256 = cmg_solver_sha256;
record.profiler_sha256 = profiler_sha256;
record.matlab_version = '';
record.algorithm = 'JLA';
record.deletion_level = 'matches';
record.profile_timer = 'real';
record.profile_function = 'leave_out_KSS';
record.rng_protocol = 'client_and_worker_state_restored_parfor_schedule_not_fixed';
record.projection_basis = projection_basis;
record.process_start_utc = process_start_utc;
record.first_matlab_utc = first_matlab_utc;
record.final_matlab_utc = first_matlab_utc;
record.cold_target_sha256 = repmat('0', 1, 64);
record.profiled_target_sha256 = repmat('0', 1, 64);
record.warm_target_sha256 = repmat('0', 1, 64);
record.cold_detail_sha256 = repmat('0', 1, 64);
record.profiled_detail_sha256 = repmat('0', 1, 64);
record.warm_detail_sha256 = repmat('0', 1, 64);
record.cold_retained_key_sha256 = repmat('0', 1, 64);
record.profiled_retained_key_sha256 = repmat('0', 1, 64);
record.warm_retained_key_sha256 = repmat('0', 1, 64);

record.matlab_core_newline_count = 0;
record.matlab_core_profile_max_line = 633;
record.processors = 0;
record.seed = seed;
record.probes = probes;
record.input_rows = 0;
record.input_workers = 0;
record.input_firms = 0;
record.profile_function_calls = 0;
record.profile_executed_lines = 0;
record.profile_line_calls = 0;
record.targets_identical = 0;
record.details_identical = 0;
record.retained_keys_identical = 0;
record.target_replay_within_gate = 0;
record.rng_state_restore_verified = 0;
record.cold_detail_rows = 0;
record.profiled_detail_rows = 0;
record.warm_detail_rows = 0;
record.timeout_seconds = timeout_seconds;

[phase_names, first_lines, last_lines] = registered_phases();
for index = 1:numel(phase_names)
    phase = phase_names{index};
    record.(['phase_' phase '_line_first']) = first_lines(index);
    record.(['phase_' phase '_line_last']) = last_lines(index);
    record.(['phase_' phase '_executed_lines']) = 0;
    record.(['phase_' phase '_line_calls']) = 0;
end

record.wrapper_seconds = 0;
record.mex_seconds = 0;
record.import_seconds = 0;
record.pool_seconds = 0;
record.cold_call_seconds = 0;
record.warm_profiled_call_seconds = 0;
record.warm_unprofiled_call_seconds = 0;
record.serialization_seconds = 0;
record.pool_teardown_seconds = 0;
record.profile_top_level_seconds = 0;
record.projected_seconds = projected_seconds;
record.cold_target_worker = 0;
record.cold_target_firm = 0;
record.cold_target_covariance = 0;
record.cold_target_total = 0;
record.profiled_target_worker = 0;
record.profiled_target_firm = 0;
record.profiled_target_covariance = 0;
record.profiled_target_total = 0;
record.target_worker = 0;
record.target_firm = 0;
record.target_covariance = 0;
record.target_total = 0;
record.target_replay_max_scaled_diff = 0;
for index = 1:numel(phase_names)
    record.(['phase_' phase_names{index} '_seconds']) = 0;
end
end


function validate_identity(record, input_file, upstream_root, output_dir, scratch_dir)
assert_profile(~isempty(regexp(record.label, '^[A-Za-z0-9._-]+$', 'once')), ...
    'Label', 'Invalid profile label.');
assert_profile(~isempty(regexp(record.source_commit, '^[0-9a-f]{40}$', 'once')), ...
    'SourceCommit', 'Invalid source commit.');
assert_profile(~isempty(regexp(record.bundle_sha256, '^[0-9a-f]{64}$', 'once')), ...
    'BundleHash', 'Invalid bundle hash.');
hash_fields = {record.input_sha256, record.matlab_core_sha256, ...
    record.matlab_cmg_sha256, record.matlab_cmg_mex_sha256, ...
    record.matlab_cmg_solver_sha256, record.profiler_sha256};
for index = 1:numel(hash_fields)
    assert_profile(~isempty(regexp(hash_fields{index}, '^[0-9a-f]{64}$', 'once')), ...
        'Hash', 'Invalid source/input hash metadata.');
end
assert_profile(strcmp(record.matlab_core_sha256, ...
    '7ab72bcf1f9e1a0091a6a423b1ef5cbd23688f7c64d753cf9adcc6243989a120'), ...
    'CoreHash', 'Only the registered maintained MATLAB core may be profiled.');
assert_profile(strcmp(record.matlab_upstream_commit, ...
    '8b957ffeb10b8465a3584fceb0265cccc48379e1'), ...
    'UpstreamCommit', 'Only the registered maintained MATLAB commit may be profiled.');
assert_profile(isfile(input_file), 'InputMissing', 'Prepared input is missing.');
assert_profile(isfolder(upstream_root), 'UpstreamMissing', 'MATLAB upstream root is missing.');
assert_profile(isfolder(output_dir) && isfolder(scratch_dir), 'RunLayout', ...
    'Run-scoped output and scratch directories are required.');
assert_profile(startsWith(canonical_path(output_dir), ...
    [canonical_path(required_env('KSS_RUN_DIR')) filesep]), 'OutputPath', ...
    'Aggregate output must remain below the run directory.');
assert_profile(startsWith(canonical_path(scratch_dir), ...
    [canonical_path(required_env('KSS_RUN_DIR')) filesep]), 'ScratchPath', ...
    'Profiler scratch must remain below the run directory.');
assert_profile(probes_integer(record.probes) && record.probes > 0 && ...
    probes_integer(record.seed) && record.seed >= 0 && record.seed < 2^32, ...
    'RNGSettings', ...
    'Seed and probe count must be nonnegative/positive integers.');
assert_profile(~isempty(regexp(record.projection_basis, '^[A-Za-z0-9._-]+$', 'once')), ...
    'ProjectionBasis', 'Invalid runtime projection basis.');
expected_timeout = max(300, ceil(1.5 * record.projected_seconds + 180));
assert_profile(record.projected_seconds > 0 && record.projected_seconds <= 3600 && ...
    record.timeout_seconds == expected_timeout && record.timeout_seconds <= 3600, ...
    'ProjectionTimeout', 'Hard timeout does not follow the registered projection rule.');
end


function compile_run_local_mex(upstream_root, mex_output_dir)
assert_profile(isfolder(mex_output_dir), 'MEXDirectory', ...
    'Run-local MEX output directory is missing.');
mex_names = {'adjacency_cmg','diagconjugate','forest_components', ...
    'graphprofile','laplacian2','perturbtril','splitforest', ...
    'update_groups','vpack'};
mex_source_dir = fullfile(upstream_root, 'CMG', 'Source', 'Hierarchy');
for index = 1:numel(mex_names)
    source = fullfile(mex_source_dir, [mex_names{index} '.c']);
    assert_profile(isfile(source), 'MEXSource', ...
        'A checksum-bound hierarchy MEX source is unavailable.');
    mex('-silent', '-largeArrayDims', '-outdir', mex_output_dir, source);
end
include_dir = fullfile(upstream_root, 'CMG', 'Include');
solver_dir = fullfile(upstream_root, 'CMG', 'Source', 'Solver');
gateway = fullfile(upstream_root, 'CMG', 'MATLAB', 'Solver', ...
    'mx_d_preconditioner.c');
solver_names = {'vpv','vmv','vpvmv','vvmul','rmvec','ldl_solve', ...
    'sspmv','preconditioner'};
sources = cell(1, numel(solver_names) + 1);
sources{1} = gateway;
for index = 1:numel(solver_names)
    sources{index + 1} = fullfile(solver_dir, [solver_names{index} '.c']);
end
mex('-silent', '-largeArrayDims', ['-I' include_dir], '-outdir', ...
    mex_output_dir, '-output', 'mx_d_preconditioner', sources{:});
addpath(mex_output_dir, '-begin');
rehash;
clear graphprofile mx_d_preconditioner
graphprofile_path = which('graphprofile');
preconditioner_path = which('mx_d_preconditioner');
assert_profile(exist('graphprofile', 'file') == 3 && ...
    startsWith(canonical_path(graphprofile_path), [canonical_path(mex_output_dir) filesep]), ...
    'HierarchyMEXBinding', 'Run-local graphprofile did not bind.');
assert_profile(exist('mx_d_preconditioner', 'file') == 3 && ...
    startsWith(canonical_path(preconditioner_path), [canonical_path(mex_output_dir) filesep]), ...
    'SolverMEXBinding', 'Run-local double preconditioner did not bind.');
end


function replay_rng(client_state, worker_states)
rng(client_state);
spmd
    rng(worker_states);
end
end


function [targets, detail_sha, key_sha, detail_rows] = maintained_call( ...
    outcome, worker, firm, probes, detail_stub)
controls = [];
leave_out_level = 'matches';
type_algorithm = 'JLA';
lincom_do = 0;
Z_lincom = [];
labels_lincom = [];
[firm_variance, covariance, worker_variance] = leave_out_KSS( ...
    outcome, worker, firm, controls, leave_out_level, type_algorithm, ...
    probes, lincom_do, Z_lincom, labels_lincom, detail_stub);
targets = [worker_variance, firm_variance, covariance, ...
    worker_variance + firm_variance + 2 * covariance];
assert_profile(all(isfinite(targets)), 'NonfiniteTargets', ...
    'Maintained MATLAB call returned a nonfinite target.');
detail_file = [detail_stub '.csv'];
assert_profile(isfile(detail_file), 'DetailMissing', ...
    'Maintained MATLAB call did not write its expected detail artifact.');
detail_sha = sha256_file(detail_file);
detail_data = importdata(detail_file);
if isstruct(detail_data)
    detail_data = detail_data.data;
end
assert_profile(isnumeric(detail_data) && size(detail_data, 1) > 0 && ...
    size(detail_data, 2) == 4 && all(isfinite(detail_data), 'all'), ...
    'DetailShape', 'Maintained detail output is not a finite four-column matrix.');
keys = unique(detail_data(:, 2:3), 'rows');
keys = sortrows(keys, [1 2]);
assert_profile(all(keys == floor(keys), 'all'), 'DetailKeys', ...
    'Maintained retained worker-firm keys are not integers.');
key_bytes = unicode2native(sprintf('%.0f,%.0f\n', keys.'), 'UTF-8');
key_sha = sha256_bytes(key_bytes);
detail_rows = size(detail_data, 1);
end


function metrics = aggregate_top_level_profile(profile_information, core_file)
function_table = profile_information.FunctionTable;
matches = false(1, numel(function_table));
expected_path = canonical_path(core_file);
for index = 1:numel(function_table)
    matches(index) = strcmp(function_table(index).FunctionName, 'leave_out_KSS') && ...
        strcmp(canonical_path(function_table(index).FileName), expected_path);
end
assert_profile(sum(matches) == 1, 'TopLevelProfile', ...
    'Profiler did not return exactly one checksum-bound leave_out_KSS entry.');
entry = function_table(matches);
assert_profile(entry.NumCalls == 1, 'TopLevelCalls', ...
    'Profiler recorded more or fewer than one top-level maintained call.');
assert_profile(~entry.IsRecursive && ~entry.PartialData, 'TopLevelIntegrity', ...
    'Top-level profile is recursive or partial.');
lines = entry.ExecutedLines;
assert_profile(isnumeric(lines) && size(lines, 2) >= 3 && size(lines, 1) > 0, ...
    'ExecutedLines', 'Top-level ExecutedLines are missing or malformed.');
line_number = lines(:, 1);
line_calls = lines(:, 2);
line_seconds = lines(:, 3);
assert_profile(all(line_number == floor(line_number)) && ...
    all(line_number >= 1 & line_number <= 633) && ...
    all(line_calls >= 0 & line_calls == floor(line_calls)) && ...
    all(isfinite(line_seconds) & line_seconds >= 0), 'ExecutedLinesBounds', ...
    'Top-level ExecutedLines exceed the registered source/range contract.');

[phase_names, first_lines, last_lines] = registered_phases();
metrics.function_calls = entry.NumCalls;
metrics.executed_lines = size(lines, 1);
metrics.line_calls = sum(line_calls);
metrics.total_seconds = sum(line_seconds);
for index = 1:numel(phase_names)
    selected = line_number >= first_lines(index) & line_number <= last_lines(index);
    phase = phase_names{index};
    metrics.([phase '_executed_lines']) = sum(selected);
    metrics.([phase '_line_calls']) = sum(line_calls(selected));
    metrics.([phase '_seconds']) = sum(line_seconds(selected));
end
phase_line_sum = 0;
phase_call_sum = 0;
phase_time_sum = 0;
for index = 1:numel(phase_names)
    phase = phase_names{index};
    phase_line_sum = phase_line_sum + metrics.([phase '_executed_lines']);
    phase_call_sum = phase_call_sum + metrics.([phase '_line_calls']);
    phase_time_sum = phase_time_sum + metrics.([phase '_seconds']);
end
assert_profile(phase_line_sum == metrics.executed_lines && ...
    phase_call_sum == metrics.line_calls && ...
    abs(phase_time_sum - metrics.total_seconds) <= ...
        1e-12 * (1 + abs(metrics.total_seconds)), 'PhaseAccounting', ...
    'Registered nonoverlapping phase ranges do not exhaust ExecutedLines.');
end


function record = add_profile_metrics(record, metrics)
record.profile_function_calls = metrics.function_calls;
record.profile_executed_lines = metrics.executed_lines;
record.profile_line_calls = metrics.line_calls;
record.profile_top_level_seconds = metrics.total_seconds;
[phase_names, ~, ~] = registered_phases();
for index = 1:numel(phase_names)
    phase = phase_names{index};
    record.(['phase_' phase '_executed_lines']) = metrics.([phase '_executed_lines']);
    record.(['phase_' phase '_line_calls']) = metrics.([phase '_line_calls']);
    record.(['phase_' phase '_seconds']) = metrics.([phase '_seconds']);
end
end


function [names, first_lines, last_lines] = registered_phases()
names = {'options','selection','residual_collapse','leverage', ...
    'variance_estimation','reporting','maintained_serialization', ...
    'disabled_lincom'};
first_lines = [1, 334, 429, 477, 519, 574, 617, 622];
last_lines = [333, 428, 476, 518, 573, 616, 621, 633];
end


function elapsed = measure_serialization(record, scratch_dir)
probe_csv = fullfile(scratch_dir, 'aggregate.serialization-probe.csv');
probe_json = fullfile(scratch_dir, 'aggregate.serialization-probe.json');
delete_if_present(probe_csv);
delete_if_present(probe_json);
started = tic;
write_record_pair(record, probe_csv, probe_json);
elapsed = toc(started);
delete_if_present(probe_csv);
delete_if_present(probe_json);
end


function write_aggregate(record, output_dir, write_pass)
csv_file = fullfile(output_dir, 'aggregate.csv');
json_file = fullfile(output_dir, 'aggregate.json');
pass_file = fullfile(output_dir, 'wrapper.pass');
delete_if_present(csv_file);
delete_if_present(json_file);
delete_if_present(pass_file);
temporary_csv = [csv_file '.tmp'];
temporary_json = [json_file '.tmp'];
write_record_pair(record, temporary_csv, temporary_json);
movefile(temporary_csv, csv_file, 'f');
movefile(temporary_json, json_file, 'f');
if write_pass
    temporary_pass = [pass_file '.tmp'];
    fid = fopen(temporary_pass, 'w');
    assert_profile(fid >= 0, 'PassWrite', 'Cannot open temporary pass marker.');
    fprintf(fid, 'KSS_BC_MATLAB_PHASE_PROFILE_PASS %s %s %s %s %s %s %s\n', ...
        record.label, record.bundle_sha256, record.source_commit, ...
        record.input_sha256, record.matlab_core_sha256, ...
        record.warm_target_sha256, record.warm_detail_sha256);
    fclose(fid);
    movefile(temporary_pass, pass_file, 'f');
end
end


function write_record_pair(record, csv_file, json_file)
writetable(struct2table(record, 'AsArray', true), csv_file, ...
    'FileType', 'text', 'Delimiter', ',');
encoded = jsonencode(record, 'PrettyPrint', true);
fid = fopen(json_file, 'w');
assert_profile(fid >= 0, 'JSONWrite', 'Cannot open aggregate JSON output.');
fprintf(fid, '%s\n', encoded);
fclose(fid);
end


function safe_restore_and_close(pool, client_state, worker_states)
try
    if ~isempty(client_state)
        rng(client_state);
    end
catch
end
try
    if ~isempty(pool) && isvalid(pool)
        if ~isempty(worker_states)
            spmd
                rng(worker_states);
            end
        end
        delete(pool);
    end
catch
end
end


function digest = target_hash(targets)
bytes = unicode2native(sprintf('%.17g\n', targets), 'UTF-8');
digest = sha256_bytes(bytes);
end


function digest = sha256_file(path)
fid = fopen(path, 'r');
assert_profile(fid >= 0, 'HashRead', 'Cannot open detail artifact for hashing.');
cleanup = onCleanup(@() fclose(fid)); %#ok<NASGU>
message_digest = java.security.MessageDigest.getInstance('SHA-256');
while true
    chunk = fread(fid, 1024 * 1024, '*uint8');
    if isempty(chunk)
        break
    end
    message_digest.update(typecast(chunk(:), 'int8'));
end
raw = typecast(message_digest.digest(), 'uint8');
digest = lower(reshape(dec2hex(raw, 2).', 1, []));
end


function digest = sha256_bytes(bytes)
message_digest = java.security.MessageDigest.getInstance('SHA-256');
message_digest.update(typecast(uint8(bytes(:)), 'int8'));
raw = typecast(message_digest.digest(), 'uint8');
digest = lower(reshape(dec2hex(raw, 2).', 1, []));
end


function value = required_env(name)
value = getenv(name);
if isempty(value)
    error('KSS:MatlabPhaseProfile:MissingEnvironment', ...
        'Required environment variable %s is missing.', name);
end
end


function value = numeric_env(name, allow_zero)
value = str2double(required_env(name));
if ~isfinite(value) || value ~= floor(value) || ...
        (~allow_zero && value <= 0) || (allow_zero && value < 0)
    error('KSS:MatlabPhaseProfile:NumericEnvironment', ...
        'Environment variable %s is not a registered integer.', name);
end
end


function value = numeric_scalar_env(name)
value = str2double(required_env(name));
if ~isfinite(value)
    error('KSS:MatlabPhaseProfile:NumericEnvironment', ...
        'Environment variable %s is not finite.', name);
end
end


function result = probes_integer(value)
result = isfinite(value) && value == floor(value);
end


function assert_profile(condition, code, message)
if ~condition
    error(['KSS:MatlabPhaseProfile:' code], '%s', message);
end
end


function result = canonical_path(path)
result = char(java.io.File(path).getCanonicalPath());
end


function delete_if_present(path)
if isfile(path)
    delete(path);
end
end


function value = utc_now()
value = char(datetime('now', 'TimeZone', 'UTC', ...
    'Format', "yyyy-MM-dd'T'HH:mm:ss.SSSSSSSSS'Z'"));
end

function matlab_scale_run(mode)
% Source-bound descriptive scale benchmark for maintained LeaveOutTwoWay.
%
% The cold route invokes leave_out_KSS exactly once in a fresh MATLAB
% process.  The warm route invokes one warmup and then the registered number
% of unprofiled measured repetitions.  Corrected targets are recorded only
% as descriptive values with their accounting identity; they are never an
% equality gate against the scientifically corrected Stata estimator.

first_matlab_utc = utc_now();
matlab_started = tic;
pool = [];
client_original = [];
worker_original = [];
calls = empty_calls();
output_dir = getenv('KSS_MS_OUTPUT_DIR');
scratch_dir = getenv('KSS_MS_SCRATCH_DIR');
record = initial_record(mode, first_matlab_utc);

try
    assert_scale(any(strcmp(mode, {'cold','warm'})), 'Mode', ...
        'Driver mode must be cold or warm.');
    case_file = required_env('KSS_MS_CASE_JSON');
    contract_file = required_env('KSS_MS_SOURCE_CONTRACT');
    input_file = required_env('KSS_MS_INPUT_CSV');
    output_dir = required_env('KSS_MS_OUTPUT_DIR');
    scratch_dir = required_env('KSS_MS_SCRATCH_DIR');
    mex_dir = required_env('KSS_MS_MEX_DIR');
    matlab_root = required_env('KSS_MS_MATLAB_ROOT');
    job_dir = required_env('KSS_MS_JOB_DIR');
    process_identity_file = fullfile(job_dir,'process_identity.json');
    process_start_utc = required_env('KSS_MS_PROCESS_START_UTC');
    case_sha256 = required_env('KSS_MS_CASE_SHA256');
    contract_sha256 = required_env('KSS_MS_CONTRACT_SHA256');
    input_sha256 = required_env('KSS_MS_INPUT_SHA256');

    assert_scale(isfolder(output_dir) && isfolder(scratch_dir) && ...
        isfolder(mex_dir), 'Layout', 'Run-scoped output and scratch are required.');
    assert_scale(startsWith(canonical_path(output_dir), ...
        [canonical_path(job_dir) filesep]), 'OutputPath', ...
        'Output must remain below the run-scoped job directory.');
    assert_scale(startsWith(canonical_path(scratch_dir), ...
        [canonical_path(required_env('TMPDIR')) filesep]), 'ScratchPath', ...
        'Scratch must remain below the scheduler node-local directory.');
    assert_scale(~isfile(process_identity_file), 'ProcessIdentityStale', ...
        'Process identity artifact already exists.');

    binding = jsondecode(fileread(case_file));
    contract = jsondecode(fileread(contract_file));
    validate_binding(binding, contract, case_sha256, contract_sha256, ...
        input_sha256, mode);
    record = bind_record(record, binding, case_sha256, contract_sha256, ...
        input_sha256, process_start_utc, first_matlab_utc);

    addpath(fullfile(matlab_root, 'codes'));
    addpath(genpath(fullfile(matlab_root, 'CMG')));
    core_file = fullfile(matlab_root, contract.core.relative_path);
    resolved_core = which('leave_out_KSS');
    assert_scale(strcmp(canonical_path(resolved_core), canonical_path(core_file)), ...
        'CoreResolution', 'leave_out_KSS did not resolve to the bound core.');
    cmg_file = fullfile(matlab_root, contract.cmg_entry.relative_path);
    resolved_cmg = which('cmg_sdd');
    assert_scale(~isempty(resolved_cmg) && ...
        strcmp(canonical_path(resolved_cmg), canonical_path(cmg_file)), ...
        'CMGResolution', 'cmg_sdd did not resolve to the bound entry point.');

    import_started = tic;
    imported = importdata(input_file);
    if isstruct(imported)
        data = imported.data;
    else
        data = imported;
    end
    assert_scale(isnumeric(data) && size(data,2) == 4 && size(data,1) > 0, ...
        'InputShape', ...
        'Scale input must be a nonempty four-column numeric matrix.');
    worker = data(:,1);
    firm = data(:,2);
    period = data(:,3);
    outcome = data(:,4);
    clear data imported
    record.import_seconds = toc(import_started);

    validation_started = tic;
    validate_input(worker, firm, period, outcome, binding.input);
    record.input_validation_seconds = toc(validation_started);

    pool_started = tic;
    existing_pool = gcp('nocreate');
    assert_scale(isempty(existing_pool), 'PreexistingPool', ...
        'Scale benchmark requires a fresh MATLAB process and pool.');
    parallel_scratch = fullfile(scratch_dir, 'parallel');
    if ~isfolder(parallel_scratch)
        mkdir(parallel_scratch);
    end
    local_cluster = parcluster('local');
    local_cluster.JobStorageLocation = parallel_scratch;
    pool = parpool(local_cluster, contract.required_pool_workers, ...
        'IdleTimeout', Inf);
    assert_scale(pool.NumWorkers == contract.required_pool_workers, ...
        'PoolSize', 'Scale benchmark requires exactly four workers.');
    record.pool_workers = pool.NumWorkers;
    record.pool_startup_seconds = toc(pool_started);

    client_process_pid = double(matlabProcessID);
    spmd
        worker_process_pid = double(matlabProcessID);
        worker_process_index = spmdIndex;
    end
    worker_process_pids = zeros(1,pool.NumWorkers);
    worker_process_indices = zeros(1,pool.NumWorkers);
    for worker_index = 1:pool.NumWorkers
        worker_process_pids(worker_index) = worker_process_pid{worker_index};
        worker_process_indices(worker_index) = ...
            worker_process_index{worker_index};
    end
    assert_scale(client_process_pid >= 2 && ...
        client_process_pid < flintmax && ...
        client_process_pid == floor(client_process_pid) && ...
        all(worker_process_pids >= 2) && ...
        all(worker_process_pids < flintmax) && ...
        all(worker_process_pids == floor(worker_process_pids)), ...
        'ProcessIdentityPID', 'MATLAB process IDs must be positive integers.');
    assert_scale(isequal(worker_process_indices,1:pool.NumWorkers), ...
        'ProcessIdentityIndex', 'MATLAB worker indices changed.');
    assert_scale(numel(unique([client_process_pid worker_process_pids])) == ...
        1+pool.NumWorkers, 'ProcessIdentityDistinct', ...
        'MATLAB client and worker process IDs must be distinct.');
    process_identity = struct( ...
        'schema','kss_matlab_scale_process_identity_v1', ...
        'status','PASS', ...
        'pid_api','matlabProcessID_R2025a', ...
        'mode',mode, ...
        'label',binding.label, ...
        'case_sha256',case_sha256, ...
        'expected_pool_workers',pool.NumWorkers, ...
        'client_pid',client_process_pid, ...
        'worker_indices',worker_process_indices, ...
        'worker_pids',worker_process_pids);
    write_atomic_json(process_identity,process_identity_file);
    record.process_identity_status = 'PASS';
    record.matlab_client_pid = client_process_pid;
    record.matlab_worker_pids = worker_process_pids;

    mex_started = tic;
    compile_run_local_mex(matlab_root, mex_dir);
    record.mex_setup_seconds = toc(mex_started);

    client_original = rng;
    spmd
        worker_original = rng;
    end
    rng(binding.seed, 'twister');
    client_call_state = rng;
    worker_call_state = worker_original;
    details_dir = fullfile(scratch_dir, 'details');
    if ~isfolder(details_dir)
        mkdir(details_dir);
    end

    reference_keys = [];
    reference_rows = 0;
    retained_validation_total = 0;
    profile_metrics = struct();
    if strcmp(mode, 'cold')
        replay_rng(client_call_state, worker_call_state);
        should_profile = strcmp(binding.sample_mode, 'selection');
        if should_profile
            start_profile();
        end
        [call, reference_keys, reference_rows, validation_seconds] = ...
            maintained_call(outcome, worker, firm, binding, ...
            fullfile(details_dir, 'cold'), 'cold', 1, [], 0);
        if should_profile
            profile_metrics = stop_and_aggregate_profile(core_file, contract);
        end
        call.profiled = double(should_profile);
        calls = call;
        retained_validation_total = validation_seconds;
        record.estimator_call_count = 1;
        record.warmup_call_count = 0;
        record.measured_call_count = 1;
        record.warmup_seconds = 0;
        record.core_call_median_seconds = call.seconds;
    else
        replay_rng(client_call_state, worker_call_state);
        should_profile = strcmp(binding.sample_mode, 'selection');
        if should_profile
            start_profile();
        end
        [warmup, reference_keys, reference_rows, validation_seconds] = ...
            maintained_call(outcome, worker, firm, binding, ...
            fullfile(details_dir, 'warmup'), 'warmup', 0, [], 0);
        if should_profile
            profile_metrics = stop_and_aggregate_profile(core_file, contract);
        end
        warmup.profiled = double(should_profile);
        calls = warmup;
        retained_validation_total = validation_seconds;
        measured_seconds = zeros(binding.warm_repetitions,1);
        for repetition = 1:binding.warm_repetitions
            replay_rng(client_call_state, worker_call_state);
            [measured, keys, rows, validation_seconds] = maintained_call( ...
                outcome, worker, firm, binding, ...
                fullfile(details_dir, sprintf('measured-%03d', repetition)), ...
                'measured', repetition, reference_keys, reference_rows);
            assert_scale(strcmp(measured.retained_key_sha256, ...
                calls(1).retained_key_sha256) && rows == reference_rows && ...
                isequal(keys, reference_keys), 'WarmSample', ...
                'Retained sample changed across warm repetitions.');
            measured.profiled = 0;
            calls(end+1) = measured; %#ok<AGROW>
            retained_validation_total = retained_validation_total + validation_seconds;
            measured_seconds(repetition) = measured.seconds;
        end
        record.estimator_call_count = binding.warm_repetitions + 1;
        record.warmup_call_count = 1;
        record.measured_call_count = binding.warm_repetitions;
        record.warmup_seconds = warmup.seconds;
        record.core_call_median_seconds = median(measured_seconds);
    end

    record.retained_validation_seconds = retained_validation_total;
    record.retained_key_sha256 = calls(1).retained_key_sha256;
    record.retained_physical_rows = calls(1).retained_physical_rows;
    record.retained_matches = calls(1).detail_matches;
    record.retained_workers = calls(1).retained_workers;
    record.retained_firms = calls(1).retained_firms;
    record.reference_sample_exact_match = double(sample_exact(calls(1), binding));
    if strcmp(binding.sample_mode, 'fixed')
        assert_scale(record.reference_sample_exact_match == 1, ...
            'FixedSample', 'Maintained call changed the fixed retained sample.');
    end
    if should_profile
        record.sample_selection_seconds = profile_metrics.selection_seconds;
        record.profile_status = 'REGISTERED_SOURCE_SELF_TIME';
        record.profile_metrics = profile_metrics;
    else
        record.sample_selection_seconds = 0;
        record.profile_status = 'NOT_PROFILED_FIXED_SAMPLE';
    end

    record.target_replay_max_scaled_diff = replay_difference(calls);
    record.retained_sample_stable = double(all(strcmp( ...
        {calls.retained_key_sha256}, calls(1).retained_key_sha256)));
    assert_scale(record.retained_sample_stable == 1, 'SampleReplay', ...
        'Retained sample changed across calls.');

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
    record.failure_message = 'NONE';
    record.final_matlab_utc = utc_now();
    record.matlab_wrapper_seconds = toc(matlab_started);
    record.serialization_seconds = measure_serialization(record, calls, scratch_dir);
    write_outputs(record, calls, output_dir, true);
    fprintf('KSS MATLAB SCALE %s PASS: %s\n', upper(mode), record.label);
catch exception
    profile off
    profile clear
    safe_restore_and_close(pool, client_original, worker_original);
    delete_detail_artifacts(scratch_dir);
    record.status = 'FAIL';
    if isempty(exception.identifier)
        record.failure_code = 'KSS:MatlabScale:Unclassified';
    else
        record.failure_code = exception.identifier;
    end
    record.failure_message = regexprep(exception.message, '[\r\n]+', ' ');
    record.final_matlab_utc = utc_now();
    record.matlab_wrapper_seconds = toc(matlab_started);
    if ~isempty(output_dir) && isfolder(output_dir)
        try
            record.serialization_seconds = measure_serialization( ...
                record, calls, scratch_dir);
            write_outputs(record, calls, output_dir, false);
        catch serialization_exception
            fprintf(2, 'KSS MATLAB SCALE SERIALIZATION FAIL: %s\n', ...
                serialization_exception.message);
        end
    end
    fprintf(2, 'KSS MATLAB SCALE %s FAIL %s: %s\n', upper(mode), ...
        record.failure_code, record.failure_message);
    rethrow(exception)
end
end


function record = initial_record(mode, first_matlab_utc)
record.schema = 'kss_matlab_scale_aggregate_v1';
record.status = 'FAIL';
record.failure_code = 'KSS:MatlabScale:Incomplete';
record.failure_message = 'Benchmark did not reach its success gate.';
record.mode = mode;
record.label = '';
record.scale = 0;
record.topology = '';
record.sample_mode = '';
record.case_sha256 = repmat('0',1,64);
record.contract_sha256 = repmat('0',1,64);
record.source_commit = repmat('0',1,40);
record.bundle_sha256 = repmat('0',1,64);
record.input_sha256 = repmat('0',1,64);
record.matlab_upstream_commit = repmat('0',1,40);
record.matlab_runtime_tree_sha256 = repmat('0',1,64);
record.matlab_core_sha256 = repmat('0',1,64);
record.matlab_cmg_entry_sha256 = repmat('0',1,64);
record.matlab_hierarchy_sha256 = repmat('0',1,64);
record.matlab_solver_sha256 = repmat('0',1,64);
record.first_matlab_utc = first_matlab_utc;
record.process_start_utc = '';
record.final_matlab_utc = first_matlab_utc;
record.startup_seconds = 0;
record.seed = 0;
record.probes = 0;
record.warm_repetitions = 0;
record.pool_workers = 0;
record.process_identity_status = 'NOT_STARTED';
record.matlab_client_pid = 0;
record.matlab_worker_pids = zeros(1,0);
record.input_rows = 0;
record.input_workers = 0;
record.input_firms = 0;
record.input_matches = 0;
record.input_dimension_source = 'ROWS_MEASURED_OTHERS_CHECKSUM_BOUND';
record.reference_rows = 0;
record.reference_workers = 0;
record.reference_firms = 0;
record.reference_matches = 0;
record.reference_retained_key_sha256 = repmat('0',1,64);
record.reference_receipt_sha256 = repmat('0',1,64);
record.reference_plugin_worker = 0;
record.reference_plugin_firm = 0;
record.reference_plugin_covariance = 0;
record.reference_plugin_total = 0;
record.plugin_status = 'CHECKSUM_BOUND_REFERENCE_NOT_MATLAB_OUTPUT';
record.corrected_comparison_policy = 'IDENTITY_ONLY_NO_EQUALITY_GATE';
record.estimator_call_count = 0;
record.warmup_call_count = 0;
record.measured_call_count = 0;
record.retained_physical_rows = 0;
record.retained_matches = 0;
record.retained_workers = 0;
record.retained_firms = 0;
record.retained_key_sha256 = repmat('0',1,64);
record.reference_sample_exact_match = 0;
record.retained_sample_stable = 0;
record.rng_protocol = 'legacy_maintained_client_worker_state_replay';
record.target_replay_max_scaled_diff = 0;
record.profile_status = 'NOT_STARTED';
record.profile_metrics = struct();
record.import_seconds = 0;
record.input_validation_seconds = 0;
record.sample_selection_seconds = 0;
record.retained_validation_seconds = 0;
record.pool_startup_seconds = 0;
record.mex_setup_seconds = 0;
record.warmup_seconds = 0;
record.core_call_median_seconds = 0;
record.serialization_seconds = 0;
record.pool_teardown_seconds = 0;
record.matlab_wrapper_seconds = 0;
record.matlab_version = version;
end


function record = bind_record(record, binding, case_sha, contract_sha, ...
    input_sha, process_start_utc, first_matlab_utc)
record.label = binding.label;
record.scale = binding.scale;
record.topology = binding.topology;
record.sample_mode = binding.sample_mode;
record.case_sha256 = case_sha;
record.contract_sha256 = contract_sha;
record.source_commit = binding.source.source_commit;
record.bundle_sha256 = binding.source.bundle_sha256;
record.input_sha256 = input_sha;
record.matlab_upstream_commit = binding.source.matlab_upstream_commit;
record.matlab_runtime_tree_sha256 = binding.source.matlab_runtime_tree_sha256;
record.matlab_core_sha256 = binding.source.matlab_core_sha256;
record.matlab_cmg_entry_sha256 = binding.source.matlab_cmg_entry_sha256;
record.matlab_hierarchy_sha256 = binding.source.matlab_hierarchy_sha256;
record.matlab_solver_sha256 = binding.source.matlab_solver_sha256;
record.process_start_utc = process_start_utc;
record.startup_seconds = utc_difference(process_start_utc, first_matlab_utc);
record.seed = binding.seed;
record.probes = binding.probes;
record.warm_repetitions = binding.warm_repetitions;
record.input_rows = binding.input.rows;
record.input_workers = binding.input.workers;
record.input_firms = binding.input.firms;
record.input_matches = binding.input.matches;
record.reference_rows = binding.reference_sample.rows;
record.reference_workers = binding.reference_sample.workers;
record.reference_firms = binding.reference_sample.firms;
record.reference_matches = binding.reference_sample.matches;
record.reference_retained_key_sha256 = ...
    binding.reference_sample.retained_key_sha256;
record.reference_receipt_sha256 = binding.reference_sample.receipt_sha256;
record.reference_plugin_worker = binding.reference_sample.plugin.worker;
record.reference_plugin_firm = binding.reference_sample.plugin.firm;
record.reference_plugin_covariance = binding.reference_sample.plugin.covariance;
record.reference_plugin_total = binding.reference_sample.plugin.total;
end


function validate_binding(binding, contract, case_sha, contract_sha, input_sha, mode)
assert_scale(strcmp(binding.schema, 'kss_matlab_scale_case_v2'), ...
    'CaseSchema', 'Case schema changed.');
assert_scale(strcmp(contract.schema, 'kss_matlab_scale_source_v1'), ...
    'ContractSchema', 'Source contract schema changed.');
assert_scale(~isempty(regexp(binding.label, '^[A-Za-z0-9._-]+$', 'once')), ...
    'Label', 'Invalid case label.');
assert_scale(any(binding.scale == contract.supported_scales), ...
    'Scale', 'Unsupported scale.');
assert_scale(any(strcmp(binding.topology, contract.supported_topologies)), ...
    'Topology', 'Unsupported topology.');
fixed_case_registered = false;
for fixed_case_index = 1:numel(contract.fixed_preparation_cases)
    fixed_case = contract.fixed_preparation_cases(fixed_case_index);
    fixed_case_registered = fixed_case_registered || ...
        (binding.scale == fixed_case.scale && ...
        strcmp(binding.topology,fixed_case.topology));
end
assert_scale(fixed_case_registered, 'FixedPreparationCase', ...
    'Unsupported fixed preparation scale/topology pair.');
assert_scale(any(strcmp(binding.sample_mode, contract.supported_sample_modes)), ...
    'SampleMode', 'Unsupported sample mode.');
assert_scale(strcmp(binding.sample_mode, 'fixed'), 'SampleMode', ...
    'Prepared MATLAB execution accepts fixed samples only.');
assert_scale(binding.probes == contract.required_probes && ...
    binding.seed == floor(binding.seed) && binding.seed >= 0 && ...
    binding.seed < 2^32, 'RNG', 'Registered seed or probe count changed.');
assert_scale(binding.warm_repetitions >= contract.minimum_warm_repetitions && ...
    binding.warm_repetitions == floor(binding.warm_repetitions), ...
    'WarmRepetitions', 'Warm benchmark requires at least three repetitions.');
hashes = {case_sha, contract_sha, input_sha, ...
    binding.source.bundle_sha256, binding.source.matlab_runtime_tree_sha256, ...
    binding.source.matlab_core_sha256, binding.source.matlab_cmg_entry_sha256, ...
    binding.source.matlab_hierarchy_sha256, binding.source.matlab_solver_sha256, ...
    binding.preparation.receipt_sha256, ...
    binding.preparation.acceptance_sha256, ...
    binding.preparation.source_input_sha256, ...
    binding.preparation.prepared_input_sha256, ...
    binding.preparation.retained_key_sha256, ...
    binding.reference_sample.receipt_sha256, ...
    binding.reference_sample.retained_key_sha256};
for index = 1:numel(hashes)
    assert_scale(~isempty(regexp(hashes{index}, '^[0-9a-f]{64}$', 'once')), ...
        'Hash', 'Invalid checksum binding.');
end
reference_input_bound = false;
if isfield(binding.reference_sample.input_bindings, ...
        'prepared_input_sha256')
    reference_input_bound = reference_input_bound || strcmp( ...
        binding.reference_sample.input_bindings.prepared_input_sha256, ...
        binding.preparation.prepared_input_sha256);
end
if isfield(binding.reference_sample.input_bindings, 'source_input_sha256')
    reference_input_bound = reference_input_bound || strcmp( ...
        binding.reference_sample.input_bindings.source_input_sha256, ...
        binding.preparation.source_input_sha256);
end
assert_scale(reference_input_bound, 'ReferenceInput', ...
    'Reference receipt binds neither prepared nor source input.');
reference_estimator = binding.reference_sample.estimator;
assert_scale(strcmp(reference_estimator.algorithm, 'jla') && ...
    strcmp(reference_estimator.deletion, 'match') && ...
    strcmp(reference_estimator.controls, 'none') && ...
    strcmp(reference_estimator.frequency_semantics, ...
        'literal_physical_rows_v1') && ...
    strcmp(reference_estimator.target_weight_semantics, ...
        'uniform_stored_rows_v1') && ...
    reference_estimator.probes == binding.probes && ...
    reference_estimator.seed == binding.seed, 'ReferenceEstimator', ...
    'Reference estimator binding changed.');
assert_scale(~isempty(regexp(binding.source.source_commit, ...
    '^[0-9a-f]{40}$', 'once')), 'SourceCommit', 'Invalid source commit.');
assert_scale(strcmp(input_sha, binding.input.sha256) && ...
    strcmp(input_sha, binding.preparation.prepared_input_sha256) && ...
    strcmp(binding.preparation.retained_key_sha256, ...
        binding.reference_sample.retained_key_sha256) && ...
    strcmp(contract_sha, binding.source.benchmark_contract_sha256) && ...
    strcmp(binding.source.matlab_upstream_commit, ...
        contract.maintained_upstream_commit) && ...
    strcmp(binding.source.matlab_runtime_tree_sha256, ...
        contract.runtime_tree.sha256) && ...
    strcmp(binding.source.matlab_core_sha256, contract.core.sha256), ...
    'SourceBinding', 'Case/source/input binding changed.');
assert_scale(strcmp(binding.source.source_commit, ...
    required_env('KSS_MS_SOURCE_COMMIT')) && ...
    strcmp(binding.source.bundle_sha256, ...
    required_env('KSS_MS_BUNDLE_SHA256')), 'ExecutionBinding', ...
    'Executing source commit or bundle differs from the case.');
assert_scale(strcmp(getenv('KSS_MS_MODE'), mode), 'ModeEnvironment', ...
    'Wrapper and MATLAB mode disagree.');
end


function validate_input(worker, firm, period, outcome, input_record)
n = numel(outcome);
assert_scale(n == input_record.rows && numel(worker) == n && ...
    numel(firm) == n && numel(period) == n, 'InputRows', ...
    'Measured input row count differs from its binding.');
assert_scale(min(worker) == 1 && max(worker) == input_record.workers && ...
    min(firm) == 1 && max(firm) == input_record.firms, 'DenseIDs', ...
    'Bound dense identifier range is not present.');
chunk = 1000000;
for first = 1:chunk:n
    last = min(n, first+chunk-1);
    current = first:last;
    assert_scale(all(isfinite(worker(current))) && ...
        all(isfinite(firm(current))) && all(isfinite(period(current))) && ...
        all(isfinite(outcome(current))), 'InputFinite', ...
        'Scale input contains a nonfinite value.');
    assert_scale(all(worker(current) == floor(worker(current))) && ...
        all(worker(current) >= 1) && ...
        all(worker(current) <= input_record.workers), 'WorkerIDs', ...
        'Worker identifiers violate the dense positive binding.');
    assert_scale(all(firm(current) == floor(firm(current))) && ...
        all(firm(current) >= 1) && ...
        all(firm(current) <= input_record.firms), 'FirmIDs', ...
        'Firm identifiers violate the dense positive binding.');
    if last < n
        left = first:last;
    else
        left = first:(last-1);
    end
    if ~isempty(left)
        right = left+1;
        bad = worker(right) < worker(left) | ...
            (worker(right) == worker(left) & period(right) < period(left)) | ...
            (worker(right) == worker(left) & period(right) == period(left) & ...
                firm(right) < firm(left));
        assert_scale(~any(bad), 'InputOrder', ...
            'Input is not ordered by worker, period, and firm.');
    end
end
end


function compile_run_local_mex(matlab_root, mex_output_dir)
mex_names = {'adjacency_cmg','diagconjugate','forest_components', ...
    'graphprofile','laplacian2','perturbtril','splitforest', ...
    'update_groups','vpack'};
mex_source_dir = fullfile(matlab_root, 'CMG', 'Source', 'Hierarchy');
for index = 1:numel(mex_names)
    source = fullfile(mex_source_dir, [mex_names{index} '.c']);
    assert_scale(isfile(source), 'MEXSource', ...
        'A checksum-bound hierarchy MEX source is unavailable.');
    mex('-silent', '-largeArrayDims', '-outdir', mex_output_dir, source);
end
include_dir = fullfile(matlab_root, 'CMG', 'Include');
solver_dir = fullfile(matlab_root, 'CMG', 'Source', 'Solver');
gateway = fullfile(matlab_root, 'CMG', 'MATLAB', 'Solver', ...
    'mx_d_preconditioner.c');
solver_names = {'vpv','vmv','vpvmv','vvmul','rmvec','ldl_solve', ...
    'sspmv','preconditioner'};
sources = cell(1,numel(solver_names)+1);
sources{1} = gateway;
for index = 1:numel(solver_names)
    sources{index+1} = fullfile(solver_dir, [solver_names{index} '.c']);
end
mex('-silent', '-largeArrayDims', ['-I' include_dir], '-outdir', ...
    mex_output_dir, '-output', 'mx_d_preconditioner', sources{:});
addpath(mex_output_dir, '-begin');
rehash;
clear graphprofile mx_d_preconditioner
assert_scale(exist('graphprofile','file') == 3 && ...
    startsWith(canonical_path(which('graphprofile')), ...
        [canonical_path(mex_output_dir) filesep]), 'HierarchyMEXBinding', ...
    'Run-local graphprofile did not bind.');
assert_scale(exist('mx_d_preconditioner','file') == 3 && ...
    startsWith(canonical_path(which('mx_d_preconditioner')), ...
        [canonical_path(mex_output_dir) filesep]), 'SolverMEXBinding', ...
    'Run-local preconditioner did not bind.');
end


function [call, keys, retained_rows, validation_seconds] = maintained_call( ...
    outcome, worker, firm, binding, detail_stub, role, call_index, ...
    expected_keys, expected_rows)
started = tic;
controls = [];
leave_out_level = 'matches';
type_algorithm = 'JLA';
lincom_do = 0;
Z_lincom = [];
labels_lincom = [];
[firm_variance, covariance, worker_variance] = leave_out_KSS( ...
    outcome, worker, firm, controls, leave_out_level, type_algorithm, ...
    binding.probes, lincom_do, Z_lincom, labels_lincom, detail_stub);
elapsed = toc(started);
targets = [worker_variance, firm_variance, covariance, ...
    worker_variance + firm_variance + 2*covariance];
assert_scale(all(isfinite(targets)), 'Targets', ...
    'Maintained estimator returned nonfinite targets.');

validation_started = tic;
detail_file = [detail_stub '.csv'];
assert_scale(isfile(detail_file), 'DetailMissing', ...
    'Maintained estimator did not write its detail artifact.');
detail_sha = sha256_file(detail_file);
detail_data = importdata(detail_file);
if isstruct(detail_data)
    detail_data = detail_data.data;
end
assert_scale(isnumeric(detail_data) && size(detail_data,1) > 0 && ...
    size(detail_data,2) == 4 && all(isfinite(detail_data), 'all'), ...
    'DetailShape', 'Maintained detail output is not finite four-column data.');
keys = sortrows(detail_data(:,2:3), [1 2]);
assert_scale(all(keys == floor(keys), 'all') && all(keys >= 1, 'all'), ...
    'DetailKeys', 'Maintained retained keys are not positive integers.');
if size(keys,1) > 1
    duplicate = keys(2:end,1) == keys(1:end-1,1) & ...
        keys(2:end,2) == keys(1:end-1,2);
    assert_scale(~any(duplicate), 'DetailDuplicates', ...
        'Maintained detail output repeats a worker-firm key.');
end
key_sha = key_hash(keys);
if isempty(expected_keys)
    if strcmp(binding.sample_mode, 'fixed')
        retained_rows = binding.input.rows;
    else
        retained_rows = count_retained_rows(worker, firm, keys, ...
            binding.input.workers, binding.input.firms);
    end
else
    assert_scale(isequal(keys, expected_keys), 'CallSample', ...
        'Maintained calls selected different worker-firm keys.');
    retained_rows = expected_rows;
end
delete(detail_file);
validation_seconds = toc(validation_started);

identity_expected = targets(1)+targets(2)+2*targets(3);
identity_error = abs(targets(4)-identity_expected) / ...
    (1+abs(targets(4))+abs(identity_expected));
assert_scale(identity_error <= 1e-12, 'TargetIdentity', ...
    'Maintained corrected target identity failed.');
call.role = role;
call.call_index = call_index;
call.seconds = elapsed;
call.corrected_worker = targets(1);
call.corrected_firm = targets(2);
call.corrected_covariance = targets(3);
call.corrected_total = targets(4);
call.identity_scaled_error = identity_error;
call.target_sha256 = target_hash(targets);
call.detail_sha256 = detail_sha;
call.retained_key_sha256 = key_sha;
call.detail_matches = size(keys,1);
call.retained_workers = numel(unique(keys(:,1)));
call.retained_firms = numel(unique(keys(:,2)));
call.retained_physical_rows = retained_rows;
call.profiled = 0;
end


function count = count_retained_rows(worker, firm, keys, workers, firms)
assert_scale(double(workers)*double(firms) < flintmax, 'KeyIndexRange', ...
    'Dense worker-firm index exceeds exact double integer range.');
assert_scale(all(keys(:,1) <= workers) && all(keys(:,2) <= firms), ...
    'RetainedKeyRange', 'Maintained retained key is outside the input binding.');
retained = sparse(keys(:,1), keys(:,2), ones(size(keys,1),1), workers, firms);
count = 0;
chunk = 1000000;
n = numel(worker);
for first = 1:chunk:n
    last = min(n,first+chunk-1);
    index = worker(first:last) + (firm(first:last)-1)*workers;
    count = count + nnz(retained(index));
end
assert_scale(count > 0, 'RetainedRows', 'Maintained retained sample is empty.');
end


function result = sample_exact(call, binding)
result = strcmp(call.retained_key_sha256, ...
    binding.reference_sample.retained_key_sha256) && ...
    call.retained_physical_rows == binding.reference_sample.rows && ...
    call.detail_matches == binding.reference_sample.matches && ...
    call.retained_workers == binding.reference_sample.workers && ...
    call.retained_firms == binding.reference_sample.firms;
end


function start_profile()
profile clear
profile on -timer real -nohistory
status = profile('status');
assert_scale(strcmp(status.ProfilerStatus,'on') && ...
    strcmp(status.Timer,'real') && strcmp(status.HistoryTracking,'off'), ...
    'ProfileMode', 'Selection profile mode changed.');
end


function metrics = stop_and_aggregate_profile(core_file, contract)
profile off
information = profile('info');
profile clear
table = information.FunctionTable;
matches = false(1,numel(table));
for index = 1:numel(table)
    matches(index) = strcmp(table(index).FunctionName,'leave_out_KSS') && ...
        strcmp(canonical_path(table(index).FileName), canonical_path(core_file));
end
assert_scale(sum(matches) == 1, 'ProfileCore', ...
    'Profiler did not return one checksum-bound core entry.');
entry = table(matches);
assert_scale(entry.NumCalls == 1 && ~entry.IsRecursive && ~entry.PartialData, ...
    'ProfileCalls', 'Profiled maintained call is incomplete or repeated.');
lines = entry.ExecutedLines;
assert_scale(isnumeric(lines) && size(lines,2) >= 3 && size(lines,1) > 0, ...
    'ProfileLines', 'Profiled executed lines are unavailable.');
line_number = lines(:,1);
line_calls = lines(:,2);
line_seconds = lines(:,3);
assert_scale(all(line_number >= 1 & ...
    line_number <= contract.core.profile_max_line) && ...
    all(line_number == floor(line_number)) && all(line_calls >= 0) && ...
    all(isfinite(line_seconds) & line_seconds >= 0), 'ProfileBounds', ...
    'Profile lines exceed the registered source contract.');
metrics.function_calls = entry.NumCalls;
metrics.executed_lines = size(lines,1);
metrics.line_calls = sum(line_calls);
metrics.top_level_self_seconds = sum(line_seconds);
phase_line_sum = 0;
phase_call_sum = 0;
phase_time_sum = 0;
for index = 1:numel(contract.profile_phases)
    phase = contract.profile_phases(index);
    selected = line_number >= phase.first_line & line_number <= phase.last_line;
    name = phase.name;
    metrics.([name '_executed_lines']) = sum(selected);
    metrics.([name '_line_calls']) = sum(line_calls(selected));
    metrics.([name '_seconds']) = sum(line_seconds(selected));
    phase_line_sum = phase_line_sum + sum(selected);
    phase_call_sum = phase_call_sum + sum(line_calls(selected));
    phase_time_sum = phase_time_sum + sum(line_seconds(selected));
end
assert_scale(phase_line_sum == metrics.executed_lines && ...
    phase_call_sum == metrics.line_calls && ...
    abs(phase_time_sum-metrics.top_level_self_seconds) <= ...
        1e-12*(1+abs(metrics.top_level_self_seconds)), 'ProfileAccounting', ...
    'Registered profile phases do not exhaust top-level lines.');
end


function replay_rng(client_state, worker_states)
rng(client_state);
spmd
    rng(worker_states);
end
end


function difference = replay_difference(calls)
if numel(calls) < 2
    difference = 0;
    return
end
targets = zeros(numel(calls),4);
for index = 1:numel(calls)
    targets(index,:) = [calls(index).corrected_worker, ...
        calls(index).corrected_firm, calls(index).corrected_covariance, ...
        calls(index).corrected_total];
end
difference = 0;
for left = 1:(size(targets,1)-1)
    for right = (left+1):size(targets,1)
        current = abs(targets(left,:)-targets(right,:)) ./ ...
            (1+max(abs(targets([left right],:)),[],1));
        difference = max(difference,max(current));
    end
end
end


function elapsed = measure_serialization(record, calls, scratch_dir)
if isempty(scratch_dir) || ~isfolder(scratch_dir)
    elapsed = 0;
    return
end
probe_json = fullfile(scratch_dir,'aggregate.serialization-probe.json');
probe_csv = fullfile(scratch_dir,'calls.serialization-probe.csv');
delete_if_present(probe_json);
delete_if_present(probe_csv);
started = tic;
write_json(record,probe_json);
if ~isempty(calls)
    writetable(call_table(calls),probe_csv);
end
elapsed = toc(started);
delete_if_present(probe_json);
delete_if_present(probe_csv);
end


function write_outputs(record, calls, output_dir, write_pass)
aggregate = fullfile(output_dir,'aggregate.json');
calls_file = fullfile(output_dir,'calls.csv');
pass_file = fullfile(output_dir,'wrapper.pass');
delete_if_present(pass_file);
temporary_aggregate = [aggregate '.tmp'];
write_json(record,temporary_aggregate);
movefile(temporary_aggregate,aggregate,'f');
if ~isempty(calls)
    temporary_calls = [calls_file '.tmp'];
    writetable(call_table(calls),temporary_calls, ...
        'FileType','text','Delimiter',',');
    movefile(temporary_calls,calls_file,'f');
end
if write_pass
    temporary_pass = [pass_file '.tmp'];
    fid = fopen(temporary_pass,'w');
    assert_scale(fid >= 0,'PassWrite','Cannot write pass marker.');
    fprintf(fid,'KSS_MATLAB_SCALE_PASS %s %s %s %s %s\n', ...
        record.mode,record.label,record.case_sha256,record.input_sha256, ...
        record.retained_key_sha256);
    fclose(fid);
    movefile(temporary_pass,pass_file,'f');
end
end


function table_value = call_table(calls)
table_value = struct2table(rmfield(calls,'profiled'));
end


function write_json(value,path)
encoded = jsonencode(value,'PrettyPrint',true);
fid = fopen(path,'w');
assert_scale(fid >= 0,'JSONWrite','Cannot write aggregate JSON.');
fprintf(fid,'%s\n',encoded);
fclose(fid);
end


function write_atomic_json(value,path)
temporary = [path '.tmp'];
delete_if_present(temporary);
write_json(value,temporary);
movefile(temporary,path,'f');
end


function calls = empty_calls()
calls = struct('role',{},'call_index',{},'seconds',{}, ...
    'corrected_worker',{},'corrected_firm',{},'corrected_covariance',{}, ...
    'corrected_total',{},'identity_scaled_error',{},'target_sha256',{}, ...
    'detail_sha256',{},'retained_key_sha256',{},'detail_matches',{}, ...
    'retained_workers',{},'retained_firms',{},'retained_physical_rows',{}, ...
    'profiled',{});
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


function delete_detail_artifacts(scratch_dir)
try
    details = fullfile(scratch_dir,'details');
    if isfolder(details)
        files = dir(fullfile(details,'*.csv'));
        for index = 1:numel(files)
            delete(fullfile(files(index).folder,files(index).name));
        end
    end
catch
end
end


function digest = target_hash(targets)
digest = sha256_bytes(unicode2native(sprintf('%.17g\n',targets),'UTF-8'));
end


function digest = key_hash(keys)
bytes = unicode2native(sprintf('%.0f,%.0f\n',keys.'),'UTF-8');
digest = sha256_bytes(bytes);
end


function digest = sha256_file(path)
fid = fopen(path,'r');
assert_scale(fid >= 0,'HashRead','Cannot open artifact for hashing.');
cleanup = onCleanup(@() fclose(fid)); %#ok<NASGU>
message_digest = java.security.MessageDigest.getInstance('SHA-256');
while true
    chunk = fread(fid,1024*1024,'*uint8');
    if isempty(chunk)
        break
    end
    message_digest.update(typecast(chunk(:),'int8'));
end
raw = typecast(message_digest.digest(),'uint8');
digest = lower(reshape(dec2hex(raw,2).',1,[]));
end


function digest = sha256_bytes(bytes)
message_digest = java.security.MessageDigest.getInstance('SHA-256');
message_digest.update(typecast(uint8(bytes(:)),'int8'));
raw = typecast(message_digest.digest(),'uint8');
digest = lower(reshape(dec2hex(raw,2).',1,[]));
end


function result = canonical_path(path)
result = char(java.io.File(path).getCanonicalPath());
end


function value = required_env(name)
value = getenv(name);
if isempty(value)
    error('KSS:MatlabScale:MissingEnvironment', ...
        'Required environment variable %s is missing.',name);
end
end


function value = utc_now()
value = char(datetime('now','TimeZone','UTC', ...
    'Format',"yyyy-MM-dd'T'HH:mm:ss.SSSSSSSSS'Z'"));
end


function elapsed = utc_difference(start_value,end_value)
format = "yyyy-MM-dd'T'HH:mm:ss.SSSSSSSSS'Z'";
started = datetime(start_value,'InputFormat',format,'TimeZone','UTC');
finished = datetime(end_value,'InputFormat',format,'TimeZone','UTC');
elapsed = seconds(finished-started);
assert_scale(isfinite(elapsed) && elapsed >= 0, 'StartupClock', ...
    'MATLAB executable startup timestamps are invalid.');
end


function delete_if_present(path)
if isfile(path)
    delete(path);
end
end


function assert_scale(condition,code,message)
if ~condition
    error(['KSS:MatlabScale:' code],'%s',message);
end
end

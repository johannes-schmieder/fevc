function cmg_cz18_matlab_run()
% Source-bound official MATLAB/C CMG comparator on the fixed CZ18 sample.

output_dir = required_env('CMG_CZ_M_OUTPUT_DIR');
scratch_dir = required_env('CMG_CZ_M_SCRATCH_DIR');
input_file = required_env('CMG_CZ_M_INPUT_CSV');
input_sha = required_env('CMG_CZ_M_INPUT_SHA256');
prepared_input_sha = required_env('CMG_CZ_M_PREPARED_INPUT_SHA256');
matlab_root = required_env('CMG_CZ_M_MATLAB_ROOT');
task_sha = required_env('CMG_CZ_M_TASK_SHA256');
source_identity_path = required_env('CMG_CZ_M_SOURCE_IDENTITY');
process_identity_file = required_env('CMG_CZ_M_PROCESS_IDENTITY');
label = required_env('CMG_CZ_M_EXPERIMENT');
source_commit = required_env('CMG_CZ_M_SOURCE_COMMIT');
bundle_sha = required_env('CMG_CZ_M_BUNDLE_SHA256');
probes = required_integer('CMG_CZ_M_PROBES');
seed = required_integer('CMG_CZ_M_SEED');
expected_rows = required_integer('CMG_CZ_M_ROWS');
expected_workers = required_integer('CMG_CZ_M_WORKERS');
expected_firms = required_integer('CMG_CZ_M_FIRMS');
expected_cells = required_integer('CMG_CZ_M_CELLS');
mex_dir = fullfile(scratch_dir,'mex');
detail_dir = fullfile(scratch_dir,'details');

assert_cz(probes == 20 && seed == 8675309,'Randomness', ...
    'CZ18 comparison requires P20 and seed 8675309.');
assert_cz(expected_rows == 8201888 && expected_workers == 117529 && ...
    expected_firms == 10603 && expected_cells == 311730,'Dimensions', ...
    'Fixed CZ18 dimensions changed.');
assert_cz(~isempty(regexp(label,'^[A-Za-z0-9._-]+$','once')) && ...
    ~isempty(regexp(task_sha,'^[0-9a-f]{64}$','once')) && ...
    ~isempty(regexp(input_sha,'^[0-9a-f]{64}$','once')) && ...
    ~isempty(regexp(prepared_input_sha,'^[0-9a-f]{64}$','once')), ...
    'Identity', ...
    'Invalid task or input identity.');
if ~isfolder(output_dir), mkdir(output_dir); end
if ~isfolder(mex_dir), mkdir(mex_dir); end
if ~isfolder(detail_dir), mkdir(detail_dir); end

pool = [];
try
    source_identity = jsondecode(fileread(source_identity_path));
    assert_cz(strcmp(source_identity.status,'PASS'),'SourceIdentity', ...
        'Maintained source identity did not pass.');
    addpath(fullfile(matlab_root,'codes'));
    addpath(genpath(fullfile(matlab_root,'CMG')));
    resolved = which('leave_out_KSS');
    assert_cz(strcmp(canonical_path(resolved),canonical_path( ...
        fullfile(matlab_root,'codes','leave_out_KSS.m'))), ...
        'CoreResolution','Maintained MATLAB entry point changed.');

    import_started = tic;
    imported = importdata(input_file);
    if isstruct(imported), data = imported.data; else, data = imported; end
    assert_cz(isnumeric(data) && size(data,2) == 4 && ...
        size(data,1) == expected_rows && all(isfinite(data),'all'), ...
        'Input','Prepared fixed-CZ18 input changed.');
    worker = data(:,1);
    firm = data(:,2);
    period = data(:,3);
    outcome = data(:,4);
    clear data imported
    assert_cz(min(worker) == 1 && max(worker) == expected_workers && ...
        min(firm) == 1 && max(firm) == expected_firms && ...
        all(worker == floor(worker)) && all(firm == floor(firm)), ...
        'Input','Prepared fixed-CZ18 identifiers changed.');
    input_validation_seconds = toc(import_started);
    clear period

    existing_pool = gcp('nocreate');
    assert_cz(isempty(existing_pool),'PreexistingPool', ...
        'Comparison requires a fresh MATLAB process.');
    parallel_dir = fullfile(scratch_dir,'parallel');
    if ~isfolder(parallel_dir), mkdir(parallel_dir); end
    cluster = parcluster('local');
    cluster.JobStorageLocation = parallel_dir;
    pool_started = tic;
    pool = parpool(cluster,4,'IdleTimeout',Inf);
    pool_seconds = toc(pool_started);
    assert_cz(pool.NumWorkers == 4,'PoolSize', ...
        'Comparison requires exactly four MATLAB workers.');

    client_pid = double(matlabProcessID);
    spmd
        worker_pid = double(matlabProcessID);
        worker_index = spmdIndex;
    end
    worker_pids = zeros(1,4);
    worker_indices = zeros(1,4);
    for index = 1:4
        worker_pids(index) = worker_pid{index};
        worker_indices(index) = worker_index{index};
    end
    assert_cz(isequal(worker_indices,1:4) && ...
        numel(unique([client_pid worker_pids])) == 5, ...
        'ProcessIdentity','MATLAB process identities changed.');
    identity = struct('schema','kss_matlab_scale_process_identity_v1', ...
        'status','PASS','pid_api','matlabProcessID_R2025a', ...
        'mode','cold','label',label,'case_sha256',task_sha, ...
        'expected_pool_workers',4,'client_pid',client_pid, ...
        'worker_indices',worker_indices,'worker_pids',worker_pids);
    write_atomic_json(identity,process_identity_file);

    mex_started = tic;
    compile_run_local_mex(matlab_root,mex_dir);
    mex_seconds = toc(mex_started);
    rng(seed,'twister');
    spmd
        rng(seed+spmdIndex,'twister');
    end

    controls = [];
    leave_out_level = 'matches';
    type_algorithm = 'JLA';
    lincom_do = 0;
    Z_lincom = [];
    labels_lincom = [];
    detail_stub = fullfile(detail_dir,[label '-detail']);
    command_started = tic;
    [firm_variance,covariance,worker_variance] = leave_out_KSS( ...
        outcome,worker,firm,controls,leave_out_level,type_algorithm, ...
        probes,lincom_do,Z_lincom,labels_lincom,detail_stub);
    command_seconds = toc(command_started);
    targets = [worker_variance firm_variance covariance ...
        worker_variance+firm_variance+2*covariance];
    assert_cz(all(isfinite(targets)),'Targets', ...
        'Maintained estimator returned nonfinite targets.');
    identity_error = abs(targets(4)-targets(1)-targets(2)-2*targets(3)) / ...
        (1+sum(abs(targets)));
    assert_cz(identity_error <= 1e-12,'TargetIdentity', ...
        'Maintained target identity failed.');
    detail_file = [detail_stub '.csv'];
    assert_cz(isfile(detail_file),'DetailMissing', ...
        'Maintained estimator detail output is missing.');
    detail_lines = file_line_count(detail_file);
    detail_bytes = dir(detail_file).bytes;
    assert_cz(detail_lines == expected_cells,'RetainedMatches', ...
        'Maintained estimator retained a different CZ18 match count.');
    delete(detail_file);

    teardown_started = tic;
    delete(pool);
    pool = [];
    teardown_seconds = toc(teardown_started);
    record = struct( ...
        'schema','CMG-MATA-1-CZ18-MATLAB-AGGREGATE-V1', ...
        'status','PASS','experiment_id',label, ...
        'comparison_source_commit',source_commit, ...
        'comparison_bundle_sha256',bundle_sha,'task_sha256',task_sha, ...
        'input_sha256',input_sha, ...
        'prepared_input_sha256',prepared_input_sha, ...
        'matlab_upstream_commit',source_identity.matlab_upstream_commit, ...
        'matlab_runtime_tree_sha256', ...
            source_identity.matlab_runtime_tree_sha256, ...
        'matlab_core_sha256',source_identity.matlab_core_sha256, ...
        'matlab_version',version,'stored_rows',expected_rows, ...
        'workers',expected_workers,'firms',expected_firms, ...
        'coefficient_cells',expected_cells,'probes',probes,'seed',seed, ...
        'pool_workers',4,'input_validation_seconds', ...
            input_validation_seconds,'pool_startup_seconds',pool_seconds, ...
        'mex_setup_seconds',mex_seconds,'command_seconds',command_seconds, ...
        'pool_teardown_seconds',teardown_seconds, ...
        'detail_matches',detail_lines,'detail_bytes',detail_bytes, ...
        'corrected_worker',targets(1),'corrected_firm',targets(2), ...
        'corrected_covariance',targets(3),'corrected_total',targets(4), ...
        'target_identity_scaled_error',identity_error, ...
        'same_retained_input',true,'same_probe_count',true, ...
        'target_weight_semantics_comparable',false, ...
        'rng_draws_comparable',false,'solver_tolerance_comparable',false, ...
        'corrected_estimate_equality_gate','NONE_DESCRIPTIVE_ONLY');
    write_atomic_json(record,fullfile(output_dir,'aggregate.json'));
    calls = table(string(label),command_seconds,targets(1),targets(2), ...
        targets(3),targets(4),identity_error, ...
        'VariableNames',{'experiment_id','command_seconds', ...
        'corrected_worker','corrected_firm','corrected_covariance', ...
        'corrected_total','target_identity_scaled_error'});
    writetable(calls,fullfile(output_dir,'calls.csv'));
    marker = fopen(fullfile(output_dir,'matlab.pass'),'w');
    assert_cz(marker >= 0,'Marker','Could not write pass marker.');
    fprintf(marker,'CMG_MATA1_CZ18_MATLAB_PASS %s %s\n',label,task_sha);
    fclose(marker);
    fprintf('CMG-MATA-1 CZ18 MATLAB PASS: %s\n',label);
catch exception
    if ~isempty(pool)
        try, delete(pool); catch, end
    end
    failure = struct('schema','CMG-MATA-1-CZ18-MATLAB-FAILURE-V1', ...
        'status','FAIL','experiment_id',label, ...
        'identifier',exception.identifier,'message',exception.message);
    write_atomic_json(failure,fullfile(output_dir,'failure.json'));
    rethrow(exception);
end
end


function compile_run_local_mex(matlab_root,mex_output_dir)
names = {'adjacency_cmg','diagconjugate','forest_components', ...
    'graphprofile','laplacian2','perturbtril','splitforest', ...
    'update_groups','vpack'};
source_dir = fullfile(matlab_root,'CMG','Source','Hierarchy');
for index = 1:numel(names)
    mex('-silent','-largeArrayDims','-outdir',mex_output_dir, ...
        fullfile(source_dir,[names{index} '.c']));
end
include_dir = fullfile(matlab_root,'CMG','Include');
solver_dir = fullfile(matlab_root,'CMG','Source','Solver');
gateway = fullfile(matlab_root,'CMG','MATLAB','Solver', ...
    'mx_d_preconditioner.c');
solver_names = {'vpv','vmv','vpvmv','vvmul','rmvec','ldl_solve', ...
    'sspmv','preconditioner'};
sources = cell(1,numel(solver_names)+1);
sources{1} = gateway;
for index = 1:numel(solver_names)
    sources{index+1} = fullfile(solver_dir,[solver_names{index} '.c']);
end
mex('-silent','-largeArrayDims',['-I' include_dir],'-outdir', ...
    mex_output_dir,'-output','mx_d_preconditioner',sources{:});
addpath(mex_output_dir,'-begin');
rehash;
clear graphprofile mx_d_preconditioner
end


function count = file_line_count(path)
[status,text] = system(['wc -l < ' path]);
assert_cz(status == 0,'DetailCount','Could not count detail rows.');
count = str2double(strtrim(text));
assert_cz(isfinite(count) && count >= 1 && count == floor(count), ...
    'DetailCount','Invalid detail row count.');
end


function value = required_env(name)
value = getenv(name);
assert_cz(~isempty(value),'Environment',['Missing ' name '.']);
end


function value = required_integer(name)
text = required_env(name);
value = str2double(text);
assert_cz(isfinite(value) && value >= 1 && value == floor(value), ...
    'Environment',['Invalid ' name '.']);
end


function path = canonical_path(path)
path = char(java.io.File(path).getCanonicalPath());
end


function write_atomic_json(value,path)
temporary = [path '.tmp'];
handle = fopen(temporary,'w');
assert_cz(handle >= 0,'JSON','Could not open JSON output.');
fprintf(handle,'%s\n',jsonencode(value));
fclose(handle);
movefile(temporary,path,'f');
end


function assert_cz(condition,code,message)
if ~condition
    error(['kss:cmgMata1Cz18Matlab:' code],'%s',message);
end
end

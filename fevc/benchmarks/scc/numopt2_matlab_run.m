function numopt2_matlab_run()
% Source-bound descriptive MATLAB comparison for one KSS-NUMOPT-2 task.

output_dir = required_env('KSS_NM_OUTPUT_DIR');
scratch_dir = required_env('KSS_NM_SCRATCH_DIR');
matlab_root = required_env('KSS_NM_MATLAB_ROOT');
task_sha = required_env('KSS_NM_TASK_SHA256');
source_identity_path = required_env('KSS_NM_SOURCE_IDENTITY');
label = required_env('KSS_NM_EXPERIMENT_ID');
workers = required_integer('KSS_NM_WORKERS');
firms = required_integer('KSS_NM_FIRMS');
density = required_integer('KSS_NM_CELLS_PER_WORKER');
rows_per_cell = required_integer('KSS_NM_ROWS_PER_CELL');
connectivity = required_env('KSS_NM_CONNECTIVITY');
probes = required_integer('KSS_NM_PROBES');
seed = required_integer('KSS_NM_SEED');
source_commit = required_env('KSS_NM_SOURCE_COMMIT');
bundle_sha = required_env('KSS_NM_BUNDLE_SHA256');
kss_source_commit = required_env('KSS_NM_KSS_SOURCE_COMMIT');
kss_bundle_sha = required_env('KSS_NM_KSS_BUNDLE_SHA256');
process_identity_file = required_env('KSS_NM_PROCESS_IDENTITY');
mex_dir = fullfile(scratch_dir,'mex');
detail_dir = fullfile(scratch_dir,'details');

assert_nm(workers == 40*firms && firms >= 8, ...
    'Dimensions','Worker/firm aspect ratio changed.');
assert_nm(ismember(density,2:7) && ...
    ismember(rows_per_cell,[1 8]),'Dimensions','Task dimensions changed.');
assert_nm(strcmp(connectivity,'strong') || strcmp(connectivity,'weak'), ...
    'Connectivity','Unknown task connectivity.');
assert_nm(probes >= 2 && seed >= 1 && seed <= 2147483646, ...
    'Randomness','Invalid probe or seed contract.');
assert_nm(~isempty(regexp(label,'^[A-Za-z0-9._-]+$','once')) && ...
    ~isempty(regexp(task_sha,'^[0-9a-f]{64}$','once')), ...
    'Identity','Invalid task identity.');
if ~isfolder(output_dir), mkdir(output_dir); end
if ~isfolder(mex_dir), mkdir(mex_dir); end
if ~isfolder(detail_dir), mkdir(detail_dir); end

pool = [];
try
    source_identity = jsondecode(fileread(source_identity_path));
    assert_nm(strcmp(source_identity.status,'PASS'), ...
        'SourceIdentity','Maintained source identity did not pass.');
    addpath(fullfile(matlab_root,'codes'));
    addpath(genpath(fullfile(matlab_root,'CMG')));
    resolved = which('leave_out_KSS');
    assert_nm(strcmp(canonical_path(resolved),canonical_path( ...
        fullfile(matlab_root,'codes','leave_out_KSS.m'))), ...
        'CoreResolution','Maintained MATLAB entry point changed.');

    generation_started = tic;
    [outcome,worker,firm] = fixture( ...
        workers,firms,density,rows_per_cell,connectivity);
    generation_seconds = toc(generation_started);
    rows = numel(outcome);
    cells = workers*density;
    assert_nm(rows == cells*rows_per_cell && ...
        min(worker) == 1 && max(worker) == workers && ...
        min(firm) == 1 && max(firm) == firms && ...
        all(isfinite(outcome)),'Fixture','Generated fixture failed.');

    existing_pool = gcp('nocreate');
    assert_nm(isempty(existing_pool),'PreexistingPool', ...
        'Comparison requires a fresh MATLAB process.');
    parallel_dir = fullfile(scratch_dir,'parallel');
    if ~isfolder(parallel_dir), mkdir(parallel_dir); end
    cluster = parcluster('local');
    cluster.JobStorageLocation = parallel_dir;
    pool_started = tic;
    pool = parpool(cluster,4,'IdleTimeout',Inf);
    pool_seconds = toc(pool_started);
    assert_nm(pool.NumWorkers == 4,'PoolSize', ...
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
    assert_nm(isequal(worker_indices,1:4) && ...
        numel(unique([client_pid worker_pids])) == 5, ...
        'ProcessIdentity','MATLAB process identities changed.');
    identity = struct( ...
        'schema','kss_matlab_scale_process_identity_v1', ...
        'status','PASS', ...
        'pid_api','matlabProcessID_R2025a', ...
        'mode','cold', ...
        'label',label, ...
        'case_sha256',task_sha, ...
        'expected_pool_workers',4, ...
        'client_pid',client_pid, ...
        'worker_indices',worker_indices, ...
        'worker_pids',worker_pids);
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
    assert_nm(all(isfinite(targets)),'Targets', ...
        'Maintained estimator returned nonfinite targets.');
    identity_error = abs(targets(4)-targets(1)-targets(2)-2*targets(3)) / ...
        (1+sum(abs(targets)));
    assert_nm(identity_error <= 1e-12,'TargetIdentity', ...
        'Maintained target identity failed.');
    detail_file = [detail_stub '.csv'];
    assert_nm(isfile(detail_file),'DetailMissing', ...
        'Maintained estimator detail output is missing.');
    detail_lines = file_line_count(detail_file);
    detail_bytes = dir(detail_file).bytes;
    assert_nm(detail_lines == cells,'RetainedMatches', ...
        'Maintained estimator retained a different match count.');
    delete(detail_file);

    teardown_started = tic;
    delete(pool);
    pool = [];
    teardown_seconds = toc(teardown_started);
    frequency_per_stored_row = 1;
    if rows_per_cell == 1, frequency_per_stored_row = 8; end
    record = struct( ...
        'schema','KSS-NUMOPT-2-MATLAB-AGGREGATE-V1', ...
        'status','PASS', ...
        'experiment_id',label, ...
        'comparison_source_commit',source_commit, ...
        'comparison_bundle_sha256',bundle_sha, ...
        'kss_source_commit',kss_source_commit, ...
        'kss_bundle_sha256',kss_bundle_sha, ...
        'task_sha256',task_sha, ...
        'matlab_upstream_commit',source_identity.matlab_upstream_commit, ...
        'matlab_runtime_tree_sha256', ...
            source_identity.matlab_runtime_tree_sha256, ...
        'matlab_core_sha256',source_identity.matlab_core_sha256, ...
        'matlab_version',version, ...
        'workers',workers, ...
        'firms',firms, ...
        'cells_per_worker',density, ...
        'rows_per_cell',rows_per_cell, ...
        'connectivity',connectivity, ...
        'stored_rows',rows, ...
        'coefficient_cells',cells, ...
        'frequency_per_stored_row_in_kss',frequency_per_stored_row, ...
        'probes',probes, ...
        'seed',seed, ...
        'pool_workers',4, ...
        'generation_seconds',generation_seconds, ...
        'pool_startup_seconds',pool_seconds, ...
        'mex_setup_seconds',mex_seconds, ...
        'command_seconds',command_seconds, ...
        'pool_teardown_seconds',teardown_seconds, ...
        'detail_matches',detail_lines, ...
        'detail_bytes',detail_bytes, ...
        'corrected_worker',targets(1), ...
        'corrected_firm',targets(2), ...
        'corrected_covariance',targets(3), ...
        'corrected_total',targets(4), ...
        'target_identity_scaled_error',identity_error, ...
        'same_stored_row_fixture',true, ...
        'frequency_semantics_comparable',frequency_per_stored_row == 1, ...
        'target_weight_semantics_comparable',false, ...
        'rng_draws_comparable',false, ...
        'solver_tolerance_comparable',false, ...
        'corrected_estimate_equality_gate','NONE_DESCRIPTIVE_ONLY');
    write_atomic_json(record,fullfile(output_dir,'aggregate.json'));
    calls = table(string(label),command_seconds,targets(1),targets(2), ...
        targets(3),targets(4),identity_error, ...
        'VariableNames',{'experiment_id','command_seconds', ...
        'corrected_worker','corrected_firm','corrected_covariance', ...
        'corrected_total','target_identity_scaled_error'});
    writetable(calls,fullfile(output_dir,'calls.csv'));
    marker = fopen(fullfile(output_dir,'matlab.pass'),'w');
    assert_nm(marker >= 0,'Marker','Could not write pass marker.');
    fprintf(marker,'KSS_NUMOPT2_MATLAB_PASS %s %s\n',label,task_sha);
    fclose(marker);
    fprintf('KSS_NUMOPT2 MATLAB PASS: %s\n',label);
catch exception
    if ~isempty(pool)
        try
            delete(pool);
        catch
        end
    end
    failure = struct('schema','KSS-NUMOPT-2-MATLAB-FAILURE-V1', ...
        'status','FAIL','experiment_id',label, ...
        'identifier',exception.identifier,'message',exception.message);
    write_atomic_json(failure,fullfile(output_dir,'failure.json'));
    rethrow(exception);
end
end


function [outcome,worker,firm] = fixture( ...
    workers,firms,density,rows_per_cell,connectivity)
cells = workers*density;
rows = cells*rows_per_cell;
outcome = zeros(rows,1);
worker = zeros(rows,1);
firm = zeros(rows,1);
chunk = 1000000;
for first = 1:chunk:rows
    last = min(rows,first+chunk-1);
    index = (first:last)';
    deletion = floor((index-1)/rows_per_cell)+1;
    replicate = mod(index-1,rows_per_cell)+1;
    slot = mod(deletion-1,density)+1;
    worker_chunk = floor((deletion-1)/density)+1;
    layer = floor((worker_chunk-1)/firms);
    base_firm = mod(worker_chunk-1,firms);
    offset = slot-1;
    if strcmp(connectivity,'strong')
        band = floor(firms/4);
        selected = slot == 2;
        offset(selected) = 1+mod(layer(selected),band-1);
        selected = slot == 3;
        offset(selected) = ceil(firms/3)+mod(97*layer(selected),band);
        selected = slot == 4;
        offset(selected) = ceil(2*firms/3)+mod(193*layer(selected),band);
        selected = slot == 5;
        offset(selected) = firms-1;
        selected = slot == 6;
        offset(selected) = ceil(2*firms/3)-1;
        selected = slot == 7;
        offset(selected) = ceil(firms/3)-1;
    end
    firm_chunk = mod(base_firm+offset,firms)+1;
    worker(first:last) = worker_chunk;
    firm(first:last) = firm_chunk;
    outcome(first:last) = sin(worker_chunk/97)+cos(firm_chunk/31)+ ...
        .03*replicate+sin(deletion/113);
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
assert_nm(status == 0,'DetailCount','Could not count detail rows.');
count = str2double(strtrim(text));
assert_nm(isfinite(count) && count >= 1 && count == floor(count), ...
    'DetailCount','Invalid detail row count.');
end


function value = required_env(name)
value = getenv(name);
assert_nm(~isempty(value),'Environment',['Missing ' name '.']);
end


function value = required_integer(name)
text = required_env(name);
value = str2double(text);
assert_nm(isfinite(value) && value >= 1 && value == floor(value), ...
    'Environment',['Invalid ' name '.']);
end


function path = canonical_path(path)
path = char(java.io.File(path).getCanonicalPath());
end


function write_atomic_json(value,path)
temporary = [path '.tmp'];
handle = fopen(temporary,'w');
assert_nm(handle >= 0,'JSON','Could not open JSON output.');
fprintf(handle,'%s\n',jsonencode(value));
fclose(handle);
movefile(temporary,path,'f');
end


function assert_nm(condition,code,message)
if ~condition
    error(['kss:numopt2Matlab:' code],'%s',message);
end
end

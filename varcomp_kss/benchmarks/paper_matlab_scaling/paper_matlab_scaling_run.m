function paper_matlab_scaling_run()
% Source-bound maintained MATLAB comparison on one registered synthetic input.

output_dir = required_env('PMS_OUTPUT_DIR');
scratch_dir = required_env('PMS_SCRATCH_DIR');
input_file = required_env('PMS_INPUT_CSV');
input_sha = required_env('PMS_INPUT_SHA256');
matlab_root = required_env('PMS_MATLAB_ROOT');
task_sha = required_env('PMS_TASK_SHA256');
source_identity_path = required_env('PMS_SOURCE_IDENTITY');
process_identity_file = required_env('PMS_PROCESS_IDENTITY');
experiment = required_env('PMS_EXPERIMENT_ID');
source_commit = required_env('PMS_SOURCE_COMMIT');
bundle_sha = required_env('PMS_BUNDLE_SHA256');
structure = required_env('PMS_STRUCTURE');
connectivity = required_env('PMS_CONNECTIVITY');
expected_rows = required_integer('PMS_ROWS');
expected_workers = required_integer('PMS_WORKERS');
expected_firms = required_integer('PMS_FIRMS');
expected_degree = required_integer('PMS_DEGREE');
probes = required_integer('PMS_PROBES');
seed = required_integer('PMS_SEED');
mappings = struct('strong_d2',{{'strong',2}}, ...
    'strong_d3',{{'strong',3}},'strong_d6',{{'strong',6}}, ...
    'weak_d3',{{'weak',3}});
assert_pms(isfield(mappings,structure),'Structure','Unknown graph structure.');
mapping = mappings.(structure);
assert_pms(strcmp(connectivity,mapping{1}) && expected_degree==mapping{2}, ...
    'Structure','Graph structure contract changed.');
assert_pms(expected_rows == expected_degree*expected_workers && ...
    expected_workers == 40*expected_firms && ...
    ismember(expected_rows,[7680 30720 122880 491520 1966080]), ...
    'Dimensions','Synthetic task dimensions changed.');
assert_pms(probes==200 && seed >= 1,'Randomness', ...
    'Probe or seed contract changed.');
assert_pms(~isempty(regexp(experiment,'^[A-Za-z0-9._-]+$','once')) && ...
    ~isempty(regexp(task_sha,'^[0-9a-f]{64}$','once')) && ...
    ~isempty(regexp(input_sha,'^[0-9a-f]{64}$','once')),'Identity', ...
    'Invalid task or input identity.');
if ~isfolder(output_dir), mkdir(output_dir); end
mex_dir = fullfile(scratch_dir,'mex');
detail_dir = fullfile(scratch_dir,'details');
if ~isfolder(mex_dir), mkdir(mex_dir); end
if ~isfolder(detail_dir), mkdir(detail_dir); end

pool = [];
client_original = [];
worker_original = [];
try
    source_identity = jsondecode(fileread(source_identity_path));
    assert_pms(strcmp(source_identity.status,'PASS'),'SourceIdentity', ...
        'Maintained source identity did not pass.');
    addpath(fullfile(matlab_root,'codes'));
    addpath(genpath(fullfile(matlab_root,'CMG')));
    resolved = which('leave_out_KSS');
    assert_pms(strcmp(canonical_path(resolved),canonical_path( ...
        fullfile(matlab_root,'codes','leave_out_KSS.m'))), ...
        'CoreResolution','Maintained MATLAB entry point changed.');

    import_started = tic;
    data = readtable(input_file,'VariableNamingRule','preserve');
    import_seconds = toc(import_started);
    validation_started = tic;
    required = {'observation_key','worker','firm','period','match','y'};
    assert_pms(isequal(data.Properties.VariableNames,required), ...
        'InputColumns','Input columns changed.');
    worker = double(data.worker);
    firm = double(data.firm);
    period = double(data.period);
    match = double(data.match);
    outcome = double(data.y);
    observation_key = double(data.observation_key);
    clear data
    assert_pms(numel(outcome)==expected_rows && all(isfinite(outcome)) && ...
        all(worker==floor(worker)) && all(firm==floor(firm)) && ...
        all(match==floor(match)) && all(observation_key==floor(observation_key)), ...
        'InputValues','Input values changed.');
    assert_pms(min(worker)==1 && max(worker)==expected_workers && ...
        min(firm)==1 && max(firm)==expected_firms && ...
        numel(unique(match))==expected_rows && ...
        size(unique([worker firm],'rows'),1)==expected_rows, ...
        'InputDimensions','Input identifiers changed.');
    assert_pms(isequal(observation_key,(1:expected_rows)') && ...
        all(period>=1 & period<=expected_degree), ...
        'InputOrder','Input order changed.');
    input_validation_seconds = toc(validation_started);
    clear period match observation_key

    existing_pool = gcp('nocreate');
    assert_pms(isempty(existing_pool),'PreexistingPool', ...
        'Comparison requires a fresh MATLAB process.');
    parallel_dir = fullfile(scratch_dir,'parallel');
    if ~isfolder(parallel_dir), mkdir(parallel_dir); end
    cluster = parcluster('local');
    cluster.JobStorageLocation = parallel_dir;
    pool_started = tic;
    pool = parpool(cluster,4,'IdleTimeout',Inf);
    pool_startup_seconds = toc(pool_started);
    assert_pms(pool.NumWorkers==4,'PoolSize', ...
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
    assert_pms(isequal(worker_indices,1:4) && ...
        numel(unique([client_pid worker_pids]))==5,'ProcessIdentity', ...
        'MATLAB process identities changed.');
    identity = struct('schema','kss_matlab_scale_process_identity_v1', ...
        'status','PASS','pid_api','matlabProcessID_R2025a', ...
        'mode','cold','label',experiment,'case_sha256',task_sha, ...
        'expected_pool_workers',4,'client_pid',client_pid, ...
        'worker_indices',worker_indices,'worker_pids',worker_pids);
    write_atomic_json(identity,process_identity_file);

    mex_started = tic;
    compile_run_local_mex(matlab_root,mex_dir);
    mex_setup_seconds = toc(mex_started);
    client_original = rng;
    spmd
        worker_original = rng;
    end
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
    detail_stub = fullfile(detail_dir,[experiment '-detail']);
    command_started = tic;
    [firm_variance,covariance,worker_variance] = leave_out_KSS( ...
        outcome,worker,firm,controls,leave_out_level,type_algorithm, ...
        probes,lincom_do,Z_lincom,labels_lincom,detail_stub);
    command_seconds = toc(command_started);
    targets = [worker_variance firm_variance covariance ...
        worker_variance+firm_variance+2*covariance];
    assert_pms(all(isfinite(targets)),'Targets', ...
        'Maintained estimator returned nonfinite targets.');
    identity_error = abs(targets(4)-targets(1)-targets(2)-2*targets(3)) / ...
        (1+sum(abs(targets)));
    assert_pms(identity_error<=1e-12,'TargetIdentity', ...
        'Maintained target identity failed.');
    detail_file = [detail_stub '.csv'];
    assert_pms(isfile(detail_file),'DetailMissing', ...
        'Maintained detail output is missing.');
    detail_lines = file_line_count(detail_file);
    detail_bytes = dir(detail_file).bytes;
    assert_pms(detail_lines==expected_rows,'RetainedMatches', ...
        'Maintained estimator changed the fixed sample.');
    delete(detail_file);

    rng(client_original);
    spmd
        rng(worker_original);
    end
    teardown_started = tic;
    delete(pool);
    pool = [];
    pool_teardown_seconds = toc(teardown_started);

    record = struct('schema','PAPER-MATLAB-SCALING-MATLAB-V1', ...
        'status','PASS','experiment_id',experiment, ...
        'source_commit',source_commit,'bundle_sha256',bundle_sha, ...
        'task_sha256',task_sha,'input_sha256',input_sha, ...
        'matlab_upstream_commit',source_identity.matlab_upstream_commit, ...
        'matlab_runtime_tree_sha256', ...
            source_identity.matlab_runtime_tree_sha256, ...
        'matlab_core_sha256',source_identity.matlab_core_sha256, ...
        'matlab_version',version,'structure',structure, ...
        'connectivity',connectivity,'rows',expected_rows, ...
        'workers',expected_workers,'firms',expected_firms, ...
        'cells_per_worker',expected_degree,'probes',probes,'seed',seed, ...
        'pool_workers',4,'import_seconds',import_seconds, ...
        'input_validation_seconds',input_validation_seconds, ...
        'pool_startup_seconds',pool_startup_seconds, ...
        'mex_setup_seconds',mex_setup_seconds, ...
        'command_seconds',command_seconds, ...
        'pool_teardown_seconds',pool_teardown_seconds, ...
        'detail_matches',detail_lines,'detail_bytes',detail_bytes, ...
        'corrected_worker',targets(1),'corrected_firm',targets(2), ...
        'corrected_covariance',targets(3),'corrected_total',targets(4), ...
        'target_identity_scaled_error',identity_error, ...
        'same_literal_match_rows',true, ...
        'uniform_stored_row_targets',true, ...
        'rng_draws_comparable',false,'solver_tolerance_comparable',false, ...
        'correction_formula_comparable',false, ...
        'corrected_estimate_equality_gate','NONE_DESCRIPTIVE_ONLY');
    write_atomic_json(record,fullfile(output_dir,'matlab_aggregate.json'));
    marker = fopen(fullfile(output_dir,'matlab.pass'),'w');
    assert_pms(marker>=0,'Marker','Could not write MATLAB pass marker.');
    fprintf(marker,'PAPER_MATLAB_SCALING_MATLAB_PASS %s %s\n', ...
        experiment,task_sha);
    fclose(marker);
    fprintf('PAPER MATLAB SCALING MATLAB PASS: %s\n',experiment);
catch exception
    if ~isempty(client_original)
        try, rng(client_original); catch, end
    end
    if ~isempty(worker_original) && ~isempty(pool)
        try
            spmd
                rng(worker_original);
            end
        catch
        end
    end
    if ~isempty(pool)
        try, delete(pool); catch, end
    end
    failure = struct('schema','PAPER-MATLAB-SCALING-MATLAB-FAILURE-V1', ...
        'status','FAIL','experiment_id',experiment, ...
        'identifier',exception.identifier,'message',exception.message);
    write_atomic_json(failure,fullfile(output_dir,'matlab_failure.json'));
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
assert_pms(status==0,'DetailCount','Could not count detail rows.');
count = str2double(strtrim(text));
assert_pms(isfinite(count) && count>=1 && count==floor(count), ...
    'DetailCount','Invalid detail row count.');
end


function value = required_env(name)
value = getenv(name);
assert_pms(~isempty(value),'Environment',['Missing ' name '.']);
end


function value = required_integer(name)
text = required_env(name);
value = str2double(text);
assert_pms(isfinite(value) && value>=1 && value==floor(value), ...
    'Environment',['Invalid ' name '.']);
end


function path = canonical_path(path)
path = char(java.io.File(path).getCanonicalPath());
end


function write_atomic_json(value,path)
temporary = [path '.tmp'];
handle = fopen(temporary,'w');
assert_pms(handle>=0,'JSON','Could not open JSON output.');
fprintf(handle,'%s\n',jsonencode(value));
fclose(handle);
movefile(temporary,path,'f');
end


function assert_pms(condition,code,message)
if ~condition
    error(['varcomp_kss:paperMatlabScaling:' code],'%s',message);
end
end

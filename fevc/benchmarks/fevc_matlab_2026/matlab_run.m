function matlab_run()
% One fresh-process maintained MATLAB LeaveOutTwoWay benchmark call.

output_dir = required_env('VCS_OUTPUT_DIR');
scratch_dir = required_env('VCS_SCRATCH_DIR');
input_file = required_env('VCS_INPUT_CSV');
input_sha = required_env('VCS_INPUT_SHA256');
matlab_root = required_env('VCS_MATLAB_ROOT');
mex_dir = required_env('VCS_MEX_DIR');
source_identity_path = required_env('VCS_SOURCE_IDENTITY');
process_identity_file = required_env('VCS_PROCESS_IDENTITY');
phase_start_file = required_env('VCS_PHASE_START');
phase_end_file = required_env('VCS_PHASE_END');
empty_ready_file = required_env('VCS_EMPTY_READY');
data_ready_file = required_env('VCS_DATA_READY');
experiment = required_env('VCS_EXPERIMENT_ID');
source_commit = required_env('VCS_SOURCE_COMMIT');
bundle_sha = required_env('VCS_BUNDLE_SHA256');
task_sha = required_env('VCS_TASK_SHA256');
structure = required_env('VCS_STRUCTURE');
connectivity = required_env('VCS_CONNECTIVITY');
expected_rows = required_integer('VCS_ROWS');
expected_workers = required_integer('VCS_WORKERS');
expected_firms = required_integer('VCS_FIRMS');
expected_degree = required_integer('VCS_DEGREE');
probes = required_integer('VCS_PROBES');
seed = required_integer('VCS_SEED');
active_cores = required_integer('VCS_ACTIVE_CORES');

mappings = struct('strong_d2',{{'strong',2}}, ...
    'strong_d3',{{'strong',3}},'strong_d6',{{'strong',6}}, ...
    'weak_d3',{{'weak',3}});
assert_vcs(isfield(mappings,structure),'Structure','Unknown graph structure.');
mapping = mappings.(structure);
assert_vcs(strcmp(connectivity,mapping{1}) && expected_degree==mapping{2}, ...
    'Structure','Graph structure contract changed.');
weak_branches = min(40,expected_workers/128);
weak_dimensions = strcmp(structure,'weak_d3') && ...
    mod(expected_workers,5)==0 && weak_branches==floor(weak_branches) && ...
    expected_firms == expected_workers/5+1+40*weak_branches;
strong_dimensions = ~strcmp(structure,'weak_d3') && ...
    expected_workers == 40*expected_firms;
assert_vcs(expected_rows == expected_degree*expected_workers && ...
    (weak_dimensions || strong_dimensions) && ...
    ismember(expected_rows,[7680 30720 122880 491520 1966080]), ...
    'Dimensions','Synthetic task dimensions changed.');
assert_vcs(ismember(active_cores,[1 2 4 8 14 28]) && probes==200 && seed>=1, ...
    'Resources','Core, probe, or seed contract changed.');
assert_vcs(~isempty(regexp(experiment,'^[A-Za-z0-9._-]+$','once')) && ...
    ~isempty(regexp(task_sha,'^[0-9a-f]{64}$','once')) && ...
    ~isempty(regexp(input_sha,'^[0-9a-f]{64}$','once')) && ...
    ~isempty(regexp(bundle_sha,'^[0-9a-f]{64}$','once')), ...
    'Identity','Invalid source, task, or input identity.');
if ~isfolder(output_dir), mkdir(output_dir); end
if ~isfolder(scratch_dir), mkdir(scratch_dir); end
detail_dir = fullfile(scratch_dir,'details');
parallel_dir = fullfile(scratch_dir,'parallel');
if ~isfolder(detail_dir), mkdir(detail_dir); end
if ~isfolder(parallel_dir), mkdir(parallel_dir); end

pool = [];
client_original = [];
worker_original = [];
phase_started = false;
phase_ended = false;
try
    source_identity = jsondecode(fileread(source_identity_path));
    assert_vcs(strcmp(source_identity.status,'PASS'),'SourceIdentity', ...
        'Maintained MATLAB source identity did not pass.');
    addpath(fullfile(matlab_root,'codes'));
    addpath(genpath(fullfile(matlab_root,'CMG')));
    addpath(mex_dir,'-begin');
    rehash;
    resolved = which('leave_out_KSS');
    assert_vcs(strcmp(canonical_path(resolved),canonical_path( ...
        fullfile(matlab_root,'codes','leave_out_KSS.m'))), ...
        'CoreResolution','Maintained MATLAB entry point changed.');

    existing_pool = gcp('nocreate');
    assert_vcs(isempty(existing_pool),'PreexistingPool', ...
        'Comparison requires a fresh MATLAB process.');
    cluster = parcluster('local');
    cluster.JobStorageLocation = parallel_dir;
    pool_started = tic;
    pool = parpool(cluster,active_cores,'IdleTimeout',Inf);
    pool_startup_seconds = toc(pool_started);
    assert_vcs(pool.NumWorkers==active_cores,'PoolSize', ...
        'MATLAB did not honor the active-core contract.');
    maxNumCompThreads(1);
    spmd
        maxNumCompThreads(1);
    end

    [client_pid,pid_api] = vcs_process_id();
    spmd
        worker_pid = vcs_process_id();
        worker_index = spmdIndex;
    end
    worker_pids = zeros(1,active_cores);
    worker_indices = zeros(1,active_cores);
    for index = 1:active_cores
        worker_pids(index) = worker_pid{index};
        worker_indices(index) = worker_index{index};
    end
    assert_vcs(isequal(worker_indices,1:active_cores) && ...
        numel(unique([client_pid worker_pids]))==active_cores+1, ...
        'ProcessIdentity','MATLAB process identities changed.');
    identity = struct('schema','FEVC-MATLAB-2026-MATLAB-PROCESS-V1', ...
        'status','PASS','pid_api',pid_api,'label',experiment, ...
        'task_sha256',task_sha,'expected_pool_workers',active_cores, ...
        'client_pid',client_pid,'worker_indices',worker_indices, ...
        'worker_pids',worker_pids);
    write_atomic_json(identity,process_identity_file);
    write_marker(empty_ready_file,['EMPTY_READY matlab ' experiment]);
    pause(1.0);

    import_started = tic;
    data = readtable(input_file,'VariableNamingRule','preserve');
    import_seconds = toc(import_started);
    validation_started = tic;
    required = {'observation_key','worker','firm','period','match','y'};
    assert_vcs(isequal(data.Properties.VariableNames,required), ...
        'InputColumns','Input columns changed.');
    worker = double(data.worker);
    firm = double(data.firm);
    period = double(data.period);
    match = double(data.match);
    outcome = double(data.y);
    observation_key = double(data.observation_key);
    clear data
    assert_vcs(numel(outcome)==expected_rows && all(isfinite(outcome)) && ...
        all(worker==floor(worker)) && all(firm==floor(firm)) && ...
        all(match==floor(match)) && all(observation_key==floor(observation_key)), ...
        'InputValues','Input values changed.');
    assert_vcs(min(worker)==1 && max(worker)==expected_workers && ...
        min(firm)==1 && max(firm)==expected_firms && ...
        numel(unique(match))==expected_rows && ...
        size(unique([worker firm],'rows'),1)==expected_rows, ...
        'InputDimensions','Input identifiers changed.');
    assert_vcs(isequal(observation_key,(1:expected_rows)') && ...
        all(period>=1 & period<=expected_degree), ...
        'InputOrder','Input ordering changed.');
    input_validation_seconds = toc(validation_started);
    clear period match observation_key
    write_marker(data_ready_file,['DATA_READY matlab ' experiment]);
    pause(1.0);

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
    write_marker(phase_start_file,['START matlab ' experiment]);
    phase_started = true;
    command_started = tic;
    [firm_variance,covariance,worker_variance] = leave_out_KSS( ...
        outcome,worker,firm,controls,leave_out_level,type_algorithm, ...
        probes,lincom_do,Z_lincom,labels_lincom,detail_stub);
    command_seconds = toc(command_started);
    write_marker(phase_end_file,['END matlab ' experiment]);
    phase_ended = true;

    targets = [worker_variance firm_variance covariance ...
        worker_variance+firm_variance+2*covariance];
    assert_vcs(all(isfinite(targets)),'Targets', ...
        'Maintained estimator returned nonfinite targets.');
    identity_error = abs(targets(4)-targets(1)-targets(2)-2*targets(3)) / ...
        (1+sum(abs(targets)));
    assert_vcs(identity_error<=1e-12,'TargetIdentity', ...
        'Maintained target identity failed.');
    detail_file = [detail_stub '.csv'];
    assert_vcs(isfile(detail_file),'DetailMissing', ...
        'Maintained detail output is missing.');
    detail_lines = file_line_count(detail_file);
    detail_bytes = dir(detail_file).bytes;
    assert_vcs(detail_lines==expected_rows,'RetainedMatches', ...
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

    record = struct('schema','FEVC-MATLAB-2026-MATLAB-V1', ...
        'status','PASS','role','matlab','experiment_id',experiment, ...
        'source_commit',source_commit,'bundle_sha256',bundle_sha, ...
        'task_sha256',task_sha,'input_sha256',input_sha, ...
        'matlab_upstream_commit',source_identity.matlab_upstream_commit, ...
        'matlab_runtime_tree_sha256',source_identity.matlab_runtime_tree_sha256, ...
        'matlab_core_sha256',source_identity.matlab_core_sha256, ...
        'matlab_version',version,'structure',structure, ...
        'connectivity',connectivity,'rows',expected_rows, ...
        'workers',expected_workers,'firms',expected_firms, ...
        'cells_per_worker',expected_degree,'probes',probes,'seed',seed, ...
        'active_cores',active_cores,'pool_workers',active_cores, ...
        'client_threads',1,'worker_threads_each',1, ...
        'import_seconds',import_seconds, ...
        'input_validation_seconds',input_validation_seconds, ...
        'pool_startup_seconds',pool_startup_seconds, ...
        'command_seconds',command_seconds, ...
        'pool_teardown_seconds',pool_teardown_seconds, ...
        'detail_matches',detail_lines,'detail_bytes',detail_bytes, ...
        'corrected_worker',targets(1),'corrected_firm',targets(2), ...
        'corrected_covariance',targets(3),'corrected_total',targets(4), ...
        'target_identity_scaled_error',identity_error, ...
        'same_literal_match_rows',true,'uniform_stored_row_targets',true, ...
        'rng_draws_comparable',false,'solver_tolerance_comparable',false, ...
        'rng_policy','MATLAB_TWISTER_SEED_PLUS_WORKER_INDEX', ...
        'tolerance_policy','MAINTAINED_UPSTREAM_INTERNAL', ...
        'numerical_status_source','APPLICATION_LOG_PCG_AND_OUTPUT_GATES', ...
        'correction_formula_comparable',false, ...
        'corrected_estimate_equality_gate','NONE_DESCRIPTIVE_ONLY');
    write_atomic_json(record,fullfile(output_dir,'matlab.json'));
    write_marker(fullfile(output_dir,'application.pass'), ...
        ['FEVC_MATLAB_2026_MATLAB_PASS ' experiment]);
    fprintf('FEVC MATLAB 2026 MATLAB PASS: %s\n',experiment);
catch exception
    if phase_started && ~phase_ended
        try, write_marker(phase_end_file,['END matlab FAIL ' experiment]); catch, end
    end
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
    failure = struct('schema','FEVC-MATLAB-2026-MATLAB-FAILURE-V1', ...
        'status','FAIL','experiment_id',experiment, ...
        'identifier',exception.identifier,'message',exception.message);
    write_atomic_json(failure,fullfile(output_dir,'failure.json'));
    rethrow(exception);
end
end

function [pid,api] = vcs_process_id()
if exist('matlabProcessID','builtin')==5 || exist('matlabProcessID','file')==2
    pid = double(matlabProcessID);
    api = 'matlabProcessID';
else
    pid = double(feature('getpid'));
    api = 'feature_getpid';
end
end

function count = file_line_count(path)
[status,text] = system(['wc -l < ' path]);
assert_vcs(status==0,'DetailCount','Could not count detail rows.');
count = str2double(strtrim(text));
assert_vcs(isfinite(count) && count>=1 && count==floor(count), ...
    'DetailCount','Invalid detail row count.');
end

function value = required_env(name)
value = getenv(name);
assert_vcs(~isempty(value),'Environment',['Missing ' name '.']);
end

function value = required_integer(name)
text = required_env(name);
value = str2double(text);
assert_vcs(isfinite(value) && value>=1 && value==floor(value), ...
    'Environment',['Invalid ' name '.']);
end

function path = canonical_path(path)
path = char(java.io.File(path).getCanonicalPath());
end

function write_marker(path,value)
handle = fopen(path,'w');
assert_vcs(handle>=0,'Marker','Could not open marker.');
fprintf(handle,'%s\n',value);
fclose(handle);
end

function write_atomic_json(value,path)
temporary = [path '.tmp'];
handle = fopen(temporary,'w');
assert_vcs(handle>=0,'JSON','Could not open JSON output.');
fprintf(handle,'%s\n',jsonencode(value));
fclose(handle);
movefile(temporary,path,'f');
end

function assert_vcs(condition,code,message)
if ~condition
    error(['fevc:comparativeScaling:' code],'%s',message);
end
end

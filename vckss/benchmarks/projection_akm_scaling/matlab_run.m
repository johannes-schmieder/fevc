function matlab_run()
% Maintained leave_out_COMPLETE JLA followed by maintained lincom_KSS.
output_dir = required_env('VPA_OUTPUT_DIR');
scratch_dir = required_env('VPA_SCRATCH_DIR');
input_csv = required_env('VPA_INPUT_CSV');
input_sha = required_env('VPA_INPUT_SHA256');
matlab_root = required_env('VPA_MATLAB_ROOT');
mex_dir = required_env('VPA_MEX_DIR');
phase_start = required_env('VPA_PHASE_START');
phase_end = required_env('VPA_PHASE_END');
process_identity = required_env('VPA_PROCESS_IDENTITY');
source_commit = required_env('VPA_SOURCE_COMMIT');
rows_expected = required_integer('VPA_ROWS');
probes = required_integer('VPA_PROBES');
seed = required_integer('VPA_SEED');
active_cores = required_integer('VPA_ACTIVE_CORES');
assert_vpa(ismember(rows_expected,[6000 480000 1920000 7680000]), ...
    'Rows','Rows changed.');
assert_vpa(ismember(active_cores,[4 16]),'Cores','Core count changed.');
registered_rows = [6000 480000 1920000 7680000];
registered_probes = [1256 1888 2088 2288];
assert_vpa(probes==registered_probes(registered_rows==rows_expected), ...
    'Probes','Probe count changed.');
assert_vpa(~isfile(process_identity),'Identity','Stale process identity.');

% Some installations report the toolbox license as available even though the
% toolbox files are not installed.  The maintained estimator uses corr only
% for a printed diagnostic, so route the committed compatibility helper by
% function availability rather than by license metadata.
if exist('corr','file')~=2
    addpath(fullfile(fileparts(mfilename('fullpath')),'compat'));
end
addpath(fullfile(matlab_root,'codes'));
addpath(genpath(fullfile(matlab_root,'CMG')));
addpath(mex_dir,'-begin');
resolved_complete = which('leave_out_COMPLETE');
resolved_lincom = which('lincom_KSS');
assert_vpa(strcmp(canonical_path(resolved_complete),canonical_path( ...
    fullfile(matlab_root,'codes','leave_out_COMPLETE.m'))), ...
    'Source','leave_out_COMPLETE resolution changed.');
assert_vpa(strcmp(canonical_path(resolved_lincom),canonical_path( ...
    fullfile(matlab_root,'codes','lincom_KSS.m'))), ...
    'Source','lincom_KSS resolution changed.');

data = readtable(input_csv,'VariableNamingRule','preserve');
required = {'observation_key','worker','firm','period','y','z1','z2'};
assert_vpa(isequal(data.Properties.VariableNames,required),'Input','Columns changed.');
y_input = double(data.y);
id_input = double(data.worker);
firm_input = double(data.firm);
assert_vpa(numel(y_input)==rows_expected && all(isfinite(y_input)), ...
    'Input','Input rows changed.');
clear data

pool = [];
phase_started = false;
phase_ended = false;
try
    parallel_dir = fullfile(scratch_dir,'parallel');
    if ~isfolder(parallel_dir), mkdir(parallel_dir); end
    cluster = parcluster('local');
    cluster.JobStorageLocation = parallel_dir;
    pool_start = tic;
    pool = parpool(cluster,active_cores,'IdleTimeout',Inf);
    pool_startup_seconds = toc(pool_start);
    assert_vpa(pool.NumWorkers==active_cores,'Pool','Pool size changed.');
    maxNumCompThreads(1);
    spmd, maxNumCompThreads(1); end

    [client_pid,pid_api] = process_id();
    spmd
        worker_pid = process_id();
        worker_index = spmdIndex;
    end
    worker_pids = zeros(1,active_cores);
    worker_indices = zeros(1,active_cores);
    for index = 1:active_cores
        worker_pids(index) = worker_pid{index};
        worker_indices(index) = worker_index{index};
    end
    assert_vpa(isequal(worker_indices,1:active_cores) && ...
        numel(unique([client_pid worker_pids]))==active_cores+1, ...
        'Identity','MATLAB process identities changed.');
    identity = struct('schema','VCKSS-PROJECTION-AKM-MATLAB-PROCESS-V1', ...
        'status','PASS','pid_api',pid_api, ...
        'expected_pool_workers',active_cores,'client_pid',client_pid, ...
        'worker_indices',worker_indices,'worker_pids',worker_pids);
    write_atomic_json(identity,process_identity);

    rng(seed,'twister');
    spmd, rng(seed+spmdIndex,'twister'); end
    write_marker(phase_start,'START matlab');
    phase_started = true;
    command_start = tic;
    stub = fullfile(scratch_dir,'maintained');
    [~,~,~] = leave_out_COMPLETE(y_input,id_input,firm_input, ...
        'obs',[],0,0,0,2,1,0,'JLL',.01,stub);
    state = load([stub '_after_step3.mat'],'y','id_old','firmid_old', ...
        'D','F','S','Lambda_P','N','J','NT');
    assert_vpa(state.NT==rows_expected,'Sample','Maintained sample changed.');
    X = [state.D,state.F*state.S];
    xx = X'*X;
    Lchol = ichol(xx,struct('type','ict','droptol',1e-2,'diagcomp',.1));
    [beta,fit_flag,fit_relres,fit_iterations] = pcg( ...
        xx,X'*state.y,1e-10,1000,Lchol,Lchol');
    assert_vpa(fit_flag==0 && fit_relres<=1e-10, ...
        'Fit','Grounded fit did not converge.');
    complete_fit_residual = norm(xx*beta-X'*state.y)/max(norm(X'*state.y),realmin);
    assert_vpa(complete_fit_residual<=1e-10,'Fit','Complete fit residual failed.');
    residual = state.y-X*beta;
    eta_h = (speye(state.NT)-state.Lambda_P)\residual;
    sigma_i = (state.y-mean(state.y)).*eta_h;
    z1 = sin(state.id_old/5)+cos(state.firmid_old/3);
    z2 = cos(state.id_old/7)-sin(state.firmid_old/4);
    Z = [z1,z2];
    Transform = [sparse(state.NT,state.N),state.F*state.S];
    [~,lincom_b,lincom_se] = lincom_KSS( ...
        state.y,X,Z,Transform,sigma_i,{'z1','z2'});
    Zfull = [ones(state.NT,1),Z];
    projection_b = Zfull\(Transform*beta);
    command_seconds = toc(command_start);
    write_marker(phase_end,'END matlab rc=0');
    phase_ended = true;
    delete(pool);
    pool = [];

    assert_vpa(all(isfinite([projection_b(:);lincom_b(:);lincom_se(:)])) && ...
        all(lincom_se>0),'Result','Maintained projection result is invalid.');
    record = struct('schema','VCKSS-PROJECTION-AKM-MATLAB-V1', ...
        'status','PASS','source_commit',source_commit,'input_sha256',input_sha, ...
        'rows',rows_expected,'workers',rows_expected/6,'firms',rows_expected/12, ...
        'probes',probes,'seed',seed,'active_cores',active_cores, ...
        'matlab_version',version,'pool_startup_seconds',pool_startup_seconds, ...
        'upstream_commit','8b957ffeb10b8465a3584fceb0265cccc48379e1', ...
        'leave_out_sha256', ...
        '54b30ebdc51b4c94873e2e3f205bbf865179220e3ad0df0e382922db7c84fc58', ...
        'lincom_sha256', ...
        '71fb47ce35d26c91dbf97926031359ed0eb31d9916b07c167c577867620ee5a9', ...
        'command_seconds',command_seconds,'probe_throughput',probes/command_seconds, ...
        'fit_flag',fit_flag,'fit_relative_residual',fit_relres, ...
        'complete_fit_residual',complete_fit_residual, ...
        'fit_iterations',fit_iterations,'b_cons',projection_b(1), ...
        'b_z1',lincom_b(1),'b_z2',lincom_b(2), ...
        'se_z1',lincom_se(1),'se_z2',lincom_se(2), ...
        'V_z1_z1',lincom_se(1)^2,'V_z2_z2',lincom_se(2)^2, ...
        'variance_proxy_min',min(sigma_i),'variance_proxy_max',max(sigma_i));
    write_atomic_json(record,fullfile(output_dir,'result.json'));
    write_marker(fullfile(output_dir,'application.pass'), ...
        sprintf('VCKSS PROJECTION AKM MATLAB PASS rows=%d cores=%d', ...
        rows_expected,active_cores));
    fprintf('VCKSS PROJECTION AKM MATLAB PASS rows=%d cores=%d\n', ...
        rows_expected,active_cores);
catch exception
    if phase_started && ~phase_ended
        write_marker(phase_end,'END matlab rc=1');
    end
    if ~isempty(pool)
        try, delete(pool); catch, end
    end
    failure = struct('schema','VCKSS-PROJECTION-AKM-MATLAB-FAILURE-V1', ...
        'status','FAIL','source_commit',source_commit,'input_sha256',input_sha, ...
        'rows',rows_expected,'active_cores',active_cores, ...
        'identifier',exception.identifier,'message',exception.message);
    write_atomic_json(failure,fullfile(output_dir,'failure.json'));
    rethrow(exception);
end
end

function [pid,api] = process_id()
if exist('matlabProcessID','builtin')==5 || exist('matlabProcessID','file')==2
    pid = double(matlabProcessID);
    api = 'matlabProcessID';
else
    pid = double(feature('getpid'));
    api = 'feature_getpid';
end
end

function value = required_env(name)
value = getenv(name);
assert_vpa(~isempty(value),'Environment',['Missing ' name '.']);
end

function value = required_integer(name)
value = str2double(required_env(name));
assert_vpa(isfinite(value) && value>=1 && value==floor(value), ...
    'Environment',['Invalid ' name '.']);
end

function path = canonical_path(path)
path = char(java.io.File(path).getCanonicalPath());
end

function write_marker(path,value)
handle = fopen(path,'w');
assert_vpa(handle>=0,'Marker','Could not create marker.');
fprintf(handle,'%s\n',value);
fclose(handle);
end

function write_atomic_json(value,path)
temporary = [path '.tmp'];
handle = fopen(temporary,'w');
assert_vpa(handle>=0,'Output','Could not create JSON output.');
fprintf(handle,'%s\n',jsonencode(value));
fclose(handle);
[ok,message] = movefile(temporary,path,'f');
assert_vpa(ok,'Output',['Could not publish JSON output: ' message]);
end

function assert_vpa(condition,code,message)
if ~condition
    error(['vckss:projectionAkm:' code],'%s',message);
end
end

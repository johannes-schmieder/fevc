function fevc_manual_matlab(input_file, output_file, matlab_root, ...
    deletion, algorithm, probes, seed, pool_workers, cmg_cache, detail_stub)
% Thin manual-test bridge to the external maintained LeaveOutTwoWay package.

arguments
    input_file (1,:) char
    output_file (1,:) char
    matlab_root (1,:) char
    deletion (1,:) char
    algorithm (1,:) char
    probes (1,1) double {mustBeInteger,mustBePositive}
    seed (1,1) double {mustBeInteger,mustBeNonnegative}
    pool_workers (1,1) double {mustBeInteger,mustBePositive}
    cmg_cache (1,:) char
    detail_stub (1,:) char
end

assert_manual(ismember(deletion, {'matches','obs'}), ...
    'Options', 'Unknown deletion mode.');
assert_manual(ismember(algorithm, {'default','exact','JLA'}), ...
    'Options', 'Unknown algorithm.');
assert_manual(isfile(input_file), 'Input', 'Input CSV does not exist.');
assert_manual(isfile(fullfile(matlab_root, 'codes', 'leave_out_KSS.m')), ...
    'Source', 'Maintained leave_out_KSS.m does not exist.');

addpath(fullfile(matlab_root, 'codes'), '-begin');
addpath(genpath(fullfile(matlab_root, 'CMG')), '-begin');
core_file = which('leave_out_KSS');
assert_manual(strcmp(canonical_path(core_file), canonical_path( ...
    fullfile(matlab_root, 'codes', 'leave_out_KSS.m'))), ...
    'Source', 'leave_out_KSS resolved outside matlab_root.');

mex_started = tic;
mex_compiled = prepare_cmg(matlab_root, cmg_cache);
mex_setup_seconds = toc(mex_started);

data = readtable(input_file, 'FileType', 'text', 'Delimiter', ',', ...
    'VariableNamingRule', 'preserve');
values = table2array(data);
assert_manual(isnumeric(values) && size(values, 2) >= 3 && ...
    size(values, 1) >= 1 && all(isfinite(values), 'all'), ...
    'Input', 'Input must contain finite numeric outcome, worker, and firm columns.');
outcome = double(values(:, 1));
worker = double(values(:, 2));
firm = double(values(:, 3));
controls = [];
if size(values, 2) > 3
    controls = double(values(:, 4:end));
end
input_rows = size(values, 1);
clear data values

pool = [];
pool_started = tic;
parallel_dir = fullfile(cmg_cache, 'parallel');
if ~isfolder(parallel_dir), mkdir(parallel_dir); end
cluster = parcluster('local');
cluster.JobStorageLocation = parallel_dir;
pool = parpool(cluster, pool_workers, 'IdleTimeout', Inf);
pool_cleanup = onCleanup(@() cleanup_pool(pool)); %#ok<NASGU>
pool_startup_seconds = toc(pool_started);
assert_manual(pool.NumWorkers == pool_workers, ...
    'Pool', 'MATLAB did not create the requested worker pool.');

rng(seed, 'twister');
spmd
    rng(seed + spmdIndex, 'twister');
end

command_started = tic;
if strcmp(algorithm, 'default')
    selected_algorithm = 'exact';
    if input_rows > 10000, selected_algorithm = 'JLA'; end
    default_directory = [detail_stub '-default'];
    assert_manual(~isfolder(default_directory), 'Details', ...
        'Temporary default-output directory already exists.');
    mkdir(default_directory);
    previous_directory = pwd;
    directory_cleanup = onCleanup(@() cd(previous_directory)); %#ok<NASGU>
    cd(default_directory);
    [firm_variance, covariance, worker_variance] = leave_out_KSS( ...
        outcome, worker, firm, controls, deletion);
    cd(previous_directory);
    clear directory_cleanup
    detail_file = fullfile(default_directory, 'leave_out_estimates.csv');
else
    selected_algorithm = algorithm;
    [firm_variance, covariance, worker_variance] = leave_out_KSS( ...
        outcome, worker, firm, controls, deletion, algorithm, probes, ...
        0, [], [], detail_stub);
    detail_file = [detail_stub '.csv'];
end
command_seconds = toc(command_started);
corrected = [worker_variance, firm_variance, covariance, ...
    worker_variance + firm_variance + 2 * covariance];
assert_manual(all(isfinite(corrected)), ...
    'Targets', 'Maintained estimator returned a nonfinite target.');
identity_error = abs(corrected(4) - corrected(1) - corrected(2) - ...
    2 * corrected(3)) / (1 + sum(abs(corrected)));
assert_manual(identity_error <= 1e-12, ...
    'Targets', 'Maintained targets failed their accounting identity.');

assert_manual(isfile(detail_file), ...
    'Details', 'Maintained estimator did not write its retained-unit file.');
detail = readmatrix(detail_file, 'FileType', 'text', 'Delimiter', '\t');
retained_units = size(detail, 1);
delete(detail_file);
if strcmp(algorithm, 'default')
    rmdir(default_directory);
end

teardown_started = tic;
delete(pool);
pool = [];
pool_teardown_seconds = toc(teardown_started);

schema = "FEVC-MANUAL-MATLAB-V1";
matlab_version = string(version);
matlab_release = string(version('-release'));
core_file = string(core_file);
selected_algorithm = string(selected_algorithm);
result = table(schema, matlab_version, matlab_release, core_file, ...
    selected_algorithm, ...
    input_rows, retained_units, pool_workers, mex_compiled, ...
    mex_setup_seconds, pool_startup_seconds, command_seconds, ...
    pool_teardown_seconds, corrected(1), corrected(2), corrected(3), ...
    corrected(4), identity_error, ...
    'VariableNames', {'schema','matlab_version','matlab_release','core_file', ...
    'selected_algorithm', ...
    'input_rows','retained_units','pool_workers','mex_compiled', ...
    'mex_setup_seconds','pool_startup_seconds','command_seconds', ...
    'pool_teardown_seconds','corrected_worker','corrected_firm', ...
    'corrected_covariance','corrected_total', ...
    'target_identity_scaled_error'});
writetable(result, output_file, 'FileType', 'text', 'Delimiter', ',');
fprintf('FEVC MANUAL MATLAB PASS: rows=%d requested=%s selected=%s\n', ...
    input_rows, algorithm, selected_algorithm);
end


function compiled = prepare_cmg(matlab_root, cmg_cache)
compiled = false;
if exist('graphprofile', 'file') == 3 && ...
        exist('mx_d_preconditioner', 'file') == 3
    return
end

release_cache = fullfile(cmg_cache, ...
    [version('-release') '-' mexext]);
if ~isfolder(release_cache), mkdir(release_cache); end
addpath(release_cache, '-begin');
rehash;
if exist('graphprofile', 'file') == 3 && ...
        exist('mx_d_preconditioner', 'file') == 3
    return
end

hierarchy_names = {'adjacency_cmg','diagconjugate','forest_components', ...
    'graphprofile','laplacian2','perturbtril','splitforest', ...
    'update_groups','vpack'};
hierarchy_source = fullfile(matlab_root, 'CMG', 'Source', 'Hierarchy');
for index = 1:numel(hierarchy_names)
    source = fullfile(hierarchy_source, [hierarchy_names{index} '.c']);
    assert_manual(isfile(source), 'CMG', ...
        ['Missing CMG hierarchy source: ' source]);
    target = fullfile(release_cache, ...
        [hierarchy_names{index} '.' mexext]);
    if ~isfile(target)
        mex('-silent', '-largeArrayDims', '-outdir', release_cache, source);
        compiled = true;
    end
end

include_dir = fullfile(matlab_root, 'CMG', 'Include');
solver_source = fullfile(matlab_root, 'CMG', 'Source', 'Solver');
gateway = fullfile(matlab_root, 'CMG', 'MATLAB', 'Solver', ...
    'mx_d_preconditioner.c');
solver_names = {'vpv','vmv','vpvmv','vvmul','rmvec','ldl_solve', ...
    'sspmv','preconditioner'};
solver_sources = cell(1, numel(solver_names) + 1);
solver_sources{1} = gateway;
for index = 1:numel(solver_names)
    solver_sources{index + 1} = fullfile(solver_source, ...
        [solver_names{index} '.c']);
    assert_manual(isfile(solver_sources{index + 1}), 'CMG', ...
        ['Missing CMG solver source: ' solver_sources{index + 1}]);
end
solver_target = fullfile(release_cache, ...
    ['mx_d_preconditioner.' mexext]);
if ~isfile(solver_target)
    mex('-silent', '-largeArrayDims', ['-I' include_dir], ...
        '-outdir', release_cache, '-output', 'mx_d_preconditioner', ...
        solver_sources{:});
    compiled = true;
end

addpath(release_cache, '-begin');
rehash;
clear graphprofile mx_d_preconditioner
graphprofile_path = which('graphprofile');
preconditioner_path = which('mx_d_preconditioner');
assert_manual(endsWith(graphprofile_path, ['.' mexext]), ...
    'CMG', ['graphprofile MEX is unavailable after compilation: ' ...
    graphprofile_path]);
assert_manual(endsWith(preconditioner_path, ['.' mexext]), ...
    'CMG', ['mx_d_preconditioner MEX is unavailable after compilation: ' ...
    preconditioner_path]);
end


function cleanup_pool(pool)
if ~isempty(pool)
    try
        delete(pool);
    catch
    end
end
end


function path = canonical_path(path)
path = char(java.io.File(path).getCanonicalPath());
end


function assert_manual(condition, identifier, message)
if ~condition
    error(['fevc:manual:' identifier], '%s', message);
end
end

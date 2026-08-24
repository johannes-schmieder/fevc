function separations_kss_reference(input_file, output_file, detailed_file, ...
    label, seed, probes, source_commit, prepared_sha256, kss_core_sha256, ...
    matlab_cmg_sha256, matlab_cmg_mex_sha256, matlab_cmg_solver_sha256)
% Thin, bounded invocation of the existing Separations LeaveOutTwoWay KSS.
% The maintained reference implementation remains external and read-only.

arguments
    input_file (1,:) char
    output_file (1,:) char
    detailed_file (1,:) char
    label (1,:) char
    seed (1,1) double {mustBeInteger,mustBeNonnegative}
    probes (1,1) double {mustBeInteger,mustBePositive}
    source_commit (1,:) char
    prepared_sha256 (1,:) char
    kss_core_sha256 (1,:) char
    matlab_cmg_sha256 (1,:) char
    matlab_cmg_mex_sha256 (1,:) char
    matlab_cmg_solver_sha256 (1,:) char
end
if probes ~= 200
    error('The registered real-data comparison requires exactly 200 probes.');
end
if isempty(regexp(label, '^[A-Za-z0-9._-]+$', 'once')) || ...
        isempty(regexp(source_commit, '^[0-9a-f]{40}$', 'once')) || ...
        isempty(regexp(prepared_sha256, '^[0-9a-f]{64}$', 'once')) || ...
        isempty(regexp(kss_core_sha256, '^[0-9a-f]{64}$', 'once')) || ...
        isempty(regexp(matlab_cmg_sha256, '^[0-9a-f]{64}$', 'once')) || ...
        isempty(regexp(matlab_cmg_mex_sha256, '^[0-9a-f]{64}$', 'once')) || ...
        isempty(regexp(matlab_cmg_solver_sha256, '^[0-9a-f]{64}$', 'once'))
    error('Invalid source-binding metadata.');
end

kss_root = getenv('KSS_MATLAB_ROOT');
if isempty(kss_root)
    error('KSS_MATLAB_ROOT is required.');
end
addpath(fullfile(kss_root, 'codes'));
addpath(genpath(fullfile(kss_root, 'CMG')));
if exist('cmg_sdd', 'file') ~= 2
    error('The checksum-bound MATLAB CMG entry point is unavailable.');
end

% The maintained project does not carry SCC binaries. Compile its
% checksum-bound hierarchy sources into the KSS run directory, never into the
% read-only Separations tree, and put only that scratch directory first.
mex_output_dir = getenv('KSS_MATLAB_MEX_DIR');
run_dir = getenv('KSS_RUN_DIR');
if isempty(mex_output_dir) || isempty(run_dir) || ...
        ~startsWith(mex_output_dir, [run_dir filesep])
    error('KSS_MATLAB_MEX_DIR must be inside the KSS run directory.');
end
if ~isfolder(mex_output_dir)
    mkdir(mex_output_dir);
end
mex_setup_started = tic;
mex_names = {'adjacency_cmg','diagconjugate','forest_components', ...
    'graphprofile','laplacian2','perturbtril','splitforest', ...
    'update_groups','vpack'};
mex_source_dir = fullfile(kss_root, 'CMG', 'Source', 'Hierarchy');
for mex_index = 1:numel(mex_names)
    mex_source = fullfile(mex_source_dir, [mex_names{mex_index} '.c']);
    if exist(mex_source, 'file') ~= 2
        error('A checksum-bound MATLAB CMG MEX source is unavailable.');
    end
    mex('-silent', '-largeArrayDims', '-outdir', mex_output_dir, mex_source);
end
solver_include_dir = fullfile(kss_root, 'CMG', 'Include');
solver_source_dir = fullfile(kss_root, 'CMG', 'Source', 'Solver');
solver_gateway = fullfile(kss_root, 'CMG', 'MATLAB', 'Solver', ...
    'mx_d_preconditioner.c');
solver_names = {'vpv','vmv','vpvmv','vvmul','rmvec','ldl_solve', ...
    'sspmv','preconditioner'};
solver_sources = cell(1, numel(solver_names) + 1);
solver_sources{1} = solver_gateway;
for solver_index = 1:numel(solver_names)
    solver_sources{solver_index + 1} = fullfile(solver_source_dir, ...
        [solver_names{solver_index} '.c']);
end
mex('-silent', '-largeArrayDims', ['-I' solver_include_dir], ...
    '-outdir', mex_output_dir, '-output', 'mx_d_preconditioner', ...
    solver_sources{:});
addpath(mex_output_dir, '-begin');
rehash;
clear graphprofile
clear mx_d_preconditioner
graphprofile_path = which('graphprofile');
if exist('graphprofile', 'file') ~= 3 || ...
        ~startsWith(graphprofile_path, [mex_output_dir filesep])
    error('Run-local MATLAB CMG MEX compilation did not bind graphprofile.');
end
preconditioner_path = which('mx_d_preconditioner');
if exist('mx_d_preconditioner', 'file') ~= 3 || ...
        ~startsWith(preconditioner_path, [mex_output_dir filesep])
    error('Run-local MATLAB CMG MEX compilation did not bind the solver.');
end
mex_setup_seconds = toc(mex_setup_started);

imported = importdata(input_file);
if isstruct(imported)
    data = imported.data;
else
    data = imported;
end
if size(data, 2) ~= 4 || size(data, 1) < 1 || any(~isfinite(data), 'all')
    error('Prepared KSS input must be a finite four-column matrix.');
end
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

pool = gcp('nocreate');
if ~isempty(pool)
    delete(pool);
end
slots = str2double(getenv('NSLOTS'));
if ~isfinite(slots) || slots < 1
    slots = 4;
end
slots = min(4, floor(slots));
pool = parpool('local', slots, 'IdleTimeout', Inf);
pool_cleanup = onCleanup(@() delete(pool)); %#ok<NASGU>
rng(seed, 'twister');

if endsWith(detailed_file, '.csv', 'IgnoreCase', true)
    detailed_stub = detailed_file(1:end-4);
else
    detailed_stub = detailed_file;
end

started = tic;
controls = [];
leave_out_level = 'matches';
type_algorithm = 'JLA';
lincom_do = 0;
Z_lincom = [];
labels_lincom = [];
[firm_variance, covariance, worker_variance] = leave_out_KSS( ...
    outcome, worker, firm, controls, leave_out_level, type_algorithm, ...
    probes, lincom_do, Z_lincom, labels_lincom, detailed_stub);
command_seconds = toc(started);
total_variance = worker_variance + firm_variance + 2 * covariance;

target = {'worker'; 'firm'; 'covariance'; 'total'};
value = [worker_variance; firm_variance; covariance; total_variance];
label_column = repmat({label}, 4, 1);
source_column = repmat({source_commit}, 4, 1);
prepared_column = repmat({prepared_sha256}, 4, 1);
core_column = repmat({kss_core_sha256}, 4, 1);
cmg_column = repmat({matlab_cmg_sha256}, 4, 1);
cmg_mex_column = repmat({matlab_cmg_mex_sha256}, 4, 1);
cmg_solver_column = repmat({matlab_cmg_solver_sha256}, 4, 1);
matlab_version = repmat({version}, 4, 1);
seed_column = repmat(seed, 4, 1);
probes_column = repmat(probes, 4, 1);
seconds_column = repmat(command_seconds, 4, 1);
mex_setup_column = repmat(mex_setup_seconds, 4, 1);
workers_column = repmat(numel(unique(worker)), 4, 1);
firms_column = repmat(numel(unique(firm)), 4, 1);
rows_column = repmat(numel(outcome), 4, 1);
result = table(label_column, source_column, prepared_column, core_column, ...
    cmg_column, cmg_mex_column, cmg_solver_column, ...
    matlab_version, target, value, seed_column, probes_column, ...
    mex_setup_column, seconds_column, rows_column, workers_column, firms_column, ...
    'VariableNames', {'label','source_commit','prepared_sha256', ...
    'kss_core_sha256','matlab_cmg_sha256','matlab_cmg_mex_sha256', ...
    'matlab_cmg_solver_sha256','matlab_version','target','value','seed', ...
    'probes','mex_setup_seconds', ...
    'command_seconds','input_rows','input_workers','input_firms'});
writetable(result, output_file);
fprintf('VCKSS SEPARATIONS MATLAB PASS: %s\n', label);
end

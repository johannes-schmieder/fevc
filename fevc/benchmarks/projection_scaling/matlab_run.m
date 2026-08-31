function matlab_run()
% Maintained leave_out_COMPLETE JLA followed by maintained lincom_KSS.
output_dir = required_env('VPS_OUTPUT_DIR');
scratch_dir = required_env('VPS_SCRATCH_DIR');
input_csv = required_env('VPS_INPUT_CSV');
matlab_root = required_env('VPS_MATLAB_ROOT');
mex_dir = required_env('VPS_MEX_DIR');
phase_start = required_env('VPS_PHASE_START');
phase_end = required_env('VPS_PHASE_END');
source_commit = required_env('VPS_SOURCE_COMMIT');
rows_expected = required_integer('VPS_ROWS');
probes = required_integer('VPS_PROBES');
seed = required_integer('VPS_SEED');
assert_vps(ismember(rows_expected,[6000 24000 96000]),'Rows','Rows changed.');
assert_vps(probes==ceil(log2(rows_expected)/.01),'Probes','Probe rule changed.');

if ~license('test','Statistics_Toolbox')
    addpath(fullfile(fileparts(mfilename('fullpath')),'compat'));
end

addpath(fullfile(matlab_root,'codes'));
addpath(genpath(fullfile(matlab_root,'CMG')));
addpath(mex_dir,'-begin');
resolved_complete = which('leave_out_COMPLETE');
resolved_lincom = which('lincom_KSS');
assert_vps(strcmp(canonical_path(resolved_complete),canonical_path( ...
    fullfile(matlab_root,'codes','leave_out_COMPLETE.m'))), ...
    'Source','leave_out_COMPLETE resolution changed.');
assert_vps(strcmp(canonical_path(resolved_lincom),canonical_path( ...
    fullfile(matlab_root,'codes','lincom_KSS.m'))), ...
    'Source','lincom_KSS resolution changed.');

data = readtable(input_csv,'VariableNamingRule','preserve');
required = {'observation_key','worker','firm','period','y','z1','z2'};
assert_vps(isequal(data.Properties.VariableNames,required),'Input','Columns changed.');
y_input = double(data.y);
id_input = double(data.worker);
firm_input = double(data.firm);
assert_vps(numel(y_input)==rows_expected && all(isfinite(y_input)), ...
    'Input','Input rows changed.');
clear data

write_marker(phase_start,'START matlab');
command_start = tic;
parallel_dir = fullfile(scratch_dir,'parallel');
if ~isfolder(parallel_dir), mkdir(parallel_dir); end
cluster = parcluster('local');
cluster.JobStorageLocation = parallel_dir;
pool = parpool(cluster,4,'IdleTimeout',Inf);
maxNumCompThreads(1);
spmd, maxNumCompThreads(1); end
rng(seed,'twister');
spmd, rng(seed+spmdIndex,'twister'); end

stub = fullfile(scratch_dir,'maintained');
[~,~,~] = leave_out_COMPLETE(y_input,id_input,firm_input, ...
    'obs',[],0,0,0,2,1,0,'JLL',.01,stub);
state = load([stub '_after_step3.mat'],'y','id_old','firmid_old', ...
    'D','F','S','Lambda_P','N','J','NT');
assert_vps(state.NT==rows_expected,'Sample','Maintained sample changed.');
X = [state.D,state.F*state.S];
xx = X'*X;
Lchol = ichol(xx,struct('type','ict','droptol',1e-2,'diagcomp',.1));
[beta,fit_flag,fit_relres,fit_iterations] = pcg( ...
    xx,X'*state.y,1e-10,1000,Lchol,Lchol');
assert_vps(fit_flag==0 && fit_relres<=1e-10,'Fit','Grounded fit did not converge.');
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
delete(pool);

record = struct('schema','FEVC-PROJECTION-SCALING-MATLAB-V1', ...
    'status','PASS','source_commit',source_commit,'rows',rows_expected, ...
    'probes',probes,'seed',seed,'matlab_version',version, ...
    'upstream_commit','8b957ffeb10b8465a3584fceb0265cccc48379e1', ...
    'command_seconds',command_seconds,'fit_flag',fit_flag, ...
    'fit_relative_residual',fit_relres,'fit_iterations',fit_iterations, ...
    'b_cons',projection_b(1),'b_z1',lincom_b(1),'b_z2',lincom_b(2), ...
    'se_z1',lincom_se(1),'se_z2',lincom_se(2), ...
    'V_z1_z1',lincom_se(1)^2,'V_z2_z2',lincom_se(2)^2, ...
    'variance_proxy_min',min(sigma_i),'variance_proxy_max',max(sigma_i));
write_json(record,fullfile(output_dir,'result.json'));
write_marker(fullfile(output_dir,'application.pass'), ...
    sprintf('FEVC PROJECTION SCALING MATLAB PASS rows=%d',rows_expected));
fprintf('FEVC PROJECTION SCALING MATLAB PASS rows=%d\n',rows_expected);
end

function value = required_env(name)
value = getenv(name);
assert_vps(~isempty(value),'Environment',['Missing ' name '.']);
end

function value = required_integer(name)
value = str2double(required_env(name));
assert_vps(isfinite(value) && value>=1 && value==floor(value), ...
    'Environment',['Invalid ' name '.']);
end

function path = canonical_path(path)
path = char(java.io.File(path).getCanonicalPath());
end

function write_marker(path,value)
handle = fopen(path,'w');
assert_vps(handle>=0,'Marker','Could not create marker.');
fprintf(handle,'%s\n',value);
fclose(handle);
end

function write_json(value,path)
handle = fopen(path,'w');
assert_vps(handle>=0,'Output','Could not create JSON output.');
fprintf(handle,'%s\n',jsonencode(value));
fclose(handle);
end

function assert_vps(condition,code,message)
if ~condition
    error(['fevc:projectionScaling:' code],'%s',message);
end
end

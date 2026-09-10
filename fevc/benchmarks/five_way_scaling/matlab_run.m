function matlab_run()
% Maintained KSS MATLAB comparator with the registered common phase boundary.
input = need('FW_INPUT'); output = need('FW_OUTPUT'); root = need('FW_MATLAB_ROOT');
mexdir = need('FW_MEX_DIR'); phase_start = need('FW_PHASE_START');
phase_end = need('FW_PHASE_END'); algorithm = need('FW_ALGORITHM');
n = integer_env('FW_ROWS'); cores = integer_env('FW_CORES');
probes = integer_env('FW_PROBES'); seed = integer_env('FW_SEED');
assert(any(n==[960 7680 30720 122880 491520]) && any(cores==[1 2 4 8 14 28]));
addpath(fullfile(root,'codes')); addpath(genpath(fullfile(root,'CMG')));
addpath(mexdir,'-begin'); rehash;
assert(strcmp(canonical(which('leave_out_KSS')),canonical(fullfile(root,'codes','leave_out_KSS.m'))));
t = tic; data = readtable(input,'VariableNamingRule','preserve'); import_seconds = toc(t);
assert(isequal(data.Properties.VariableNames,{'observation_key','worker','firm','period','match','y'}));
worker=double(data.worker); firm=double(data.firm); y=double(data.y);
assert(numel(y)==n && size(unique([worker firm],'rows'),1)==n && ...
    numel(unique(worker))==n/3 && numel(unique(firm))==n/120 && all(isfinite(y)));
clear data
write_marker(phase_start,'START matlab'); phase_clock=tic;
pool=[];
try
    cluster=parcluster('local'); cluster.JobStorageLocation=need('FW_SCRATCH');
    pool=parpool(cluster,cores,'IdleTimeout',Inf); assert(pool.NumWorkers==cores);
    maxNumCompThreads(1); spmd, maxNumCompThreads(1); end
    rng(seed,'twister');
    if strcmp(algorithm,'exact'), method='exact'; else, method='JLA'; end
    detail=fullfile(need('FW_SCRATCH'),'detail');
    [firm_v,cov_v,worker_v]=leave_out_KSS(y,worker,firm,[], ...
        'matches',method,probes,0,[],[],detail);
    primary_seconds=toc(phase_clock); write_marker(phase_end,'END matlab');
    raw=[worker_v firm_v cov_v worker_v+firm_v+2*cov_v]; factor=(n-1)/n;
    normalized=raw*factor;
    assert(all(isfinite(raw)) && abs(normalized(4)-normalized(1)-normalized(2)-2*normalized(3))<=1e-10*(1+sum(abs(normalized))));
    delete(pool); pool=[];
    record=struct('schema','FEVC-FIVE-WAY-ROLE-V1','status','PASS','role','matlab', ...
        'algorithm',algorithm,'rows',n,'cores',cores,'probes',probes,'seed',seed, ...
        'import_seconds',import_seconds,'primary_seconds',primary_seconds, ...
        'estimator_seconds',primary_seconds,'raw_worker',raw(1),'raw_firm',raw(2), ...
        'raw_covariance',raw(3),'raw_total',raw(4),'normalization_factor',factor, ...
        'normalized_worker',normalized(1),'normalized_firm',normalized(2), ...
        'normalized_covariance',normalized(3),'normalized_total',normalized(4), ...
        'retained_rows',n,'rng_policy','MATLAB_TWISTER_CLIENT_UPSTREAM_PARALLEL_POLICY', ...
        'pool_workers',cores,'matlab_version',version);
    atomic_json(output,record);
    fprintf('FEVC_FIVE_WAY_ROLE_PASS matlab %s rows=%d cores=%d\n',algorithm,n,cores);
catch error
    if ~isempty(pool), try, delete(pool); catch, end, end
    if ~isfile(phase_end), try, write_marker(phase_end,'END matlab FAIL'); catch, end, end
    rethrow(error);
end
end
function value=need(name), value=getenv(name); assert(~isempty(value),['missing ' name]); end
function value=integer_env(name), value=str2double(need(name)); assert(isfinite(value)&&value==floor(value)); end
function value=canonical(path), value=char(java.io.File(path).getCanonicalPath()); end
function write_marker(path,text), f=fopen(path,'w'); assert(f>=0); fprintf(f,'%s\n',text); fclose(f); end
function atomic_json(path,value)
temporary=[path '.tmp']; f=fopen(temporary,'w'); assert(f>=0); fprintf(f,'%s\n',jsonencode(value)); fclose(f); movefile(temporary,path,'f');
end

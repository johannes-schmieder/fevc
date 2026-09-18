function paper_matlab()
input=getenv('LR_INPUT'); out=getenv('LR_OUTPUT');
meta=jsondecode(fileread(getenv('LR_META')));
threads=str2double(getenv('LR_THREADS')); seed=str2double(getenv('LR_SEED'));
assert(ismember(threads,[1 4 7 28]) && seed>=1);
data=readtable(input,'VariableNamingRule','preserve');
assert(isequal(data.Properties.VariableNames,{'observation_key','worker','firm','period','match','y'}));
assert(height(data)==meta.rows && all(isfinite(data.y)));
assert(isequal(data.observation_key,(1:meta.rows)'));
assert(numel(unique(data.worker))==meta.workers && numel(unique(data.firm))==meta.firms);
assert(size(unique([data.worker data.firm],'rows'),1)==meta.rows);
assert(numel(unique(data.match))==meta.rows);
y=data.y; worker=data.worker; firm=data.firm; clear data
assert(isempty(gcp('nocreate')),'Preexisting pool');
deletion=getenv('LR_DELETION');
if strcmp(deletion,'match'), level='matches'; else
    assert(strcmp(deletion,'observation')); level='obs';
end
marker(fullfile(out,'primary.started'));
primary=tic;
addpath(fullfile(getenv('LR_MATLAB_ROOT'),'codes'));
addpath(genpath(fullfile(getenv('LR_MATLAB_ROOT'),'CMG')));
addpath(getenv('LR_MEX_DIR'),'-begin');
assert(strcmp(which('leave_out_KSS'),fullfile(getenv('LR_MATLAB_ROOT'),'codes','leave_out_KSS.m')));
cluster=parcluster('local');
cluster.JobStorageLocation=getenv('LR_PARALLEL');
marker(fullfile(out,'pool.started'));
pool_timer=tic;
pool=parpool(cluster,threads,'IdleTimeout',Inf);
pool_seconds=toc(pool_timer);
marker(fullfile(out,'pool.ready'));
cleanup=onCleanup(@() delete(pool));
assert(pool.NumWorkers==threads);
maxNumCompThreads(1);
spmd
    maxNumCompThreads(1);
    rng(seed+spmdIndex,'twister');
    worker_before=rng;
    worker_thread_count=maxNumCompThreads;
end
rng(seed,'twister'); client_before=rng;
estimate=tic;
[vf,cv,vw]=leave_out_KSS(y,worker,firm,[],level,'JLA',200,0,[],[],fullfile(getenv('LR_PARALLEL'),'detail'));
estimator_seconds=toc(estimate);
primary_seconds=toc(primary);
marker(fullfile(out,'primary.ended'));
raw_targets=[vw vf cv vw+vf+2*cv];
normalization_factor=(meta.rows-1)/meta.rows;
targets=raw_targets*normalization_factor;
assert(all(isfinite(targets)));
client_after=rng;
spmd, worker_after=rng; end
workers=cell(1,threads);
for k=1:threads
    assert(worker_thread_count{k}==1);
    workers{k}=struct('index',k,'before',worker_before{k},'after',worker_after{k});
end
write_json(fullfile(out,'rng.json'),struct('client_before',client_before,'client_after',client_after,'workers',{workers}));
detail=readmatrix(fullfile(getenv('LR_PARALLEL'),'detail.csv'));
assert(size(detail,1)==meta.rows && size(detail,2)==4,'Retained sample changed');
assert(isequal(detail(:,2:3),[worker firm]),'Retained IDs or order changed');
assert(max(abs(detail(:,1)-y))<1e-12,'Retained outcomes changed');
result=struct('status','PASS','rows',meta.rows,'threads',threads,'probes',200,...
    'seed',seed,'primary_seconds',primary_seconds,'estimator_seconds',estimator_seconds,...
    'pool_seconds',pool_seconds,'targets',targets,'raw_targets',raw_targets,...
    'normalization_factor',normalization_factor,'version',version,...
    'sample_identity_verified',true,'retained_input_sha256',meta.sha256,...
    'deletion',deletion,'pool_workers',pool.NumWorkers,'worker_threads',1,...
    'finite_probe_equality_claim',false);
write_json(fullfile(out,'result.json'),result);
fprintf('LIGHT PAPER MATLAB PASS\n');
end
function marker(path)
fid=fopen(path,'w'); assert(fid>=0); fprintf(fid,'PASS\n'); fclose(fid);
end
function write_json(path,value)
fid=fopen(path,'w'); assert(fid>=0); fprintf(fid,'%s\n',jsonencode(value)); fclose(fid);
end

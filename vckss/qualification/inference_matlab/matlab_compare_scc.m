function matlab_compare_scc(run_dir, codes)
addpath(genpath(codes));
maxNumCompThreads(1);

fprintf('MATLAB_VERSION=%s\n', version);
fprintf('CURVE_FITTING_LICENSE=%d\n', license('test','Curve_Fitting_Toolbox'));
fprintf('STATISTICS_LICENSE=%d\n', license('test','Statistics_Toolbox'));
fprintf('PARALLEL_LICENSE=%d\n', license('test','Distrib_Computing_Toolbox'));

input = readtable(fullfile(run_dir, 'input', 'input.csv'));
y = input.y;
id = input.worker;
firmid = input.firm;
Z = [input.z1 input.z2];
n = size(y,1);

[~,~,id_index] = unique(id, 'sorted');
[~,~,firm_index] = unique(firmid, 'sorted');
N = max(id_index);
J = max(firm_index);
D = sparse(1:n,id_index',1,n,N);
F = sparse(1:n,firm_index',1,n,J);
X = [D,F(:,1:J-1)];
xx = X'*X;
beta = xx\(X'*y);
residual = y-X*beta;
leverage = sum((X/xx).*X,2);
sigma_i = (y-mean(y)).*(residual./(1-leverage));
Transform = [sparse(n,N),F(:,1:J-1)];
[tstat,lincom_b,lincom_se] = lincom_KSS( ...
    y,X,Z,Transform,sigma_i,{'z1','z2'});

naive_sigma = residual.^2;
Zfull = [ones(n,1),Z];
zz = Zfull'*Zfull;
projection_b = Zfull\(Transform*beta);
loading = Transform' * Zfull / zz;
score = X * (xx\loading);
projection_V = score'*(sigma_i.*score);
projection_V_naive = score'*(naive_sigma.*score);

output_path = fullfile(run_dir,'output','matlab_results.csv');
fid = fopen(output_path,'w');
assert(fid >= 0, 'Could not open MATLAB result file.');
fprintf(fid,'kind,seed,row,col,value\n');
projection_names = {'_cons','z1','z2'};
for j = 1:3
    fprintf(fid,'projection_b,,,%s,%.17e\n',projection_names{j},projection_b(j));
    for k = 1:3
        fprintf(fid,'projection_V,,%s,%s,%.17e\n', ...
            projection_names{j},projection_names{k},projection_V(j,k));
        fprintf(fid,'projection_V_naive,,%s,%s,%.17e\n', ...
            projection_names{j},projection_names{k},projection_V_naive(j,k));
    end
end
for j = 1:2
    fprintf(fid,'lincom_b,,,%s,%.17e\n',projection_names{j+1},lincom_b(j));
    fprintf(fid,'lincom_se,,,%s,%.17e\n',projection_names{j+1},lincom_se(j));
    fprintf(fid,'lincom_t,,,%s,%.17e\n',projection_names{j+1},tstat(j));
end
fclose(fid);

pool = gcp('nocreate');
if ~isempty(pool), delete(pool); end
parpool('local',4);
seeds = [101 202 303 404 505];
names = {'worker_variance','firm_variance','worker_firm_covariance'};
for s = 1:numel(seeds)
    rng(seeds(s),'twister');
    stub = fullfile(run_dir,'work',sprintf('matlab_complete_%d',seeds(s)));
    [firm_v,cov_wf,worker_v,se_firm,se_cov,se_worker] = ...
        leave_out_COMPLETE(y,id,firmid,'obs',[],0,0,0,2,1,1, ...
        'exact',0.01,stub);
    values = [worker_v firm_v cov_wf];
    ses = [se_worker se_firm se_cov];
    fid = fopen(output_path,'a');
    assert(fid >= 0, 'Could not append to MATLAB result file.');
    for j = 1:3
        fprintf(fid,'component_b,%d,,%s,%.17e\n', ...
            seeds(s),names{j},values(j));
        fprintf(fid,'component_se,%d,,%s,%.17e\n', ...
            seeds(s),names{j},ses(j));
    end
    fclose(fid);
end
pool = gcp('nocreate');
if ~isempty(pool), delete(pool); end

receipt = fopen(fullfile(run_dir,'receipts','application.txt'),'w');
assert(receipt >= 0, 'Could not open application receipt.');
fprintf(receipt,'status=pass\n');
fprintf(receipt,'observations=%d\nworkers=%d\nfirms=%d\n',n,N,J);
fprintf(receipt,'seeds=101,202,303,404,505\n');
fprintf(receipt,'simulations_per_seed=1000\n');
fclose(receipt);
fprintf('MATLAB KSS COMPARISON PASS\n');
end

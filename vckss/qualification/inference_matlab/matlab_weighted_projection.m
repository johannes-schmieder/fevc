function matlab_weighted_projection(outdir, codes, input_path)
addpath(genpath(codes));
maxNumCompThreads(4);

input = readtable(input_path);
y0 = input.y;
id0 = input.worker;
firm0 = input.firm;
z10 = input.z1;
z20 = input.z2;
frequency = 1 + mod(id0 + firm0, 3);
target_mass = 1 + mod(2*id0 + firm0, 4);

copy_index = repelem((1:height(input))', frequency);
y = y0(copy_index);
id = id0(copy_index);
firmid = firm0(copy_index);
z1 = z10(copy_index);
z2 = z20(copy_index);
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

output_path = fullfile(outdir,'matlab_weighted_results.csv');
fid = fopen(output_path,'w');
assert(fid >= 0, 'Could not open weighted MATLAB result file.');
fprintf(fid,'route,projection,kind,row,col,value\n');

projection_names = {'_cons','z1','z2'};
for projection_index = 1:2
    if projection_index == 1
        projection_name = 'firm_frequency';
        Z = [z10(copy_index),z20(copy_index)];
        Transform = [sparse(n,N),F(:,1:J-1)];
    else
        projection_name = 'worker_target';
        target_index = repelem((1:height(input))', target_mass);
        Z = [z10(target_index),z20(target_index)];
        target_workers = id0(target_index);
        [present, target_worker_index] = ismember(target_workers, unique(id0,'sorted'));
        assert(all(present));
        p = numel(target_index);
        target_D = sparse(1:p,target_worker_index',1,p,N);
        Transform = [target_D,sparse(p,J-1)];
    end

    [~,lincom_b,lincom_se] = lincom_KSS( ...
        y,X,Z,Transform,sigma_i,{'z1','z2'});

    Zfull = [ones(size(Z,1),1),Z];
    zz = Zfull'*Zfull;
    projection_b = Zfull\(Transform*beta);
    loading = Transform' * Zfull / zz;
    score = X * (xx\loading);
    projection_V = score'*(sigma_i.*score);
    projection_V_naive = score'*((residual.^2).*score);

    for j = 1:3
        fprintf(fid,'same_formula,%s,projection_b,,%s,%.17e\n', ...
            projection_name,projection_names{j},projection_b(j));
        for k = 1:3
            fprintf(fid,'same_formula,%s,projection_V,%s,%s,%.17e\n', ...
                projection_name,projection_names{j},projection_names{k}, ...
                projection_V(j,k));
            fprintf(fid,'same_formula,%s,projection_V_naive,%s,%s,%.17e\n', ...
                projection_name,projection_names{j},projection_names{k}, ...
                projection_V_naive(j,k));
        end
    end
    for j = 1:2
        fprintf(fid,'lincom_KSS,%s,projection_b,,%s,%.17e\n', ...
            projection_name,projection_names{j+1},lincom_b(j));
        fprintf(fid,'lincom_KSS,%s,projection_se,,%s,%.17e\n', ...
            projection_name,projection_names{j+1},lincom_se(j));
    end
end
fclose(fid);
fprintf('MATLAB WEIGHTED PROJECTION PASS\n');
end

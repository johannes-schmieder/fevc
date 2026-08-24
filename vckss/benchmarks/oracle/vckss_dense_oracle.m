function vckss_dense_oracle(output_dir, source_commit)
% Independent dense exact KSS oracle for the registered 24-row fixture.
% This clean-room calculation contains no LeaveOutTwoWay source code.

if nargin ~= 2 || strlength(string(source_commit)) < 7
    error('vckss:invalidInput','output_dir and source_commit are required');
end

worker = repelem((0:5)',4);
period = repmat((0:3)',6,1);
firm = [0;0;1;1; 0;2;2;1; 1;2;3;3; 2;3;0;0; ...
        3;1;1;2; 3;3;2;0];
match_id = [10;10;11;11; 20;21;21;22; 30;31;32;32; ...
            40;41;42;42; 50;51;51;52; 60;60;61;62];
noise = [.2;-.1;.1;-.2; -.2;.3;-.1;.1; .1;-.2;.2;-.1; ...
         -.1;.2;-.2;.1; .3;-.2;.1;-.2; -.2;.1;.2;-.1];
c1 = period - 1.5;
c2 = double(period == 2);
y = 1.5 + .3*worker - .2*firm + .4*c1 - .15*c2 + noise;

n = numel(y);
workers = max(worker)+1;
firms = max(firm)+1;
D = sparse((1:n)',worker+1,1,n,workers);
F = sparse((1:n)',firm+1,1,n,firms);
X = full([D,F(:,1:(firms-1)),c1,c2]);
information = X'*X;
A = information\eye(size(information));
beta = A*(X'*y);
residual = y-X*beta;

target_worker = full(D);
target_firm = full(F(:,1:(firms-1)));
target_firm = [target_firm,zeros(n,1)];
center = eye(n)/n - ones(n,n)/(n*n);
worker_map = [target_worker,zeros(n,firms-1+2)];
firm_map = [zeros(n,workers),target_firm(:,1:(firms-1)),zeros(n,2)];
Qw = worker_map'*center*worker_map;
Qf = firm_map'*center*firm_map;
Qc = .5*(worker_map'*center*firm_map + firm_map'*center*worker_map);
targets = {Qw,Qf,Qc};

plugin = zeros(1,4);
correction = zeros(1,4);
for k = 1:3
    plugin(k) = beta'*targets{k}*beta;
end
groups = unique(match_id,'stable');
for g = 1:numel(groups)
    idx = find(match_id == groups(g));
    Xg = X(idx,:);
    maker = eye(numel(idx))-Xg*A*Xg';
    deleted_residual = maker\residual(idx);
    for k = 1:3
        bias_block = Xg*A*targets{k}*A*Xg';
        correction(k) = correction(k) + y(idx)'*bias_block*deleted_residual;
    end
end
plugin(4) = plugin(1)+plugin(2)+2*plugin(3);
correction(4) = correction(1)+correction(2)+2*correction(3);
corrected = plugin-correction;

target = ["worker_variance";"firm_variance"; ...
          "worker_firm_covariance";"total_variance"];
source = repmat(string(source_commit),4,1);
oracle = repmat("independent_dense_matlab",4,1);
result = table(source,oracle,target,plugin',correction',corrected', ...
    'VariableNames',{'source_commit','oracle','target','plugin', ...
    'correction','corrected'});
writetable(result,fullfile(output_dir,'matlab_oracle.csv'));

expected_plugin = [.23886949630417731,.029429019126843384, ...
                   -.026942626261056212,.21441326290890825];
expected_correction = [-.019250119561150449,-.0078437874259815812, ...
                       -.0043493658055976927,-.035792638598327396];
if max(abs(plugin-expected_plugin)) > 2e-10 || ...
        max(abs(correction-expected_correction)) > 2e-9
    error('vckss:oracleMismatch','dense MATLAB oracle missed registered values');
end
fprintf('VCKSS MATLAB ORACLE PASS: %s\n',source_commit);
end

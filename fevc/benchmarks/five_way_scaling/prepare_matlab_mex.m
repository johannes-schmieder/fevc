function prepare_matlab_mex(matlab_root,output_dir)
if nargin~=2 || ~isfolder(matlab_root) || isfolder(output_dir) || isfile(output_dir), error('invalid arguments'); end
mkdir(output_dir);
names={'adjacency_cmg','diagconjugate','forest_components','graphprofile','laplacian2','perturbtril','splitforest','update_groups','vpack'};
source_dir=fullfile(matlab_root,'CMG','Source','Hierarchy');
for index=1:numel(names), mex('-silent','-largeArrayDims','-outdir',output_dir,fullfile(source_dir,[names{index} '.c'])); end
include_dir=fullfile(matlab_root,'CMG','Include'); solver_dir=fullfile(matlab_root,'CMG','Source','Solver');
gateway=fullfile(matlab_root,'CMG','MATLAB','Solver','mx_d_preconditioner.c');
solver_names={'vpv','vmv','vpvmv','vvmul','rmvec','ldl_solve','sspmv','preconditioner'}; sources=cell(1,numel(solver_names)+1); sources{1}=gateway;
for index=1:numel(solver_names), sources{index+1}=fullfile(solver_dir,[solver_names{index} '.c']); end
mex('-silent','-largeArrayDims',['-I' include_dir],'-outdir',output_dir,'-output','mx_d_preconditioner',sources{:});
fprintf('FEVC_FIVE_WAY_MATLAB_MEX_PASS\n');
end

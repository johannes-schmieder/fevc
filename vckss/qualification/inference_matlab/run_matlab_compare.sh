#!/bin/bash -l
#$ -P welfgr
#$ -N vckss_infmat
#$ -pe omp 4
#$ -l h_rt=00:30:00
#$ -l mem_per_core=4G
#$ -j y
#$ -m n

set -euo pipefail

run_dir="$1"
matlab_root="$2"

module load matlab/2024b
export OMP_NUM_THREADS=1
export MKL_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1

hostname > "$run_dir/receipts/hostname.txt"
env | sort > "$run_dir/receipts/environment.txt"
sha256sum \
  "$run_dir/input/input.csv" \
  "$run_dir/output/vckss_results.csv" \
  "$run_dir/code/matlab_compare_scc.m" \
  "$matlab_root/codes/leave_out_COMPLETE.m" \
  "$matlab_root/codes/lincom_KSS.m" \
  "$matlab_root/codes/leave_out_KSS.m" \
  > "$run_dir/receipts/source_hashes.sha256"

matlab -batch "addpath('$run_dir/code'); matlab_compare_scc('$run_dir','$matlab_root')"

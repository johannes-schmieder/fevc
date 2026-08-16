#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf '%s\n' \
    "usage: submit_prod_dag.sh RUN_DIR BUNDLE_DIR BUNDLE_SHA DATA_MANIFEST PHASE [--authorize-production KSS-PROD-1]" >&2
  exit 198
}
(( $# >= 5 )) || usage
run_dir=$1
bundle_dir=$2
bundle_sha=$3
data_manifest=$4
phase=$5
shift 5
case "$run_dir" in /projectnb/welfgr/kss-bc/runs/*) ;; *) usage ;; esac
case "$bundle_dir" in /projectnb/welfgr/kss-bc/bundles/*) ;; *) usage ;; esac
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]] || usage
[[ "$phase" =~ ^(preflight|calibration|production)$ ]] || usage

reject_qsub_value() {
  local label=$1 value=$2
  if [[ "$value" == *','* || "$value" == *$'\n'* || "$value" == *$'\r'* ]]; then
    printf '%s\n' "unsafe comma/newline in qsub value: $label" >&2
    exit 198
  fi
}
reject_qsub_value run_dir "$run_dir"
reject_qsub_value bundle_dir "$bundle_dir"
reject_qsub_value bundle_sha "$bundle_sha"
reject_qsub_value data_manifest "$data_manifest"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
test -s "$data_manifest"
production_authorized=0
if (( $# == 2 )) && [[ "$1" == --authorize-production && "$2" == KSS-PROD-1 ]]; then
  production_authorized=1
elif (( $# != 0 )); then
  usage
fi
if [[ "$phase" == production && "$production_authorized" != 1 ]]; then
  printf '%s\n' "production phase requires --authorize-production KSS-PROD-1" >&2
  exit 198
fi

source_dir="$bundle_dir/source"
plan="$source_dir/kss_bc/benchmarks/prod_experiments.tsv"
runner="$source_dir/kss_bc/benchmarks/scc/run_prod_stage.sge"
test -s "$plan"
test -s "$runner"
# SCC's unversioned Python 3 is 3.6 and cannot parse the bound validators.
# Bind a supported central module instead of inheriting the caller's state.
module load python3/3.12.4
command -v python3 >/dev/null
python3 "$source_dir/kss_bc/benchmarks/validate_prod_scc.py" \
  --plan "$plan" --static
source_commit=$(tr -d '[:space:]' < "$source_dir/SOURCE_COMMIT.txt")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
reject_qsub_value source_dir "$source_dir"
reject_qsub_value source_commit "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
mkdir -p "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct" \
  "$run_dir/input" "$run_dir/validation"

# Freeze the exact raw-input manifest once. Later phases must present the same
# bytes; no prepared data or MATLAB-derived retained set is accepted as input.
manifest_sha=$(sha256sum "$data_manifest" | awk '{print $1}')
[[ "$manifest_sha" =~ ^[0-9a-f]{64}$ ]]
frozen_manifest="$run_dir/input/data_manifest.tsv"
reject_qsub_value frozen_manifest "$frozen_manifest"
if [[ -e "$frozen_manifest" ]]; then
  test "$(sha256sum "$frozen_manifest" | awk '{print $1}')" = "$manifest_sha"
  test "$(tr -d '[:space:]' < "$run_dir/input/data_manifest.sha256")" = "$manifest_sha"
else
  [[ "$phase" == preflight ]] || {
    printf '%s\n' "only preflight may freeze the run data manifest" >&2
    exit 198
  }
  cp "$data_manifest" "$frozen_manifest"
  printf '%s\n' "$manifest_sha" > "$run_dir/input/data_manifest.sha256"
  chmod a-w "$frozen_manifest" "$run_dir/input/data_manifest.sha256"
fi

declare -A wage_path wage_sha sample_mode max_workers separations_commit
declare -A matlab_detail matlab_sha job_ids plan_phase
header=$(head -n 1 "$frozen_manifest")
test "$header" = $'dataset\twage_input_dta\twage_input_sha256\tsample_mode\tmax_workers\tseparations_commit\tmatlab_detail\tmatlab_detail_sha256'
while IFS=$'\t' read -r dataset raw_path raw_sha mode maximum sep_commit \
    detail detail_sha extra; do
  [[ "$dataset" == dataset ]] && continue
  [[ -z "$dataset" ]] && continue
  [[ -z "${extra:-}" ]]
  [[ "$dataset" =~ ^(cz18|cz24|cz25)$ ]]
  case "$raw_path" in /projectnb/welfgr/*) ;; *) exit 198 ;; esac
  [[ "$raw_sha" =~ ^[0-9a-f]{64}$ ]]
  [[ "$mode" =~ ^(small|full)$ && "$maximum" =~ ^[0-9]+$ ]]
  if [[ "$mode" == full ]]; then (( maximum == 0 )); else (( maximum >= 100 )); fi
  [[ "$sep_commit" =~ ^[0-9a-f]{40}$ ]]
  if [[ "$dataset" =~ ^cz(24|25)$ ]]; then
    case "$detail" in /projectnb/welfgr/*) ;; *) exit 198 ;; esac
    [[ "$detail_sha" =~ ^[0-9a-f]{64}$ ]]
  else
    [[ "$detail" == - && "$detail_sha" == - ]]
  fi
  wage_path[$dataset]=$raw_path
  wage_sha[$dataset]=$raw_sha
  sample_mode[$dataset]=$mode
  max_workers[$dataset]=$maximum
  separations_commit[$dataset]=$sep_commit
  matlab_detail[$dataset]=$detail
  matlab_sha[$dataset]=$detail_sha
done < "$frozen_manifest"
test "${#wage_path[@]}" -eq 3

require_phase_pass() {
  local prior=$1 pass="$run_dir/validation/$1.pass" evidence_manifest \
    recorded_evidence_sha actual_evidence_sha
  test -s "$pass"
  grep -Fx "status=PASS" "$pass"
  grep -Fx "phase=$prior" "$pass"
  grep -Fx "bundle_sha256=$bundle_sha" "$pass"
  grep -Fx "source_commit=$source_commit" "$pass"
  grep -Fx "data_manifest_sha256=$manifest_sha" "$pass"
  evidence_manifest="$run_dir/validation/$prior.evidence.sha256"
  test -s "$evidence_manifest"
  recorded_evidence_sha=$(awk -F= '
    $1=="evidence_manifest_sha256" {count++; value=$2}
    END {if(count != 1) exit 1; print value}
  ' "$pass")
  [[ "$recorded_evidence_sha" =~ ^[0-9a-f]{64}$ ]]
  actual_evidence_sha=$(sha256sum "$evidence_manifest" | awk '{print $1}')
  test "$recorded_evidence_sha" = "$actual_evidence_sha"
  (
    cd "$run_dir"
    sha256sum -c "validation/$prior.evidence.sha256"
  )
}

validate_prior_phase() {
  local prior=$1
  python3 "$source_dir/kss_bc/benchmarks/validate_prod_scc.py" \
    --plan "$plan" --run-dir "$run_dir" --bundle-sha "$bundle_sha" \
    --source-commit "$source_commit" --data-manifest "$frozen_manifest" \
    --phase "$prior"
  require_phase_pass "$prior"
}
if [[ "$phase" == calibration ]]; then
  validate_prior_phase preflight
elif [[ "$phase" == production ]]; then
  validate_prior_phase calibration
fi

selection="$run_dir/experiments/calibration_selector/calibration_selection.csv"
selected_route= selected_batch= selected_processors= selected_memory= selected_timeout=
if [[ "$phase" == production ]]; then
  test -s "$selection"
  selection_sha=$(sha256sum "$selection" | awk '{print $1}')
  grep -Fx "selection_sha256=$selection_sha" "$run_dir/validation/calibration.pass"
  IFS=$'\t' read -r selected_route selected_batch selected_processors \
      selected_memory selected_timeout < <(
    awk -F, '
      NR==1 {for(i=1;i<=NF;i++) h[$i]=i; next}
      NR==2 {print $h["preconditioner"] "\t" $h["batch"] "\t" $h["processors"] "\t" $h["memory_gib"] "\t" $h["timeout_seconds"]}
    ' "$selection"
  )
  [[ "$selected_route" =~ ^(auto|diagonal|cmg)$ ]]
  [[ "$selected_batch" =~ ^[0-9]+$ && "$selected_processors" =~ ^(4|8)$ ]]
  [[ "$selected_memory" =~ ^([1-9]|[1-4][0-9]|5[0-6])$ ]]
  [[ "$selected_timeout" =~ ^[0-9]+$ ]]
  (( selected_timeout <= 5400 ))
fi

ledger="$run_dir/submissions/ledger.tsv"
if [[ ! -e "$ledger" ]]; then
  printf '%s\n' $'experiment_id\tjob_id\tdirect_dependencies\tbundle_sha256\tdata_manifest_sha256\tsubmitted_utc\tresource_request' > "$ledger"
fi

# Only jobs submitted in this invocation belong in hold_jid. Dependencies from
# an accepted earlier phase are enforced by immutable phase certificates and
# by the wrapper's pass-marker/qacct checks; retired SGE job IDs are not held.
while IFS=$'\t' read -r experiment row_phase _; do
  [[ "$experiment" == experiment_id ]] && continue
  [[ -z "$experiment" ]] && continue
  plan_phase[$experiment]=$row_phase
done < "$plan"

while IFS=$'\t' read -r experiment row_phase stage dataset stata_version \
    processors memory_gib probes batch route temperature repetitions depends extra; do
  [[ "$experiment" == experiment_id ]] && continue
  [[ -z "$experiment" ]] && continue
  [[ -z "${extra:-}" ]]
  [[ "$row_phase" == "$phase" ]] || continue
  if [[ -s "$run_dir/submissions/$experiment.job_id" ]]; then
    job_ids[$experiment]=$(tr -d '[:space:]' < "$run_dir/submissions/$experiment.job_id")
    continue
  fi
  if [[ "$processors" == 0 ]]; then processors=$selected_processors; fi
  if [[ "$memory_gib" == selected ]]; then memory_gib=$selected_memory; fi
  if [[ "$batch" == selected ]]; then batch=$selected_batch; fi
  if [[ "$route" == selected ]]; then route=$selected_route; fi
  timeout=3600
  case "$stage" in
    bundle_smoke|install_auto|install_cmg|license) timeout=600 ;;
    hierarchy_stress|sample_compare) timeout=1800 ;;
    # The first successful real CZ18 preflight took 1,490 command seconds.
    # Bind the retry to ceil(1.25*1490+120), rounded up to 2,100 seconds,
    # rather than leaving only a contention-sensitive five-minute margin.
    cz18_preflight) timeout=2100 ;;
    selector|calibration_selector) timeout=900 ;;
    prepare|fixed|calibration) timeout=3600 ;;
    full|stress2x) timeout=5400 ;;
  esac
  if [[ "$row_phase" == production ]]; then timeout=$selected_timeout; fi
  if [[ "$stage" == stress2x ]]; then
    timeout=$(( 2 * selected_timeout ))
    (( timeout < 1800 )) && timeout=1800
    (( timeout > 10800 )) && timeout=10800
  fi
  hard_seconds=$(( timeout + 600 ))
  printf -v hard_runtime '%02d:%02d:%02d' \
    $(( hard_seconds / 3600 )) $(( (hard_seconds % 3600) / 60 )) \
    $(( hard_seconds % 60 ))
  mem_per_core=$(( (memory_gib + processors - 1) / processors ))
  dependencies=()
  if [[ "$depends" != - ]]; then
    IFS=, read -ra dependency_names <<< "$depends"
    for dependency in "${dependency_names[@]}"; do
      if [[ -z "${job_ids[$dependency]:-}" ]]; then
        test -s "$run_dir/submissions/$dependency.job_id"
        job_ids[$dependency]=$(tr -d '[:space:]' < "$run_dir/submissions/$dependency.job_id")
      fi
      [[ "${job_ids[$dependency]}" =~ ^[0-9]+([.][0-9:-]+)?$ ]]
      if [[ "${plan_phase[$dependency]:?missing dependency phase}" == "$phase" ]]; then
        dependencies+=("${job_ids[$dependency]%%.*}")
      fi
    done
  fi
  if [[ "$stage" == calibration ]]; then
    # Every calibration repetition is an independently schedulable job.  The
    # accepted preflight certificate and wrapper dependency bind the shared
    # input; no same-phase hold may serialize or pair timing cells.
    [[ "$depends" == cz18_preflight ]]
    (( ${#dependencies[@]} == 0 ))
    [[ "$repetitions" == 1 ]]
  fi
  hold_args=()
  dependency_job_csv=-
  if (( ${#dependencies[@]} )); then
    dependency_job_csv=$(IFS=,; printf '%s' "${dependencies[*]}")
    hold_args=(-hold_jid "$dependency_job_csv")
  fi

  raw_path=- input_hash=- mode=- maximum=- sep_commit=- detail=- detail_hash=-
  if [[ "$dataset" != synthetic ]]; then
    input_hash=${wage_sha[$dataset]:?missing wage hash for $dataset}
    sep_commit=${separations_commit[$dataset]:?missing Separations commit for $dataset}
    if [[ "$stage" == prepare ]]; then
      raw_path=${wage_path[$dataset]}
      mode=${sample_mode[$dataset]}
      maximum=${max_workers[$dataset]}
    fi
    if [[ "$stage" == sample_compare ]]; then
      detail=${matlab_detail[$dataset]}
      detail_hash=${matlab_sha[$dataset]}
    fi
  fi
  dependency_environment=${depends//,/:}
  for qsub_pair in \
      "experiment=$experiment" "stage=$stage" "dataset=$dataset" \
      "stata_version=$stata_version" "processors=$processors" \
      "memory_gib=$memory_gib" "probes=$probes" "batch=$batch" \
      "route=$route" "temperature=$temperature" \
      "repetitions=$repetitions" "timeout=$timeout" \
      "dependencies=$dependency_environment" "raw_path=$raw_path" \
      "input_hash=$input_hash" "sample_mode=$mode" \
      "maximum=$maximum" "separations_commit=$sep_commit" \
      "matlab_detail=$detail" "matlab_detail_hash=$detail_hash"; do
    reject_qsub_value "${qsub_pair%%=*}" "${qsub_pair#*=}"
  done
  environment="KSS_RUN_DIR=$run_dir,KSS_BUNDLE_DIR=$bundle_dir,KSS_BUNDLE_SHA256=$bundle_sha,KSS_DATA_MANIFEST_SHA256=$manifest_sha,KSS_EXPERIMENT_ID=$experiment,KSS_STAGE=$stage,KSS_DATASET=$dataset,KSS_STATA_VERSION=$stata_version,KSS_PROCESSORS=$processors,KSS_MEMORY_GIB=$memory_gib,KSS_PROBES=$probes,KSS_BATCH=$batch,KSS_PRECONDITIONER=$route,KSS_TEMPERATURE=$temperature,KSS_REPETITIONS=$repetitions,KSS_TIMEOUT_SECONDS=$timeout,KSS_DEPENDENCY_EXPERIMENTS=$dependency_environment,KSS_WAGE_INPUT_DTA=$raw_path,KSS_WAGE_INPUT_SHA256=$input_hash,KSS_SAMPLE_MODE=$mode,KSS_MAX_WORKERS=$maximum,KSS_SEPARATIONS_COMMIT=$sep_commit,KSS_MATLAB_DETAIL=$detail,KSS_MATLAB_DETAIL_SHA256=$detail_hash"
  raw_id=$(qsub -terse -P welfgr -pe omp "$processors" \
    -l "h_rt=$hard_runtime" -l "mem_per_core=${mem_per_core}G" -j y \
    -o "$run_dir/logs/$experiment.stdout.txt" "${hold_args[@]}" \
    -v "$environment" "$runner")
  [[ "$raw_id" =~ ^[0-9]+([.][0-9:-]+)?$ ]]
  job_ids[$experiment]=$raw_id
  printf '%s\n' "$raw_id" > "$run_dir/submissions/$experiment.job_id"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$experiment" "$raw_id" \
    "$dependency_job_csv" "$bundle_sha" "$manifest_sha" \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    "slots=$processors,mem_per_core=${mem_per_core}G,timeout=$timeout,h_rt=$hard_runtime" >> "$ledger"
  printf '%s %s\n' "$experiment" "$raw_id"
done < "$plan"

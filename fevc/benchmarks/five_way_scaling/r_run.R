suppressPackageStartupMessages({library(data.table); library(LeaveOutKSS); library(foreach); library(doParallel); library(jsonlite)})
need <- function(name) { value <- Sys.getenv(name); if (!nzchar(value)) stop(paste("missing",name)); value }
integer_env <- function(name) { value <- as.integer(need(name)); if (is.na(value)) stop(name); value }
marker <- function(path,text) writeLines(text,path,useBytes=TRUE)
atomic_json <- function(path,value) { temporary <- paste0(path,".tmp"); write_json(value,temporary,auto_unbox=TRUE,digits=17,pretty=TRUE); file.rename(temporary,path) }
input <- need("FW_INPUT"); output <- need("FW_OUTPUT"); phase_start <- need("FW_PHASE_START"); phase_end <- need("FW_PHASE_END")
algorithm <- need("FW_ALGORITHM"); n <- integer_env("FW_ROWS"); cores <- integer_env("FW_CORES"); probes <- integer_env("FW_PROBES"); seed <- integer_env("FW_SEED")
stopifnot(n %in% c(960L,7680L,30720L,122880L,491520L), cores %in% c(1L,2L,4L,8L,14L,28L))
started <- proc.time()[["elapsed"]]; data <- fread(input); import_seconds <- proc.time()[["elapsed"]]-started
stopifnot(identical(names(data),c("observation_key","worker","firm","period","match","y")),nrow(data)==n,uniqueN(data,c("worker","firm"))==n,uniqueN(data$worker)==n/3,uniqueN(data$firm)==n/120,all(is.finite(data$y)))
namespace <- asNamespace("LeaveOutKSS"); imports <- parent.env(namespace)
stopifnot(identical(environmentName(imports),"imports:LeaveOutKSS"))
original_detect <- get("detectCores",envir=imports,inherits=FALSE)
unlockBinding("detectCores",imports); assign("detectCores",function(...) cores+1L,envir=imports); lockBinding("detectCores",imports)
on.exit({try(stopImplicitCluster(),silent=TRUE); unlockBinding("detectCores",imports); assign("detectCores",original_detect,envir=imports); lockBinding("detectCores",imports)},add=TRUE)
marker(phase_start,"START r"); phase_clock <- proc.time()[["elapsed"]]
result <- leave_out_KSS(data$y,data$worker,data$firm,leave_out_level="matches",type_algorithm=if (algorithm=="exact") "exact" else "JLA",simulations_JLA=max(1L,probes),paral=TRUE,Cd=seed,progress=FALSE)
primary_seconds <- proc.time()[["elapsed"]]-phase_clock; marker(phase_end,"END r")
workers_registered <- foreach::getDoParWorkers(); stopifnot(workers_registered==cores)
worker_pids <- foreach::foreach(index=seq_len(cores),.combine=c) %dopar% Sys.getpid()
worker_pids <- unique(as.integer(worker_pids)); stopifnot(length(worker_pids)==cores)
est <- result$estimates$bias_corrected
raw <- c(worker=unname(est[["variance_person_effects"]]),firm=unname(est[["variance_firm_effects"]]),covariance=unname(est[["covariance_firm_person_effects"]]))
raw <- c(raw,total=raw[["worker"]]+raw[["firm"]]+2*raw[["covariance"]]); stopifnot(all(is.finite(raw)))
factor <- (n-1)/n; normalized <- raw*factor
record <- list(schema="FEVC-FIVE-WAY-ROLE-V1",status="PASS",role="r",algorithm=algorithm,rows=n,cores=cores,probes=probes,seed=seed,import_seconds=import_seconds,primary_seconds=primary_seconds,estimator_seconds=result$elapsed_seconds,raw_worker=raw[["worker"]],raw_firm=raw[["firm"]],raw_covariance=raw[["covariance"]],raw_total=raw[["total"]],normalization_factor=factor,normalized_worker=normalized[["worker"]],normalized_firm=normalized[["firm"]],normalized_covariance=normalized[["covariance"]],normalized_total=normalized[["total"]],retained_rows=n,workers_registered=workers_registered,worker_pids=worker_pids,rng_policy=if (algorithm=="exact") "NONE_EXACT" else "UPSTREAM_SET_SEED_WITH_PARALLEL_FOREACH",worker_adapter="BENCHMARK_IMPORTS_DETECTCORES_RETURNS_CORES_PLUS_ONE")
atomic_json(output,record); cat(sprintf("FEVC_FIVE_WAY_ROLE_PASS r %s rows=%d cores=%d\n",algorithm,n,cores))

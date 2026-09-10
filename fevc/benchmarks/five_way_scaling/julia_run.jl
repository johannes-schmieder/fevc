using CSV, DataFrames, LinearAlgebra, Random, VarianceComponentsHDFE

need(name) = (value=get(ENV,name,""); isempty(value) && error("missing $name"); value)
integer_env(name) = parse(Int,need(name))
function marker(path,text)
    open(path,"w") do io; println(io,text); end
end
function json_string(value)
    value isa AbstractString && return "\"" * replace(value,"\\"=>"\\\\","\""=>"\\\"") * "\""
    value isa Bool && return value ? "true" : "false"
    return string(value)
end
function atomic_json(path, values)
    temporary=path*".tmp"
    open(temporary,"w") do io
        print(io,"{")
        for (index,(key,value)) in enumerate(values)
            index>1 && print(io,",")
            print(io,json_string(key),":",json_string(value))
        end
        println(io,"}")
    end
    mv(temporary,path;force=true)
end

input=need("FW_INPUT"); output=need("FW_OUTPUT"); phase_start=need("FW_PHASE_START")
phase_end=need("FW_PHASE_END"); algorithm=need("FW_ALGORITHM")
n=integer_env("FW_ROWS"); cores=integer_env("FW_CORES"); probes=integer_env("FW_PROBES"); seed=integer_env("FW_SEED")
n in (960,7680,30720,122880,491520) || error("row grid changed")
cores in (1,2,4,8,14,28) || error("core grid changed")
Threads.nthreads()==cores || error("Julia thread count changed")
BLAS.set_num_threads(1); BLAS.get_num_threads()==1 || error("BLAS thread cap failed")
started=time_ns(); data=CSV.read(input,DataFrame); import_seconds=(time_ns()-started)/1e9
names(data)==["observation_key","worker","firm","period","match","y"] || error("columns changed")
size(data,1)==n || error("rows changed")
length(unique(zip(data.worker,data.firm)))==n || error("matches not unique")
length(unique(data.worker))==n÷3 || error("workers changed")
length(unique(data.firm))==n÷120 || error("firms changed")
y=Vector{Float64}(data.y); worker=Vector{Int}(data.worker); firm=Vector{Int}(data.firm); data=nothing; GC.gc()
settings = algorithm=="exact" ? VCHDFESettings(leverage_algorithm=ExactAlgorithm(),first_id_effects=true,cov_effects=true,leave_out_level="match",print_level=0) : VCHDFESettings(leverage_algorithm=JLAAlgorithm(num_simulations=probes),first_id_effects=true,cov_effects=true,leave_out_level="match",print_level=0)
Random.seed!(seed); marker(phase_start,"START julia"); phase_clock=time_ns()
subset=get_leave_one_out_set(y,worker,firm,settings,nothing)
length(subset.obs)==n || error("Julia changed retained sample")
result=leave_out_estimation(subset.y,subset.first_id,subset.second_id,subset.controls,settings)
primary_seconds=(time_ns()-phase_clock)/1e9; marker(phase_end,"END julia")
raw=[result.θ_first,result.θ_second,result.θCOV,result.θ_first+result.θ_second+2*result.θCOV]
all(isfinite,raw) || error("nonfinite targets")
factor=(n-1)/n; normalized=raw.*factor
abs(normalized[4]-normalized[1]-normalized[2]-2normalized[3])<=1e-10*(1+sum(abs,normalized)) || error("target identity")
values=[
 "schema"=>"FEVC-FIVE-WAY-ROLE-V1","status"=>"PASS","role"=>"julia",
 "algorithm"=>algorithm,"rows"=>n,"cores"=>cores,"probes"=>probes,"seed"=>seed,
 "import_seconds"=>import_seconds,"primary_seconds"=>primary_seconds,"estimator_seconds"=>primary_seconds,
 "raw_worker"=>raw[1],"raw_firm"=>raw[2],"raw_covariance"=>raw[3],"raw_total"=>raw[4],
 "normalization_factor"=>factor,"normalized_worker"=>normalized[1],"normalized_firm"=>normalized[2],
 "normalized_covariance"=>normalized[3],"normalized_total"=>normalized[4],"retained_rows"=>n,
 "rng_policy"=>(algorithm=="exact" ? "NONE_EXACT" : "UPSTREAM_THREAD_LOCAL_MERSENNE_TWISTER")]
atomic_json(output,values)
println("FEVC_FIVE_WAY_ROLE_PASS julia $algorithm rows=$n cores=$cores")

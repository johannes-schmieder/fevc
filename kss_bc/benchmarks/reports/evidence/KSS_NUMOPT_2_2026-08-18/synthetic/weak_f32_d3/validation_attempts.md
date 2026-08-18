# Validation attempts

1. The validator frozen in runtime bundle
   `d1d824a592381ba39403b587a3bcdc6180a17d338e4b42d5a229f4c5b4ec0e5f`
   rejected the successful job because it required literal
   `granted_pe=omp`; qacct reported SCC's queue-specific `omp16` name for the
   submitted `-pe omp 16` request. No scientific or application assertion
   failed.
2. Validator-only commit
   `187c298` accepts `ompN` only when `N` equals both the frozen task slot
   request and qacct slot count. That committed validator accepted job
   7214618 and wrote `validation.json` from the collected immutable receipts.

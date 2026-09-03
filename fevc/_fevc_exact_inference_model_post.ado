program define _fevc_exact_inference_model_post, eclass
    version 18.0
    args inference supplied
    if "`inference'"!="none" {
        ereturn local inference_model "target_lowess"
        ereturn local inference_kss_scope                         ///
            "target-specific MATLAB approximation; not the paper's unrestricted variance-product construction"
    }
    ereturn scalar inference_model_option_supplied = `supplied'
end

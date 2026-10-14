*! Forwards internal public-interface calls to fevc_rust and preserves r().

program define fevc__rust_public_call, rclass
    version 18.0
    fevc_rust `0'
    return add
end

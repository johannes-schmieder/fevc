// SPDX-License-Identifier: GPL-3.0-only

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    const TEMPLATE: &str = "src/cmg_impl.rs.in";
    println!("cargo:rerun-if-changed={TEMPLATE}");

    let template = fs::read_to_string(TEMPLATE).expect("read CMG Rust template");
    let mut generated = template
        .lines()
        .map(|line| {
            line.strip_prefix("//!")
                .map_or(line.to_owned(), |rest| format!("//{rest}"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    generated.push('\n');

    let pointer_declaration = "let mut incidence_ptr = Vec::with_capacity(vertices + 1);";
    let typed_pointer_declaration =
        "let mut incidence_ptr = Vec::<u64>::with_capacity(vertices + 1);";
    assert_eq!(
        generated.matches(pointer_declaration).count(),
        1,
        "CMG template CSR declaration changed unexpectedly"
    );
    generated = generated.replace(pointer_declaration, typed_pointer_declaration);

    let pointer_origin = "incidence_ptr.push(0);";
    let typed_pointer_origin = "incidence_ptr.push(0_u64);";
    assert_eq!(
        generated.matches(pointer_origin).count(),
        1,
        "CMG template CSR origin changed unexpectedly"
    );
    generated = generated.replace(pointer_origin, typed_pointer_origin);

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set"))
        .join("cmg_impl_generated.rs");
    fs::write(output, generated).expect("write generated CMG Rust source");
}

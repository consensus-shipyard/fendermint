// Copyright 2021-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::io::Write;
use std::path::{Path, PathBuf};

/// Generate Rust bindings from the IPC Solidity Actors ABI artifacts.
///
/// These are built by `make ipc-actors-abi`, here we just add the final step
/// so we have better code completion with Rust Analyzer.
fn main() {
    // Run with `cargo build -vv` to see output from any `eprintln!` or `println!`.

    // Use the env var set by the Makefile, or fall back to the default.
    let ipc_actors_dir = std::env::var("IPC_ACTORS_DIR").unwrap_or_else(|_| {
        workspace_dir()
            .join("../ipc-solidity-actors")
            .to_string_lossy()
            .into_owned()
    });

    if !Path::new(&ipc_actors_dir).is_dir() {
        eprintln!("{ipc_actors_dir} doesn't exist. Skip regenerating Rust bindings.");
        return;
    }

    let mut lib = std::fs::File::create("./src/lib.rs").expect("failed to create lib.rs");
    writeln!(lib, "// DO NOT EDIT! This file was generated by build.rs").unwrap();

    // The list of actors we need bindings for, based on how the ipc-actor uses `abigen!`.
    // With the diamond pattern, there is a contract that holds state, and there are these facets which have the code,
    // so we need bindings for the facets, but well (I think) use the same address with all of them.
    for contract_name in [
        "GatewayDiamond",
        "GatewayManagerFacet",
        "GatewayGetterFacet",
        "GatewayRouterFacet",
        "SubnetActorDiamond",
        "SubnetActorGetterFacet",
        "SubnetActorManagerFacet",
        "SubnetRegistry",
    ] {
        let module_name = camel_to_snake(contract_name);
        let input_path = format!("{ipc_actors_dir}/out/{contract_name}.sol/{contract_name}.json");
        let output_path = format!("./src/{}.rs", module_name);

        ethers::prelude::Abigen::new(contract_name, &input_path)
            .expect("failed to create Abigen")
            .generate()
            .expect("failed to generate Rust bindings")
            .write_to_file(output_path)
            .expect("failed to write Rust code");

        writeln!(lib, "#[allow(clippy::all)]\npub mod {module_name};").unwrap();

        println!("cargo:rerun-if-changed={input_path}");
    }
    println!("cargo:rerun-if-changed=build.rs");
}

/// Convert ContractName to contract_name so we can use it as a Rust module.
///
/// We could just lowercase, but this is what `Abigen` does as well, and it's more readable with complex names.
fn camel_to_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        match (i, c) {
            (0, c) if c.is_uppercase() => {
                out.push(c.to_ascii_lowercase());
            }
            (_, c) if c.is_uppercase() => {
                out.push('_');
                out.push(c.to_ascii_lowercase());
            }
            (_, c) => {
                out.push(c);
            }
        }
    }
    out
}

// Find the root of the workspace, not this crate, which is what `env!("CARGO_MANIFEST_DIR")` would return
fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;

    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

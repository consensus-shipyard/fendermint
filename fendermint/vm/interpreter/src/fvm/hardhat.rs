// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{bail, Context};
use ethers::core::types as et;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

/// e.g. `"src/lib/AccountHelper.sol"`
pub type ContractSource = String;

/// e.g. `"AccountHelper"`.
pub type ContractName = String;

/// Utility to link bytecode from Hardhat build artifacts.
#[derive(Clone, Debug)]
pub struct Hardhat {
    /// Directory with Hardhat build artifacts, the full-fat JSON files
    /// that contain ABI, bytecode, link references, etc.
    contracts_dir: PathBuf,
}

impl Hardhat {
    pub fn new(contracts_dir: PathBuf) -> Self {
        Self { contracts_dir }
    }

    /// Concatenate the contracts directory with the expected layout to get
    /// the path to the JSON file of a contract, which is under the directory
    /// `<contract-name>.sol`.
    fn contract_path(&self, contract_name: &str) -> PathBuf {
        self.contracts_dir
            .join(format!("{contract_name}.sol"))
            .join(format!("{contract_name}.json"))
    }

    /// Parse the Hardhat artifact of a contract.
    fn artifact(&self, contract_name: &str) -> anyhow::Result<Artifact> {
        let contract_path = self.contract_path(contract_name);

        let json = std::fs::read_to_string(&contract_path)
            .with_context(|| format!("failed to read {contract_path:?}"))?;

        let artifact =
            serde_json::from_str::<Artifact>(&json).context("failed to parse Hardhat artifact")?;

        Ok(artifact)
    }

    /// Read the bytecode of the contract and replace all links in it with library addresses,
    /// similar to how the [hardhat-ethers](https://github.com/NomicFoundation/hardhat/blob/7cc06ab222be8db43265664c68416fdae3030418/packages/hardhat-ethers/src/internal/helpers.ts#L165C42-L165C42)
    /// plugin does it.
    pub fn bytecode(
        &self,
        contract_name: &str,
        libraries: &HashMap<ContractName, et::Address>,
    ) -> anyhow::Result<Vec<u8>> {
        let artifact = self.artifact(contract_name)?;

        // Get the bytecode which is in hex format with placeholders for library references.
        let mut bytecode = artifact.bytecode.object.clone();

        // Replace all library references with their address.
        // Here we differ slightly from the TypeScript version in that we don't return an error
        // for entries in the library address map that we end up not needing, so we can afford
        // to know less about which contract needs which exact references when we call them,
        for (lib_src, lib_name) in artifact.libraries_needed() {
            // References can be given with Fully Qualified Name, or just the contract name,
            // but they must be unique and unambiguous.
            let fqn = &format!("{lib_src}:{lib_name}");

            let lib_addr = match (libraries.get(fqn), libraries.get(&lib_name)) {
                (None, None) => {
                    bail!("failed to resolve library: {fqn}")
                }
                (Some(_), Some(_)) => bail!("ambiguous library: {fqn}"),
                (Some(addr), None) => addr,
                (None, Some(addr)) => addr,
            };

            let lib_addr = hex::encode(lib_addr.0);

            for pos in artifact.library_positions(&lib_src, &lib_name) {
                let start = 2 + pos.start * 2;
                let end = start + pos.length * 2;
                bytecode.replace_range(start..end, &lib_addr);
            }
        }

        let bytecode = hex::decode(bytecode.trim_start_matches("0x"))
            .context("failed to decode contract from hex")?;

        Ok(bytecode)
    }
}

#[derive(Deserialize)]
struct Artifact {
    pub bytecode: Bytecode,
}

impl Artifact {
    // Collect the libraries this contract needs.
    pub fn libraries_needed(&self) -> Vec<(ContractSource, ContractName)> {
        self.bytecode
            .link_references
            .iter()
            .flat_map(|(lib_src, links)| {
                links
                    .keys()
                    .map(|lib_name| (lib_src.to_owned(), lib_name.to_owned()))
            })
            .collect()
    }

    pub fn library_positions(
        &self,
        lib_src: &str,
        lib_name: &str,
    ) -> impl Iterator<Item = &Position> {
        match self
            .bytecode
            .link_references
            .get(lib_src)
            .and_then(|links| links.get(lib_name))
        {
            Some(ps) => ps.iter(),
            None => [].iter(),
        }
    }
}

/// Match the `"bytecode"` entry in the Hardhat build artifact.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bytecode {
    /// Hexadecimal format with placeholders for links.
    pub object: String,
    pub link_references: HashMap<ContractSource, HashMap<ContractName, Vec<Position>>>,
}

/// Indicate where a placeholder appears in the bytecode object.
#[derive(Deserialize)]
struct Position {
    pub start: usize,
    pub length: usize,
}

#[cfg(test)]
mod tests {
    use ethers::core::types as et;
    use std::collections::HashMap;

    use crate::fvm::bundle;

    use super::Hardhat;

    #[test]
    fn bytecode_linking() {
        let contracts_dir = bundle::contracts_path();
        let hardhat = Hardhat::new(contracts_dir);

        let mut libraries = HashMap::new();

        for lib in [
            "AccountHelper",
            "CheckpointHelper",
            "EpochVoteSubmissionHelper",
            "ExecutableQueueHelper",
            "SubnetIDHelper",
            "CrossMsgHelper",
            "StorableMsgHelper",
        ] {
            libraries.insert(lib.to_owned(), et::Address::default());
        }

        let _bytecode = hardhat.bytecode("Gateway", &libraries).unwrap();
    }

    #[test]
    fn bytecode_missing_link() {
        let contracts_dir = bundle::contracts_path();
        let hardhat = Hardhat::new(contracts_dir);

        let result = hardhat.bytecode("Gateway", &Default::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("AccountHelper"));
    }
}

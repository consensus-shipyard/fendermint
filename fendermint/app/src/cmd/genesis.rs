// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{anyhow, Context};
use fvm_shared::address::Address;
use fvm_shared::econ::TokenAmount;
use libsecp256k1::PublicKey;
use std::path::PathBuf;

use fendermint_vm_genesis::{Account, Actor, ActorAddr, ActorMeta, Genesis};

use crate::cmd;
use crate::options::{GenesisAddAccountArgs, GenesisNewArgs};

use super::keygen::b64_to_public;

cmd! {
  GenesisNewArgs(self, genesis_path: PathBuf) {
    let genesis = Genesis {
      network_name: self.network_name.clone(),
      network_version: self.network_version,
      base_fee: self.base_fee.clone(),
      validators: Vec::new(),
      accounts: Vec::new()
    };

    let json = serde_json::to_string_pretty(&genesis)?;
    std::fs::write(genesis_path, json)?;

    Ok(())
  }
}

cmd! {
  GenesisAddAccountArgs(self, genesis_path: PathBuf) {
    add_account(&genesis_path, &self.public_key, self.balance.clone())
  }
}

fn add_account(
    genesis_path: &PathBuf,
    public_key_path: &PathBuf,
    balance: TokenAmount,
) -> anyhow::Result<()> {
    update_genesis(genesis_path, |mut genesis| {
        let pk = read_public_key(&public_key_path)?;
        let addr = Address::new_secp256k1(&pk.serialize())?;
        let meta = ActorMeta::Account(Account {
            owner: ActorAddr(addr),
        });
        if genesis.accounts.iter().any(|a| a.meta == meta) {
            return Err(anyhow!("account already exists in the genesis file"));
        }
        let actor = Actor {
            meta,
            balance: balance.clone(),
        };
        genesis.accounts.push(actor);
        Ok(genesis)
    })
}

fn read_public_key(public_key: &PathBuf) -> anyhow::Result<PublicKey> {
    let b64 = std::fs::read_to_string(&public_key).context("failed to read public key")?;
    let pk = b64_to_public(&b64).context("public key from base64")?;
    Ok(pk)
}

fn update_genesis<F>(genesis_path: &PathBuf, f: F) -> anyhow::Result<()>
where
    F: FnOnce(Genesis) -> anyhow::Result<Genesis>,
{
    let json = std::fs::read_to_string(genesis_path).context("failed to read genesis")?;
    let genesis = serde_json::from_str::<Genesis>(&json).context("failed to parse genesis")?;
    let genesis = f(genesis)?;
    let json = serde_json::to_string_pretty(&genesis)?;
    std::fs::write(genesis_path, json)?;
    Ok(())
}

// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod check;
mod exec;
mod genesis;
mod query;

pub use check::FvmCheckState;
pub use exec::{FvmExecState, FvmStateParams};
use fendermint_vm_actor_interface::account::State as AccountState;
use fvm::state_tree::StateTree;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::address::{Address, Protocol};
pub use genesis::{empty_state_tree, FvmGenesisState};
pub use query::FvmQueryState;

pub trait CanResolveAddress {
    /// Resolve an [`Address`] to another which has [`Payload`] that wraps a public key, if possible.
    fn address_to_public_key(&self, addr: Address) -> anyhow::Result<Option<Address>>;
}

impl<DB> CanResolveAddress for StateTree<DB>
where
    DB: Blockstore + 'static,
{
    /// Look up the actor in the state tree and return its original address, if it's an account.
    fn address_to_public_key(&self, addr: Address) -> anyhow::Result<Option<Address>> {
        if let Protocol::Secp256k1 | Protocol::BLS = addr.protocol() {
            return Ok(Some(addr));
        }
        tracing::info!(addr = ?addr, "resolving address to public key");
        if let Some(id) = self.lookup_id(&addr)? {
            if let Some(state) = self.get_actor(id)? {
                if let Some(bz) = self.store().get(&state.state)? {
                    if let Some(state) = fvm_ipld_encoding::from_slice::<AccountState>(&bz).ok() {
                        return Ok(Some(state.address));
                    }
                }
            }
        }
        Ok(None)
    }
}

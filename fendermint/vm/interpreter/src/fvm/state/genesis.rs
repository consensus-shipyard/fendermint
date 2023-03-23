// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{anyhow, Context};
use cid::{multihash::Code, Cid};
use fendermint_vm_actor_interface::{account, cron, eam, init, system};
use fendermint_vm_genesis::{Actor, ActorMeta, Genesis};
use fvm::{
    machine::Manifest,
    state_tree::{ActorState, StateTree},
};
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::load_car_unchecked;
use fvm_ipld_encoding::CborStore;
use fvm_shared::{econ::TokenAmount, state::StateTreeVersion, ActorID};
use num_traits::Zero;
use serde::Serialize;

/// A state we create for the execution of genesis initialisation.
pub struct FvmGenesisState<DB>
where
    DB: Blockstore,
{
    manifest_data_cid: Cid,
    manifest: Manifest,
    pub state_tree: StateTree<DB>,
}

impl<DB> FvmGenesisState<DB>
where
    DB: Blockstore,
{
    pub async fn new(store: DB, bundle: &[u8]) -> anyhow::Result<Self> {
        // Load the actor bundle.
        let bundle_roots = load_car_unchecked(&store, bundle).await?;
        let bundle_root = match bundle_roots.as_slice() {
            [root] => root,
            roots => {
                return Err(anyhow!(
                    "expected one root in actor bundle; got {}",
                    roots.len()
                ))
            }
        };

        let (manifest_version, manifest_data_cid): (u32, Cid) = match store.get_cbor(bundle_root)? {
            Some(vd) => vd,
            None => {
                return Err(anyhow!(
                    "no manifest information in bundle root {}",
                    bundle_root
                ))
            }
        };
        let manifest = Manifest::load(&store, &manifest_data_cid, manifest_version)?;

        // Create an empty state tree.
        let state_tree = StateTree::new(store, StateTreeVersion::V5)?;

        let state = Self {
            manifest_data_cid,
            manifest,
            state_tree,
        };

        Ok(state)
    }

    /// Initialize actor states from the Genesis spec.
    ///
    /// This method doesn't create all builtin Filecoin actors,
    /// it leaves out the ones specific to file storage.
    ///
    /// The ones included are:
    /// * system
    /// * init
    /// * cron
    /// * EAM
    ///
    /// TODO:
    /// * burnt funds?
    /// * faucet?
    /// * IPC
    ///
    /// See [Lotus](https://github.com/filecoin-project/lotus/blob/v1.20.4/chain/gen/genesis/genesis.go) for reference
    /// and the [ref-fvm tester](https://github.com/filecoin-project/ref-fvm/blob/fvm%40v3.1.0/testing/integration/src/tester.rs#L99-L103).
    pub fn create_genesis_actors(&mut self, genesis: &Genesis) -> anyhow::Result<()> {
        // System actor
        let system_state = system::State {
            builtin_actors: self.manifest_data_cid,
        };
        self.create_singleton_actor(
            system::SYSTEM_ACTOR_CODE_ID,
            system::SYSTEM_ACTOR_ID,
            &system_state,
            TokenAmount::zero(),
        )?;

        // Init actor
        let (init_state, addr_to_id) = init::State::new(
            self.state_tree.store(),
            genesis.network_name.clone(),
            &genesis.accounts,
        )?;
        self.create_singleton_actor(
            init::INIT_ACTOR_CODE_ID,
            init::INIT_ACTOR_ID,
            &init_state,
            TokenAmount::zero(),
        )?;

        // Cron actor
        let cron_state = cron::State {
            entries: vec![], // TODO: Maybe with the IPC.
        };
        self.create_singleton_actor(
            cron::CRON_ACTOR_CODE_ID,
            cron::CRON_ACTOR_ID,
            &cron_state,
            TokenAmount::zero(),
        )?;

        // Ethereum Account Manager (EAM) actor
        let eam_state = [(); 0]; // Based on how it's done in `Tester`.
        self.create_singleton_actor(
            eam::EAM_ACTOR_CODE_ID,
            eam::EAM_ACTOR_ID,
            &eam_state,
            TokenAmount::zero(),
        )?;

        // Create accounts
        for a in genesis.accounts.iter() {
            self.create_account_actor(a, &addr_to_id)?;
        }

        Ok(())
    }

    /// Flush the data to the block store.
    pub fn commit(mut self) -> anyhow::Result<Cid> {
        let root = self.state_tree.flush()?;
        Ok(root)
    }

    /// Creates a singleton built-in actor using code specified in the manifest.
    /// A singleton actor does not have a robust/key address resolved via the Init actor.
    fn create_singleton_actor(
        &mut self,
        code_id: u32,
        id: ActorID,
        state: &impl Serialize,
        balance: TokenAmount,
    ) -> anyhow::Result<()> {
        // Retrieve the CID of the actor code by the numeric ID.
        let code_cid = self
            .manifest
            .code_by_id(code_id)
            .ok_or_else(|| anyhow!("can't find {code_id} in the manifest"))?;

        let state_cid = self
            .state_tree
            .store()
            .put_cbor(state, Code::Blake2b256)
            .context("failed to put actor state while installing")?;

        let actor_state = ActorState {
            code: *code_cid,
            state: state_cid,
            sequence: 0,
            balance,
            delegated_address: None,
        };

        self.state_tree.set_actor(id, actor_state);

        Ok(())
    }

    fn create_account_actor(
        &mut self,
        actor: &Actor,
        ids: &init::AddressMap,
    ) -> anyhow::Result<()> {
        let code_cid = self.manifest.get_account_code();
        match actor.meta {
            ActorMeta::Account { ref owner } => {
                let address = owner.0;
                let state = account::State { address };

                let id = ids
                    .get(&address)
                    .ok_or_else(|| anyhow!("can't find ID for {address}"))?;

                let state_cid = self
                    .state_tree
                    .store()
                    .put_cbor(&state, Code::Blake2b256)
                    .context("failed to put actor state while installing")?;

                let actor_state = ActorState {
                    code: *code_cid,
                    state: state_cid,
                    sequence: 0,
                    balance: actor.balance.clone(),
                    delegated_address: None,
                };

                self.state_tree.set_actor(*id, actor_state);

                Ok(())
            }

            ActorMeta::MultiSig { .. } => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use fendermint_vm_genesis::Genesis;
    use fvm_ipld_blockstore::MemoryBlockstore;
    use quickcheck::Arbitrary;

    use crate::fvm::bundle::bundle_path;

    use super::FvmGenesisState;

    #[tokio::test]
    async fn load_genesis() {
        let mut g = quickcheck::Gen::new(5);
        let genesis = Genesis::arbitrary(&mut g);
        let bundle = std::fs::read(bundle_path()).expect("failed to read bundle");
        let store = MemoryBlockstore::new();

        let mut state = FvmGenesisState::new(&store, &bundle)
            .await
            .expect("failed to create state");

        state
            .create_genesis_actors(&genesis)
            .expect("failed to create actors");

        let _state_root = state.commit().expect("failed to commit");
    }
}

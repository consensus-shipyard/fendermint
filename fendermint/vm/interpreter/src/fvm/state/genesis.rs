// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::sync::Arc;

use anyhow::{anyhow, bail, Context};
use cid::{multihash::Code, Cid};
use fendermint_vm_actor_interface::{
    account::{self, ACCOUNT_ACTOR_CODE_ID},
    eam,
    ethaccount::ETHACCOUNT_ACTOR_CODE_ID,
    init,
    multisig::{self, MULTISIG_ACTOR_CODE_ID},
    EMPTY_ARR,
};
use fendermint_vm_core::Timestamp;
use fendermint_vm_genesis::{Account, Multisig};
use fvm::{
    engine::MultiEngine,
    machine::Manifest,
    state_tree::{ActorState, StateTree},
};
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::load_car_unchecked;
use fvm_ipld_encoding::CborStore;
use fvm_shared::{
    address::{Address, Payload},
    clock::ChainEpoch,
    econ::TokenAmount,
    state::StateTreeVersion,
    version::NetworkVersion,
    ActorID,
};
use num_traits::Zero;
use serde::Serialize;

use super::{exec::MachineBlockstore, FvmExecState, FvmStateParams};

/// Create an empty state tree.
pub fn empty_state_tree<DB: Blockstore>(store: DB) -> anyhow::Result<StateTree<DB>> {
    let state_tree = StateTree::new(store, StateTreeVersion::V5)?;
    Ok(state_tree)
}

/// Initially we can only set up an empty state tree.
/// Then we have to create the built-in actors' state that the FVM relies on.
/// Then we can instantiate an FVM execution engine, which we can use to construct FEVM based actors.
enum Stage<DB: Blockstore + 'static> {
    Tree(StateTree<DB>),
    Exec(FvmExecState<DB>),
}

/// A state we create for the execution of genesis initialisation.
pub struct FvmGenesisState<DB>
where
    DB: Blockstore + 'static,
{
    pub manifest_data_cid: Cid,
    pub manifest: Manifest,
    store: DB,
    multi_engine: Arc<MultiEngine>,
    stage: Stage<DB>,
}

impl<DB> FvmGenesisState<DB>
where
    DB: Blockstore + Clone + 'static,
{
    pub async fn new(
        store: DB,
        multi_engine: Arc<MultiEngine>,
        bundle: &[u8],
    ) -> anyhow::Result<Self> {
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

        let state_tree = empty_state_tree(store.clone())?;

        let state = Self {
            manifest_data_cid,
            manifest,
            store,
            multi_engine,
            stage: Stage::Tree(state_tree),
        };

        Ok(state)
    }

    /// Instantiate the execution state, once the basic genesis parameters are known.
    ///
    /// This must be called before we try to instantiate any EVM actors in genesis.
    pub fn init_exec_state(
        &mut self,
        timestamp: Timestamp,
        network_version: NetworkVersion,
        base_fee: TokenAmount,
        circ_supply: TokenAmount,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        self.stage = match self.stage {
            Stage::Exec(_) => bail!("execution engine already initialized"),
            Stage::Tree(ref mut state_tree) => {
                // We have to flush the data at this point.
                let state_root = state_tree.flush()?;

                let params = FvmStateParams {
                    state_root,
                    timestamp,
                    network_version,
                    base_fee,
                    circ_supply,
                    chain_id,
                };

                let exec_state =
                    FvmExecState::new(self.store.clone(), &self.multi_engine, 1, params)
                        .context("failed to create exec state")?;

                Stage::Exec(exec_state)
            }
        };
        Ok(())
    }

    /// Flush the data to the block store.
    pub fn commit(self) -> anyhow::Result<Cid> {
        match self.stage {
            Stage::Tree(mut state_tree) => Ok(state_tree.flush()?),
            Stage::Exec(exec_state) => exec_state.commit(),
        }
    }

    /// Creates an actor using code specified in the manifest.
    pub fn create_actor(
        &mut self,
        code_id: u32,
        id: ActorID,
        state: &impl Serialize,
        balance: TokenAmount,
        delegated_address: Option<Address>,
    ) -> anyhow::Result<()> {
        // Retrieve the CID of the actor code by the numeric ID.
        let code_cid = *self
            .manifest
            .code_by_id(code_id)
            .ok_or_else(|| anyhow!("can't find {code_id} in the manifest"))?;

        let state_cid = self.put_state(state)?;

        let actor_state = ActorState {
            code: code_cid,
            state: state_cid,
            sequence: 0,
            balance,
            delegated_address,
        };

        self.with_state_tree(
            |s| s.set_actor(id, actor_state.clone()),
            |s| s.set_actor(id, actor_state.clone()),
        );

        Ok(())
    }

    pub fn create_account_actor(
        &mut self,
        acct: Account,
        balance: TokenAmount,
        ids: &init::AddressMap,
    ) -> anyhow::Result<()> {
        let owner = acct.owner.0;

        let id = ids
            .get(&owner)
            .ok_or_else(|| anyhow!("can't find ID for owner {owner}"))?;

        match owner.payload() {
            Payload::Secp256k1(_) => {
                let state = account::State { address: owner };
                self.create_actor(ACCOUNT_ACTOR_CODE_ID, *id, &state, balance, None)
            }
            Payload::Delegated(d) if d.namespace() == eam::EAM_ACTOR_ID => {
                let state = EMPTY_ARR;
                // NOTE: Here we could use the placeholder code ID as well.
                self.create_actor(ETHACCOUNT_ACTOR_CODE_ID, *id, &state, balance, Some(owner))
            }
            other => Err(anyhow!("unexpected actor owner: {other:?}")),
        }
    }

    pub fn create_multisig_actor(
        &mut self,
        ms: Multisig,
        balance: TokenAmount,
        ids: &init::AddressMap,
        next_id: ActorID,
    ) -> anyhow::Result<()> {
        let mut signers = Vec::new();

        // Make sure every signer has their own account.
        for signer in ms.signers {
            let id = ids
                .get(&signer.0)
                .ok_or_else(|| anyhow!("can't find ID for signer {}", signer.0))?;

            if self
                .with_state_tree(|s| s.get_actor(*id), |s| s.get_actor(*id))?
                .is_none()
            {
                self.create_account_actor(Account { owner: signer }, TokenAmount::zero(), ids)?;
            }

            signers.push(*id)
        }

        // Now create a multisig actor that manages group transactions.
        let state = multisig::State::new(
            self.store(),
            signers,
            ms.threshold,
            ms.vesting_start as ChainEpoch,
            ms.vesting_duration as ChainEpoch,
            balance.clone(),
        )?;

        self.create_actor(MULTISIG_ACTOR_CODE_ID, next_id, &state, balance, None)
    }

    pub fn store(&mut self) -> &DB {
        &self.store
    }

    fn put_state(&mut self, state: impl Serialize) -> anyhow::Result<Cid> {
        self.store()
            .put_cbor(&state, Code::Blake2b256)
            .context("failed to store actor state")
    }

    /// A horrible way of unifying the state tree under the two different stages.
    ///
    /// We only use this a few times, so perhaps it's not that much of a burden to duplicate some code.
    fn with_state_tree<F, G, T>(&mut self, f: F, g: G) -> T
    where
        F: FnOnce(&mut StateTree<DB>) -> T,
        G: FnOnce(&mut StateTree<MachineBlockstore<DB>>) -> T,
    {
        match self.stage {
            Stage::Tree(ref mut state_tree) => f(state_tree),
            Stage::Exec(ref mut exec_state) => g(exec_state.state_tree_mut()),
        }
    }
}

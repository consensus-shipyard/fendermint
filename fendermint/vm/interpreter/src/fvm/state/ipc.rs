// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{anyhow, Context};
use ethers::contract::decode_function_data;
use ethers::types as et;
use ethers::{abi::Tokenize, utils::keccak256};

use num_traits::Zero;

use fvm_ipld_blockstore::Blockstore;
use fvm_shared::ActorID;

use fendermint_crypto::SecretKey;
use fendermint_vm_actor_interface::{
    eam::EthAddress,
    ipc::{ValidatorMerkleTree, GATEWAY_ACTOR_ID},
};
use fendermint_vm_genesis::{Power, Validator};
use fendermint_vm_message::signed::sign_secp256k1;
use fendermint_vm_topdown::IPCParentFinality;
use ipc_actors_abis::gateway_getter_facet as getter;
use ipc_actors_abis::gateway_getter_facet::GatewayGetterFacet;
use ipc_actors_abis::gateway_router_facet as router;
use ipc_actors_abis::gateway_router_facet::GatewayRouterFacet;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::staking::StakingChangeRequest;

use super::{
    fevm::{ContractCaller, MockProvider},
    FvmExecState,
};
use crate::fvm::FvmMessage;
use fendermint_vm_actor_interface::{ipc, system};
use fvm_ipld_encoding::{BytesDe, BytesSer, RawBytes};
use fvm_shared::econ::TokenAmount;

#[derive(Clone)]
pub struct GatewayCaller<DB> {
    addr: EthAddress,
    getter: ContractCaller<GatewayGetterFacet<MockProvider>, DB>,
    router: ContractCaller<GatewayRouterFacet<MockProvider>, DB>,
}

impl<DB> Default for GatewayCaller<DB> {
    fn default() -> Self {
        Self::new(GATEWAY_ACTOR_ID)
    }
}

impl<DB> GatewayCaller<DB> {
    pub fn new(gateway_actor_id: ActorID) -> Self {
        let addr = EthAddress::from_id(gateway_actor_id);
        Self {
            addr,
            getter: ContractCaller::new(addr, GatewayGetterFacet::new),
            router: ContractCaller::new(addr, GatewayRouterFacet::new),
        }
    }

    pub fn addr(&self) -> EthAddress {
        self.addr
    }
}

impl<DB: Blockstore> GatewayCaller<DB> {
    /// Check that IPC is configured in this deployment.
    pub fn enabled(&self, state: &mut FvmExecState<DB>) -> anyhow::Result<bool> {
        match state.state_tree_mut().get_actor(GATEWAY_ACTOR_ID)? {
            None => Ok(false),
            Some(a) => Ok(!state.builtin_actors().is_placeholder_actor(&a.code)),
        }
    }

    /// Return true if the current subnet is the root subnet.
    pub fn is_root(&self, state: &mut FvmExecState<DB>) -> anyhow::Result<bool> {
        self.subnet_id(state).map(|id| id.route.is_empty())
    }

    /// Return the current subnet ID.
    pub fn subnet_id(&self, state: &mut FvmExecState<DB>) -> anyhow::Result<getter::SubnetID> {
        self.getter.call(state, |c| c.get_network_name())
    }

    /// Fetch the period with which the current subnet has to submit checkpoints to its parent.
    pub fn bottom_up_check_period(&self, state: &mut FvmExecState<DB>) -> anyhow::Result<u64> {
        self.getter.call(state, |c| c.bottom_up_check_period())
    }

    /// Fetch the bottom-up messages enqueued for a given checkpoint height.
    pub fn bottom_up_msgs(
        &self,
        state: &mut FvmExecState<DB>,
        height: u64,
    ) -> anyhow::Result<Vec<getter::CrossMsg>> {
        self.getter.call(state, |c| c.bottom_up_messages(height))
    }

    /// Fetch the bottom-up messages enqueued in a given checkpoint.
    pub fn bottom_up_msgs_hash(
        &self,
        state: &mut FvmExecState<DB>,
        height: u64,
    ) -> anyhow::Result<[u8; 32]> {
        let msgs = self.bottom_up_msgs(state, height)?;
        Ok(abi_hash(msgs))
    }

    /// Insert a new checkpoint at the period boundary.
    pub fn create_bottom_up_checkpoint(
        &self,
        state: &mut FvmExecState<DB>,
        checkpoint: router::BottomUpCheckpoint,
        power_table: &[Validator<Power>],
    ) -> anyhow::Result<()> {
        // Construct a Merkle tree from the power table, which we can use to validate validator set membership
        // when the signatures are submitted in transactions for accumulation.
        let tree =
            ValidatorMerkleTree::new(power_table).context("failed to create validator tree")?;

        let total_power = power_table.iter().fold(et::U256::zero(), |p, v| {
            p.saturating_add(et::U256::from(v.power.0))
        });

        self.router.call(state, |c| {
            c.create_bottom_up_checkpoint(checkpoint, tree.root_hash().0, total_power)
        })
    }

    /// Retrieve checkpoints which have not reached a quorum.
    pub fn incomplete_checkpoints(
        &self,
        state: &mut FvmExecState<DB>,
    ) -> anyhow::Result<Vec<getter::BottomUpCheckpoint>> {
        self.getter.call(state, |c| c.get_incomplete_checkpoints())
    }

    /// Apply all pending validator changes, returning the newly adopted configuration number, or 0 if there were no changes.
    pub fn apply_validator_changes(&self, state: &mut FvmExecState<DB>) -> anyhow::Result<u64> {
        self.router.call(state, |c| c.apply_finality_changes())
    }

    /// Get the currently active validator set.
    pub fn current_validator_set(
        &self,
        state: &mut FvmExecState<DB>,
    ) -> anyhow::Result<getter::Membership> {
        self.getter.call(state, |c| c.get_current_membership())
    }

    /// Construct the input parameters for adding a signature to the checkpoint.
    ///
    /// This will need to be broadcasted as a transaction.
    pub fn add_checkpoint_signature_calldata(
        &self,
        checkpoint: router::BottomUpCheckpoint,
        power_table: &[Validator<Power>],
        validator: &Validator<Power>,
        secret_key: &SecretKey,
    ) -> anyhow::Result<et::Bytes> {
        debug_assert_eq!(validator.public_key.0, secret_key.public_key());

        let height = checkpoint.block_height;
        let weight = et::U256::from(validator.power.0);

        let hash = abi_hash(checkpoint);
        let signature = et::Bytes::from(sign_secp256k1(secret_key, &hash));

        let tree =
            ValidatorMerkleTree::new(power_table).context("failed to construct Merkle tree")?;

        let membership_proof = tree
            .prove(validator)
            .context("failed to construct Merkle proof")?
            .into_iter()
            .map(|p| p.into())
            .collect();

        let call = self.router.contract().add_checkpoint_signature(
            height,
            membership_proof,
            weight,
            signature,
        );

        let calldata = call
            .calldata()
            .ok_or_else(|| anyhow!("no calldata for adding signature"))?;

        Ok(calldata)
    }

    pub fn commit_parent_finality_msg(
        &self,
        finality: fendermint_vm_topdown::IPCParentFinality,
    ) -> anyhow::Result<FvmMessage> {
        let evm_finality = router::ParentFinality::try_from(finality)?;
        let call = self.router.contract().commit_parent_finality(evm_finality);
        let calldata = call
            .calldata()
            .ok_or_else(|| anyhow!("no calldata for commit parent finality"))?;

        encode_to_fvm_implicit(calldata.as_ref())
    }

    pub fn decode_commit_parent_finality_return(
        &self,
        bytes: RawBytes,
    ) -> anyhow::Result<(bool, IPCParentFinality)> {
        let return_data = bytes
            .deserialize::<BytesDe>()
            .context("failed to deserialize return data")?;

        let function = router::GATEWAYROUTERFACET_ABI
            .functions
            .get("commitParentFinality")
            .ok_or_else(|| anyhow!("broken abi"))?
            .get(0)
            .ok_or_else(|| anyhow!("function not found, abi wrong?"))?;

        let (committed, finality): (bool, router::ParentFinality) =
            decode_function_data(function, return_data.0, false)?;
        Ok((committed, IPCParentFinality::try_from(finality)?))
    }

    pub fn store_validator_changes_msg(
        &self,
        changes: Vec<StakingChangeRequest>,
    ) -> anyhow::Result<FvmMessage> {
        let mut change_requests = vec![];
        for c in changes {
            change_requests.push(router::StakingChangeRequest::try_from(c)?);
        }

        let call = self
            .router
            .contract()
            .store_validator_changes(change_requests);
        let calldata = call
            .calldata()
            .ok_or_else(|| anyhow!("no calldata for store validator changes"))?;

        encode_to_fvm_implicit(calldata.as_ref())
    }

    pub fn apply_cross_messages_msg(
        &self,
        cross_messages: Vec<CrossMsg>,
    ) -> anyhow::Result<FvmMessage> {
        let mut messages = vec![];
        for c in cross_messages {
            messages.push(router::CrossMsg::try_from(c)?);
        }

        let call = self.router.contract().apply_cross_messages(messages);
        let calldata = call
            .calldata()
            .ok_or_else(|| anyhow!("no calldata for apply cross messages"))?;

        encode_to_fvm_implicit(calldata.as_ref())
    }

    pub fn get_latest_parent_finality(
        &self,
        state: &mut FvmExecState<DB>,
    ) -> anyhow::Result<IPCParentFinality> {
        let r = self
            .getter
            .call(state, |c| c.get_latest_parent_finality())?;
        Ok(IPCParentFinality::try_from(r)?)
    }
}

/// Hash some value in the same way we'd hash it in Solidity.
fn abi_hash<T: Tokenize>(value: T) -> [u8; 32] {
    keccak256(ethers::abi::encode(&value.into_tokens()))
}

/// Encode to fvm implicit message
fn encode_to_fvm_implicit(bytes: &[u8]) -> anyhow::Result<FvmMessage> {
    let params = RawBytes::serialize(BytesSer(bytes))?;
    let msg = FvmMessage {
        version: 0,
        from: system::SYSTEM_ACTOR_ADDR,
        to: ipc::GATEWAY_ACTOR_ADDR,
        value: TokenAmount::zero(),
        method_num: ipc::gateway::METHOD_INVOKE_CONTRACT,
        params,
        // we are sending a implicit message, no need to set sequence
        sequence: 0,
        gas_limit: fvm_shared::BLOCK_GAS_LIMIT,
        gas_fee_cap: TokenAmount::zero(),
        gas_premium: TokenAmount::zero(),
    };

    Ok(msg)
}

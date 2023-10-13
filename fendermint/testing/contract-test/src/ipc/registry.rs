// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Context;
use fendermint_vm_actor_interface::eam::EthAddress;
use fendermint_vm_actor_interface::ipc::SUBNETREGISTRY_ACTOR_ID;
use fendermint_vm_interpreter::fvm::state::fevm::{ContractCaller, MockProvider};
use fendermint_vm_interpreter::fvm::state::FvmExecState;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::ActorID;
use ipc_actors_abis::subnet_actor_getter_facet::SubnetActorGetterFacet as SubnetGetterFacet;
use ipc_actors_abis::subnet_actor_manager_facet::SubnetActorManagerFacet as SubnetManagerFacet;
use ipc_actors_abis::subnet_registry::{
    ConstructorParams as SubnetConstructorParams, SubnetRegistry,
};

#[derive(Clone)]
pub struct RegistryCaller<DB> {
    addr: EthAddress,
    registry: ContractCaller<SubnetRegistry<MockProvider>, DB>,
    _getter: ContractCaller<SubnetGetterFacet<MockProvider>, DB>,
    _manager: ContractCaller<SubnetManagerFacet<MockProvider>, DB>,
}

impl<DB> Default for RegistryCaller<DB> {
    fn default() -> Self {
        Self::new(SUBNETREGISTRY_ACTOR_ID)
    }
}

impl<DB> RegistryCaller<DB> {
    pub fn new(registry_actor_id: ActorID) -> Self {
        let addr = EthAddress::from_id(registry_actor_id);
        Self {
            addr,
            registry: ContractCaller::new(addr, SubnetRegistry::new),
            _getter: ContractCaller::new(addr, SubnetGetterFacet::new),
            _manager: ContractCaller::new(addr, SubnetManagerFacet::new),
        }
    }

    pub fn addr(&self) -> EthAddress {
        self.addr
    }
}

impl<DB: Blockstore> RegistryCaller<DB> {
    /// Create a new instance of the built-in subnet implemetation.
    ///
    /// Returns the address of the deployed contract.
    pub fn new_subnet(
        &self,
        state: &mut FvmExecState<DB>,
        params: SubnetConstructorParams,
    ) -> anyhow::Result<EthAddress> {
        let addr = self
            .registry
            .call(state, |c| c.new_subnet_actor(params))
            .context("failed to create new subnet")?;
        Ok(EthAddress(addr.0))
    }
}

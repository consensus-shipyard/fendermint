// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use fendermint_vm_actor_interface::eam::EthAddress;
use fendermint_vm_interpreter::fvm::state::fevm::{ContractCaller, MockProvider};
use fvm_ipld_blockstore::Blockstore;
use ipc_actors_abis::subnet_actor_getter_facet::SubnetActorGetterFacet as SubnetGetterFacet;
use ipc_actors_abis::subnet_actor_manager_facet::SubnetActorManagerFacet as SubnetManagerFacet;

pub use ipc_actors_abis::subnet_registry::ConstructorParams as SubnetConstructorParams;

#[derive(Clone)]
pub struct SubnetCaller<DB> {
    addr: EthAddress,
    _getter: ContractCaller<SubnetGetterFacet<MockProvider>, DB>,
    _manager: ContractCaller<SubnetManagerFacet<MockProvider>, DB>,
}

impl<DB> SubnetCaller<DB> {
    pub fn new(addr: EthAddress) -> Self {
        Self {
            addr,
            _getter: ContractCaller::new(addr, SubnetGetterFacet::new),
            _manager: ContractCaller::new(addr, SubnetManagerFacet::new),
        }
    }

    pub fn addr(&self) -> EthAddress {
        self.addr
    }
}

impl<DB: Blockstore> SubnetCaller<DB> {}

// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use super::{
    fevm::{ContractCaller, MockProvider},
    FvmExecState,
};
use fendermint_vm_actor_interface::{eam::EthAddress, ipc::GATEWAY_ACTOR_ID};
use fendermint_vm_ipc_actors::gateway_getter_facet::GatewayGetterFacet;
use fvm_ipld_blockstore::Blockstore;

pub struct GatewayCaller {
    getter: ContractCaller<GatewayGetterFacet<MockProvider>>,
}

impl GatewayCaller {
    pub fn new() -> Self {
        let addr = EthAddress::from_id(GATEWAY_ACTOR_ID);
        Self {
            getter: ContractCaller::new(addr, GatewayGetterFacet::new),
        }
    }

    /// Check that IPC is configured in this deployment.
    pub fn enabled<DB: Blockstore>(&self, state: &mut FvmExecState<DB>) -> anyhow::Result<bool> {
        match state.state_tree_mut().get_actor(GATEWAY_ACTOR_ID)? {
            None => Ok(false),
            Some(a) => Ok(!state.builtin_actors().is_placeholder_actor(&a.code)),
        }
    }

    /// Return true if the current subnet is the root subnet.
    pub fn is_root<DB: Blockstore>(&self, state: &mut FvmExecState<DB>) -> anyhow::Result<bool> {
        let subnet_id = self.getter.call(state, |c| c.get_network_name())?;
        Ok(subnet_id.route.is_empty())
    }

    pub fn bottom_up_check_period<DB: Blockstore>(
        &self,
        state: &mut FvmExecState<DB>,
    ) -> anyhow::Result<u64> {
        self.getter.call(state, |c| c.bottom_up_check_period())
    }
}

impl Default for GatewayCaller {
    fn default() -> Self {
        Self::new()
    }
}

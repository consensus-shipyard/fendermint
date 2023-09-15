// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use super::{
    fevm::{ContractCaller, MockProvider},
    FvmExecState,
};
use fendermint_vm_actor_interface::{eam::EthAddress, ipc::GATEWAY_ACTOR_ID};
use fendermint_vm_ipc_actors::gateway_getter_facet::GatewayGetterFacet;
use fendermint_vm_ipc_actors::gateway_manager_facet::{GatewayManagerFacet, Membership};
use fvm_ipld_blockstore::Blockstore;

/// Result of attempting to activate the next configuration.
pub enum Configuration {
    /// No change - return the current configuration number.
    Unchanged(u64),
    /// Activated the next configuration.
    Activated(Membership),
}

#[derive(Clone)]
pub struct GatewayCaller {
    getter: ContractCaller<GatewayGetterFacet<MockProvider>>,
    manager: ContractCaller<GatewayManagerFacet<MockProvider>>,
}

impl GatewayCaller {
    pub fn new() -> Self {
        let addr = EthAddress::from_id(GATEWAY_ACTOR_ID);
        Self {
            getter: ContractCaller::new(addr, GatewayGetterFacet::new),
            manager: ContractCaller::new(addr, GatewayManagerFacet::new),
        }
    }

    /// Check that IPC is configured in this deployment.
    pub fn enabled<DB: Blockstore>(&self, state: &mut FvmExecState<DB>) -> anyhow::Result<bool> {
        match state.state_tree_mut().get_actor(GATEWAY_ACTOR_ID)? {
            None => Ok(false),
            Some(a) => Ok(!state.builtin_actors().is_placeholder_actor(&a.code)),
        }
    }

    /// Fetch the period at which this subnet should checkpoint itself to the parent.
    pub fn bottom_up_check_period<DB: Blockstore>(
        &self,
        state: &mut FvmExecState<DB>,
    ) -> anyhow::Result<u64> {
        self.getter.call(state, |c| c.bottom_up_check_period())
    }

    /// Activate the next available configuration at the end of the checkpoint period
    /// and return the new validator set. If there was no change, return nothing.
    pub fn activate_next_configuration<DB: Blockstore>(
        &self,
        state: &mut FvmExecState<DB>,
    ) -> anyhow::Result<Configuration> {
        let current_cnr = self
            .getter
            .call(state, |c| c.get_current_configuration_number())?;

        // In theory we could just call the transition, but currently it emits an event even if there is no change.
        let next_cnr = self
            .getter
            .call(state, |c| c.get_last_configuration_number())?;

        if current_cnr == next_cnr {
            return Ok(Configuration::Unchanged(current_cnr));
        }

        let membership = self.manager.call(state, |c| c.update_membership())?;

        Ok(Configuration::Activated(membership))
    }
}

impl Default for GatewayCaller {
    fn default() -> Self {
        Self::new()
    }
}

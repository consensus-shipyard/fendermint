// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use ethers::types as et;
use fendermint_vm_actor_interface::ipc::BottomUpCheckpoint;
use fendermint_vm_genesis::{Power, Validator, ValidatorKey};
use fendermint_vm_ipc_actors::gateway_router_facet::SubnetID;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::address::SECP_PUB_LEN;
use tendermint::block::Height;
use tendermint_rpc::{endpoint::validators, Client, Paging};

use super::state::{ipc::GatewayCaller, FvmExecState};

pub type Checkpoint = BottomUpCheckpoint;

/// Validator voting power snapshot.
#[derive(Debug, Clone)]
pub struct PowerTable(pub Vec<Validator>);

/// Changes in the power table.
#[derive(Debug, Clone, Default)]
pub struct PowerUpdates(pub Vec<Validator>);

/// Construct and store a checkpoint if this is the end of the checkpoint period.
/// Perform end-of-checkpoint-period transitions in the ledger.
pub async fn maybe_create_checkpoint<C, DB>(
    client: &C,
    gateway: &GatewayCaller<DB>,
    state: &mut FvmExecState<DB>,
) -> anyhow::Result<Option<(Checkpoint, PowerUpdates)>>
where
    C: Client + Sync + Send + 'static,
    DB: Blockstore + Sync + Send + 'static,
{
    // Epoch transitions for checkpointing.
    let height: tendermint::block::Height = state
        .block_height()
        .try_into()
        .context("block height is not u64")?;

    match should_create_checkpoint(gateway, state, height)? {
        None => Ok(None),
        Some(subnet_id) => {
            // Get the current power table.
            let power_table = power_table(client, height)
                .await
                .context("failed to get the power table")?;

            // TODO #252: Take the next changes from the gateway.
            let power_updates = PowerUpdates(Vec::new());

            // TODO #252: Merge the changes into the power table.
            let next_power_table = merge_power(power_table, power_updates.clone());

            // TODO: #252: Take the configuration number of the last change.
            let next_configuration_number = 0;

            let block_hash = state
                .block_hash()
                .ok_or_else(|| anyhow!("block hash not set"))?;

            // Construct checkpoint.
            let checkpoint = BottomUpCheckpoint {
                subnet_id,
                block_height: height.value(),
                block_hash,
                next_configuration_number,
                cross_messages_hash: et::H256::zero().0,
            };

            // Save the checkpoint in the ledger.
            gateway
                .create_bottom_up_checkpoint(state, checkpoint.clone(), &next_power_table.0)
                .context("failed to store checkpoint")?;

            Ok(Some((checkpoint, power_updates)))
        }
    }
}

fn should_create_checkpoint<DB>(
    gateway: &GatewayCaller<DB>,
    state: &mut FvmExecState<DB>,
    height: Height,
) -> anyhow::Result<Option<SubnetID>>
where
    DB: Blockstore,
{
    if gateway.enabled(state)? {
        let id = gateway.subnet_id(state)?;
        let is_root = id.route.is_empty();

        if !is_root && height.value() % gateway.bottom_up_check_period(state)? == 0 {
            let id = SubnetID {
                root: id.root,
                route: id.route,
            };
            return Ok(Some(id));
        }
    }
    Ok(None)
}

async fn power_table<C>(client: &C, height: Height) -> anyhow::Result<PowerTable>
where
    C: Client + Sync + Send + 'static,
{
    let mut power_table = Vec::new();
    let validators: validators::Response = client.validators(height, Paging::All).await?;

    for v in validators.validators {
        power_table.push(Validator {
            public_key: ValidatorKey::try_from(v.pub_key)?,
            power: Power(v.power()),
        });
    }

    Ok(PowerTable(power_table))
}

fn merge_power(curr: PowerTable, updates: PowerUpdates) -> PowerTable {
    // Serializing the key because the wrapped types don't implement Hash or Ord.
    let mut next = HashMap::<[u8; SECP_PUB_LEN], Validator>::new();

    for v in curr.0 {
        let pk = v.public_key.0.serialize();
        next.insert(pk, v);
    }

    for v in updates.0 {
        let pk = v.public_key.0.serialize();
        if v.power.0 == 0 {
            next.remove(&pk);
        } else {
            next.insert(pk, v);
        }
    }

    PowerTable(next.drain().map(|(_, v)| v).collect())
}

// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Context;
use fendermint_vm_actor_interface::ipc::ValidatorMerkleTree;
use fendermint_vm_genesis::{Power, Validator, ValidatorKey};
use fvm_ipld_blockstore::Blockstore;
use tendermint::block::Height;
use tendermint_rpc::{endpoint::validators, Client, Paging};

use super::state::{ipc::GatewayCaller, FvmExecState};

// TODO #248: Define checkpoint type.
pub type Checkpoint = ();

/// Validator voting power.
pub type PowerTable = Vec<Validator>;

/// Construct and store a checkpoint if this is the end of the checkpoint period.
/// Perform end-of-checkpoint-period transitions in the ledger.
pub async fn maybe_create_checkpoint<C, DB>(
    client: &C,
    gateway: &GatewayCaller,
    state: &mut FvmExecState<DB>,
) -> anyhow::Result<Option<(Checkpoint, PowerTable)>>
where
    C: Client + Sync + Send + 'static,
    DB: Blockstore + Sync + Send + 'static,
{
    // Epoch transitions for checkpointing.
    let height: tendermint::block::Height = state
        .block_height()
        .try_into()
        .context("block height is not u64")?;

    if !should_create_checkpoint(gateway, state, height)? {
        return Ok(None);
    }

    let power_table = power_table(client, height)
        .await
        .context("failed to get the power table")?;

    // TODO #252: Take the next changes from the gateway.
    // TODO #252: Merge the changes into the power table.

    // TODO #254: Construct a Merkle tree from the power table.
    let _tree =
        ValidatorMerkleTree::new(&power_table).context("failed to create validator tree")?;

    // TODO #254: Put the next configuration number for the parent to expect in the checkpoint.
    // TODO #254: Construct checkpoint.
    let checkpoint = ();

    Ok(Some((checkpoint, power_table)))
}

fn should_create_checkpoint<DB>(
    gateway: &GatewayCaller,
    state: &mut FvmExecState<DB>,
    height: Height,
) -> anyhow::Result<bool>
where
    DB: Blockstore,
{
    if !gateway.enabled(state)? {
        Ok(false)
    } else if gateway.is_root(state)? {
        Ok(false)
    } else {
        Ok(height.value() % gateway.bottom_up_check_period(state)? == 0)
    }
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

    Ok(power_table)
}

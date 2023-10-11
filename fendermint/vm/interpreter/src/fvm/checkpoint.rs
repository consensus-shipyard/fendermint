// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use ethers::types::U256;
use fendermint_crypto::PublicKey;
use fendermint_vm_genesis::Collateral;
use fendermint_vm_genesis::PowerScale;
use fvm_shared::bigint::BigInt;
use fvm_shared::bigint::Sign;
use fvm_shared::econ::TokenAmount;
use ipc_actors_abis::gateway_getter_facet::Membership;
use tendermint::block::Height;
use tendermint_rpc::{endpoint::validators, Client, Paging};

use fvm_ipld_blockstore::Blockstore;
use fvm_shared::{address::Address, chainid::ChainID};

use fendermint_crypto::SecretKey;
use fendermint_vm_actor_interface::ipc::BottomUpCheckpoint;
use fendermint_vm_genesis::{Power, Validator, ValidatorKey};
use ipc_actors_abis::gateway_getter_facet as getter;
use ipc_actors_abis::gateway_router_facet as router;

use super::{
    broadcast::Broadcaster,
    state::{ipc::GatewayCaller, FvmExecState},
    ValidatorContext,
};

/// Validator voting power snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerTable(pub Vec<Validator<Power>>);

/// Changes in the power table.
#[derive(Debug, Clone, Default)]
pub struct PowerUpdates(pub Vec<Validator<Power>>);

/// Construct and store a checkpoint if this is the end of the checkpoint period.
/// Perform end-of-checkpoint-period transitions in the ledger.
///
/// If we are the boundary, return the validators eligible to sign and any updates
/// to the power table, along with the checkpoint that needs to be signed by validators.
pub async fn maybe_create_checkpoint<C, DB>(
    client: &C,
    gateway: &GatewayCaller<DB>,
    state: &mut FvmExecState<DB>,
) -> anyhow::Result<Option<(router::BottomUpCheckpoint, PowerTable, PowerUpdates)>>
where
    C: Client + Sync + Send + 'static,
    DB: Blockstore + Sync + Send + 'static,
{
    // Epoch transitions for checkpointing.
    let height: tendermint::block::Height = state
        .block_height()
        .try_into()
        .context("block height is not u64")?;

    let block_hash = state
        .block_hash()
        .ok_or_else(|| anyhow!("block hash not set"))?;

    match should_create_checkpoint(gateway, state, height)? {
        None => Ok(None),
        Some(subnet_id) => {
            // Get the current power table from CometBFT.
            // NB: Here we could also get it from the IPC Gateway.
            let power_table = bft_power_table(client, height)
                .await
                .context("failed to get the power table")?;

            // Apply any validator set transitions.
            let next_configuration_number = gateway
                .apply_validator_changes(state)
                .context("failed to apply validator changes")?;

            // Figure out the power updates if there was some change in the configuration.
            let power_updates = if next_configuration_number == 0 {
                PowerUpdates(Vec::new())
            } else {
                let next_membership = gateway
                    .current_validator_set(state)
                    .context("failed to get current validator set")?;

                debug_assert_eq!(
                    next_membership.configuration_number,
                    next_configuration_number
                );

                let next_power_table =
                    membership_to_power_table(&next_membership, state.power_scale());

                power_diff(&power_table, next_power_table)
            };

            // Retrieve the bottom-up messages so we can put their hash into the checkpoint.
            let cross_messages_hash = gateway
                .bottom_up_msgs_hash(state, height.value())
                .context("failed to retrieve bottom-up messages hash")?;

            // Construct checkpoint.
            let checkpoint = BottomUpCheckpoint {
                subnet_id,
                block_height: height.value(),
                block_hash,
                next_configuration_number,
                cross_messages_hash,
            };

            // Save the checkpoint in the ledger.
            // Pass in the current power table, because these are the validators who can sign this checkpoint.
            gateway
                .create_bottom_up_checkpoint(state, checkpoint.clone(), &power_table.0)
                .context("failed to store checkpoint")?;

            Ok(Some((checkpoint, power_table, power_updates)))
        }
    }
}

/// Sign the current and any incomplete checkpoints.
pub async fn broadcast_incomplete_signatures<C, DB>(
    client: &C,
    validator_ctx: &ValidatorContext<C>,
    gateway: &GatewayCaller<DB>,
    chain_id: ChainID,
    incomplete_checkpoints: Vec<getter::BottomUpCheckpoint>,
) -> anyhow::Result<()>
where
    C: Client + Clone + Send + Sync + 'static,
    DB: Blockstore + Send + Sync + 'static,
{
    for cp in incomplete_checkpoints {
        let height = Height::try_from(cp.block_height)?;
        let power_table = bft_power_table(client, height)
            .await
            .context("failed to get power table")?;

        if let Some(validator) = power_table
            .0
            .iter()
            .find(|v| v.public_key.0 == validator_ctx.public_key)
            .cloned()
        {
            // TODO: Code generation in the ipc-solidity-actors repo should cater for this.
            let checkpoint = router::BottomUpCheckpoint {
                subnet_id: router::SubnetID {
                    root: cp.subnet_id.root,
                    route: cp.subnet_id.route,
                },
                block_height: cp.block_height,
                block_hash: cp.block_hash,
                next_configuration_number: cp.next_configuration_number,
                cross_messages_hash: cp.cross_messages_hash,
            };

            // We mustn't do these in parallel because of how nonces are fetched.
            broadcast_signature(
                &validator_ctx.broadcaster,
                gateway,
                checkpoint,
                &power_table,
                &validator,
                &validator_ctx.secret_key,
                chain_id,
            )
            .await
            .context("failed to broadcast checkpoint signature")?;

            tracing::debug!(?height, "submitted checkpoint signature");
        }
    }
    Ok(())
}

/// As a validator, sign the checkpoint and broadcast a transaction to add our signature to the ledger.
pub async fn broadcast_signature<C, DB>(
    broadcaster: &Broadcaster<C>,
    gateway: &GatewayCaller<DB>,
    checkpoint: router::BottomUpCheckpoint,
    power_table: &PowerTable,
    validator: &Validator<Power>,
    secret_key: &SecretKey,
    chain_id: ChainID,
) -> anyhow::Result<()>
where
    C: Client + Clone + Send + Sync + 'static,
    DB: Blockstore + Send + Sync + 'static,
{
    let calldata = gateway
        .add_checkpoint_signature_calldata(checkpoint, &power_table.0, validator, secret_key)
        .context("failed to produce checkpoint signature calldata")?;

    broadcaster
        .fevm_invoke(Address::from(gateway.addr()), calldata, chain_id)
        .await
        .context("failed to broadcast signature")?;

    Ok(())
}

fn should_create_checkpoint<DB>(
    gateway: &GatewayCaller<DB>,
    state: &mut FvmExecState<DB>,
    height: Height,
) -> anyhow::Result<Option<router::SubnetID>>
where
    DB: Blockstore,
{
    if gateway.enabled(state)? {
        let id = gateway.subnet_id(state)?;
        let is_root = id.route.is_empty();

        if !is_root && height.value() % gateway.bottom_up_check_period(state)? == 0 {
            let id = router::SubnetID {
                root: id.root,
                route: id.route,
            };
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Get the power table from CometBFT.
async fn bft_power_table<C>(client: &C, height: Height) -> anyhow::Result<PowerTable>
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

/// Convert the collaterals and metadata in the membership to the public key and power expected by the system.
fn membership_to_power_table(m: &Membership, power_scale: PowerScale) -> PowerTable {
    let mut pt = Vec::new();

    for v in m.validators.iter() {
        // Ignoring any metadata that isn't a public key.
        if let Ok(pk) = PublicKey::parse_slice(&v.metadata, None) {
            let c = u256_to_tokens(v.weight);
            pt.push(Validator {
                public_key: ValidatorKey(pk),
                power: Collateral(c).into_power(power_scale),
            })
        }
    }

    PowerTable(pt)
}

fn u256_to_tokens(value: U256) -> TokenAmount {
    let mut bz = [0u8; 32];
    value.to_big_endian(&mut bz);
    let atto = BigInt::from_bytes_be(Sign::Plus, &bz);
    TokenAmount::from_atto(atto)
}

/// Calculate the difference between the current and the next power table, to return to CometBFT only what changed.
fn power_diff(current: &PowerTable, next: PowerTable) -> PowerUpdates {
    let current = current
        .0
        .iter()
        .map(|v| {
            // Unfortunately the keys don't implement `Hash`.
            let k = v.public_key.0.serialize();
            (k, v)
        })
        .collect::<HashMap<_, _>>();

    let next = into_power_map(next);

    let mut diff = Vec::new();

    // Validators in current but not in next should be removed.
    for (k, v) in current.iter() {
        if !next.contains_key(k) {
            let delete = Validator {
                public_key: v.public_key.clone(),
                power: Power(0),
            };
            diff.push(delete);
        }
    }

    // Validators in next that differ from current should be updated.
    for (k, v) in next.into_iter() {
        let insert = match current.get(&k) {
            Some(w) if **w == v => None,
            _ => Some(v),
        };
        if let Some(insert) = insert {
            diff.push(insert);
        }
    }

    PowerUpdates(diff)
}

fn into_power_map(value: PowerTable) -> HashMap<[u8; 65], Validator<Power>> {
    value
        .0
        .into_iter()
        .map(|v| {
            // Unfortunately the keys don't implement `Hash`.
            let k = v.public_key.0.serialize();
            (k, v)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ethers::core::types::U256;
    use fendermint_vm_genesis::{Power, Validator};
    use quickcheck_macros::quickcheck;

    use crate::fvm::checkpoint::{into_power_map, power_diff, u256_to_tokens};

    use super::{PowerTable, PowerUpdates};

    fn power_update(current: PowerTable, updates: PowerUpdates) -> PowerTable {
        let mut current = into_power_map(current);

        for v in updates.0 {
            let k = v.public_key.0.serialize();
            if v.power.0 == 0 {
                current.remove(&k);
            } else {
                current.insert(k, v);
            }
        }

        PowerTable(current.into_values().collect())
    }

    #[derive(Debug, Clone)]
    struct TestPowerTables {
        current: PowerTable,
        next: PowerTable,
    }

    impl quickcheck::Arbitrary for TestPowerTables {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let v = 1 + usize::arbitrary(g) % 10;
            let c = 1 + usize::arbitrary(g) % v;
            let n = 1 + usize::arbitrary(g) % v;

            let vs = (0..v).map(|_| Validator::arbitrary(g)).collect::<Vec<_>>();
            let cvs = vs.iter().take(c).cloned().collect();
            let nvs = vs
                .into_iter()
                .skip(v - n)
                .map(|mut v| {
                    v.power = Power::arbitrary(g);
                    v
                })
                .collect();

            TestPowerTables {
                current: PowerTable(cvs),
                next: PowerTable(nvs),
            }
        }
    }

    #[quickcheck]
    fn prop_u256_to_tokens(value: u64) {
        let atto = U256::from(value);
        let tokens = u256_to_tokens(atto);
        let atto: u64 = tokens.atto().try_into().unwrap();
        assert_eq!(atto, value);
    }

    #[quickcheck]
    fn prop_power_diff_update(powers: TestPowerTables) {
        let diff = power_diff(&powers.current, powers.next.clone());
        let next = power_update(powers.current, diff);

        // Order shouldn't matter.
        let next = into_power_map(next);
        let expected = into_power_map(powers.next);

        assert_eq!(next, expected)
    }

    #[quickcheck]
    fn prop_power_diff_nochange(v1: Validator<Power>, v2: Validator<Power>) {
        let current = PowerTable(vec![v1.clone(), v2.clone()]);
        let next = PowerTable(vec![v2, v1]);
        assert!(power_diff(&current, next).0.is_empty());
    }
}
